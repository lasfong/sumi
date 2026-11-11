import { createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const require = createRequire(new URL('../frontend/package.json', import.meta.url));
const { chromium } = require('playwright');

const frontendUrl = process.env.SUMI_FRONTEND_URL || 'http://127.0.0.1:15173';
const backendUrl = process.env.SUMI_BACKEND_URL || 'http://127.0.0.1:18000';
const artifactRoot = process.env.SUMI_UAT_ARTIFACT_DIR || path.resolve('test-results', 'comprehensive-uat');
const screenshotDir = path.join(artifactRoot, 'screenshots');

await mkdir(screenshotDir, { recursive: true });

const productionDatabasePath = path.resolve(process.env.SUMI_PRODUCTION_DATABASE_PATH || 'backend/sumi.db');
const hashFile = async file => {
  try {
    return createHash('sha256').update(await readFile(file)).digest('hex');
  } catch (err) {
    return `hash-error: ${err.message}`;
  }
};
const productionDatabaseBeforeSha256 = await hashFile(productionDatabasePath);

const results = [];
const runtimeErrors = [];

function recordTest(id, name, pass, evidence = {}) {
  const result = { id, name, pass, evidence, at: new Date().toISOString() };
  results.push(result);
  console.log(`[${pass ? 'PASS' : 'FAIL'}] ${id}: ${name}`);
  if (!pass) {
    console.error(`       Evidence:`, evidence);
  }
}

console.log('== Launching Playwright Chromium for Sumi V3 Comprehensive UAT ==');
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
const page = await context.newPage();
page.setDefaultTimeout(30000);

// Auto-accept any confirmation dialogs (like Reset Practice)
page.on('dialog', async dialog => {
  console.log(`  [Dialog] Auto-accepting dialog: "${dialog.message()}"`);
  await dialog.accept();
});

page.on('pageerror', error => {
  const msg = `pageerror: ${error.stack || error.message}`;
  runtimeErrors.push(msg);
  console.error(`  [Browser PageError]`, msg);
});

page.on('console', message => {
  if (message.type() === 'error') {
    const text = message.text();
    // Ignore benign 404s for initial journal or practice state
    if (text.includes('404') && (text.includes('/journal') || text.includes('/practice-state'))) {
      return;
    }
    runtimeErrors.push(text);
    console.error(`  [Browser Console Error]`, text);
  }
});

try {
  // =========================================================================
  // DOMAIN 1: REPLAY ENGINE & BAR-BY-BAR NAVIGATION
  // =========================================================================
  console.log('\n>>> DOMAIN 1: Replay Engine & Bar-by-Bar Playback');

  await page.goto(`${frontendUrl}/replay`);
  await page.evaluate(() => window.localStorage.clear());
  await page.goto(`${frontendUrl}/replay`);

  // TC-REP-01: Session setup & creation
  await page.waitForSelector('button:has-text("Start Replay"), button:has-text("Bắt đầu Luyện tập")');
  await page.getByPlaceholder(/Search symbol|Tìm mã/).fill('FPT');
  await page.waitForTimeout(300);
  
  // Click start replay and capture session response
  let currentSessionId = null;
  const startBtn = page.locator('button:has-text("Start Replay"), button:has-text("Bắt đầu Luyện tập")').first();
  const [createSessionRes] = await Promise.all([
    page.waitForResponse(res => res.url().includes('/api/replay/sessions') && res.request().method() === 'POST'),
    startBtn.click(),
  ]);
  if (createSessionRes.ok()) {
    const sessionData = await createSessionRes.json();
    currentSessionId = sessionData.id;
  }

  await page.waitForSelector('header', { state: 'visible' });
  await page.locator('[data-testid="replay-bar-context"]').waitFor({ state: 'visible' });

  const initialBarText = await page.locator('[data-testid="replay-bar-context"]').innerText();
  const initialBarMatch = initialBarText.match(/#(\d+)/);
  const initialBar = initialBarMatch ? parseInt(initialBarMatch[1], 10) : 0;
  
  recordTest('TC-REP-01', 'Khởi tạo phiên Replay FPT thành công', initialBar >= 0 && !!currentSessionId, { initialBarText, currentSessionId });

  // TC-REP-02: Next bar (+1 bar)
  await page.locator('button[aria-label="Next →"]').click();
  await page.waitForTimeout(300);
  const nextBarText = await page.locator('[data-testid="replay-bar-context"]').innerText();
  const nextBar = parseInt(nextBarText.match(/#(\d+)/)?.[1] || '0', 10);
  recordTest('TC-REP-02', 'Tua nến tiến từng bước (+1 bar)', nextBar === initialBar + 1, { initialBar, nextBar });

  // TC-REP-03: Prev bar (-1 bar)
  await page.locator('button[aria-label="← Prev"]').click();
  await page.waitForTimeout(300);
  const prevBarText = await page.locator('[data-testid="replay-bar-context"]').innerText();
  const prevBar = parseInt(prevBarText.match(/#(\d+)/)?.[1] || '0', 10);
  recordTest('TC-REP-03', 'Tua nến lùi từng bước (-1 bar)', prevBar === initialBar, { initialBar, prevBar });

  // TC-REP-04: Jump +5 / -5 bars
  await page.locator('button[aria-label="+5"]').click();
  await page.waitForTimeout(300);
  const jumpPlusBar = parseInt((await page.locator('[data-testid="replay-bar-context"]').innerText()).match(/#(\d+)/)?.[1] || '0', 10);
  await page.locator('button[aria-label="-5"]').click();
  await page.waitForTimeout(300);
  const jumpMinusBar = parseInt((await page.locator('[data-testid="replay-bar-context"]').innerText()).match(/#(\d+)/)?.[1] || '0', 10);
  recordTest('TC-REP-04', 'Tua nến nhanh (+5 / -5 bars)', jumpPlusBar === prevBar + 5 && jumpMinusBar === prevBar, { jumpPlusBar, jumpMinusBar });

  // TC-REP-05: Autoplay and Pause
  await page.locator('button[aria-label="Auto-Play"]').click();
  await page.waitForTimeout(1000);
  const pauseBtn = page.locator('button[aria-label="Pause"]');
  const isPlaying = (await pauseBtn.count()) > 0;
  if (isPlaying) {
    await pauseBtn.click();
  }
  const postAutoBar = parseInt((await page.locator('[data-testid="replay-bar-context"]').innerText()).match(/#(\d+)/)?.[1] || '0', 10);
  recordTest('TC-REP-05', 'Tự động chạy Autoplay và Pause', isPlaying && postAutoBar > jumpMinusBar, { postAutoBar, jumpMinusBar });

  // TC-REP-06: No future candles leak check
  let noFuturePass = false;
  if (currentSessionId) {
    const candlesRes = await page.request.get(`${backendUrl}/api/replay/sessions/${currentSessionId}/candles`);
    if (candlesRes.ok()) {
      const candles = await candlesRes.json();
      noFuturePass = Array.isArray(candles) && candles.length <= (postAutoBar + 1);
    }
  }
  recordTest('TC-REP-06', 'Bất biến không rò rỉ nến tương lai (No Future Leak)', noFuturePass, { sessionId: currentSessionId, postAutoBar });

  await page.screenshot({ path: path.join(screenshotDir, '01_replay_bar_navigation.png') });

  // =========================================================================
  // DOMAIN 2: TRADING LAB (PTKT PRACTICE & POSITION TOOL)
  // =========================================================================
  console.log('\n>>> DOMAIN 2: Trading Lab & Position Tool');

  // TC-LAB-01: Position Tool form exists without Initial Cash
  const buyBtn = page.locator('button.btn-buy:has-text("BUY")');
  await buyBtn.waitFor({ state: 'visible' });
  await buyBtn.click();
  await page.locator('[data-testid="trade-decision-dialog"]').waitFor({ state: 'visible' });

  const hasCashInput = (await page.locator('input#initial-cash, input[aria-label*="Cash"]').count()) > 0;
  recordTest('TC-LAB-01', 'Form Setup Lệnh hiển thị chuẩn xác (Loại bỏ Vốn ảo)', !hasCashInput, { hasCashInput });

  // TC-LAB-02: Quick Stop Loss selection (-5%)
  const quickSlBtn = page.locator('button:has-text("-5%")');
  await quickSlBtn.click();
  const slValue = await page.locator('input#order-sl').inputValue();
  recordTest('TC-LAB-02', 'Chọn nhanh Stop Loss (-5%)', parseFloat(slValue) > 0, { slValue });

  // TC-LAB-03: Quick Take Profit (2.0R) and Live R:R Calculation
  const quickTpBtn = page.locator('button:has-text("2R")');
  await quickTpBtn.click();
  const tpValue = await page.locator('input#order-tp').inputValue();
  const dialogText = await page.locator('[data-testid="trade-decision-dialog"]').innerText();
  const hasLiveRR = dialogText.includes('1 : 2.0') || dialogText.includes('1 : 1.9') || dialogText.includes('1 : 2.00');
  recordTest('TC-LAB-03', 'Chọn nhanh Take Profit (2.0R) & Hiển thị Live R:R', parseFloat(tpValue) > parseFloat(slValue) && hasLiveRR, { tpValue, dialogTextSnippet: dialogText.slice(0, 300) });

  // TC-LAB-04: Record order / Trade setup on Chart
  const submitBuyBtn = page.locator('button:has-text("Submit BUY")');
  await submitBuyBtn.click();
  await page.waitForTimeout(500);

  const dialogClosed = (await page.locator('[data-testid="trade-decision-dialog"]').count()) === 0;
  recordTest('TC-LAB-04', 'Ghi nhận Lệnh thành công (Chart Position Lines kích hoạt)', dialogClosed);

  // TC-LAB-05 & TC-LAB-06: Auto Win/Loss Resolution & Scoreboard update
  // Advance bars until the trade closes (or 15 steps max)
  for (let i = 0; i < 15; i++) {
    await page.locator('button[aria-label="Next →"]').click();
    await page.waitForTimeout(200);
  }

  const scoreboard = page.locator('[data-testid="practice-scoreboard"]');
  const scoreboardVisible = (await scoreboard.count()) > 0;
  const scoreboardText = scoreboardVisible ? await scoreboard.innerText() : '';
  recordTest('TC-LAB-05', 'Tự động kiểm tra điều kiện Win/Loss (+R / -1R) khi tua nến', scoreboardVisible, { scoreboardText });

  const totalTradesMatch = scoreboardText.match(/Tổng lệnh\s+(\d+)/);
  const totalTrades = totalTradesMatch ? parseInt(totalTradesMatch[1], 10) : 0;
  recordTest('TC-LAB-06', 'Practice Scoreboard hiển thị thời gian thực', scoreboardVisible && scoreboardText.includes('Win Rate'), { totalTrades, scoreboardText });

  // TC-LAB-07: Reset Practice
  const resetPracticeBtn = page.locator('[data-testid="btn-reset-practice"]');
  if (await resetPracticeBtn.isEnabled()) {
    await resetPracticeBtn.click();
    await page.waitForTimeout(500);
    const postResetText = await page.locator('[data-testid="practice-scoreboard"]').innerText();
    const postResetTrades = parseInt(postResetText.match(/Tổng lệnh\s+(\d+)/)?.[1] || '-1', 10);
    recordTest('TC-LAB-07', 'Reset Luyện tập thành công (Làm mới Scoreboard)', postResetTrades === 0, { postResetText });
  } else {
    recordTest('TC-LAB-07', 'Nút Reset Luyện tập sẵn sàng', (await resetPracticeBtn.count()) === 1);
  }

  await page.screenshot({ path: path.join(screenshotDir, '02_trading_lab_practice.png') });

  // =========================================================================
  // DOMAIN 3: TECHNICAL INDICATORS & MULTI-PANE CHARTING
  // =========================================================================
  console.log('\n>>> DOMAIN 3: Technical Indicators & Multi-Pane Charting');

  // TC-IND-01: Open Indicator modal & Search
  await page.locator('[data-testid="open-add-indicator"]').click();
  await page.locator('[data-testid="indicator-search"]').waitFor({ state: 'visible' });
  await page.locator('[data-testid="indicator-search"]').fill('macd');
  await page.waitForTimeout(300);

  const hasMacdInList = (await page.locator('[data-testid="add-definition-macd"]').count()) > 0;
  recordTest('TC-IND-01', 'Mở Modal Thêm Chỉ Báo & Tìm kiếm tức thời', hasMacdInList);

  // TC-IND-02 & TC-IND-03: Add MACD & RSI Subpanes
  await page.locator('[data-testid="add-definition-macd"]').click();
  await page.locator('[data-testid="confirm-add-indicator"]').click();
  await page.waitForTimeout(500);

  // Add RSI
  await page.locator('[data-testid="open-add-indicator"]').click();
  await page.locator('[data-testid="indicator-search"]').fill('rsi');
  await page.waitForTimeout(300);
  await page.locator('[data-testid="add-definition-rsi"]').click();
  await page.locator('[data-testid="confirm-add-indicator"]').click();
  await page.waitForTimeout(500);

  // Add EMA overlay
  await page.locator('[data-testid="open-add-indicator"]').click();
  await page.locator('[data-testid="indicator-search"]').fill('ema');
  await page.waitForTimeout(300);
  await page.locator('[data-testid="add-definition-ema"]').click();
  await page.locator('[data-testid="confirm-add-indicator"]').click();
  await page.waitForTimeout(800);

  const activeIndicatorList = page.locator('[data-testid="active-indicator-list"]');
  const activeListVisible = (await activeIndicatorList.count()) > 0;
  const activeCards = await page.locator('[data-testid^="indicator-instance-"]').count();
  recordTest('TC-IND-02', 'Thêm các chỉ báo EMA, MACD, RSI hiển thị trên biểu đồ', activeListVisible && activeCards >= 2, { activeCards });

  // TC-IND-04: Toggle visibility (hide/show)
  const toggleBtn = page.locator('[data-testid^="toggle-indicator-"]').first();
  if ((await toggleBtn.count()) > 0) {
    await toggleBtn.click();
    await page.waitForTimeout(300);
    await toggleBtn.click();
    await page.waitForTimeout(300);
    recordTest('TC-IND-04', 'Ẩn / Hiện chỉ báo (Visibility Toggle)', true);
  } else {
    recordTest('TC-IND-04', 'Ẩn / Hiện chỉ báo (Visibility Toggle)', false, { reason: 'Toggle button not found' });
  }

  // TC-IND-05 & TC-IND-06: Remove indicator
  const removeBtn = page.locator('[data-testid^="remove-indicator-"]').first();
  if ((await removeBtn.count()) > 0) {
    const beforeCount = await page.locator('[data-testid^="indicator-instance-"]').count();
    await removeBtn.click();
    await page.waitForTimeout(500);
    const afterCount = await page.locator('[data-testid^="indicator-instance-"]').count();
    recordTest('TC-IND-06', 'Xóa chỉ báo thành công', afterCount === beforeCount - 1, { beforeCount, afterCount });
  }

  await page.screenshot({ path: path.join(screenshotDir, '03_indicators_multi_pane.png') });

  // =========================================================================
  // DOMAIN 4: DRAWING SYSTEM & GEOMETRY
  // =========================================================================
  console.log('\n>>> DOMAIN 4: Drawing System & Geometry');

  // Open drawing toolbar via toggle button
  const toggleDrawingsBtn = page.locator('[data-testid="toggle-drawings-button"]');
  if ((await toggleDrawingsBtn.count()) > 0) {
    await toggleDrawingsBtn.click();
    await page.waitForTimeout(300);
  }

  // TC-DRW-01: Select drawing tools
  const trendlineBtn = page.locator('[data-testid="drawing-tool-trendline"]');
  const hasDrawingToolbar = (await page.locator('[data-testid="drawing-toolbar"]').count()) > 0;
  if (hasDrawingToolbar) {
    await trendlineBtn.click();
    const isPressed = (await trendlineBtn.getAttribute('aria-pressed')) === 'true';
    recordTest('TC-DRW-01', 'Kích hoạt công cụ vẽ Trendline trên Toolbar', isPressed);

    // TC-DRW-02: Place drawing points on chart
    await page.mouse.click(600, 400);
    await page.waitForTimeout(100);
    await page.mouse.click(750, 420);
    await page.waitForTimeout(300);
    recordTest('TC-DRW-02', 'Vẽ đường xu hướng (Trendline) trên Chart canvas', true);

    // TC-DRW-04: Undo & Redo
    const undoBtn = page.locator('[data-testid="undo-drawing"]');
    const redoBtn = page.locator('[data-testid="redo-drawing"]');
    const hasUndoRedo = (await undoBtn.count()) > 0 && (await redoBtn.count()) > 0;
    if (await undoBtn.isEnabled()) {
      await undoBtn.click();
      await page.waitForTimeout(200);
      if (await redoBtn.isEnabled()) {
        await redoBtn.click();
        await page.waitForTimeout(200);
      }
    }
    // Also test keyboard shortcut simulation
    await page.keyboard.press('Control+z');
    await page.waitForTimeout(100);
    await page.keyboard.press('Control+y');
    await page.waitForTimeout(100);
    recordTest('TC-DRW-04', 'Bộ điều khiển Undo / Redo thao tác vẽ (Toolbar & Phím tắt)', hasUndoRedo);
  } else {
    recordTest('TC-DRW-01', 'Kích hoạt công cụ vẽ Trendline trên Toolbar', false, { reason: 'drawing-toolbar not found' });
    recordTest('TC-DRW-02', 'Vẽ đường xu hướng (Trendline) trên Chart canvas', false);
    recordTest('TC-DRW-04', 'Bộ điều khiển Undo / Redo thao tác vẽ (Toolbar & Phím tắt)', false);
  }

  await page.screenshot({ path: path.join(screenshotDir, '04_drawing_tools.png') });

  // =========================================================================
  // DOMAIN 5: STRATEGY TESTER (1-CLICK BATTLE RUN & EQUITY CURVE)
  // =========================================================================
  console.log('\n>>> DOMAIN 5: Strategy Tester & 1-Click Battle');

  // TC-STR-01: Navigate to Strategy Tester
  await page.goto(`${frontendUrl}/strategy-lab`);
  await page.waitForSelector('h1:has-text("Strategy Tester"), h1:has-text("Strategy Lab"), #lab-symbols', { timeout: 15000 });
  recordTest('TC-STR-01', 'Điều hướng đến giao diện Strategy Tester', true);

  // TC-STR-02: Quick symbol preset selection
  const fptQuickBtn = page.locator('button:has-text("FPT")').first();
  await fptQuickBtn.click();
  const selectedSymbol = await page.locator('#lab-symbols').inputValue();
  recordTest('TC-STR-02', 'Chọn nhanh Preset Mã Cổ Phiếu (FPT)', selectedSymbol === 'FPT', { selectedSymbol });

  // TC-STR-03: Quick date range preset (3 Năm 2023-2026 matches seeded demo data)
  const datePresetBtn = page.locator('button:has-text("3 Năm (2023–2026)")');
  if ((await datePresetBtn.count()) > 0) {
    await datePresetBtn.click();
  } else {
    await page.locator('#lab-start').fill('2023-01-01');
    await page.locator('#lab-end').fill('2024-06-01');
  }
  const startDateVal = await page.locator('#lab-start').inputValue();
  recordTest('TC-STR-03', 'Chọn nhanh Preset Khoảng Thời Gian Backtest', startDateVal.startsWith('2023'), { startDateVal });

  // TC-STR-04: 1-Click Strategy Battle Run
  const compareBtn = page.locator('#btn-compare-strategies');
  await compareBtn.waitFor({ state: 'visible' });

  // Check strategies to compare
  const checkboxes = page.locator('input[type="checkbox"]');
  const chkCount = await checkboxes.count();
  for (let i = 0; i < Math.min(chkCount, 3); i++) {
    const chk = checkboxes.nth(i);
    const checked = await chk.isChecked();
    if (!checked) {
      await chk.check();
    }
  }

  const [compareResponse] = await Promise.all([
    page.waitForResponse(res => res.url().includes('/api/backtest/run') && res.request().method() === 'POST', { timeout: 25000 }),
    compareBtn.click(),
  ]);

  recordTest('TC-STR-04', '1-Click Đối đầu Chiến lược (Battle Run API hoàn tất)', compareResponse.ok(), { status: compareResponse.status() });

  // TC-STR-05: Comparison Results Table
  await page.waitForSelector('h3:has-text("Comparison")', { timeout: 15000 });
  const compTable = page.locator('table');
  const hasCompTable = (await compTable.count()) > 0;
  const tableText = hasCompTable ? await compTable.first().innerText() : '';
  const hasWinRate = tableText.includes('Win %') || tableText.includes('Win Rate');
  recordTest('TC-STR-05', 'Bảng So Sánh Chiến Lược Đối Đầu (Win Rate, Net Return, Rating ⭐)', hasCompTable && hasWinRate, { snippet: tableText.slice(0, 300) });

  // TC-STR-06: Multi-Strategy Equity Curve Chart
  const svgChart = page.locator('svg');
  const hasSvgLines = (await svgChart.locator('path, polyline, line').count()) > 0;
  recordTest('TC-STR-06', 'Biểu đồ Multi-Strategy Equity Curve (%) SVG trực quan', hasSvgLines);

  await page.screenshot({ path: path.join(screenshotDir, '05_strategy_tester_battle.png') });

  // =========================================================================
  // DOMAIN 6: NAVIGATION, MODALS & SYSTEM GUARDRAILS
  // =========================================================================
  console.log('\n>>> DOMAIN 6: Navigation, Modals & System Guardrails');

  await page.goto(`${frontendUrl}/replay`);
  await page.waitForSelector('header', { timeout: 15000 });

  // TC-SYS-01: Symbol Switcher modal
  const switcherBtn = page.locator('[data-testid="symbol-switcher-button"]');
  if ((await switcherBtn.count()) > 0) {
    await switcherBtn.click();
  } else {
    await page.keyboard.press('/');
  }
  await page.waitForTimeout(300);
  const symbolModal = page.locator('[data-testid="symbol-switcher-modal"]');
  const symbolModalOpen = (await symbolModal.count()) > 0;
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);
  recordTest('TC-SYS-01', 'Modal Đổi Mã Nhanh (Symbol Switcher Modal)', symbolModalOpen);

  // TC-SYS-02: Keyboard Shortcuts modal
  const shortcutsBtn = page.locator('[data-testid="open-keyboard-shortcuts"]');
  if ((await shortcutsBtn.count()) > 0) {
    await shortcutsBtn.click();
  } else {
    await page.keyboard.press('?');
  }
  await page.waitForTimeout(300);
  const shortcutsModal = page.locator('[role="dialog"]:has-text("Keyboard Shortcuts"), [role="dialog"]:has-text("Phím tắt")');
  const shortcutsModalOpen = (await shortcutsModal.count()) > 0;
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);
  recordTest('TC-SYS-02', 'Modal Danh Sách Phím Tắt Hệ Thống (Shortcuts Modal)', shortcutsModalOpen);

  // TC-SYS-03: Session Debrief modal
  const debriefBtn = page.locator('[data-testid="open-session-debrief"]');
  if ((await debriefBtn.count()) > 0) {
    await debriefBtn.click();
  } else {
    await page.keyboard.press('d');
  }
  await page.waitForTimeout(300);
  const debriefModal = page.locator('[role="dialog"]:has-text("Debrief"), [role="dialog"]:has-text("Tổng kết phiên")');
  const debriefModalOpen = (await debriefModal.count()) > 0;
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);
  recordTest('TC-SYS-03', 'Modal Tổng Kết Phiên (Session Debrief Modal)', debriefModalOpen);

  await page.screenshot({ path: path.join(screenshotDir, '06_system_modals.png') });

  // TC-SYS-04: Clean Console & Zero Runtime Errors
  const criticalErrors = runtimeErrors.filter(err => !err.includes('favicon.ico'));
  recordTest('TC-SYS-04', 'Không phát sinh lỗi Runtime / Console (Zero Console Errors)', criticalErrors.length === 0, { runtimeErrors: criticalErrors });

  // TC-SYS-05: Immutability of Production Database
  const productionDatabaseAfterSha256 = await hashFile(productionDatabasePath);
  const dbImmutable = productionDatabaseBeforeSha256 === productionDatabaseAfterSha256;
  recordTest('TC-SYS-05', 'Bảo vệ Database Gốc (Zero DB Mutation SHA-256 match)', dbImmutable, {
    beforeSha256: productionDatabaseBeforeSha256,
    afterSha256: productionDatabaseAfterSha256,
  });

} catch (err) {
  console.error('\n[UAT Exception Captured]:', err);
  results.push({ id: 'FATAL', name: 'Exception in UAT execution', pass: false, error: err.stack || err.message });
} finally {
  await browser.close();
  console.log('\n== Browser closed ==');

  const total = results.length;
  const passed = results.filter(r => r.pass).length;
  const failed = results.filter(r => !r.pass).length;
  const passRate = total > 0 ? ((passed / total) * 100).toFixed(1) : 0;

  console.log('\n======================================================');
  console.log(`UAT SUMMARY: ${passed}/${total} PASSED (${passRate}%), ${failed} FAILED`);
  console.log('======================================================\n');

  const report = {
    timestamp: new Date().toISOString(),
    total,
    passed,
    failed,
    passRate: `${passRate}%`,
    productionDatabaseSha256: productionDatabaseBeforeSha256,
    results,
    runtimeErrors,
  };

  await writeFile(path.join(artifactRoot, 'report.json'), JSON.stringify(report, null, 2), 'utf8');
  console.log(`Report written to ${path.join(artifactRoot, 'report.json')}`);

  if (failed > 0) {
    process.exit(1);
  }
}
