import { test, expect } from '@playwright/test';
import { readFileSync } from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { mockFsAccess, haveFixtures } from './helpers.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIX = path.join(__dirname, 'fixtures');
const SW = readFileSync(path.join(FIX, 'switch_data001Slot.bin'));
const PC_SLOT = readFileSync(path.join(FIX, 'pc_data002Slot.bin'));
const PC_SYS = readFileSync(path.join(FIX, 'pc_data00-1.bin'));

async function pickBoth(page) {
  await page.goto('http://127.0.0.1:5173/');
  await page.getByRole('button', { name: /Next/ }).click();
  await page.getByRole('button', { name: /Choose Switch folder/ }).click();
  await expect(page.getByText(/Character 1/).first()).toBeVisible();

  await page.getByRole('button', { name: /Next/ }).click();
  await page.getByRole('button', { name: /Choose Steam userdata folder/ }).click();
  await expect(page.getByText(/auto-detected|Found account/).first()).toBeVisible({ timeout: 20000 });
}

test('pre-flight summary: cancel writes nothing, accept completes', async ({ page }) => {
  await mockFsAccess(page, {
    switchFiles: { 'data001Slot.bin': Array.from(SW) },
    pcFiles: { 'data00-1.bin': Array.from(PC_SYS), 'data001Slot.bin': Array.from(PC_SLOT) },
  });
  await pickBoth(page);

  await page.getByRole('button', { name: /Next/ }).click();
  await page.getByRole('button', { name: /Run migration/ }).click();
  const modal = page.getByRole('dialog');
  await expect(modal.getByText(/backed up first/)).toBeVisible();
  await expect(modal.getByText(/SLOT 1/)).toBeVisible();
  await expect(modal.getByText(/Steam ID/)).toBeVisible();
  await expect(modal.getByText(/Steam Cloud/)).toBeVisible();
  await page.getByRole('button', { name: /Cancel/ }).click();
  await expect(modal.getByText(/backed up first/)).toHaveCount(0);
  expect(await page.evaluate(() => Object.keys(window.__captures).length)).toBe(0);

  await page.getByRole('button', { name: /Run migration/ }).click();
  await page.getByRole('button', { name: /Start migration/ }).click();
  await expect(page.getByText('Migration complete!')).toBeVisible({ timeout: 60000 });
});

test('un-kept characters: warning shown; run removes them (kept in backup)', async ({ page }) => {
  await mockFsAccess(page, {
    switchFiles: { 'data001Slot.bin': Array.from(SW) },
    pcFiles: {
      'data00-1.bin': Array.from(PC_SYS),
      'data001Slot.bin': Array.from(PC_SLOT),
      'data002Slot.bin': Array.from(PC_SLOT),
      'data003Slot.bin': Array.from(PC_SLOT),
    },
  });
  await pickBoth(page);
  await page.getByRole('button', { name: /Next/ }).click();

  // deselect the 2nd and 3rd Steam characters in step 3 -> warning must appear
  await page.locator('li').nth(3).getByRole('button', { name: /data002Slot\.bin/ }).click();

  const warn = page.getByText(/not in your final slots/);
  await expect(warn).toBeVisible();
  await expect(warn).toContainText('Enix');

  await page.getByRole('button', { name: /Run migration/ }).click();
  await page.getByRole('button', { name: /Start migration/ }).click();
  await expect(page.getByText('Migration complete!')).toBeVisible({ timeout: 60000 });
  await expect(page.getByText(/data003Slot\.bin ← empty/)).toBeVisible();
});

test('picking win64_save directly works when Steam ID is saved', async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem('mhsave_steamid', '76561198180024509'));
  await mockFsAccess(page, {
    switchFiles: { 'data001Slot.bin': Array.from(SW) },
    pcFiles: { 'data00-1.bin': Array.from(PC_SYS), 'data001Slot.bin': Array.from(PC_SLOT) },
    pcAsWin64: true,
  });
  await page.goto('http://127.0.0.1:5173/');
  await page.getByRole('button', { name: /Next/ }).click();
  await page.getByRole('button', { name: /Choose Switch folder/ }).click();
  await expect(page.getByText(/Character 1/).first()).toBeVisible();
  await page.getByRole('button', { name: /Next/ }).click();

  await page.getByRole('button', { name: /Choose Steam userdata folder/ }).click();
  await expect(page.getByText(/Found your save folder directly/)).toBeVisible({ timeout: 20000 });
});

test('win64_save picked directly without a saved Steam ID → friendly guidance', async ({ page }) => {
  await mockFsAccess(page, {
    switchFiles: { 'data001Slot.bin': Array.from(SW) },
    pcFiles: { 'data00-1.bin': Array.from(PC_SYS), 'data001Slot.bin': Array.from(PC_SLOT) },
    pcAsWin64: true,
  });
  await page.goto('http://127.0.0.1:5173/');
  await page.getByRole('button', { name: /Next/ }).click();
  await page.getByRole('button', { name: /Choose Switch folder/ }).click();
  await expect(page.getByText(/Character 1/).first()).toBeVisible();

  await page.getByRole('button', { name: /Next/ }).click();
  await page.getByRole('button', { name: /Choose Steam userdata folder/ }).click();
  await expect(page.getByText(/folder instead/)).toBeVisible();
  // no slots should be listed (nothing usable yet)
  await expect(page.locator('li').nth(2).getByRole('button', { name: /data001Slot\.bin/ })).toHaveCount(0);
});
