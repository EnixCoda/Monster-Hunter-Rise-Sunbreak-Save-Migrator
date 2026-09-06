import { test, expect } from '@playwright/test';
import { readFileSync } from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { mockFsAccess, buffersEqual, haveFixtures } from './helpers.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIX = path.join(__dirname, 'fixtures');
const SW = readFileSync(path.join(FIX, 'switch_data001Slot.bin'));
const PC_SLOT = readFileSync(path.join(FIX, 'pc_data002Slot.bin'));
const PC_SYS = readFileSync(path.join(FIX, 'pc_data00-1.bin'));
const PRE = 'backup_2026-09-04T00-00-00';

test('backups: list pre-existing, auto-create full snapshot, restore, delete', async ({ page }) => {
  test.setTimeout(240000);
  await mockFsAccess(page, {
    switchFiles: { 'data001Slot.bin': Array.from(SW) },
    pcFiles: { 'data00-1.bin': Array.from(PC_SYS), 'data001Slot.bin': Array.from(PC_SLOT) },
    preBackups: { [PRE]: { 'data00-1.bin': Array.from(PC_SYS), 'data001Slot.bin': Array.from(PC_SLOT) } },
  });
  page.on('dialog', d => d.accept());
  await page.goto('http://127.0.0.1:5173/');

  await page.getByRole('button', { name: /Choose Switch folder/ }).click();
  await expect(page.getByText(/Character 1/).first()).toBeVisible();

  await page.getByRole('button', { name: /Next/ }).click();
  await page.getByRole('button', { name: /Choose Steam userdata folder/ }).click();

  // pre-existing backup is listed right after picking userdata
  await expect(page.getByText(/Found account/).first()).toBeVisible({ timeout: 20000 });
  await expect(page.getByText(/backup_2026-09-04T00-00-00/).first()).toBeVisible();

  // run migration -> a full backup of the ORIGINAL save is captured first

  await page.getByRole('button', { name: /Next/ }).click();
  await page.getByRole('button', { name: /Run migration/ }).click();
  await page.getByRole('button', { name: /Start migration/ }).click();
  await expect(page.getByText('Migration complete!')).toBeVisible({ timeout: 60000 });
  const bakKeys = await page.evaluate(() => Object.keys(window.__captures).filter(k => /^backup_[^/]+\//.test(k)));
  expect(bakKeys.length).toBeGreaterThanOrEqual(2); // data00-1.bin + data001Slot.bin originals
  const bakSlot = await page.evaluate(() => {
    const k = Object.keys(window.__captures).find(k => /^backup_[^/]+\/data001Slot\.bin$/.test(k));
    return window.__captures[k];
  });
  expect(buffersEqual(bakSlot, new Uint8Array(PC_SLOT))).toBe(true); // backup holds the ORIGINAL char

  // restore the pre-existing backup -> current save replaced + pre-restore snapshot created
  await expect(page.locator('div.flex.items-center').filter({ hasText: PRE })).toHaveCount(1);
  await page.locator('div.flex.items-center').filter({ hasText: PRE }).first().scrollIntoViewIfNeeded();
  // (newest-first order: PRE is the last row)
  await page.evaluate((pre) => {
    const row = Array.from(document.querySelectorAll('div.flex.items-center')).find(d => (d.textContent||'').includes(pre));
    const b = row && Array.from(row.querySelectorAll('button')).find(x => /Restore/.test(x.textContent||''));
    if (b) b.click();
  }, PRE);
  // Wait for the restore to actually finish (pre-restore snapshot is written on complete),
  // then assert the deterministic bytes the restore wrote back (feedback pill is transient).
  await expect.poll(() => page.evaluate(() => Object.keys(window.__captures).some(k => /_prerestore\//.test(k))), { timeout: 20000 }).toBe(true);
  const afterRestore = await page.evaluate(() => ({
    slot: window.__captures['data001Slot.bin'],
    sys: window.__captures['data00-1.bin'],
    preKeys: Object.keys(window.__captures).filter(k => /_prerestore\//.test(k)),
  }));
  expect(buffersEqual(afterRestore.slot, new Uint8Array(PC_SLOT))).toBe(true);
  expect(buffersEqual(afterRestore.sys, new Uint8Array(PC_SYS))).toBe(true);
  expect(afterRestore.preKeys.length).toBeGreaterThanOrEqual(2); // pre-restore snapshot captured

  // delete the pre-existing backup
  await page.evaluate((pre) => {
    const row = Array.from(document.querySelectorAll('div.flex.items-center')).find(d => (d.textContent||'').includes(pre));
    const b = row && Array.from(row.querySelectorAll('button')).find(x => /Delete/.test(x.textContent||''));
    if (b) b.click();
  }, PRE);
  await expect(page.locator('div.flex.items-center').filter({ hasText: PRE })).toHaveCount(0);
});
