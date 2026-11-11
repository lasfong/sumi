// Automation script for Phase 0b Audit Walkthrough
import { createRequire } from 'node:module';
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';

const require = createRequire('/Users/mizuhara/workspace/sumi/frontend/package.json');
const { chromium } = require('playwright');

delete process.env.HTTP_PROXY;
delete process.env.HTTPS_PROXY;
delete process.env.http_proxy;
delete process.env.https_proxy;
process.env.NO_PROXY = '*';
process.env.no_proxy = '*';

const ROOT = '/Users/mizuhara/workspace/sumi';
const OUT_DIR = path.join(ROOT, 'test-results', 'p0b');
const SHOTS_DIR = path.join(OUT_DIR, 'screenshots');
await mkdir(SHOTS_DIR, { recursive: true });

const FRONTEND_URL = process.env.SUMI_FRONTEND_URL || 'http://127.0.0.1:15300';
const BACKEND_URL = process.env.SUMI_BACKEND_URL || 'http://127.0.0.1:18200';

console.log(`Connecting to Frontend: ${FRONTEND_URL}, Backend: ${BACKEND_URL}`);

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
const page = await context.newPage();
page.setDefaultTimeout(25000);
page.on('dialog', async d => await d.accept());

const results = [];
let currentTestId = 'INIT';

const logBucket = {};
const getBucket = (id) => (logBucket[id] ||= { consoleErrors: [], pageErrors: [], badHttp: [], notes: [] });

page.on('console', m => {
  if (m.type() === 'error') {
    getBucket(currentTestId).consoleErrors.push(m.text().slice(0, 300));
  }
});
page.on('pageerror', e => getBucket(currentTestId).pageErrors.push(String(e.message).slice(0, 300)));
page.on('response', r => {
  if (r.url().includes('/api/') && r.status() >= 400) {
    getBucket(currentTestId).badHttp.push(`${r.status()} ${r.request().method()} ${r.url().replace(FRONTEND_URL, '')}`);
  }
});

const settle = async (ms = 1200) => {
  try { await page.waitForLoadState('networkidle', { timeout: 12000 }); } catch {}
  await page.waitForTimeout(ms);
};

const takeShot = async (name) => {
  const p = path.join(SHOTS_DIR, `${name}.png`);
  await page.screenshot({ path: p, fullPage: false });
  return p;
};

async function runTest(id, name, testFn) {
  currentTestId = id;
  const b = getBucket(id);
  console.log(`\n▶ [${id}] ${name}...`);
  let status = 'PASS';
  let defect = '';
  try {
    await testFn(b);
  } catch (err) {
    status = 'FAIL';
    defect = err.message.split('\n')[0];
    b.notes.push(`EXCEPTION: ${defect}`);
    console.error(`  ❌ Failed: ${defect}`);
  }
  const shotPath = await takeShot(id);
  const entry = {
    id,
    name,
    status,
    defect,
    consoleCount: b.consoleErrors.length,
    badHttpCount: b.badHttp.length,
    badHttpSample: b.badHttp.slice(0, 3),
    notes: b.notes,
    screenshot: path.relative(ROOT, shotPath)
  };
  results.push(entry);
  console.log(`  ✔ Status: ${status} | Consoles: ${entry.consoleCount} | HTTP Errors: ${entry.badHttpCount}`);
}

// ---------------- EXECUTION ----------------
try {
  // GL-01 Menu & Settings
  await runTest('GL-01', 'Menu phẳng và nút Settings', async (b) => {
    await page.goto(`${FRONTEND_URL}/`);
    await settle();
    const beforeUrl = page.url();
    await page.locator('.config-btn').click();
    await page.waitForTimeout(500);
    const afterUrl = page.url();
    if (beforeUrl === afterUrl) {
      b.notes.push('Nút Settings không đổi URL và không hiển thị modal nào (Dead button).');
    }
  });

  // DF-01 CafeF Raw Upload
  await runTest('DF-01', 'Upload file CafeF gốc ngược thời gian', async (b) => {
    await page.goto(`${FRONTEND_URL}/import`);
    await settle();
    // Chuyển sang tab CafeF
    const tabCafeF = page.getByText('Nhập tập tin CafeF', { exact: false });
    if (await tabCafeF.isVisible()) await tabCafeF.click();
    await settle(500);

    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles(path.join(OUT_DIR, 'data', 'raw_HSX_subset.csv'));
    await settle(2500);

    const text = await page.locator('main').innerText();
    if (text.includes('ngược thứ tự thời gian') || text.includes('không hợp lệ')) {
      b.notes.push('Hệ thống từ chối nạp file do ngược thời gian. Cần công cụ tiền xử lý hoặc backend tự sort.');
    }
  });

  // RP-01 Tạo phiên Replay
  let sessionId = null;
  await runTest('RP-01', 'Tạo phiên Replay FPT', async (b) => {
    await page.goto(`${FRONTEND_URL}/replay`);
    await settle();
    
    // Kiểm tra xem có cần tạo phiên mới không
    const symbolInput = page.locator('input[placeholder*="Search symbol"]').first();
    if (await symbolInput.isVisible()) {
      await symbolInput.fill('FPT');
      const startInput = page.locator('input[type="date"]').nth(0);
      const endInput = page.locator('input[type="date"]').nth(1);
      await startInput.fill('2023-01-02');
      await endInput.fill('2024-12-27');
      await page.getByRole('button', { name: /Bắt đầu|Start|Tạo phiên/i }).first().click();
      await settle(3000);
    }
    const currentUrl = page.url();
    b.notes.push(`Replay URL: ${currentUrl}`);
    const match = currentUrl.match(/session=(\d+)/);
    if (match) sessionId = match[1];
  });

  // RP-02 Tua nến
  await runTest('RP-02', 'Tua nến và Auto-play', async (b) => {
    for (let i = 0; i < 10; i++) {
      await page.keyboard.press('ArrowRight');
      await page.waitForTimeout(60);
    }
    await settle(1000);
    const toasts = await page.locator('.toaster, [role="status"]').allInnerTexts();
    if (toasts.some(t => t.includes('End of data') || t.includes('error'))) {
      b.notes.push(`Gặp toast thông báo: ${toasts.join('; ')}`);
    }
  });

  // RP-05 Đặt lệnh & T+2 & Lô 100
  await runTest('RP-05', 'Đặt lệnh Mua và kiểm tra T+2', async (b) => {
    const buyBtn = page.getByRole('button', { name: 'BUY', exact: true });
    if (await buyBtn.isVisible()) {
      await buyBtn.click();
      await settle(600);
      // Kiểm tra dialog
      const dialog = page.getByTestId('trade-decision-dialog');
      if (await dialog.isVisible()) {
        const qtyInput = dialog.locator('input[aria-label="Quantity"]');
        await qtyInput.fill('100');
        const submitBtn = dialog.getByRole('button', { name: /Submit BUY/i });
        await submitBtn.click();
        await settle(1500);
        b.notes.push('Đã gửi lệnh Mua 100 cổ phiếu.');
      }
    }
    // Kiểm tra nút SELL có bị disabled ngay không (luật T+2)
    const sellBtn = page.getByRole('button', { name: 'SELL', exact: true });
    const isSellDisabled = await sellBtn.isDisabled();
    b.notes.push(`Nút SELL bị vô hiệu hóa khi cổ phiếu chưa về (T+2): ${isSellDisabled}`);
  });

  // SC-01 Signal Scanner
  await runTest('SC-01', 'Chạy bộ quét Signal Scanner', async (b) => {
    await page.goto(`${FRONTEND_URL}/scanner`);
    await settle();
    const symbolsInput = page.locator('input').first();
    if (await symbolsInput.isVisible()) {
      await symbolsInput.fill('FPT, SSI');
    }
    const dates = page.locator('input[type="date"]');
    if (await dates.count() >= 2) {
      await dates.nth(0).fill('2023-01-02');
      await dates.nth(1).fill('2023-12-29');
    }
    const runBtn = page.getByRole('button', { name: /Quét Tín Hiệu|Run Scan/i }).first();
    if (await runBtn.isVisible()) {
      await runBtn.click();
      await settle(4000);
    }
  });

  // ST-01 Strategy Tester - Battle
  await runTest('ST-01', 'Strategy Tester - Đối đầu 3 chiến lược', async (b) => {
    await page.goto(`${FRONTEND_URL}/strategy-lab`);
    await settle();
    const battleBtn = page.getByText('Chọn 3 Chiến Lược Đối Đầu', { exact: false });
    if (await battleBtn.isVisible()) await battleBtn.click();
    
    // Điền ngày hợp lệ 2023
    const dates = page.locator('form input[type="date"]');
    if (await dates.count() >= 2) {
      await dates.nth(0).fill('2023-01-02');
      await dates.nth(1).fill('2023-12-29');
    }
    const submitBtn = page.locator('form button[type="submit"]').first();
    await submitBtn.click();
    await page.waitForTimeout(5000);
    await settle();
  });

  // ST-02 Multi-Phase Batch
  await runTest('ST-02', 'Multi-Phase Batch - Lỗi chặn Submit Form', async (b) => {
    await page.goto(`${FRONTEND_URL}/strategy-lab`);
    await settle();
    await page.getByTestId('lab-tab-multiphase').click();
    await settle(800);
    
    const runBatchBtn = page.getByTestId('run-batch-btn');
    await runBatchBtn.click();
    await page.waitForTimeout(1000);
    
    // Check validation message on capital input
    const invalidInput = page.locator('input:invalid');
    if (await invalidInput.count() > 0) {
      b.notes.push('Phát hiện ô nhập vốn vi phạm step/min HTML5 khiến form không submit được.');
    }
  });

  // ST-04 Rule Builder
  await runTest('ST-04', 'Strategy Rule Builder - Ngõ cụt AST', async (b) => {
    await page.goto(`${FRONTEND_URL}/strategy-lab`);
    await settle();
    await page.getByTestId('lab-tab-builder').click();
    await settle(800);
    const copyBtn = page.locator('button:has-text("Sao Chép Cấu Hình YAML")');
    const isCopyVisible = await copyBtn.isVisible();
    const hasBacktestBtn = await page.locator('button:has-text("Chạy Backtest")').isVisible();
    b.notes.push(`Nút Copy YAML có mặt: ${isCopyVisible}, Có nút Chạy Backtest ngay: ${hasBacktestBtn}`);
  });

  // ST-05 Money Flow BB Crash
  await runTest('ST-05', 'Technical Flow BB - Chẩn đoán crash toFixed', async (b) => {
    await page.goto(`${FRONTEND_URL}/strategy-lab`);
    await settle();
    await page.getByTestId('lab-tab-flow').click();
    await settle(2500);
    const errText = await page.locator('.glass-panel:has-text("Something went wrong")').innerText().catch(() => '');
    if (errText.includes('toFixed')) {
      b.notes.push('Xác nhận crash màn hình: TypeError reading toFixed do lệch contract API.');
    }
  });

} finally {
  await writeFile(path.join(OUT_DIR, 'audit_run_results.json'), JSON.stringify(results, null, 2));
  await browser.close();
  console.log(`\nHoàn thành bộ test tự động. Kết quả lưu tại: ${path.join(OUT_DIR, 'audit_run_results.json')}`);
}
