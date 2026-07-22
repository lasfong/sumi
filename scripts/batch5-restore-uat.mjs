import { createRequire } from 'node:module';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const require = createRequire(new URL('../frontend/package.json', import.meta.url));
const { chromium } = require('playwright');
const frontendUrl = process.env.SUMI_FRONTEND_URL || 'http://127.0.0.1:15173';
const backendUrl = process.env.SUMI_BACKEND_URL || 'http://127.0.0.1:18000';
const workspacePath = process.env.SUMI_BATCH5_WORKSPACE_EXPORT;
const artifactDir = process.env.SUMI_BATCH5_RESTORE_ARTIFACT_DIR || path.resolve('test-results', 'batch5-hardening', 'restore');
if (!workspacePath) throw new Error('SUMI_BATCH5_WORKSPACE_EXPORT is required');
await mkdir(artifactDir, { recursive: true });
const expected = JSON.parse(await readFile(workspacePath, 'utf8'));
const browser = await chromium.launch({ headless: true, channel: process.env.SUMI_BROWSER_CHANNEL || 'chrome' }).catch(() => chromium.launch({ headless: true }));
const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
const page = await context.newPage();
page.setDefaultTimeout(20_000);
const runtimeErrors = [];
page.on('pageerror', error => runtimeErrors.push(error.stack || error.message));
page.on('console', message => { if (message.type() === 'error') runtimeErrors.push(message.text()); });
let result;
try {
  await page.goto(frontendUrl);
  await page.evaluate(storage => { localStorage.clear(); for (const [key, value] of Object.entries(storage)) localStorage.setItem(key, value); }, expected.storage);
  await page.goto(`${frontendUrl}/replay`);
  await page.locator('header').getByText(`Session #${expected.sessionId}`).waitFor();
  await page.waitForFunction(expectedState => {
    const value = id => document.querySelector(`[data-testid="${id}"]`)?.textContent || '';
    return value('practice-workflow-state') === JSON.stringify(expectedState.practice)
      && value('indicator-domain-state') === JSON.stringify(expectedState.indicators)
      && value('drawing-domain-state') === JSON.stringify(expectedState.drawings);
  }, { practice: expected.practice, indicators: expected.indicators, drawings: expected.drawings });
  const readOutput = async testId => JSON.parse(await page.getByTestId(testId).textContent());
  const practice = await readOutput('practice-workflow-state');
  const indicators = await readOutput('indicator-domain-state');
  const drawings = await readOutput('drawing-domain-state');
  const journal = await (await page.request.get(`${backendUrl}/api/replay/sessions/${expected.sessionId}/journal`)).json();
  const comparisons = {
    practice: JSON.stringify(practice) === JSON.stringify(expected.practice),
    indicators: JSON.stringify(indicators) === JSON.stringify(expected.indicators),
    drawings: JSON.stringify(drawings) === JSON.stringify(expected.drawings),
    journal: JSON.stringify(journal) === JSON.stringify(expected.journal),
  };
  await page.screenshot({ path: path.join(artifactDir, 'restored-workspace.png'), fullPage: true });
  result = { pass: Object.values(comparisons).every(Boolean) && runtimeErrors.length === 0, comparisons, runtimeErrors, sessionId: expected.sessionId };
  await writeFile(path.join(artifactDir, 'restore-results.json'), JSON.stringify(result, null, 2));
  if (!result.pass) process.exitCode = 1;
  console.log(JSON.stringify(result, null, 2));
} finally {
  await context.close(); await browser.close();
}
