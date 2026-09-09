// Cross-slot mapping: a Switch character that lives in a non-first Switch slot
// (data002Slot.bin -> Switch sys hunter entry 1) must sync ITS OWN sys entry into the
// target entry it lands on (Steam slot 2 -> hunter entry 1). This proves the source
// index is parsed from the file name and the target index follows the layout.
import { test, expect } from '@playwright/test';
import { readFileSync } from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';
import { mockFsAccess } from './helpers.js';

const require = createRequire(import.meta.url);
const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIX = path.join(__dirname, 'fixtures');
const SW = readFileSync(path.join(FIX, 'switch_data001Slot.bin'));
const SW_SYS = readFileSync(path.join(FIX, 'switch_data00-1_multi.bin'));
const PC_SLOT = readFileSync(path.join(FIX, 'pc_data002Slot.bin'));
const PC_SYS = readFileSync(path.join(FIX, 'pc_data00-1.bin'));
const ID = 76561198180024509n;

test('slot mapping: switch slot 2 syncs its own sys entry into its target slot', async ({ page }) => {
  await mockFsAccess(page, {
    switchFiles: { 'data002Slot.bin': Array.from(SW), 'data00-1.bin': Array.from(SW_SYS) },
    pcFiles: { 'data00-1.bin': Array.from(PC_SYS), 'data001Slot.bin': Array.from(PC_SLOT) },
  });

  await page.goto('http://127.0.0.1:5173/');
  await page.getByRole('button', { name: /Next/ }).click();
  await page.getByRole('button', { name: /Choose Switch folder/ }).click();
  // the Switch sys fixture names entry 1 "AltCha" — the UI must show it for data002Slot.bin
  await expect(page.getByText(/AltCha/).first()).toBeVisible();
  await page.getByRole('button', { name: /Next/ }).click();
  await page.getByRole('button', { name: /Choose Steam userdata folder/ }).click();
  await expect(page.getByText(/auto-detected|Found account/).first()).toBeVisible({ timeout: 20000 });
  await page.getByRole('button', { name: /Next/ }).click();
  await page.getByRole('button', { name: /Run migration/ }).click();
  await page.getByRole('button', { name: /Start migration/ }).click();
  await expect(page.getByText('Migration complete!')).toBeVisible({ timeout: 60000 });

  // reference: same pipeline in Node with target entry 1 <- source entry 1
  const expected = (() => {
    const nodeWasm = require('../../wasm/pkgnode/mhsave_wasm.js');
    const mig = nodeWasm.migrate(new Uint8Array(SW), new Uint8Array(PC_SLOT), new Uint8Array(PC_SYS), ID, 107);
    const synced = nodeWasm.sync_hunter_entry(new Uint8Array(mig.sys), new Uint8Array(SW_SYS), ID, 107, 1, 1);
    return {
      sig: nodeWasm.content_sig(new Uint8Array(synced.sys), ID),
      names: nodeWasm.hunter_names(new Uint8Array(synced.sys), ID),
    };
  })();

  const got = await page.evaluate(async () => {
    const mod = await import('/src/wasm/mhsave_wasm.js');
    await mod.default();
    const sys = window.__captures['data00-1.bin'];
    return { sig: mod.content_sig(sys, BigInt('76561198180024509')), names: mod.hunter_names(sys, BigInt('76561198180024509')) };
  });

  expect(got.sig).toBe(expected.sig);
  expect(got.names[1]).toBe('AltCha');       // source entry 1, not entry 0 ("Enix")
  expect(got.names[0]).not.toBe('Enix');      // target entry 0 untouched (kept Steam char)
});
