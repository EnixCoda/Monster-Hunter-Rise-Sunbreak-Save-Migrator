import { test, expect } from '@playwright/test';
import { readFileSync } from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';
import { mockFsAccess, buffersEqual, haveFixtures } from './helpers.js';

const require = createRequire(import.meta.url);
const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIX = path.join(__dirname, 'fixtures');
const SW = readFileSync(path.join(FIX, 'switch_data001Slot.bin'));
const SW_SYS = readFileSync(path.join(FIX, 'switch_data00-1.bin'));
const PC_SLOT = readFileSync(path.join(FIX, 'pc_data002Slot.bin'));
const PC_SYS = readFileSync(path.join(FIX, 'pc_data00-1.bin'));

test('reorder: kept char moved to slot 1 stays original; switch lands in slot 2', async ({ page }) => {
  await mockFsAccess(page, {
    switchFiles: { 'data001Slot.bin': Array.from(SW), 'data00-1.bin': Array.from(SW_SYS) },
    pcFiles: { 'data00-1.bin': Array.from(PC_SYS), 'data001Slot.bin': Array.from(PC_SLOT) },
  });
  await page.goto('http://127.0.0.1:5173/');
  await page.getByRole('button', { name: /Next/ }).click();

  await page.getByRole('button', { name: /Choose Switch folder/ }).click();
  await expect(page.getByText(/Enix/).first()).toBeVisible();

  await page.getByRole('button', { name: /Next/ }).click();
  await page.getByRole('button', { name: /Choose Steam userdata folder/ }).click();
  await expect(page.getByText(/auto-detected|Found account/).first()).toBeVisible({ timeout: 20000 });
  await page.getByRole('button', { name: /Next/ }).click();

  const layout = page.locator('li').nth(3);
  const kinds = () => layout.locator('div[data-kind]').evaluateAll(es => es.map(e => e.getAttribute('data-kind')));
  // default: Steam (kept) character first, Switch char after -> minimal change to the Steam save
  expect(await kinds()).toEqual(['keep', 'switch']);

  // reorder: move the kept Steam char down -> Switch char to slot 1
  await layout.getByRole('button', { name: 'Move down' }).first().click();
  expect(await kinds()).toEqual(['switch', 'keep']);

  await page.getByRole('button', { name: /Run migration/ }).click();
  await page.getByRole('button', { name: /Start migration/ }).click();
  await expect(page.getByText('Migration complete!')).toBeVisible({ timeout: 60000 });

  const captures = await page.evaluate(() => window.__captures);
  // CRITICAL: kept char must be byte-identical to the original PC character (snapshot fix),
  // now in slot 2 (data002Slot.bin) after the reorder.
  expect(buffersEqual(captures['data002Slot.bin'], new Uint8Array(PC_SLOT))).toBe(true);
  // migrated switch went to slot 1 (data001Slot.bin) with a link matching the final sys
  const slot1 = captures['data001Slot.bin'];
  expect(slot1.length).toBeGreaterThan(100000);
  const sys = captures['data00-1.bin'];
  const linkSlot = new DataView(slot1.buffer).getUint32(0x0C, true);
  const linkSys = new DataView(sys.buffer).getUint32(0x0C, true);
  expect(linkSlot).toBe(linkSys);

  // cross-slot sync: switch char is in slot 1 here (target entry 0) while its source
  // is Switch entry 0 — written sys must match the same pipeline run in Node.
  const expectedSig = (() => {
    const nodeWasm = require('../../wasm/pkgnode/mhsave_wasm.js');
    const id = 76561198180024509n;
    const mig = nodeWasm.migrate(new Uint8Array(SW), new Uint8Array(PC_SLOT), new Uint8Array(PC_SYS), id, 107);
    const synced = nodeWasm.sync_hunter_entry(new Uint8Array(mig.sys), new Uint8Array(SW_SYS), id, 107, 0, 0);
    return nodeWasm.content_sig(new Uint8Array(synced.sys), id);
  })();
  const writtenSig = await page.evaluate(async () => {
    const mod = await import('/src/wasm/mhsave_wasm.js');
    await mod.default();
    return mod.content_sig(window.__captures['data00-1.bin'], BigInt('76561198180024509'));
  });
  expect(writtenSig).toBe(expectedSig);
});
