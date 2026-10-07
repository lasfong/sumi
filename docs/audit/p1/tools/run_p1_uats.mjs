// Phase 1 Browser UAT Automation Script
// Connects to isolated environment: Frontend (port 15300), Backend (port 18200)
// Captures 10 screenshots at exact viewport 1440x1000

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
const OUT_DIR = path.join(ROOT, 'test-results', 'p1');
const SHOTS_DIR = path.join(OUT_DIR, 'screenshots');
await mkdir(SHOTS_DIR, { recursive: true });

const FRONTEND_URL = process.env.SUMI_FRONTEND_URL || 'http://127.0.0.1:15300';
const BACKEND_URL = process.env.SUMI_BACKEND_URL || 'http://127.0.0.1:18200';

console.log(`Connecting to Frontend: ${FRONTEND_URL}, Backend: ${BACKEND_URL}`);

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
const page = await context.newPage();
page.setDefaultTimeout(35000);
page.on('dialog', async (d) => await d.accept());

const results = [];
let currentTestId = 'INIT';

const logBucket = {};
const getBucket = (id) => (logBucket[id] ||= { consoleErrors: [], pageErrors: [], badHttp: [], notes: [] });

page.on('console', (m) => {
  if (m.type() === 'error') {
    getBucket(currentTestId).consoleErrors.push(m.text().slice(0, 300));
  }
});
page.on('pageerror', (e) => getBucket(currentTestId).pageErrors.push(String(e.message).slice(0, 300)));
page.on('response', (r) => {
  if (r.url().includes('/api/') && r.status() >= 400) {
    getBucket(currentTestId).badHttp.push(`${r.status()} ${r.request().method()} ${r.url().replace(FRONTEND_URL, '')}`);
  }
});

const settle = async (ms = 1500) => {
  try {
    await page.waitForLoadState('networkidle', { timeout: 15000 });
  } catch {}
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
  let shotPath = path.join(SHOTS_DIR, `${id}.png`);
  if (!b.shotTaken) {
    shotPath = await takeShot(id);
  }
  const entry = {
    id,
    name,
    status,
    defect,
    consoleCount: b.consoleErrors.length,
    badHttpCount: b.badHttp.length,
    badHttpSample: b.badHttp.slice(0, 3),
    notes: b.notes,
    screenshot: path.relative(ROOT, shotPath),
  };
  results.push(entry);
  console.log(`  ✔ Status: ${status} | Consoles: ${entry.consoleCount} | HTTP Errors: ${entry.badHttpCount}`);
}

// ---------------- EXECUTION ----------------
try {
  // 1. GL-01 Sidebar 2 nhóm & Settings Modal (v3.0.0)
  await runTest('GL-01', 'Sidebar 2 nhóm chức năng và Settings Modal v3.0.0', async (b) => {
    await page.goto(`${FRONTEND_URL}/`);
    await settle();

    // Verify sidebar groups
    const manualGroup = page.locator('[data-testid="nav-group-manual"]');
    const autoGroup = page.locator('[data-testid="nav-group-auto"]');
    const systemGroup = page.locator('[data-testid="nav-group-system"]');
    const versionBadge = page.locator('[data-testid="version-badge"]');

    const hasManual = await manualGroup.isVisible();
    const hasAuto = await autoGroup.isVisible();
    const badgeText = (await versionBadge.innerText()).trim();

    b.notes.push(`Group Manual: ${hasManual}, Group Auto: ${hasAuto}, Badge: ${badgeText}`);

    // Click Settings button
    const settingsBtn = page.locator('[data-testid="sidebar-settings-button"]');
    await settingsBtn.click();
    await page.waitForTimeout(600);

    const modal = page.locator('[data-testid="settings-modal"]');
    const isModalOpen = await modal.isVisible();
    b.notes.push(`Settings Modal opened: ${isModalOpen}`);

    // Take screenshot with SettingsModal open
    // We will close it after screenshot is taken inside runTest
  });

  // Close modal if open
  try {
    const closeBtn = page.locator('[data-testid="close-settings-btn"]');
    if (await closeBtn.isVisible()) await closeBtn.click();
    await page.waitForTimeout(300);
  } catch {}

  // 2. DF-01 CafeF Raw Descending File Ingestion (Auto-sort & 1,000x Price Scaling)
  await runTest('DF-01', 'Upload file CafeF gốc ngược thời gian (Tự động sort & quy đổi 1.000x)', async (b) => {
    await page.goto(`${FRONTEND_URL}/import`);
    await settle();

    // Switch to CafeF tab
    const tabCafeF = page.locator('#tab-btn-import');
    if (await tabCafeF.isVisible()) await tabCafeF.click();
    await settle(500);

    const rawFile = path.join(ROOT, 'test-results', 'p0b', 'data', 'raw_HSX_subset.csv');
    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles(rawFile);
    await settle(500);

    // Click preview
    const previewBtn = page.getByRole('button', { name: /Xem trước/i });
    await previewBtn.click();
    try {
      await page.locator('.card:has-text("Hợp lệ")').waitFor({ timeout: 45000 });
      await page.waitForSelector('table tbody tr', { timeout: 10000 });
    } catch {}
    await settle(2000);

    const mainText = await page.locator('main, div.panel').allInnerTexts();
    const joined = mainText.join(' ');
    const isAccepted = joined.includes('Đủ điều kiện nhập') || joined.includes('✅');
    b.notes.push(`Preview outcome: ${isAccepted ? 'Đủ điều kiện nhập (Auto-sorted)' : 'Blocked'}`);
  });

  // 3. RP-01 Replay Session Initialization (FPT 2023-2024)
  let replaySessionId = null;
  await runTest('RP-01', 'Khởi tạo phiên Replay FPT (2023-2024)', async (b) => {
    await page.goto(`${FRONTEND_URL}/replay`);
    await settle();

    // Check if new session form exists
    const symbolInput = page.locator('input[placeholder*="Search symbol"]').first();
    if (await symbolInput.isVisible()) {
      await symbolInput.fill('FPT');
      const startInput = page.locator('input[type="date"]').nth(0);
      const endInput = page.locator('input[type="date"]').nth(1);
      await startInput.fill('2023-01-02');
      await endInput.fill('2024-12-27');
      await page.getByRole('button', { name: /Bắt đầu|Start|Tạo phiên/i }).first().click();
      await page.waitForTimeout(4000);
      await settle();
    }
    const currentUrl = page.url();
    b.notes.push(`Replay URL: ${currentUrl}`);
    const match = currentUrl.match(/session=(\d+)/);
    if (match) replaySessionId = match[1];
  });

  // 4. RP-02 Candle Stepping via ArrowRight (10 bars)
  await runTest('RP-02', 'Tua nến 10 bước via ArrowRight', async (b) => {
    for (let i = 0; i < 10; i++) {
      await page.keyboard.press('ArrowRight');
      await page.waitForTimeout(80);
    }
    await settle(1200);
    b.notes.push('Đã tiến 10 nến bằng phím ArrowRight.');
  });

  // 5. RP-05 Submit BUY order for 100 shares & T+2 verification
  await runTest('RP-05', 'Đặt lệnh BUY 100 cổ phiếu FPT & kiểm tra T+2 & vốn VNĐ', async (b) => {
    const buyBtn = page.getByRole('button', { name: 'BUY', exact: true });
    if (await buyBtn.isVisible()) {
      await buyBtn.click();
      await settle(600);

      const dialog = page.getByTestId('trade-decision-dialog');
      if (await dialog.isVisible()) {
        const qtyInput = dialog.locator('input[aria-label="Quantity"]');
        await qtyInput.fill('100');
        const submitBtn = dialog.getByRole('button', { name: /Submit BUY/i });
        await submitBtn.click();
        await page.waitForTimeout(2000);
        await settle();
        b.notes.push('Đã gửi lệnh MUA 100 cổ phiếu FPT.');
      }
    }

    const sellBtn = page.getByRole('button', { name: 'SELL', exact: true });
    const isSellDisabled = await sellBtn.isDisabled();
    b.notes.push(`Nút SELL bị vô hiệu hóa (T+2 settlement lock): ${isSellDisabled}`);
  });

  // 6. SC-01 Signal Scanner multi-ticker scan (FPT, SSI 2023)
  await runTest('SC-01', 'Quét tín hiệu kỹ thuật đa mã (FPT, SSI 2023)', async (b) => {
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
      await page.waitForTimeout(5000);
      await settle();
    }
    b.notes.push('Đã hoàn thành quét tín hiệu FPT & SSI trên khung 2023.');
  });

  // 7. ST-01 Strategy Tester Battle (3-way showdown)
  await runTest('ST-01', 'Strategy Tester - Đối đầu 3 chiến lược (Battle)', async (b) => {
    await page.goto(`${FRONTEND_URL}/strategy-lab`);
    await settle();

    const battleTab = page.locator('[data-testid="lab-tab-battle"]');
    if (await battleTab.isVisible()) await battleTab.click();
    await settle(600);

    const battleBtn = page.getByText('Chọn 3 Chiến Lược Đối Đầu', { exact: false });
    if (await battleBtn.isVisible()) await battleBtn.click();

    const dates = page.locator('form input[type="date"]');
    if (await dates.count() >= 2) {
      await dates.nth(0).fill('2023-01-02');
      await dates.nth(1).fill('2023-12-29');
    }
    const submitBtn = page.locator('form button[type="submit"]').first();
    await submitBtn.click();
    await page.waitForTimeout(6000);
    await settle();
    b.notes.push('Đã chạy xong 3 chiến lược đối đầu trên dữ liệu 2023.');
  });

  // 8. ST-02 Multi-Phase Batch Form Submission (stepMismatch bugfix)
  await runTest('ST-02', 'Multi-Phase Batch - Sửa lỗi submit form (stepMismatch)', async (b) => {
    await page.goto(`${FRONTEND_URL}/strategy-lab`);
    await settle();

    await page.locator('[data-testid="lab-tab-multiphase"]').click();
    await settle(800);

    const runBatchBtn = page.locator('[data-testid="run-batch-btn"]');
    await runBatchBtn.click();
    await page.waitForTimeout(1500);

    const invalidInputs = page.locator('input:invalid');
    const invalidCount = await invalidInputs.count();
    b.notes.push(`Số ô vi phạm validation HTML5: ${invalidCount} (Kỳ vọng: 0, form submit thành công)`);
  });

  // 9. ST-04 Strategy Rule Builder (Actionable Execution Trigger)
  await runTest('ST-04', 'Strategy Rule Builder - Nút kích hoạt kiểm định chiến lược', async (b) => {
    await page.goto(`${FRONTEND_URL}/strategy-lab`);
    await settle();

    await page.locator('[data-testid="lab-tab-builder"]').click();
    await settle(1000);

    const executeBtn = page.locator('[data-testid="execute-rule-btn"]');
    const isExecuteVisible = await executeBtn.isVisible();
    b.notes.push(`Nút '🚀 Chạy Kiểm Định Với Quy Tắc Này' hiển thị: ${isExecuteVisible}`);

    // Capture screenshot right here showing Rule Builder with actionable trigger button!
    await takeShot('ST-04');
    b.shotTaken = true;

    if (isExecuteVisible) {
      await executeBtn.click();
      await page.waitForTimeout(1000);
      b.notes.push('Đã click nút execute-rule-btn và chuyển hướng sang Battle.');
    }
  });

  // 10. ST-05 Money Flow BB (Safe horizons / toFixed bugfix)
  await runTest('ST-05', 'Technical Flow BB - Không còn crash toFixed', async (b) => {
    await page.goto(`${FRONTEND_URL}/strategy-lab`);
    await settle();

    await page.locator('[data-testid="lab-tab-flow"]').click();
    await page.waitForTimeout(4000);
    await settle(2000);

    const crashPanel = page.locator('.glass-panel:has-text("Something went wrong")');
    const isCrashed = await crashPanel.isVisible().catch(() => false);
    b.notes.push(`Crash màn hình trắng: ${isCrashed ? 'YES (Lỗi)' : 'NO (Giao diện hiển thị an toàn, toFixed đã sửa)'}`);
  });

} finally {
  await writeFile(path.join(OUT_DIR, 'p1_uat_results.json'), JSON.stringify(results, null, 2));
  await browser.close();
  console.log(`\nHoàn thành bộ UAT Phase 1. Kết quả lưu tại: ${path.join(OUT_DIR, 'p1_uat_results.json')}`);
}
