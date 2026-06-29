import { createRequire } from 'node:module';

const require = createRequire(new URL('../frontend/package.json', import.meta.url));
const { chromium } = require('playwright');

const baseUrl = process.env.SUMI_FRONTEND_URL || 'http://127.0.0.1:5173';
const backendUrl = process.env.SUMI_BACKEND_URL || 'http://127.0.0.1:8000';
const headless = process.env.SUMI_BROWSER_HEADLESS !== 'false';

const pageErrors = [];

async function waitForHealth() {
  const response = await fetch(`${backendUrl}/api/health`);
  if (!response.ok) {
    throw new Error(`Backend health failed: ${response.status}`);
  }
}

async function selectStrategy(page, value) {
  await page.locator('select').selectOption(value);
}

async function selectIndicator(page, label) {
  await page.getByRole('button', { name: /Indicators/ }).click();
  await page.getByRole('button', { name: label }).click();
  await page.waitForTimeout(500);
}

async function submitOrder(page, buttonSelector) {
  await page.locator(buttonSelector).click();
  await page.locator('button.btn-primary').click();
  await page.waitForTimeout(900);
}

async function assertNoBlankPage(page, label) {
  const bodyText = (await page.locator('body').innerText()).trim();
  if (!bodyText) {
    throw new Error(`${label}: page body is blank`);
  }
}

async function run() {
  await waitForHealth();

  const browser = await launchBrowser();
  const page = await browser.newPage({ viewport: { width: 1366, height: 768 } });
  page.setDefaultTimeout(20000);
  page.on('pageerror', (error) => pageErrors.push(error.message));
  page.on('console', (message) => {
    if (message.type() === 'error') {
      const text = message.text();
      if (
        text.includes('Invalid date string') ||
        text.includes('Assertion failed') ||
        text.includes('Sumi UI runtime error') ||
        text.includes('status code 500')
      ) {
        pageErrors.push(text);
      }
    }
  });

  try {
    await page.goto(`${baseUrl}/replay`);
    await page.waitForLoadState('domcontentloaded');
    await page.waitForTimeout(1000);
    if (await page.getByRole('button', { name: 'New Session' }).isVisible().catch(() => false)) {
      await page.getByRole('button', { name: 'New Session' }).click();
    }
    await page.getByPlaceholder('Search symbol').fill('FPT');
    await page.getByRole('button', { name: 'Start Replay' }).click();
    await page.getByText(/Session #/).first().waitFor();
    const replayText = await page.locator('body').innerText();
    const sessionMatch = replayText.match(/Session #(\d+)/);
    const sessionId = sessionMatch?.[1];
    if (!sessionId) throw new Error('Replay session id was not visible');

    await selectIndicator(page, 'EMA (20)');
    await selectIndicator(page, 'RSI (14)');
    await selectIndicator(page, 'MACD (12, 26, 9)');

    await submitOrder(page, 'button.btn-buy');
    await page.getByText(/LONG 100/).waitFor();
    await page.getByRole('button', { name: /Next/ }).click();
    await submitOrder(page, 'button.btn-sell');
    await page.getByText(/T\+2 constraint/).waitFor();
    await page.getByRole('button', { name: /Next/ }).click();
    await submitOrder(page, 'button.btn-sell');
    await page.getByText('No open positions.').waitFor();
    await assertNoBlankPage(page, 'Replay trade flow');

    await page.goto(`${baseUrl}/backtest`);
    await selectStrategy(page, 'macd_rsi_momentum.yaml');
    await page.getByRole('button', { name: 'Run Backtest' }).click();
    await page.getByText(/Results \(Session #/).waitFor();
    await page.getByText('Regime Slices').waitFor();
    await assertNoBlankPage(page, 'Backtest');

    await page.goto(`${baseUrl}/strategy-lab`);
    await page.getByRole('button', { name: 'Select All' }).click();
    await page.getByRole('button', { name: 'Compare Strategies' }).click();
    await page.getByRole('heading', { name: 'Comparison' }).waitFor();
    await page.getByText('SUCCEEDED').first().waitFor();
    await page.getByRole('button', { name: 'Clear', exact: true }).click();
    await page.locator('input[type="checkbox"]').first().check();
    await page.getByRole('button', { name: 'Run Sweep' }).click();
    await page.getByText('Sweep Results').waitFor();
    await assertNoBlankPage(page, 'Strategy Lab');

    await page.goto(`${baseUrl}/scanner`);
    await selectStrategy(page, 'macd_rsi_momentum.yaml');
    await page.getByRole('button', { name: 'Run Scanner' }).click();
    await page.getByText('Signals').waitFor();
    await page.getByRole('button', { name: 'Replay' }).first().click();
    await page.getByText('Scanner Signal').first().waitFor();
    await assertNoBlankPage(page, 'Scanner to replay');

    await page.goto(`${baseUrl}/analytics`);
    await page.locator('input').fill(sessionId);
    await page.getByRole('button', { name: 'Load Session' }).click();
    await page.getByText('Trade History').waitFor();
    await page.getByText('Equity Curve & Drawdown').waitFor();
    await assertNoBlankPage(page, 'Analytics');

    if (pageErrors.length > 0) {
      throw new Error(`Browser runtime errors:\n${pageErrors.join('\n---\n')}`);
    }
  } finally {
    await browser.close();
  }
}

async function launchBrowser() {
  const launchOptions = [{ headless }];
  if (process.env.SUMI_BROWSER_CHANNEL) {
    launchOptions.push({ headless, channel: process.env.SUMI_BROWSER_CHANNEL });
  }
  launchOptions.push({ headless, channel: 'msedge' });
  launchOptions.push({ headless, channel: 'chrome' });

  const errors = [];
  for (const options of launchOptions) {
    try {
      return await chromium.launch(options);
    } catch (error) {
      errors.push(error instanceof Error ? error.message : String(error));
    }
  }

  throw new Error(
    `Could not launch a browser for smoke testing.\n` +
    `Install Playwright Chromium with: cd frontend && npx playwright install chromium\n\n` +
    errors.join('\n---\n')
  );
}

run().then(() => {
  console.log('Sumi browser smoke passed');
}).catch((error) => {
  console.error(error);
  process.exit(1);
});
