import { createRequire } from 'node:module';
const require = createRequire(new URL('../frontend/package.json', import.meta.url));
const { chromium } = require('playwright');

(async () => {
  console.log("Launching browser...");
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
  
  try {
    console.log("Navigating to http://localhost:5173/replay");
    await page.goto('http://localhost:5173/replay');
    
    console.log("Setting up FPT test session...");
    await page.getByPlaceholder('Search symbol').fill('FPT');
    await page.getByRole('button', { name: 'Start Replay' }).click();
    await page.waitForSelector('header:has-text("Session #")');
    
    await page.waitForTimeout(1000);

    // Test 1: Check Indicator Flashing (Performance)
    console.log("\n--- TEST 1: Indicator Flashing ---");
    await page.click('[data-testid="open-add-indicator"]');
    await page.fill('[data-testid="indicator-search"]', 'macd');
    await page.click('[data-testid="add-definition-macd"]');
    await page.click('[data-testid="confirm-add-indicator"]');
    await page.waitForTimeout(500);
    
    console.log("Clicking Next 5 times...");
    await page.evaluate(() => window.performance.clearResourceTimings());
    for (let i = 0; i < 5; i++) {
      await page.getByRole('button', { name: 'Next →' }).click();
      await page.waitForTimeout(400); // Wait for API calls
    }
    
    const apiRequests = await page.evaluate(() => {
      return window.performance.getEntriesByType("resource").filter(r => r.name.includes('/indicators')).length;
    });
    console.log(`Number of Indicator API requests made during 5 steps: ${apiRequests}`);
    
    // Test 2: Drawing into future whitespace
    console.log("\n--- TEST 2: Future Whitespace Drawing ---");
    await page.click('[data-testid="drawing-tool-risk-reward"]');
    
    // Try to draw by clicking in the middle, then on the right edge (future)
    console.log("Clicking at x: 700, y: 500 (Center)");
    await page.mouse.click(700, 500);
    console.log("Clicking at x: 800, y: 500 (Center)");
    await page.mouse.click(800, 500);
    console.log("Clicking at x: 1300, y: 500 (Future edge)");
    await page.mouse.click(1300, 500);
    
    const drawingState = await page.evaluate(() => {
      const container = document.querySelector('[data-drawing-interaction-state]');
      return container ? container.getAttribute('data-drawing-interaction-state') : "No state found";
    });
    const stateObj = JSON.parse(drawingState);
    const hasRRDrawn = stateObj.drawings && stateObj.drawings.length > 0 && stateObj.drawings[0].tool === 'risk-reward';
    console.log(`Risk-Reward tool successfully created: ${hasRRDrawn}`);
    
    // Test 3: Open Positions display
    console.log("\n--- TEST 3: Position Lines on Chart ---");
    // Click BUY button in the panel first
    await page.click('button.btn-buy:has-text("BUY")');
    await page.waitForSelector('[data-testid="trade-decision-dialog"]');
    
    // Place a limit buy order
    await page.getByLabel('Quantity').fill('100');
    await page.getByLabel('Order type').selectOption('LIMIT');
    
    const currentPrice = await page.evaluate(() => document.querySelector('.price-display')?.textContent || "50");
    await page.getByLabel('Limit price').fill(String(Number(currentPrice) * 0.95));
    
    console.log("Clicking Record BUY...");
    await page.click('button:has-text("Record BUY")');
    await page.waitForTimeout(1000);
    
    // Test 4: Lot Size Validation
    console.log("\n--- TEST 4: Lot Size Validation ---");
    await page.click('button.btn-buy:has-text("BUY")');
    await page.waitForSelector('[data-testid="trade-decision-dialog"]');
    
    await page.getByLabel('Quantity').fill('150');
    await page.getByLabel('Order type').selectOption('MARKET_AT_CLOSE');
    await page.click('button:has-text("Record BUY")');
    
    await page.waitForTimeout(1000);
    
    const positionText = await page.evaluate(() => {
      return document.querySelector('.panel')?.textContent || "";
    });
    console.log("Position Panel text after buying 150 shares (if success, it will show 150, if error, will show alert):");
    console.log(positionText.includes('150') ? "Bought 150 shares successfully (BUG: Allowed odd lot)" : "Did not buy 150 shares (PASS: Odd lot blocked)");
    
  } catch (e) {
    console.error("Test error:", e);
  } finally {
    await browser.close();
    console.log("Browser closed.");
  }
})();
