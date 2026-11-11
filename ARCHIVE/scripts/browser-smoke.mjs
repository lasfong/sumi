import { createRequire } from 'node:module';
import { mkdir, rm } from 'node:fs/promises';
import path from 'node:path';

const require = createRequire(new URL('../frontend/package.json', import.meta.url));
const { chromium } = require('playwright');

const baseUrl = process.env.SUMI_FRONTEND_URL || 'http://127.0.0.1:5173';
const backendUrl = process.env.SUMI_BACKEND_URL || 'http://127.0.0.1:8000';
const headless = process.env.SUMI_BROWSER_HEADLESS !== 'false';
const artifactRoot = process.env.SUMI_BROWSER_ARTIFACT_DIR || path.resolve('..', 'test-results', 'browser-smoke');
const artifactRunDir = path.join(artifactRoot, new Date().toISOString().replace(/[:.]/g, '-'));

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

async function assertNoHorizontalPageOverflow(page, label) {
  const dimensions = await page.evaluate(() => ({
    viewportWidth: document.documentElement.clientWidth,
    pageWidth: document.documentElement.scrollWidth,
  }));
  if (dimensions.pageWidth > dimensions.viewportWidth + 1) {
    throw new Error(
      `${label}: horizontal overflow ${dimensions.pageWidth}px > ${dimensions.viewportWidth}px`,
    );
  }
}

async function run() {
  await waitForHealth();

  const browser = await launchBrowser();
  await mkdir(artifactRunDir, { recursive: true });
  const context = await browser.newContext({
    viewport: { width: 1366, height: 768 },
    recordVideo: { dir: artifactRunDir, size: { width: 1366, height: 768 } },
  });
  await context.tracing.start({ screenshots: true, snapshots: true, sources: true });
  const page = await context.newPage();
  page.setDefaultTimeout(20000);
  page.on('pageerror', (error) => pageErrors.push(error.message));
  page.on('console', (message) => {
    if (message.type() === 'error') {
      const text = message.text();
      const isExpectedRejectedRequest =
        text.includes('Failed to load resource') && text.includes('400 (Bad Request)');
      if (!isExpectedRejectedRequest) {
        pageErrors.push(text);
      }
    }
  });

  let failed = false;
  try {
    await page.goto(baseUrl);
    await page.evaluate(() => window.localStorage.clear());

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

    await selectIndicator(page, 'Exponential Moving Average');
    await selectIndicator(page, 'Relative Strength Index');
    await selectIndicator(page, 'MACD');
    await selectIndicator(page, 'Commodity Channel Index');

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
    await page.waitForTimeout(250);
    const sweepResponsePromise = page.waitForResponse(
      response => response.url().includes('/api/strategy-lab/sweep') && response.status() === 200,
      { timeout: 60000 },
    );
    await page.getByRole('button', { name: 'Run Sweep' }).click();
    const sweepResponse = await sweepResponsePromise;
    const sweepPayload = await sweepResponse.json();
    if (!Array.isArray(sweepPayload.variants) || sweepPayload.variants.length === 0) {
      throw new Error(`Strategy Lab sweep returned no variants: ${JSON.stringify(sweepPayload).slice(0, 500)}`);
    }
    await page.getByText('Sweep Results').waitFor({ timeout: 30000 });
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

    await page.setViewportSize({ width: 390, height: 844 });
    for (const route of ['/replay', '/backtest', '/strategy-lab', '/scanner', '/analytics', '/journal', '/import']) {
      await page.goto(`${baseUrl}${route}`);
      await page.waitForLoadState('domcontentloaded');
      await assertNoBlankPage(page, `Mobile ${route}`);
      await assertNoHorizontalPageOverflow(page, `Mobile ${route}`);
    }

    if (pageErrors.length > 0) {
      throw new Error(`Browser runtime errors:\n${pageErrors.join('\n---\n')}`);
    }
    await context.tracing.stop();
  } catch (error) {
    failed = true;
    const screenshotPath = path.join(artifactRunDir, 'failure.png');
    const tracePath = path.join(artifactRunDir, 'trace.zip');
    await page.screenshot({ path: screenshotPath, fullPage: true }).catch(() => {});
    await context.tracing.stop({ path: tracePath }).catch(() => {});
    await context.close();
    console.error(`Browser smoke artifacts saved to ${artifactRunDir}`);
    throw error;
  } finally {
    if (!failed) {
      await context.close();
      await rm(artifactRunDir, { recursive: true, force: true });
    }
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
