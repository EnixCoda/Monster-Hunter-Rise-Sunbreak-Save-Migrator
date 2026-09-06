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

test('cancel picker shows friendly state and writes nothing', async ({ page }) => {
  await mockFsAccess(page, {
    switchFiles: { 'data001Slot.bin': Array.from(SW) },
    pcFiles: { 'data00-1.bin': Array.from(PC_SYS), 'data001Slot.bin': Array.from(PC_SLOT) },
    cancelOn: 1,
  });
  await page.goto('http://127.0.0.1:5173/');
  await page.getByRole('button', { name: /Choose Switch folder/ }).click();
  await expect(page.getByText(/Picking cancelled/)).toBeVisible();
  const wrote = await page.evaluate(() => Object.keys(window.__captures).length);
  expect(wrote).toBe(0);
});

test('more than 3 characters: hard guard, nothing written', async ({ page }) => {
  await mockFsAccess(page, {
    switchFiles: { 'data001Slot.bin': Array.from(SW), 'data002Slot.bin': Array.from(SW) },
    pcFiles: { 'data00-1.bin': Array.from(PC_SYS), 'data001Slot.bin': Array.from(PC_SLOT), 'data002Slot.bin': Array.from(PC_SLOT) },
  });
  await page.goto('http://127.0.0.1:5173/');
  await page.getByRole('button', { name: /Choose Switch folder/ }).click();
  await expect(page.getByText(/Character 1/).first()).toBeVisible();

  await page.getByRole('button', { name: /Next/ }).click();
  await page.getByRole('button', { name: /Choose Steam userdata folder/ }).click();
  await expect(page.getByText(/auto-detected|Found account/).first()).toBeVisible({ timeout: 20000 });

  await page.getByRole('button', { name: /Next/ }).click();
  await page.getByRole('button', { name: /Run migration/ }).click();
  await expect(page.getByText(/only 3 slots allowed/)).toBeVisible();
  const wrote = await page.evaluate(() => Object.keys(window.__captures).length);
  expect(wrote).toBe(0);
});

test('corrupt switch slot: flagged unreadable, not auto-selected', async ({ page }) => {
  await mockFsAccess(page, {
    switchFiles: { 'data001Slot.bin': Array.from(SW), 'data003Slot.bin': Array.from(Buffer.alloc(512, 7)) },
    pcFiles: { 'data00-1.bin': Array.from(PC_SYS), 'data001Slot.bin': Array.from(PC_SLOT) },
  });
  await page.goto('http://127.0.0.1:5173/');
  await page.getByRole('button', { name: /Choose Switch folder/ }).click();

  const switchList = page.locator('li').nth(0);
  await expect(switchList).toContainText("can't read");   // corrupt slot flagged
  await expect(switchList).toContainText('· OK');            // valid slot ok
  await expect(switchList).toContainText('Character 1');     // unnamed fallback label, no file names
  // slots area stays hidden until both folders are picked
  await expect(page.locator('li').nth(2)).toContainText('Pick your Switch and Steam folders above to see your final slots.');
});

test('no FS Access API -> guidance pill', async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem('mhsave_intro_done', '1');
    Object.defineProperty(window, 'showDirectoryPicker', { value: undefined, configurable: true });
  });
  await page.goto('http://127.0.0.1:5173/');
  // support banner appears immediately
  await expect(page.getByText(/File System Access API/)).toBeVisible();
  await page.getByRole('button', { name: /Choose Switch folder/ }).click();
  await expect(page.getByText(/Need Chrome\/Edge/)).toBeVisible();
});
