import { test, expect } from '@playwright/test';
import { readFileSync } from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';
import { mockFsAccess, haveFixtures } from './helpers.js';

const require = createRequire(import.meta.url);

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIX = path.join(__dirname, 'fixtures');
const EXPECTED_LINK = 707; // from pc fixture data00-1
const SW = readFileSync(path.join(FIX, 'switch_data001Slot.bin'));
const SW_SYS = readFileSync(path.join(FIX, 'switch_data00-1.bin'));
const PC_SLOT = readFileSync(path.join(FIX, 'pc_data002Slot.bin'));
const PC_SYS = readFileSync(path.join(FIX, 'pc_data00-1.bin'));

test('full migration flow (mock FS Access)', async ({ page }) => {
  await mockFsAccess(page, {
    switchFiles: { 'data001Slot.bin': Array.from(SW), 'data00-1.bin': Array.from(SW_SYS) },
    pcFiles: { 'data00-1.bin': Array.from(PC_SYS), 'data001Slot.bin': Array.from(PC_SLOT) },
  });

  await page.goto('http://127.0.0.1:5173/');
  await page.getByRole('button', { name: /Next/ }).click();
  await expect(page.getByText('SAVE MIGRATOR')).toBeVisible();

  // Step 1: choose Switch folder -> valid slots auto-selected -> listed
  await page.getByRole('button', { name: /Choose Switch folder/ }).click();
  await expect(page.getByText(/Enix/).first()).toBeVisible();
  await expect(page.getByText(/· OK/).first()).toBeVisible();

  // Step 2: choose Steam userdata -> auto ID + slots + backups panel

  await page.getByRole('button', { name: /Next/ }).click();
  await page.getByRole('button', { name: /Choose Steam userdata folder/ }).click();
  await expect(page.getByText(/auto-detected|Found account/).first()).toBeVisible({ timeout: 20000 });
  await page.getByRole('button', { name: /Backups/ }).click();
  await expect(page.getByText("No backups yet — they're created automatically before every migration.")).toBeVisible();

  // Step 3: run

  await page.getByRole('button', { name: /Next/ }).click();
  await page.getByRole('button', { name: /Run migration/ }).click();
  await page.getByRole('button', { name: /Start migration/ }).click();
  await expect(page.getByText('Migration complete!')).toBeVisible({ timeout: 60000 });
  await expect(page.getByText(/backup_/).first()).toBeVisible({ timeout: 10000 });

  // verify: captured written slot matches expected (0-diff vs switch applying pc template)
  const written = await page.evaluate(() => window.__captures['data001Slot.bin']);
  expect(written.length).toBeGreaterThan(100000);
  const link = new DataView(written.buffer).getUint32(0x0C, true);
  expect(link).toBe(707);

  // verify: the written sys was synced with the Switch title-screen entry.
  // The Switch character lands in slot 2 (Steam char keeps slot 1), so hunter entry 1
  // must now match the Switch sys entry 0 — compare against the same pipeline run in Node.
  const expectedSig = (() => {
    const nodeWasm = require('../../wasm/pkgnode/mhsave_wasm.js');
    const id = 76561198180024509n;
    const mig = nodeWasm.migrate(new Uint8Array(SW), new Uint8Array(PC_SLOT), new Uint8Array(PC_SYS), id, 107);
    const synced = nodeWasm.sync_hunter_entry(new Uint8Array(mig.sys), new Uint8Array(SW_SYS), id, 107, 1, 0);
    return nodeWasm.content_sig(new Uint8Array(synced.sys), id);
  })();
  const writtenSig = await page.evaluate(async () => {
    const mod = await import('/src/wasm/mhsave_wasm.js');
    await mod.default();
    return mod.content_sig(window.__captures['data00-1.bin'], BigInt('76561198180024509'));
  });
  expect(writtenSig).toBe(expectedSig);
});
