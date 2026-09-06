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

test('language switch renders zh/ja and persists across reload', async ({ page }) => {
  await mockFsAccess(page, {
    switchFiles: { 'data001Slot.bin': Array.from(SW) },
    pcFiles: { 'data00-1.bin': Array.from(PC_SYS), 'data001Slot.bin': Array.from(PC_SLOT) },
  });
  await page.goto('http://127.0.0.1:5173/');
  await page.getByRole('button', { name: /Next/ }).click();

  // default EN
  await expect(page.getByText('SAVE MIGRATOR')).toBeVisible();
  await expect(page.getByText(/Take your hunt from Switch to Steam/)).toBeVisible();

  // switch to 中文
  await page.getByRole('button', { name: /中文/ }).click();
  await expect(page.getByText(/从 Switch 继续到 Steam/)).toBeVisible();
  await expect(page.getByRole('button', { name: /选择 Switch 文件夹/ })).toBeVisible();

  // switch to 日本語
  await page.getByRole('button', { name: /日本語/ }).click();
  await expect(page.getByText(/Steamへ/)).toBeVisible();
  await expect(page.getByRole('button', { name: /Switch フォルダーを選択/ })).toBeVisible();
  await expect(page.getByRole('button', { name: /次へ/ })).toBeVisible();

  // persists after reload
  await page.reload();
  await expect(page.getByText(/Steamへ/)).toBeVisible();
});

test('full flow works in 中文', async ({ page }) => {
  await mockFsAccess(page, {
    switchFiles: { 'data001Slot.bin': Array.from(SW) },
    pcFiles: { 'data00-1.bin': Array.from(PC_SYS), 'data001Slot.bin': Array.from(PC_SLOT) },
  });
  await page.goto('http://127.0.0.1:5173/');
  await page.getByRole('button', { name: /Next/ }).click();
  await page.getByRole('button', { name: /中文/ }).click();

  await page.getByRole('button', { name: /选择 Switch 文件夹/ }).click();
  await expect(page.getByText(/角色 1/).first()).toBeVisible();
  await expect(page.getByText(/角色 1/).first()).toBeVisible();

  await page.getByRole('button', { name: /下一步/ }).click();
  await page.getByRole('button', { name: /选择 Steam userdata 文件夹/ }).click();
  await expect(page.getByText(/找到账号 219758781/)).toBeVisible({ timeout: 20000 });
  await page.getByRole('button', { name: /备份/ }).click();
  await expect(page.getByText('还没有备份——每次迁移前都会自动创建。')).toBeVisible();

  await page.getByRole('button', { name: /下一步/ }).click();
  await page.getByRole('button', { name: /开始迁移/ }).click();
  await page.getByRole('button', { name: /开始迁移/ }).last().click();
  await expect(page.getByText('迁移完成！')).toBeVisible({ timeout: 60000 });
  await expect(page.getByText(/欢迎回来，猎人/)).toBeVisible();
});
