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

test('reorder: kept char moved to slot 1 stays original; switch lands in slot 2', async ({ page }) => {
  await mockFsAccess(page, {
    switchFiles: { 'data001Slot.bin': Array.from(SW) },
    pcFiles: { 'data00-1.bin': Array.from(PC_SYS), 'data001Slot.bin': Array.from(PC_SLOT) },
  });
  await page.goto('http://127.0.0.1:5173/');

  await page.getByRole('button', { name: /Choose Switch folder/ }).click();
  await expect(page.getByText(/Character 1/).first()).toBeVisible();

  await page.getByRole('button', { name: /Next/ }).click();
  await page.getByRole('button', { name: /Choose Steam userdata folder/ }).click();
  await expect(page.getByText(/auto-detected|Found account/).first()).toBeVisible({ timeout: 20000 });
  await page.getByRole('button', { name: /Next/ }).click();

  const layout = page.locator('li').nth(2);
  const kinds = () => layout.locator('div[data-kind]').evaluateAll(es => es.map(e => e.getAttribute('data-kind')));
  // default: switch first, kept after
  expect(await kinds()).toEqual(['switch', 'keep']);

  // move switch (slot 1) down -> kept char becomes slot 1
  await layout.getByRole('button', { name: 'Move down' }).first().click();
  expect(await kinds()).toEqual(['keep', 'switch']);

  await page.getByRole('button', { name: /Run migration/ }).click();
  await page.getByRole('button', { name: /Start migration/ }).click();
  await expect(page.getByText('Migration complete!')).toBeVisible({ timeout: 60000 });

  const captures = await page.evaluate(() => window.__captures);
  // CRITICAL: kept char must be byte-identical to the original PC character (snapshot fix)
  expect(buffersEqual(captures['data001Slot.bin'], new Uint8Array(PC_SLOT))).toBe(true);
  // migrated switch went to slot 2 with a link matching the final sys
  const slot2 = captures['data002Slot.bin'];
  expect(slot2.length).toBeGreaterThan(100000);
  const sys = captures['data00-1.bin'];
  const linkSlot = new DataView(slot2.buffer).getUint32(0x0C, true);
  const linkSys = new DataView(sys.buffer).getUint32(0x0C, true);
  expect(linkSlot).toBe(linkSys);
});
