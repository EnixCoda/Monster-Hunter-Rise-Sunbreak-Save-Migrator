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

// Both accounts have a save folder. Only the first account's save is keyed to its
// own Steam ID; the second one is keyed to another account (the mismatch case).
const MULTI = {
  '219758781': { files: { 'data00-1.bin': Array.from(PC_SYS), 'data001Slot.bin': Array.from(PC_SLOT) } },
  '999999999': { files: { 'data00-1.bin': Array.from(PC_SYS), 'data001Slot.bin': Array.from(PC_SLOT) } },
};

async function openWithSwitchSlot(page) {
  await page.goto('http://127.0.0.1:5173/');
  await page.getByRole('button', { name: /Choose Switch folder/ }).click();
  await expect(page.getByText(/Character 1/).first()).toBeVisible();
}

test('multiple accounts: chooser lists both; choose correct one → migrate writes only there', async ({ page }) => {
  await mockFsAccess(page, { switchFiles: { 'data001Slot.bin': Array.from(SW) }, multiAccounts: MULTI });
  await openWithSwitchSlot(page);

  await page.getByRole('button', { name: /Next/ }).click();
  await page.getByRole('button', { name: /Choose Steam userdata folder/ }).click();

  // ambiguous -> chooser lists both accounts, no slots yet
  await expect(page.getByText(/Steam account\(s\)/).first()).toBeVisible();
  await expect(page.getByText(/219758781/).first()).toBeVisible();
  await expect(page.getByText(/999999999/).first()).toBeVisible();
  await expect(page.locator('li').nth(1).getByRole('button', { name: /data001Slot\.bin/ })).toHaveCount(0);

  await page.getByRole('button', { name: /219758781/ }).click();
  await expect(page.getByText(/Found account 219758781/)).toBeVisible({ timeout: 20000 });
  await page.getByRole('button', { name: /Next/ }).click();
  await expect(page.locator('li').nth(2).getByRole('button', { name: /data001Slot\.bin/ }).first()).toBeVisible();

  await page.getByRole('button', { name: /Run migration/ }).click();
  await page.getByRole('button', { name: /Start migration/ }).click();
  await expect(page.getByText('Migration complete!')).toBeVisible({ timeout: 60000 });

  const caps = await page.evaluate(() => window.__captures);
  expect(caps['m219758781/data001Slot.bin'] && caps['m219758781/data001Slot.bin'].length > 100000).toBe(true);
  expect(Object.keys(caps).some(k => k.startsWith('m999999999/'))).toBe(false); // other account untouched
});

test('multiple accounts: wrong account → safety error, nothing written', async ({ page }) => {
  await mockFsAccess(page, { switchFiles: { 'data001Slot.bin': Array.from(SW) }, multiAccounts: MULTI });
  await openWithSwitchSlot(page);
  await page.getByRole('button', { name: /Next/ }).click();
  await page.getByRole('button', { name: /Choose Steam userdata folder/ }).click();
  await expect(page.getByText(/Steam account\(s\)/).first()).toBeVisible();

  await page.getByRole('button', { name: /999999999/ }).click();
  await expect(page.getByText(/Found account 999999999/)).toBeVisible({ timeout: 20000 });

  await page.getByRole('button', { name: /Next/ }).click();
  await page.getByRole('button', { name: /Run migration/ }).click();
  await page.getByRole('button', { name: /Start migration/ }).click();
  await expect(page.getByText(/belong to a different Steam account/)).toBeVisible({ timeout: 60000 });
  await expect(page.getByText(/Nothing was written/)).toBeVisible();

  const caps = await page.evaluate(() => window.__captures);
  expect(Object.keys(caps).some(k => k.startsWith('m999999999/'))).toBe(false); // nothing written into that account
});

test('remembered account auto-selected (no chooser)', async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem('mhsave_accountid', '219758781'));
  await mockFsAccess(page, { switchFiles: { 'data001Slot.bin': Array.from(SW) }, multiAccounts: MULTI });
  await openWithSwitchSlot(page);
  await page.getByRole('button', { name: /Next/ }).click();
  await page.getByRole('button', { name: /Choose Steam userdata folder/ }).click();

  await expect(page.getByText(/Found account 219758781/)).toBeVisible({ timeout: 20000 });
  await expect(page.getByText(/Steam account\(s\)/)).toHaveCount(0);
  await page.getByRole('button', { name: /Next/ }).click();
  await expect(page.locator('li').nth(2).getByRole('button', { name: /data001Slot\.bin/ }).first()).toBeVisible();
});
