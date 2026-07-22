import { createRequire } from 'node:module';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { validateFutureBoundary } from './batch5-closure-contract.mjs';
import { auditEvidence } from './batch5-evidence-audit.mjs';

const require = createRequire(new URL('../frontend/package.json', import.meta.url));
const { chromium } = require('playwright');
const Ajv2020 = require('ajv/dist/2020').default;

const readJson = async relative => JSON.parse(await readFile(new URL(relative, import.meta.url), 'utf8'));
const drawingSchema = await readJson('../docs/decision-packs/sumi-drawing-document-v1.schema.json');
const drawingCorpus = await readJson('../frontend/src/features/drawings/__fixtures__/drawing-contract-corpus.json');
const drawingCorpusBases = {
  horizontal: await readJson('../frontend/src/features/drawings/__fixtures__/valid-horizontal-document.json'),
  'all-tools': await readJson('../frontend/src/features/drawings/__fixtures__/valid-all-tools-document.json'),
};
const batch5ReturnedBaseline = await readJson('../test-results/batch5-hardening/2026-07-19T01-05-23Z/product-uat/2026-07-19T01-05-26-343Z/results.json');
const ajv = new Ajv2020({ allErrors: true, strict: true });
ajv.addFormat('uuid', /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i);
const validateDrawingStructure = ajv.compile(drawingSchema);
const drawingStructuralCorpusResults = drawingCorpus.cases.map(item => {
  const document = materializeCorpusCaseForUat(drawingCorpusBases[item.base], item.patches);
  return { id: item.id, structural: validateDrawingStructure(document), expectedStructural: item.expectedStructural };
});

const frontendUrl = process.env.SUMI_FRONTEND_URL || 'http://127.0.0.1:15173';
const backendUrl = process.env.SUMI_BACKEND_URL || 'http://127.0.0.1:18000';
const artifactRoot = process.env.SUMI_PRODUCT_UAT_ARTIFACT_DIR
  || path.resolve('test-results', 'product-uat');
const runId = new Date().toISOString().replace(/[:.]/g, '-');
const runDir = path.join(artifactRoot, runId);
const sustainedSeconds = Math.max(0, Number(process.env.SUMI_BATCH5_DURATION_SECONDS || 0));
const allowShortHardeningSmoke = process.env.SUMI_BATCH5_ALLOW_SHORT_SMOKE === '1';
const sustainedActionIntervalMs = Math.max(250, Number(process.env.SUMI_BATCH5_ACTION_INTERVAL_MS || 15_000));

await mkdir(runDir, { recursive: true });

const browser = await launchBrowser();
const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
const page = await context.newPage();
page.setDefaultTimeout(sustainedSeconds > 0 ? 60_000 : 20_000);

const checks = [];
const runtimeErrors = [];
const expectedPracticeConsoleErrors = [];
let expectingPracticeRejection = false;
const indicatorResponses = [];
const indicatorRequestFailures = [];
const providerErrors = [];
const networkOrigins = new Set();
const indicatorInflight = new Map();
const indicatorRequestStates = new Map();
const indicatorRequestIntervals = new Map();
const indicatorLatencies = [];
const indicatorOverlapCandidates = [];
const indicatorRequestCancellations = [];
const hardening = { requestedSeconds: sustainedSeconds, timeline: [], navigationMs: [], workspaceUsableMs: [], noFutureSamples: [], accessibility: {}, performance: {}, categoryCoverage: null, workspace: null };
const confirmedIndicatorDuplicates = () => indicatorOverlapCandidates.filter(candidate => candidate.requests.every(request => indicatorRequestStates.get(request) === 'completed')
  && candidate.requests.every(request => indicatorRequestIntervals.has(request))
  && Math.max(...candidate.requests.map(request => indicatorRequestIntervals.get(request).start))
    < Math.min(...candidate.requests.map(request => indicatorRequestIntervals.get(request).end)));
const check = (id, pass, evidence) => checks.push({ id, pass, evidence, at: new Date().toISOString() });
const recordAction = (action, detail = '', category = null) => hardening.timeline.push({ at: new Date().toISOString(), elapsedSeconds: practiceSessionStartedMs ? Math.round((Date.now() - practiceSessionStartedMs) / 1000) : null, action, detail, category });
let practiceSessionStartedMs = 0;

await page.addInitScript(() => {
  window.__sumiBatch5Metrics = { rafGaps: [], longTasks: [] };
  let prior = performance.now();
  const frame = now => { window.__sumiBatch5Metrics.rafGaps.push(now - prior); prior = now; requestAnimationFrame(frame); };
  requestAnimationFrame(frame);
  try {
    new PerformanceObserver(list => window.__sumiBatch5Metrics.longTasks.push(...list.getEntries().map(entry => ({ start: entry.startTime, duration: entry.duration })))).observe({ type: 'longtask', buffered: true });
  } catch { /* unsupported engines are recorded explicitly by the hardening gate */ }
});

page.on('pageerror', error => runtimeErrors.push(`pageerror: ${error.stack || error.message}`));
page.on('console', message => {
  if (message.type() !== 'error') return;
  const detail = `console: ${message.text()}`;
  if (expectingPracticeRejection && message.text().includes('Failed to load resource')) expectedPracticeConsoleErrors.push(detail);
  else runtimeErrors.push(detail);
});
page.on('response', response => {
  if (/\/replay\/sessions\/\d+\/indicators/.test(response.url())) {
    indicatorResponses.push({ url: response.url(), status: response.status() });
    const request = response.request();
    const active = indicatorInflight.get(request);
    if (active) indicatorLatencies.push({ url: active.url, elapsed: performance.now() - active.started, status: response.status() });
    indicatorRequestStates.set(request, 'completed');
    indicatorInflight.delete(request);
  }
});
page.on('requestfailed', request => {
  if (/\/replay\/sessions\/\d+\/indicators/.test(request.url())) {
    const failure = { url: request.url(), error: request.failure()?.errorText ?? 'unknown' };
    if (failure.error === 'net::ERR_ABORTED') {
      indicatorRequestCancellations.push(failure);
      indicatorRequestStates.set(request, 'canceled');
    } else {
      indicatorRequestFailures.push(failure);
      indicatorRequestStates.set(request, 'failed');
    }
    indicatorInflight.delete(request);
  }
});
page.on('requestfinished', request => {
  if (!/\/replay\/sessions\/\d+\/indicators/.test(request.url())) return;
  const timing = request.timing();
  if (timing.startTime >= 0 && timing.responseEnd >= 0) indicatorRequestIntervals.set(request, {
    start: timing.startTime,
    end: timing.startTime + timing.responseEnd,
  });
});
page.on('request', request => {
  try { networkOrigins.add(new URL(request.url()).origin); } catch { networkOrigins.add(request.url().split(':')[0]); }
  if (/\/replay\/sessions\/\d+\/indicators/.test(request.url())) {
    const normalized = normalizeUrl(request.url());
    const overlapping = [...indicatorInflight.entries()].filter(([, active]) => active.url === normalized).map(([activeRequest]) => activeRequest);
    overlapping.forEach(activeRequest => indicatorOverlapCandidates.push({ url: normalized, at: new Date().toISOString(), requests: [activeRequest, request] }));
    indicatorRequestStates.set(request, 'inflight');
    indicatorInflight.set(request, { url: normalized, started: performance.now() });
  }
});
await page.exposeFunction('recordSumiProviderError', detail => providerErrors.push(detail));
await page.addInitScript(() => window.addEventListener('sumi:drawing-provider-error', event => window.recordSumiProviderError(event.detail)));

try {
  await page.goto(frontendUrl);
  await page.evaluate(() => window.localStorage.clear());
  await page.goto(`${frontendUrl}/replay`);
  await page.getByPlaceholder('Search symbol').fill('FPT');
  await page.getByRole('button', { name: 'Start Replay' }).click();
  await page.locator('header').getByText(/Session #/).waitFor();
  practiceSessionStartedMs = Date.now();
  recordAction('session-started', 'FPT deterministic temporary-data replay');

  const headerText = await page.locator('header').innerText();
  const sessionId = Number(headerText.match(/Session #(\d+)/)?.[1]);
  check('replay.session-created', Number.isFinite(sessionId), headerText);

  const readIndicatorDomain = async () => JSON.parse(await page.getByTestId('indicator-domain-state').textContent());
  const readIndicatorRuntime = async () => JSON.parse(await page.getByTestId('indicator-runtime-state').textContent());
  const readIndicatorChart = async () => page.getByTestId('chart-workspace').evaluate(element => {
    element.dispatchEvent(new Event('sumi:indicator-snapshot-request'));
    return JSON.parse(element.dataset.indicatorChartState || '{"keys":[],"panes":[],"instances":[]}');
  });
  const addIndicator = async (definitionId, params = {}) => {
    await page.getByTestId('open-add-indicator').click();
    await page.getByTestId('indicator-search').fill(definitionId);
    await page.getByTestId(`add-definition-${definitionId}`).click();
    for (const [name, value] of Object.entries(params)) await page.getByTestId(`indicator-param-${name}`).fill(String(value));
    await page.getByTestId('confirm-add-indicator').click();
    await page.waitForTimeout(120);
    return (await readIndicatorDomain()).instances.at(-1);
  };
  const instanceCard = id => page.getByTestId(`indicator-instance-${id}`);
  const instanceAction = (id, action) => instanceCard(id).locator(`[data-testid^="${{
    remove: 'remove-indicator-', toggle: 'toggle-indicator-', settings: 'indicator-settings-',
  }[action]}"]`);
  const focusWorkspaceBackground = async () => page.getByTestId('chart-workspace').evaluate(node => { node.setAttribute('tabindex', '-1'); node.focus(); });
  const batch2 = (id, pass, evidence) => check(`batch2.${id}`, pass, evidence);
  const inspectIndicatorLayout = (document, chart) => {
    const expected = document.instances.filter(instance => instance.visible && instance.placement !== 'price').map(instance => instance.paneId);
    const panes = chart.panes.filter(pane => pane.id === 'price' || expected.includes(pane.id));
    const actual = panes.filter(pane => pane.id !== 'price').map(pane => pane.id);
    const price = panes.find(pane => pane.id === 'price');
    const subpanes = panes.filter(pane => pane.id !== 'price');
    const ratios = subpanes.map(pane => price?.height / pane.height);
    const siblingSpread = subpanes.length ? Math.max(...subpanes.map(pane => pane.height)) - Math.min(...subpanes.map(pane => pane.height)) : 0;
    return {
      pass: JSON.stringify(actual) === JSON.stringify(expected)
        && subpanes.length === expected.length
        && ratios.every(ratio => Number.isFinite(ratio) && Math.abs(ratio - 4) <= 0.20)
        && siblingSpread <= 4
        && subpanes.every(pane => pane.height >= 60 && pane.stretchFactor === 1)
        && price?.stretchFactor === 4,
      expected, actual, ratios, siblingSpread, panes,
    };
  };
  const inspectActiveRuntime = (document, runtime) => {
    const active = document.instances.filter(instance => instance.visible);
    const errors = active.filter(instance => runtime[instance.id]?.status === 'error').map(instance => ({ id: instance.id, runtime: runtime[instance.id] }));
    return { pass: errors.length === 0, errors, active: active.map(instance => ({ id: instance.id, status: runtime[instance.id]?.status })) };
  };

  // I-13 is exercised before warming the session so backend nulls cannot masquerade as zeros.
  const earlyRsi = await addIndicator('rsi', { length: 14 });
  await page.waitForTimeout(350);
  const earlyRuntime = JSON.parse(await page.getByTestId('indicator-runtime-state').textContent());
  const earlyChart = await readIndicatorChart();
  batch2('I-13', ['warming', 'loading'].includes(earlyRuntime[earlyRsi.id]?.status)
    && !earlyChart.instances.some(instance => instance.id === earlyRsi.id && instance.series.length > 0),
  JSON.stringify({ runtime: earlyRuntime[earlyRsi.id], chart: earlyChart.instances.find(instance => instance.id === earlyRsi.id) ?? null }));
  await instanceAction(earlyRsi.id, 'remove').click();

  for (let index = 0; index < 12; index += 1) {
    await page.getByRole('button', { name: '+5', exact: true }).click();
    await page.waitForTimeout(150);
  }
  const warmedHeader = await page.locator('header').innerText();
  check('replay.warmup-available', /Bar:\s*#6[01]/.test(warmedHeader), warmedHeader);
  const warmedBar = Number(warmedHeader.match(/Bar:\s*#(\d+)/)?.[1]);
  await focusWorkspaceBackground(); await page.keyboard.press('ArrowLeft'); await page.waitForTimeout(200);
  const previousBar = Number((await page.locator('header').innerText()).match(/Bar:\s*#(\d+)/)?.[1]);
  await focusWorkspaceBackground(); await page.keyboard.press('ArrowRight'); await page.waitForTimeout(250);
  const restoredBar = Number((await page.locator('header').innerText()).match(/Bar:\s*#(\d+)/)?.[1]);
  check('batch1.replay-navigation', previousBar === warmedBar - 1 && restoredBar === warmedBar, `${warmedBar} -> ${previousBar} -> ${restoredBar}; ±5 controls exercised during warmup`);
  await page.locator('header select').selectOption('100');
  await page.getByRole('button', { name: 'Auto-Play', exact: true }).click();
  await page.getByRole('button', { name: 'Pause', exact: true }).waitFor();
  await page.waitForTimeout(250);
  await page.getByRole('button', { name: 'Pause', exact: true }).click();
  check('batch1.autoplay-speed', await page.getByRole('button', { name: 'Auto-Play', exact: true }).count() === 1, '10x speed start/pause completed');
  recordAction('replay-navigation-autoplay-lifecycle', 'keyboard/Prev/Next/±5/speed/autoplay/pause exercised through UI', 'replay.navigation-autoplay');

  const ema20 = await addIndicator('ema', { length: 20, offset: 0 });
  const rsi14 = await addIndicator('rsi', { length: 14 });
  const macd = await addIndicator('macd', { fast: 12, slow: 26, signal: 9 });
  const cci20 = await addIndicator('cci', { length: 20 });
  const volume = await addIndicator('volume');
  const ema50 = await addIndicator('ema', { length: 50, offset: 0 });
  recordAction('indicator-complete-lifecycle', 'EMA/RSI/MACD/CCI/Volume and a second EMA configured through UI', 'indicator.lifecycle');
  await page.waitForTimeout(1_200);

  const duplicateDocument = await readIndicatorDomain();
  const duplicateEmas = duplicateDocument.instances.filter(instance => instance.definitionId === 'ema');
  batch2('I-02', duplicateDocument.instances.length === 6
    && ['ema', 'rsi', 'macd', 'cci', 'volume'].every(id => duplicateDocument.instances.some(instance => instance.definitionId === id)),
  JSON.stringify(duplicateDocument.instances.map(instance => ({ id: instance.id, type: instance.definitionId, params: instance.params }))));
  batch2('I-06', duplicateEmas.length === 2 && duplicateEmas[0].id !== duplicateEmas[1].id
    && duplicateEmas[0].params.length === 20 && duplicateEmas[1].params.length === 50,
  JSON.stringify(duplicateEmas));

  // Apply one edit, then prove a second draft can be cancelled without mutation.
  await instanceAction(ema50.id, 'settings').click();
  await page.getByTestId('indicator-param-length').fill('55');
  await page.getByTestId('apply-indicator-settings').click();
  await instanceAction(ema20.id, 'settings').click();
  await page.getByTestId('indicator-param-length').fill('99');
  await page.getByTestId('cancel-indicator-dialog').click();
  const afterSettings = await readIndicatorDomain();
  batch2('I-05', afterSettings.instances.find(instance => instance.id === ema50.id)?.params.length === 55
    && afterSettings.instances.find(instance => instance.id === ema20.id)?.params.length === 20,
  JSON.stringify(afterSettings.instances.filter(instance => instance.definitionId === 'ema')));

  await instanceAction(rsi14.id, 'toggle').click();
  const hiddenRsi = (await readIndicatorDomain()).instances.find(instance => instance.id === rsi14.id);
  await instanceAction(rsi14.id, 'toggle').click();
  const restoredRsi = (await readIndicatorDomain()).instances.find(instance => instance.id === rsi14.id);
  batch2('I-04', hiddenRsi?.visible === false && hiddenRsi.params.length === 14
    && restoredRsi?.visible === true && restoredRsi.params.length === 14, JSON.stringify({ hiddenRsi, restoredRsi }));
  await instanceAction(ema20.id, 'remove').click();
  const exactDocument = await readIndicatorDomain();
  batch2('I-03', !exactDocument.instances.some(instance => instance.id === ema20.id)
    && exactDocument.instances.some(instance => instance.id === ema50.id), JSON.stringify(exactDocument.instances));

  const bodyText = await page.locator('body').innerText();
  const activeNames = ['EMA', 'RSI', 'MACD', 'CCI'].filter(name => bodyText.includes(name));
  check(
    'indicators.active-list-visible',
    await page.getByTestId('active-indicator-list').count() === 1,
    `Visible text hints: ${activeNames.join(', ') || 'none'}; required test id active-indicator-list missing`,
  );
  check(
    'indicators.individual-remove',
    await instanceAction(rsi14.id, 'remove').count() === 1,
    'Required individual RSI remove control',
  );
  check(
    'indicators.settings',
    await instanceAction(ema50.id, 'settings').count() === 1,
    'Required EMA settings control',
  );
  check(
    'indicators.visibility-toggle',
    await instanceAction(rsi14.id, 'toggle').count() === 1,
    'Required RSI visibility control',
  );
  check(
    'indicators.pane-legends',
    await page.getByTestId('indicator-pane-rsi').count() === 1
      && await page.getByTestId('indicator-pane-macd').count() === 1
      && await page.getByTestId('indicator-pane-cci').count() === 1,
    'Required labeled pane containers',
  );
  check(
    'indicators.reference-lines',
    await page.getByTestId('rsi-reference-lines').count() === 1
      && await page.getByTestId('cci-reference-lines').count() === 1,
    'Required RSI 30/50/70 and CCI -100/0/100 references',
  );

  await page.waitForTimeout(1_200);
  const readyChart = await readIndicatorChart();
  const activeCards = await page.locator('[data-testid^="indicator-instance-"]').count();
  batch2('I-01', activeCards === 5 && exactDocument.instances.every(instance => instance.id && instance.paneId && instance.styles && Number.isInteger(instance.order)), JSON.stringify(exactDocument));
  batch2('I-08', [rsi14.id, macd.id, cci20.id, volume.id].every(id => {
    const pane = readyChart.panes.find(item => item.id === exactDocument.instances.find(instance => instance.id === id)?.paneId);
    return pane && pane.height >= 60 && pane.seriesCount > 0;
  }), JSON.stringify(readyChart.panes));
  batch2('I-09', readyChart.panes[0]?.id === 'price'
    && readyChart.panes.filter(pane => pane.id !== 'price').every(pane => pane.height > 0), JSON.stringify(readyChart.panes));
  const macdSnapshot = readyChart.instances.find(instance => instance.id === macd.id);
  batch2('I-10', JSON.stringify(macdSnapshot?.series) === JSON.stringify(['macd', 'signal', 'histogram'])
    && macdSnapshot?.references.includes('Zero'), JSON.stringify(macdSnapshot));
  batch2('I-11', await page.getByTestId('rsi-reference-lines').textContent() === '30,50,70', await page.getByTestId('rsi-reference-lines').textContent());
  batch2('I-12', await page.getByTestId('cci-reference-lines').textContent() === '-100,0,100', await page.getByTestId('cci-reference-lines').textContent());

  const initialLayout = inspectIndicatorLayout(exactDocument, readyChart);
  const initialRuntime = inspectActiveRuntime(exactDocument, await readIndicatorRuntime());
  batch2('layout-initial-1440x1000', initialLayout.pass, JSON.stringify(initialLayout));
  batch2('runtime-initial-no-active-error', initialRuntime.pass, JSON.stringify(initialRuntime));
  const visibleChrome = await page.locator('[data-testid^="indicator-pane-chrome-"]').evaluateAll(nodes => nodes.map(node => {
    const box = node.getBoundingClientRect();
    return { testId: node.getAttribute('data-testid'), text: node.textContent, visible: box.width > 0 && box.height > 0 };
  }));
  const requiredChromeIds = [rsi14.id, macd.id, cci20.id, volume.id];
  const requiredChromeControlCounts = await Promise.all(requiredChromeIds.map(async id => ({
    id, settings: await page.getByTestId(`pane-settings-${id}`).count(), toggle: await page.getByTestId(`pane-toggle-${id}`).count(), remove: await page.getByTestId(`pane-remove-${id}`).count(),
  })));
  batch2('visible-pane-chrome', requiredChromeIds.every(id => visibleChrome.some(item => item.testId === `indicator-pane-chrome-${id}` && item.visible))
    && requiredChromeControlCounts.every(item => item.settings === 1 && item.toggle === 1 && item.remove === 1), JSON.stringify({ visibleChrome, requiredChromeControlCounts }));
  await page.getByTestId(`pane-settings-${rsi14.id}`).click();
  const paneSettingsOpened = await page.getByTestId('indicator-dialog').count() === 1;
  await page.getByTestId('cancel-indicator-dialog').click();
  batch2('pane-chrome-controls', paneSettingsOpened
    && await page.getByTestId(`pane-toggle-${rsi14.id}`).count() === 1
    && await page.getByTestId(`pane-remove-${rsi14.id}`).count() === 1,
  `settingsOpened=${paneSettingsOpened}; stableId=${rsi14.id}`);
  batch2('visible-reference-evidence', macdSnapshot?.references.includes('Zero')
    && readyChart.instances.find(instance => instance.id === rsi14.id)?.references.join(',') === 'RSI 30,RSI 50,RSI 70'
    && readyChart.instances.find(instance => instance.id === cci20.id)?.references.join(',') === 'CCI -100,CCI 0,CCI 100'
    && await page.getByTestId('rsi-reference-lines').isVisible()
    && await page.getByTestId('cci-reference-lines').isVisible()
    && await page.getByTestId('macd-components').isVisible(),
  JSON.stringify({ chart: readyChart.instances, rsi: await page.getByTestId('rsi-reference-lines').innerText(), cci: await page.getByTestId('cci-reference-lines').innerText(), macd: await page.getByTestId('macd-components').innerText() }));

  await instanceAction(cci20.id, 'toggle').click(); await page.waitForTimeout(180);
  const hiddenLayoutDocument = await readIndicatorDomain(); const hiddenLayoutChart = await readIndicatorChart();
  await instanceAction(cci20.id, 'toggle').click(); await page.waitForTimeout(450);
  const shownLayoutDocument = await readIndicatorDomain(); const shownLayoutChart = await readIndicatorChart();
  batch2('layout-visibility-cycle', inspectIndicatorLayout(hiddenLayoutDocument, hiddenLayoutChart).pass
    && inspectIndicatorLayout(shownLayoutDocument, shownLayoutChart).pass,
  JSON.stringify({ hidden: inspectIndicatorLayout(hiddenLayoutDocument, hiddenLayoutChart), shown: inspectIndicatorLayout(shownLayoutDocument, shownLayoutChart) }));

  await page.getByRole('button', { name: `Move ${volume.label} up` }).click(); await page.waitForTimeout(180);
  const movedLayoutDocument = await readIndicatorDomain(); const movedLayoutChart = await readIndicatorChart();
  await page.getByRole('button', { name: `Move ${volume.label} down` }).click(); await page.waitForTimeout(180);
  const restoredOrderDocument = await readIndicatorDomain(); const restoredOrderChart = await readIndicatorChart();
  batch2('layout-reorder-cycle', inspectIndicatorLayout(movedLayoutDocument, movedLayoutChart).pass
    && inspectIndicatorLayout(restoredOrderDocument, restoredOrderChart).pass,
  JSON.stringify({ moved: inspectIndicatorLayout(movedLayoutDocument, movedLayoutChart), restored: inspectIndicatorLayout(restoredOrderDocument, restoredOrderChart) }));

  const temporaryRsi = await addIndicator('rsi', { length: 21 }); await page.waitForTimeout(450);
  const addedLayoutDocument = await readIndicatorDomain(); const addedLayoutChart = await readIndicatorChart();
  await instanceAction(temporaryRsi.id, 'remove').click(); await page.waitForTimeout(180);
  const removedLayoutDocument = await readIndicatorDomain(); const removedLayoutChart = await readIndicatorChart();
  batch2('layout-add-remove-cycle', inspectIndicatorLayout(addedLayoutDocument, addedLayoutChart).pass
    && inspectIndicatorLayout(removedLayoutDocument, removedLayoutChart).pass,
  JSON.stringify({ added: inspectIndicatorLayout(addedLayoutDocument, addedLayoutChart), removed: inspectIndicatorLayout(removedLayoutDocument, removedLayoutChart) }));

  await page.getByTestId('active-indicator-list').evaluate(element => { element.scrollLeft = 0; });
  await page.screenshot({ path: path.join(runDir, '01-indicators.png'), fullPage: true });

  const beforeReload = indicatorResponses.length;
  const beforeReloadIndicatorDocument = await readIndicatorDomain();
  const beforeReloadIndicatorChart = await readIndicatorChart();
  const beforeReloadIndicatorStorage = await page.evaluate(id => localStorage.getItem(`sumi:workspace:${id}`), sessionId);
  await page.reload();
  await page.locator('header').getByText(/Session #/).waitFor();
  await page.waitForTimeout(1_000);
  const afterReloadIndicatorDocument = await readIndicatorDomain();
  const afterReloadIndicatorChart = await readIndicatorChart();
  const afterReloadIndicatorStorage = await page.evaluate(id => localStorage.getItem(`sumi:workspace:${id}`), sessionId);
  check(
    'indicators.persist-after-reload',
    indicatorResponses.length - beforeReload >= 4,
    `Indicator responses after reload: ${indicatorResponses.length - beforeReload}`,
  );
  batch2('I-07', JSON.stringify(afterReloadIndicatorDocument) === JSON.stringify(beforeReloadIndicatorDocument)
    && new Set(afterReloadIndicatorChart.keys).size === afterReloadIndicatorChart.keys.length
    && afterReloadIndicatorChart.keys.length === beforeReloadIndicatorChart.keys.length,
  JSON.stringify({ document: afterReloadIndicatorDocument, chart: afterReloadIndicatorChart,
    storagePreserved: beforeReloadIndicatorStorage === afterReloadIndicatorStorage, storage: afterReloadIndicatorStorage }));
  const reloadLayout = inspectIndicatorLayout(afterReloadIndicatorDocument, afterReloadIndicatorChart);
  const reloadRuntime = inspectActiveRuntime(afterReloadIndicatorDocument, await readIndicatorRuntime());
  batch2('layout-after-reload', reloadLayout.pass, JSON.stringify(reloadLayout));
  batch2('runtime-after-reload-no-active-error', reloadRuntime.pass, JSON.stringify(reloadRuntime));

  const rapidBefore = indicatorResponses.length;
  for (let index = 0; index < 5; index += 1) {
    await page.getByRole('button', { name: 'Next →', exact: true }).click();
    await page.waitForTimeout(45);
  }
  for (let index = 0; index < 5; index += 1) {
    await page.getByRole('button', { name: '← Prev', exact: true }).click();
    await page.waitForTimeout(45);
  }
  await page.waitForTimeout(700);
  const rapidChart = await readIndicatorChart();
  batch2('request-lifecycle', new Set(rapidChart.keys).size === rapidChart.keys.length
    && rapidChart.keys.length === afterReloadIndicatorChart.keys.length
    && indicatorResponses.slice(rapidBefore).every(response => response.status === 200),
  JSON.stringify({ responses: indicatorResponses.slice(rapidBefore), failures: indicatorRequestFailures, chart: rapidChart }));
  batch2('request-outcome-observation', indicatorResponses.length > 0 && Array.isArray(indicatorRequestFailures),
    JSON.stringify({ responses: indicatorResponses, failedOrAborted: indicatorRequestFailures }));

  const drawingTitles = await page.locator('.drawing-toolbar button').evaluateAll(nodes =>
    nodes.map(node => node.getAttribute('title')).filter(Boolean),
  );
  for (const [id, title] of [
    ['trendline', 'Trendline'],
    ['horizontal', 'Horizontal Line'],
    ['ray', 'Ray'],
    ['rectangle', 'Rectangle'],
    ['fibonacci', 'Fibonacci Retracement'],
    ['text', 'Text'],
  ]) {
    check(`drawings.tool-${id}`, drawingTitles.includes(title), drawingTitles.join(', '));
  }
  check(
    'drawings.undo',
    await page.getByTestId('undo-drawing').count() === 1,
    'Required undo control',
  );

  const workspace = page.getByTestId('chart-workspace');
  const readDomain = async () => JSON.parse(await page.getByTestId('drawing-domain-state').textContent());
  const readInteraction = async () => workspace.evaluate(element => {
    element.dispatchEvent(new Event('sumi:drawing-snapshot-request'));
    return JSON.parse(element.dataset.drawingInteractionState || 'null');
  });
  const drawingSemantic = drawing => drawing ? JSON.stringify({ ...drawing, order: 0 }) : null;
  const findDrawing = (document, id) => document.drawings.find(drawing => drawing.id === id);
  const readPersistedDrawing = document => page.evaluate(({ id, symbol }) =>
    localStorage.getItem(`sumi:drawing-document:v1:${id}:${encodeURIComponent(symbol)}`),
  { id: sessionId, symbol: document.symbol });
  const readHistoryAvailability = async () => ({
    canUndo: !(await page.getByTestId('undo-drawing').isDisabled()),
    canRedo: !(await page.getByTestId('redo-drawing').isDisabled()),
  });
  const waitForRevision = revision => page.waitForFunction(expected => {
    const output = document.querySelector('[data-testid="drawing-domain-state"]');
    return output && JSON.parse(output.textContent || '{}').revision === expected;
  }, revision);
  const pointForDrawing = async id => {
    const state = await readInteraction();
    const drawing = state.drawings.find(item => item.id === id);
    if (!state.pricePane || !drawing || drawing.coordinate === null) throw new Error(`No interactive price-pane coordinate for ${id}`);
    return { x: state.pricePane.left + state.pricePane.width * 0.55, y: state.pricePane.top + drawing.coordinate, state };
  };
  const selectDrawing = async id => {
    const point = await pointForDrawing(id);
    await page.mouse.click(point.x, point.y);
    await page.getByTestId('drawing-selection-toolbar').waitFor();
    return point;
  };
  const pricePane = (await readInteraction()).pricePane;
  if (!pricePane) throw new Error('Official Lightweight Charts price pane bounds unavailable');
  await page.getByTitle('Horizontal Line').click();
  await page.mouse.click(pricePane.left + pricePane.width * 0.68, pricePane.top + pricePane.height * 0.25);
  await page.getByTestId('drawing-selection-toolbar').waitFor();
  check(
    'drawings.selection-contract',
    await page.getByTestId('drawing-selection-toolbar').count() === 1,
    'Selected drawing toolbar visible after an exercised price-pane selection',
  );

  const drawingResponse = await page.request.get(
    `${backendUrl}/api/replay/sessions/${sessionId}/drawings`,
  );
  const drawingState = await drawingResponse.json().catch(() => null);
  check(
    'drawings.persist-after-create',
    drawingResponse.ok() && drawingState?.state_data && drawingState.state_data !== '[]',
    JSON.stringify(drawingState),
  );

  await page.screenshot({ path: path.join(runDir, '02-drawing.png'), fullPage: true });
  const box = await workspace.boundingBox();
  if (!box) throw new Error('Chart workspace has no browser bounding box');
  const scoped = (id, pass, evidence) => check(`batch1.${id}`, pass, evidence);
  const initialDocument = await readDomain();
  const initialCount = initialDocument.drawings.length;
  scoped('toolbar-subset', await page.getByTestId('drawing-tool-select').count() === 1 && await page.getByTestId('drawing-tool-horizontal').count() === 1, 'Cursor/Select and Horizontal subset only');

  await page.getByTestId('drawing-tool-horizontal').click();
  scoped('active-tool-highlight', await page.getByTestId('drawing-tool-horizontal').getAttribute('aria-pressed') === 'true', 'Horizontal aria-pressed');
  await focusWorkspaceBackground(); await page.keyboard.press('Escape');
  await page.mouse.click(pricePane.left + pricePane.width * 0.60, pricePane.top + pricePane.height * 0.35);
  scoped('escape-cancel', (await readDomain()).drawings.length === initialCount, 'Escape leaves no orphan');
  await page.getByTestId('drawing-tool-horizontal').click();
  await page.getByTestId('drawing-tool-select').click();
  await page.mouse.click(pricePane.left + pricePane.width * 0.60, pricePane.top + pricePane.height * 0.38);
  scoped('cursor-cancel', (await readDomain()).drawings.length === initialCount, 'Cursor leaves no orphan');

  await page.getByTestId('drawing-tool-horizontal').click();
  const createX = pricePane.left + pricePane.width * 0.62;
  const createY = pricePane.top + pricePane.height * 0.30;
  await page.mouse.click(createX, createY);
  await page.getByTestId('drawing-selection-toolbar').waitFor();
  const createdDocument = await readDomain();
  const createdDrawing = createdDocument.drawings.find(drawing => !initialDocument.drawings.some(item => item.id === drawing.id));
  if (!createdDrawing) throw new Error('Created drawing ID unavailable');
  const createdId = createdDrawing.id;
  scoped('create-horizontal', createdDocument.drawings.length === initialCount + 1, JSON.stringify(createdDocument));
  scoped('selected-handle', await page.getByTestId('drawing-selection-toolbar').count() === 1, 'Selection toolbar visible; primitive handle captured in screenshot');
  await page.screenshot({ path: path.join(runDir, '03-horizontal-created-selected.png'), fullPage: true });

  await page.getByTestId('undo-drawing').click();
  await waitForRevision(createdDocument.revision + 1);
  const createUndone = await readDomain();
  scoped('undo-create-exact-id', !findDrawing(createUndone, createdId), JSON.stringify(createUndone));
  await page.getByTestId('redo-drawing').click();
  await waitForRevision(createUndone.revision + 1);
  const createRedone = await readDomain();
  scoped('redo-create-same-semantic', drawingSemantic(findDrawing(createRedone, createdId)) === drawingSemantic(createdDrawing), JSON.stringify(findDrawing(createRedone, createdId)));

  const createdPrice = findDrawing(createRedone, createdId).anchors[0].price;
  const movePoint = await selectDrawing(createdId);
  const moveRevision = createRedone.revision;
  await page.mouse.move(movePoint.x, movePoint.y); await page.mouse.down();
  await page.mouse.move(movePoint.x, movePoint.y + 55, { steps: 8 }); await page.mouse.up();
  await waitForRevision(moveRevision + 1);
  const movedDocument = await readDomain();
  const movedPrice = findDrawing(movedDocument, createdId).anchors[0].price;
  scoped('move-one-command', movedPrice !== createdPrice && movedDocument.revision === moveRevision + 1, `${createdPrice} -> ${movedPrice}; revision ${moveRevision} -> ${movedDocument.revision}`);
  await page.getByTestId('undo-drawing').click(); await waitForRevision(movedDocument.revision + 1);
  const moveUndone = await readDomain();
  scoped('undo-move-exact-price', findDrawing(moveUndone, createdId).anchors[0].price === createdPrice, JSON.stringify(findDrawing(moveUndone, createdId)));
  await page.getByTestId('redo-drawing').click(); await waitForRevision(moveUndone.revision + 1);
  const moveRedone = await readDomain();
  scoped('redo-move-exact-price', findDrawing(moveRedone, createdId).anchors[0].price === movedPrice, JSON.stringify(findDrawing(moveRedone, createdId)));

  await selectDrawing(createdId);
  const editPrice = Math.max(1, Math.round(movedPrice * 1.01 * 100) / 100);
  await page.getByTestId('drawing-price-input').fill(String(editPrice));
  await page.getByTestId('apply-drawing-settings').click();
  await waitForRevision(moveRedone.revision + 1);
  const editedDocument = await readDomain();
  scoped('edit-price', findDrawing(editedDocument, createdId).anchors[0].price === editPrice, JSON.stringify(editedDocument));
  await page.getByTestId('undo-drawing').click(); await waitForRevision(editedDocument.revision + 1);
  const editUndone = await readDomain();
  scoped('undo-edit-exact-price', findDrawing(editUndone, createdId).anchors[0].price === movedPrice, JSON.stringify(findDrawing(editUndone, createdId)));
  await page.getByTestId('redo-drawing').click(); await waitForRevision(editUndone.revision + 1);
  const editRedone = await readDomain();
  scoped('redo-edit-exact-price', findDrawing(editRedone, createdId).anchors[0].price === editPrice, JSON.stringify(findDrawing(editRedone, createdId)));
  await page.screenshot({ path: path.join(runDir, '04-horizontal-moved-edited.png'), fullPage: true });

  await selectDrawing(createdId);
  const uiDeleteRevision = (await readDomain()).revision;
  await page.getByTestId('delete-selected-drawing').click();
  await waitForRevision(uiDeleteRevision + 1);
  scoped('ui-delete', (await readDomain()).drawings.length === initialCount, 'UI delete');
  await page.getByTestId('undo-drawing').click();
  await waitForRevision(uiDeleteRevision + 2);
  scoped('undo-ui-delete', !!findDrawing(await readDomain(), createdId), 'Exact drawing restored after UI delete');
  await page.getByTestId('redo-drawing').click();
  await waitForRevision(uiDeleteRevision + 3);
  scoped('redo-ui-delete', !findDrawing(await readDomain(), createdId), 'Exact drawing removed again');
  await page.getByTestId('undo-drawing').click();
  await waitForRevision(uiDeleteRevision + 4);
  await selectDrawing(createdId);
  const keyboardDeleteRevision = (await readDomain()).revision;
  await focusWorkspaceBackground(); await page.keyboard.press('Delete');
  await waitForRevision(keyboardDeleteRevision + 1);
  scoped('keyboard-delete', !findDrawing(await readDomain(), createdId), 'Keyboard deleted exact ID');
  await page.getByTestId('undo-drawing').click();
  await waitForRevision(keyboardDeleteRevision + 2);
  scoped('undo-keyboard-delete', !!findDrawing(await readDomain(), createdId), 'Keyboard delete undo restored exact ID');
  await page.getByTestId('redo-drawing').click();
  await waitForRevision(keyboardDeleteRevision + 3);
  scoped('redo-keyboard-delete', !findDrawing(await readDomain(), createdId), 'Keyboard delete redo removed exact ID');
  await page.getByTestId('undo-drawing').click();
  await waitForRevision(keyboardDeleteRevision + 4);

  await page.getByTestId('clear-all-drawings').click();
  scoped('clear-confirm-required', (await readDomain()).drawings.length === initialCount + 1 && await page.getByTestId('confirm-clear-drawings').count() === 1, 'No clear before confirmation');
  await page.getByTestId('confirm-clear-drawings').click();
  const clearRevision = keyboardDeleteRevision + 5;
  await waitForRevision(clearRevision);
  scoped('clear-confirmed', (await readDomain()).drawings.length === 0, 'Confirmed clear');
  await page.getByTestId('undo-drawing').click();
  await waitForRevision(clearRevision + 1);
  scoped('undo-clear', !!findDrawing(await readDomain(), createdId), 'Undo clear restored exact ID');
  await page.getByTestId('redo-drawing').click();
  await waitForRevision(clearRevision + 2);
  scoped('redo-clear', (await readDomain()).drawings.length === 0, 'Redo clear removed all drawings');
  await page.getByTestId('undo-drawing').click();
  await waitForRevision(clearRevision + 3);

  const cancelBefore = await readDomain();
  const cancelPersistedBefore = await readPersistedDrawing(cancelBefore);
  const cancelPoint = await selectDrawing(createdId);
  await focusWorkspaceBackground();
  await page.mouse.move(cancelPoint.x, cancelPoint.y); await page.mouse.down();
  await page.mouse.move(cancelPoint.x, cancelPoint.y + 35, { steps: 5 });
  await page.keyboard.press('Escape'); await page.mouse.up();
  const cancelAfter = await readDomain();
  const cancelPersistedAfter = await readPersistedDrawing(cancelAfter);
  const cancelInteraction = await readInteraction();
  scoped('cancel-drag-transaction', JSON.stringify(cancelAfter) === JSON.stringify(cancelBefore) && cancelPersistedAfter === cancelPersistedBefore && !cancelInteraction.dragging && cancelInteraction.selectedIds.includes(createdId), `revision=${cancelAfter.revision}; dragging=${cancelInteraction.dragging}; selected=${cancelInteraction.selectedIds.join(',')}`);

  const nativeCancelBefore = await readDomain();
  const nativePersistedBefore = await readPersistedDrawing(nativeCancelBefore);
  const nativeHistoryBefore = await readHistoryAvailability();
  const nativeCancelPoint = await selectDrawing(createdId);
  await page.mouse.move(nativeCancelPoint.x, nativeCancelPoint.y); await page.mouse.down();
  await page.mouse.move(nativeCancelPoint.x, nativeCancelPoint.y + 28, { steps: 4 });
  await workspace.evaluate((element, { x, y }) => {
    element.dispatchEvent(new PointerEvent('pointercancel', { pointerId: 1, pointerType: 'mouse', clientX: x, clientY: y, bubbles: true }));
  }, { x: nativeCancelPoint.x, y: nativeCancelPoint.y + 28 });
  await page.mouse.up();
  const nativeCancelAfter = await readDomain();
  const nativePersistedAfter = await readPersistedDrawing(nativeCancelAfter);
  const nativeHistoryAfter = await readHistoryAvailability();
  const nativeCancelInteraction = await readInteraction();
  scoped('native-pointercancel-transaction', JSON.stringify(nativeCancelAfter) === JSON.stringify(nativeCancelBefore)
    && nativePersistedAfter === nativePersistedBefore
    && JSON.stringify(nativeHistoryAfter) === JSON.stringify(nativeHistoryBefore)
    && !nativeCancelInteraction.dragging
    && nativeCancelInteraction.selectedIds.includes(createdId),
  `revision=${nativeCancelAfter.revision}; history=${JSON.stringify(nativeHistoryAfter)}; dragging=${nativeCancelInteraction.dragging}; selected=${nativeCancelInteraction.selectedIds.join(',')}`);

  const nonPriceBefore = await readDomain();
  const currentBox = await workspace.boundingBox();
  const paneState = await readInteraction();
  const lowerTop = paneState.pricePane.top + paneState.pricePane.height + 4;
  const lowerHeight = Math.max(40, currentBox.y + currentBox.height - lowerTop - 20);
  const nonPriceEvidence = [];
  for (const [index, paneName] of ['Volume', 'RSI', 'MACD', 'CCI'].entries()) {
    await page.getByTestId('drawing-tool-horizontal').click();
    const y = lowerTop + lowerHeight * ((index + 0.5) / 4);
    await page.mouse.click(paneState.pricePane.left + paneState.pricePane.width * 0.55, y);
    nonPriceEvidence.push({ paneName, y, count: (await readDomain()).drawings.length });
  }
  scoped('non-price-pane-isolation', (await readDomain()).drawings.length === nonPriceBefore.drawings.length, JSON.stringify(nonPriceEvidence));

  await page.getByTestId('drawing-tool-select').click();
  await page.mouse.wheel(0, -450);
  await page.mouse.move(paneState.pricePane.left + paneState.pricePane.width * 0.5, paneState.pricePane.top + paneState.pricePane.height * 0.5);
  await page.mouse.down(); await page.mouse.move(paneState.pricePane.left + paneState.pricePane.width * 0.42, paneState.pricePane.top + paneState.pricePane.height * 0.5); await page.mouse.up();
  const panPoint = await selectDrawing(createdId);
  const panBefore = await readDomain();
  await page.mouse.move(panPoint.x, panPoint.y); await page.mouse.down(); await page.mouse.move(panPoint.x, panPoint.y + 18, { steps: 4 }); await page.mouse.up();
  await waitForRevision(panBefore.revision + 1);
  const panAfter = await readDomain();
  scoped('pan-zoom-hit-move', findDrawing(panAfter, createdId).anchors[0].price !== findDrawing(panBefore, createdId).anchors[0].price, JSON.stringify(await readInteraction()));

  await page.getByRole('button', { name: 'Next →', exact: true }).click(); await page.waitForTimeout(400);
  let replayPoint = await selectDrawing(createdId);
  await page.getByRole('button', { name: '← Prev', exact: true }).click(); await page.waitForTimeout(400);
  replayPoint = await selectDrawing(createdId);
  const replayBefore = await readDomain();
  await page.mouse.move(replayPoint.x, replayPoint.y); await page.mouse.down(); await page.mouse.move(replayPoint.x, replayPoint.y - 12, { steps: 3 }); await page.mouse.up();
  await waitForRevision(replayBefore.revision + 1);
  const replayAfter = await readDomain();
  scoped('replay-advance-rewind-interactive', replayAfter.drawings.filter(drawing => drawing.id === createdId).length === 1 && findDrawing(replayAfter, createdId).anchors[0].price !== findDrawing(replayBefore, createdId).anchors[0].price, JSON.stringify(await readInteraction()));

  const wideInteraction = await readInteraction();
  const wideDrawing = wideInteraction.drawings.find(drawing => drawing.id === createdId);
  scoped('resize-1440-alignment', wideDrawing?.coordinate !== null && wideDrawing?.visible && wideDrawing.price === findDrawing(replayAfter, createdId).anchors[0].price, JSON.stringify(wideDrawing));
  await page.screenshot({ path: path.join(runDir, '05-horizontal-pan-zoom-replay.png'), fullPage: true });

  const candlesResponse = await page.request.get(`${backendUrl}/api/replay/sessions/${sessionId}/candles`);
  const sessionResponse = await page.request.get(`${backendUrl}/api/replay/sessions/${sessionId}`);
  const candlePayload = await candlesResponse.json(); const sessionPayload = await sessionResponse.json();
  scoped('no-future-candles', candlePayload.length === sessionPayload.current_index + 1, `candles=${candlePayload.length}; current_index=${sessionPayload.current_index}`);

  const beforeReloadDocument = await readDomain();
  await page.reload(); await page.locator('header').getByText(/Session #/).waitFor();
  await page.waitForFunction(() => {
    const state = document.querySelector('[data-testid="drawing-domain-state"]');
    const persistence = document.querySelector('[data-testid="drawing-persistence-status"]');
    return state && JSON.parse(state.textContent || '{}').symbol === 'FPT' && persistence?.textContent === 'ready';
  });
  const afterReloadDocument = await readDomain();
  scoped('persist-reload-equality', JSON.stringify(afterReloadDocument) === JSON.stringify(beforeReloadDocument), JSON.stringify(afterReloadDocument));
  await page.screenshot({ path: path.join(runDir, '06-horizontal-reloaded.png'), fullPage: true });

  const batch3 = (id, pass, evidence) => check(`batch3.${id}`, pass, evidence);
  const createRequiredTool = async (tool, first, second, text) => {
    const before = await readDomain();
    await page.getByTestId(`drawing-tool-${tool}`).click();
    batch3(`${tool}.active`, await page.getByTestId(`drawing-tool-${tool}`).getAttribute('aria-pressed') === 'true', `active ${tool}`);
    await page.mouse.click(first.x, first.y);
    if (tool === 'text') {
      await page.getByTestId('drawing-text-dialog').waitFor();
      await page.getByTestId('new-drawing-text').fill(text);
      await page.getByTestId('commit-drawing-text').click();
    } else if (second) await page.mouse.click(second.x, second.y);
    await waitForRevision(before.revision + 1);
    const after = await readDomain(); const drawing = after.drawings.find(item => !before.drawings.some(old => old.id === item.id));
    if (!drawing) throw new Error(`No committed ${tool} drawing`);
    batch3(`${tool}.create`, drawing.tool === tool && drawing.id && drawing.anchors.length === (tool === 'text' ? 1 : 2), JSON.stringify(drawing));
    const interaction = await readInteraction(); const projected = interaction.drawings.find(item => item.id === drawing.id);
    batch3(`${tool}.selected-geometry`, projected?.selected && projected.anchors.every(anchor => anchor.x !== null && anchor.y !== null), JSON.stringify(projected));
    return drawing;
  };
  const currentPane = (await readInteraction()).pricePane;
  const point = (x, y) => ({ x: currentPane.left + currentPane.width * x, y: currentPane.top + currentPane.height * y });
  const visibleDrawingXs = (await readInteraction()).magnet.visibleCandles.map(item => item.x).filter(x => x !== null && x > 24 && x < currentPane.width - 24).sort((a, b) => a - b);
  if (visibleDrawingXs.length < 8) throw new Error(`Insufficient visible candle coordinates: ${visibleDrawingXs.length}`);
  const visiblePoint = (fraction, y) => ({ x: currentPane.left + visibleDrawingXs[Math.min(visibleDrawingXs.length - 1, Math.max(0, Math.floor((visibleDrawingXs.length - 1) * fraction)))], y: currentPane.top + currentPane.height * y });
  const toolSpecs = [
    ['trendline', visiblePoint(.15, .25), visiblePoint(.38, .48)],
    ['ray', visiblePoint(.35, .62), visiblePoint(.72, .42)],
    ['rectangle', visiblePoint(.25, .22), visiblePoint(.62, .60)],
    ['fibonacci-retracement', visiblePoint(.32, .68), visiblePoint(.68, .28)],
    ['text', visiblePoint(.55, .52), null, 'Breakout review note'],
  ];
  const createdToolIds = {};
  for (const [tool, first, second] of toolSpecs) {
    const beforeCancel = await readDomain(); await page.getByTestId(`drawing-tool-${tool}`).click(); await page.mouse.click(first.x, first.y);
    if (tool === 'text') await page.getByTestId('drawing-text-dialog').waitFor(); else await focusWorkspaceBackground();
    await page.keyboard.press('Escape'); await page.waitForTimeout(80);
    const afterCancel = await readDomain(); const cancelledInteraction = await readInteraction();
    batch3(`${tool}.escape-cancel`, JSON.stringify(afterCancel) === JSON.stringify(beforeCancel) && !cancelledInteraction.preview && cancelledInteraction.tool === 'select', JSON.stringify(cancelledInteraction));
  }

  for (const [tool, first, second, text] of toolSpecs) {
    const created = await createRequiredTool(tool, first, second, text);
    createdToolIds[tool] = created.id;
    let state = await readDomain(); const createRevision = state.revision;
    await page.getByTestId('undo-drawing').click(); await waitForRevision(createRevision + 1); batch3(`${tool}.undo-create`, !findDrawing(await readDomain(), created.id), created.id);
    await page.getByTestId('redo-drawing').click(); await waitForRevision(createRevision + 2); batch3(`${tool}.redo-create`, JSON.stringify({ ...findDrawing(await readDomain(), created.id), order: 0 }) === JSON.stringify({ ...created, order: 0 }), JSON.stringify(findDrawing(await readDomain(), created.id)));
    const editProjection = (await readInteraction()).drawings.find(item => item.id === created.id); await page.mouse.click(currentPane.left + editProjection.anchors[0].x, currentPane.top + editProjection.anchors[0].y); await page.getByTestId('drawing-selection-toolbar').waitFor();
    state = await readDomain(); const editRevision = state.revision; const original = findDrawing(state, created.id); const editedPrice = Math.round((original.anchors[0].price + 1) * 100) / 100;
    await page.getByTestId('drawing-price-input').fill(String(editedPrice)); await page.getByTestId('apply-drawing-settings').click(); await waitForRevision(editRevision + 1);
    state = await readDomain(); batch3(`${tool}.anchor-edit`, findDrawing(state, created.id).anchors[0].price === editedPrice, JSON.stringify(findDrawing(state, created.id)));
    await page.getByTestId('undo-drawing').click(); await waitForRevision(editRevision + 2); batch3(`${tool}.undo-edit`, findDrawing(await readDomain(), created.id).anchors[0].price === original.anchors[0].price, created.id);
    await page.getByTestId('redo-drawing').click(); await waitForRevision(editRevision + 3); batch3(`${tool}.redo-edit`, findDrawing(await readDomain(), created.id).anchors[0].price === editedPrice, created.id);
    const reselectProjection = (await readInteraction()).drawings.find(item => item.id === created.id); const reselectAnchor = reselectProjection.anchors[0];
    await page.mouse.click(currentPane.left + reselectAnchor.x, currentPane.top + reselectAnchor.y); await page.getByTestId('drawing-selection-toolbar').waitFor();
    if (tool !== 'text') {
      const endpointBefore = await readDomain(); const endpointDrawing = findDrawing(endpointBefore, created.id); const endpointPrice = Math.round((endpointDrawing.anchors[1].price + 2) * 100) / 100;
      await page.getByTestId('drawing-anchor-1-price').fill(String(endpointPrice)); await page.getByTestId('apply-drawing-settings').click(); await waitForRevision(endpointBefore.revision + 1);
      const endpointAfter = findDrawing(await readDomain(), created.id); batch3(`${tool}.endpoint-edit`, endpointAfter.anchors[0].price === endpointDrawing.anchors[0].price && endpointAfter.anchors[1].price === endpointPrice, JSON.stringify(endpointAfter.anchors));
    }
    if (tool === 'fibonacci-retracement') {
      const reverseBefore = await readDomain(); const oldDirection = findDrawing(reverseBefore, created.id).geometry.direction;
      await page.getByTestId('reverse-fibonacci').click(); await page.getByTestId('apply-drawing-settings').click(); await waitForRevision(reverseBefore.revision + 1);
      const reversed = findDrawing(await readDomain(), created.id); batch3('fibonacci.levels-direction', reversed.geometry.levels.map(level => level.ratio).join(',') === '0,0.236,0.382,0.5,0.618,0.786,1' && reversed.geometry.direction !== oldDirection, JSON.stringify(reversed.geometry));
      await page.screenshot({ path: path.join(runDir, '08-fibonacci-selected.png'), fullPage: true });
    }
    if (tool === 'text') {
      const textBefore = await readDomain(); await page.getByTestId('drawing-text-input').fill('Edited breakout review'); await page.getByTestId('apply-drawing-settings').click(); await waitForRevision(textBefore.revision + 1);
      batch3('text.edit', findDrawing(await readDomain(), created.id).geometry.text === 'Edited breakout review', JSON.stringify(findDrawing(await readDomain(), created.id)));
      await page.screenshot({ path: path.join(runDir, '09-text-edited.png'), fullPage: true });
    }
    const projected = (await readInteraction()).drawings.find(item => item.id === created.id); const anchors = projected.anchors;
    const body = tool === 'text' ? { x: anchors[0].x + 20, y: anchors[0].y + 4 } : { x: (anchors[0].x + anchors.at(-1).x) / 2, y: (anchors[0].y + anchors.at(-1).y) / 2 };
    const nextBodyX = visibleDrawingXs.find(x => x > anchors.at(-1).x + 2); const previousBodyX = [...visibleDrawingXs].reverse().find(x => x < anchors[0].x - 2);
    const bodyDx = nextBodyX !== undefined ? nextBodyX - anchors.at(-1).x : previousBodyX !== undefined ? previousBodyX - anchors[0].x : 22;
    const moveBefore = await readDomain(); await page.mouse.move(currentPane.left + body.x, currentPane.top + body.y); await page.mouse.down(); const dragStarted = await readInteraction(); await page.mouse.move(currentPane.left + body.x + bodyDx, currentPane.top + body.y + 14, { steps: 5 }); await page.mouse.up();
    await page.waitForFunction(expected => JSON.parse(document.querySelector('[data-testid="drawing-domain-state"]')?.textContent || '{}').revision >= expected, moveBefore.revision + 1, { timeout: 3_000 }).catch(() => null);
    const moveAfter = await readDomain(); const beforeAnchors = findDrawing(moveBefore, created.id).anchors; const afterAnchors = findDrawing(moveAfter, created.id).anchors;
    const visibleTimes = (await readInteraction()).magnet.visibleCandles.map(item => item.time); const logicalDeltas = afterAnchors.map((anchor, index) => visibleTimes.indexOf(anchor.time) - visibleTimes.indexOf(beforeAnchors[index].time));
    const priceDeltas = afterAnchors.map((anchor, index) => anchor.price - beforeAnchors[index].price);
    const exactBodyTranslation = afterAnchors.length === 1 ? afterAnchors[0].time !== beforeAnchors[0].time && afterAnchors[0].price !== beforeAnchors[0].price
      : logicalDeltas.every(delta => delta !== 0 && delta === logicalDeltas[0]) && priceDeltas.every(delta => Math.abs(delta - priceDeltas[0]) <= 1e-7)
        && (tool !== 'ray' || afterAnchors[1].time > afterAnchors[0].time);
    batch3(`${tool}.body-move`, moveAfter.revision === moveBefore.revision + 1 && exactBodyTranslation,
      JSON.stringify({ before: beforeAnchors, after: afterAnchors, logicalDeltas, priceDeltas, dragStarted, gesture: { dx: bodyDx, dy: 14 } }));
    const deleteProjection = (await readInteraction()).drawings.find(item => item.id === created.id); const deleteAnchor = deleteProjection.anchors[0];
    await page.mouse.click(currentPane.left + deleteAnchor.x, currentPane.top + deleteAnchor.y); await page.getByTestId('drawing-selection-toolbar').waitFor();
    const deleteRevision = (await readDomain()).revision; await page.getByTestId('delete-selected-drawing').click(); await waitForRevision(deleteRevision + 1);
    batch3(`${tool}.ui-delete`, !findDrawing(await readDomain(), created.id), created.id); await page.getByTestId('undo-drawing').click(); await waitForRevision(deleteRevision + 2);
    batch3(`${tool}.undo-delete`, !!findDrawing(await readDomain(), created.id), created.id);
    const keyboardProjection = (await readInteraction()).drawings.find(item => item.id === created.id); await page.mouse.click(currentPane.left + keyboardProjection.anchors[0].x, currentPane.top + keyboardProjection.anchors[0].y); await page.getByTestId('drawing-selection-toolbar').waitFor();
    const keyboardRevision = (await readDomain()).revision; await focusWorkspaceBackground(); await page.keyboard.press('Delete'); await waitForRevision(keyboardRevision + 1); batch3(`${tool}.keyboard-delete`, !findDrawing(await readDomain(), created.id), created.id);
    await page.getByTestId('undo-drawing').click(); await waitForRevision(keyboardRevision + 2); batch3(`${tool}.undo-keyboard-delete`, !!findDrawing(await readDomain(), created.id), created.id);
  }

  const dragCanvasHandle = async (tool, part, direction, dy) => {
    const id = createdToolIds[tool]; const interaction = await readInteraction(); const projected = interaction.drawings.find(item => item.id === id);
    const handle = projected.handles.find(item => item.part === part); if (!handle || handle.x === null || handle.y === null) throw new Error(`Missing ${tool} ${part}`);
    const targetX = direction > 0 ? visibleDrawingXs.find(x => x > handle.x + 2) : [...visibleDrawingXs].reverse().find(x => x < handle.x - 2);
    if (targetX === undefined) throw new Error(`No nonzero x target for ${tool} ${part}`); const dx = targetX - handle.x;
    const before = await readDomain(); const beforeDrawing = structuredClone(findDrawing(before, id));
    await page.mouse.move(currentPane.left + handle.x, currentPane.top + handle.y); await page.mouse.down();
    const started = await readInteraction(); await page.mouse.move(currentPane.left + handle.x + dx, currentPane.top + handle.y + dy, { steps: 5 }); await page.mouse.up();
    await waitForRevision(before.revision + 1); const after = await readDomain(); const afterDrawing = findDrawing(after, id);
    const same = (left, right) => JSON.stringify(left) === JSON.stringify(right); let exactFields = false;
    if (part.startsWith('anchor:')) {
      const index = Number(part.split(':')[1]); exactFields = afterDrawing.anchors.every((anchor, anchorIndex) => anchorIndex === index
        ? anchor.time !== beforeDrawing.anchors[anchorIndex].time && anchor.price !== beforeDrawing.anchors[anchorIndex].price
        : same(anchor, beforeDrawing.anchors[anchorIndex]));
    } else if (part.startsWith('corner:')) {
      const corner = Number(part.split(':')[1]); const [a0, b0] = beforeDrawing.anchors; const [a1, b1] = afterDrawing.anchors;
      exactFields = corner === 0 ? a1.time !== a0.time && a1.price !== a0.price && same(b1, b0)
        : corner === 1 ? a1.time === a0.time && a1.price !== a0.price && b1.time !== b0.time && b1.price === b0.price
          : corner === 2 ? same(a1, a0) && b1.time !== b0.time && b1.price !== b0.price
            : a1.time !== a0.time && a1.price === a0.price && b1.time === b0.time && b1.price !== b0.price;
    }
    const directionValid = tool !== 'ray' || afterDrawing.anchors[1].time > afterDrawing.anchors[0].time;
    batch3(`hardening.canvas-${tool}-${part}`, started.dragPart === part && after.revision === before.revision + 1 && exactFields && directionValid,
      JSON.stringify({ gesture: { tool, part, dx, dy }, exactFields, before: beforeDrawing.anchors, after: afterDrawing.anchors, revision: [before.revision, after.revision] }));
    await page.getByTestId('undo-drawing').click(); await waitForRevision(after.revision + 1); const undone = await readDomain();
    await page.getByTestId('redo-drawing').click(); await waitForRevision(after.revision + 2); const redone = await readDomain();
    batch3(`hardening.canvas-${tool}-${part}-one-history`, JSON.stringify(findDrawing(undone, id).anchors) === JSON.stringify(beforeDrawing.anchors)
      && JSON.stringify(findDrawing(redone, id).anchors) === JSON.stringify(afterDrawing.anchors), JSON.stringify({ before: beforeDrawing.anchors, undone: findDrawing(undone, id).anchors, redone: findDrawing(redone, id).anchors }));
  };
  const createHardeningTool = async tool => {
    const spec = toolSpecs.find(item => item[0] === tool); const before = await readDomain(); await page.getByTestId(`drawing-tool-${tool}`).click(); await page.mouse.click(spec[1].x, spec[1].y);
    if (tool === 'text') { await page.getByTestId('drawing-text-dialog').waitFor(); await page.getByTestId('new-drawing-text').fill('Handle evidence note'); await page.getByTestId('commit-drawing-text').click(); }
    else await page.mouse.click(spec[2].x, spec[2].y);
    await waitForRevision(before.revision + 1); const after = await readDomain(); const created = after.drawings.find(item => !before.drawings.some(previous => previous.id === item.id)); if (!created) throw new Error(`No hardening ${tool}`);
    createdToolIds[tool] = created.id; return created;
  };
  for (const tool of ['trendline', 'ray', 'fibonacci-retracement']) {
    await createHardeningTool(tool);
    await dragCanvasHandle(tool, 'anchor:0', -1, 9); await dragCanvasHandle(tool, 'anchor:1', 1, -9);
  }
  await createHardeningTool('rectangle');
  for (let corner = 0; corner < 4; corner += 1) await dragCanvasHandle('rectangle', `corner:${corner}`, corner < 2 ? -1 : 1, corner < 2 ? 8 : -8);
  await createHardeningTool('text');
  await dragCanvasHandle('text', 'anchor:0', 1, 10);
  const rejectPartialBodyConversion = async kind => {
    const id = createdToolIds.trendline; const interaction = await readInteraction(); const projected = interaction.drawings.find(item => item.id === id);
    const body = { x: (projected.anchors[0].x + projected.anchors[1].x) / 2, y: (projected.anchors[0].y + projected.anchors[1].y) / 2 };
    const before = await readDomain(); const persistedBefore = await readPersistedDrawing(before); const historyBefore = await readHistoryAvailability();
    await workspace.evaluate((element, value) => { element.dataset.sumiDrawingBodyConversionFailure = `${value}:1`; }, kind);
    await page.mouse.move(currentPane.left + body.x, currentPane.top + body.y); await page.mouse.down();
    const started = await readInteraction(); await page.mouse.move(currentPane.left + body.x + 18, currentPane.top + body.y + 12, { steps: 4 }); await page.mouse.up();
    await workspace.evaluate(element => { delete element.dataset.sumiDrawingBodyConversionFailure; }); await page.waitForTimeout(80);
    const after = await readDomain(); const interactionAfter = await readInteraction(); const persistedAfter = await readPersistedDrawing(after); const historyAfter = await readHistoryAvailability();
    batch3(`second-closure.partial-body-${kind}-rejected`, started.dragPart === 'body' && JSON.stringify(after) === JSON.stringify(before)
      && persistedAfter === persistedBefore && JSON.stringify(historyAfter) === JSON.stringify(historyBefore) && !interactionAfter.dragging
      && await page.getByTestId('drawing-persistence-status').textContent() === 'ready', JSON.stringify({ before: findDrawing(before, id).anchors, after: findDrawing(after, id).anchors, started, interactionAfter }));
  };
  await rejectPartialBodyConversion('time'); await rejectPartialBodyConversion('price');
  const rectangleHandleSnapshot = (await readInteraction()).drawings.find(item => item.id === createdToolIds.rectangle);
  batch3('hardening.rectangle-four-real-handles', rectangleHandleSnapshot.handles.map(item => item.part).join(',') === 'corner:0,corner:1,corner:2,corner:3', JSON.stringify(rectangleHandleSnapshot.handles));
  await page.mouse.click(currentPane.left + rectangleHandleSnapshot.handles[0].x, currentPane.top + rectangleHandleSnapshot.handles[0].y);
  await page.getByTestId('drawing-selection-toolbar').waitFor(); await page.screenshot({ path: path.join(runDir, '13-rectangle-four-handles.png'), fullPage: true });

  const switchDragBefore = await readDomain(); const switchPersistedBefore = await readPersistedDrawing(switchDragBefore); const switchHistoryBefore = await readHistoryAvailability();
  const switchDrawing = (await readInteraction()).drawings.find(item => item.id === createdToolIds.text); const switchHandle = switchDrawing.handles[0];
  await page.mouse.move(currentPane.left + switchHandle.x, currentPane.top + switchHandle.y); await page.mouse.down(); await page.mouse.move(currentPane.left + switchHandle.x, currentPane.top + switchHandle.y + 24, { steps: 4 });
  await page.getByTestId('drawing-tool-rectangle').dispatchEvent('click'); await page.waitForTimeout(80); await page.mouse.up();
  const switchDragAfter = await readDomain(); const switchInteraction = await readInteraction();
  batch3('hardening.tool-switch-mid-drag-rollback', JSON.stringify(switchDragAfter) === JSON.stringify(switchDragBefore)
    && await readPersistedDrawing(switchDragAfter) === switchPersistedBefore && JSON.stringify(await readHistoryAvailability()) === JSON.stringify(switchHistoryBefore)
    && !switchInteraction.dragging && switchInteraction.tool === 'rectangle', JSON.stringify({ before: findDrawing(switchDragBefore, createdToolIds.text).anchors, after: findDrawing(switchDragAfter, createdToolIds.text).anchors, interaction: switchInteraction }));
  const switchPreviewBefore = await readDomain(); await page.mouse.click(...Object.values(point(.30, .30))); await page.getByTestId('drawing-tool-ray').click();
  const switchPreviewInteraction = await readInteraction(); batch3('hardening.tool-switch-mid-preview', JSON.stringify(await readDomain()) === JSON.stringify(switchPreviewBefore)
    && !switchPreviewInteraction.preview && switchPreviewInteraction.tool === 'ray', JSON.stringify(switchPreviewInteraction)); await page.getByTestId('drawing-tool-select').click();

  const captureBefore = await readDomain(); const captureDrawing = (await readInteraction()).drawings.find(item => item.id === createdToolIds.text); const captureHandle = captureDrawing.handles[0];
  await page.mouse.move(currentPane.left + captureHandle.x, currentPane.top + captureHandle.y); await page.mouse.down(); await page.mouse.move(currentPane.left + captureHandle.x, currentPane.top + captureHandle.y + 20, { steps: 3 });
  await workspace.evaluate((element, { x, y }) => element.dispatchEvent(new PointerEvent('lostpointercapture', { pointerId: 1, pointerType: 'mouse', clientX: x, clientY: y, bubbles: true })), { x: currentPane.left + captureHandle.x, y: currentPane.top + captureHandle.y + 20 });
  await page.waitForFunction(() => { const value = document.querySelector('[data-testid="chart-workspace"]')?.dataset.drawingInteractionState; return value && !JSON.parse(value).dragging; });
  await page.mouse.up(); await page.getByTestId('drawing-persistence-status').filter({ hasText: 'ready' }).waitFor();
  const captureAfter = await readDomain(); batch3('hardening.capture-loss-rollback', JSON.stringify(captureAfter) === JSON.stringify(captureBefore) && !(await readInteraction()).dragging, JSON.stringify({ before: findDrawing(captureBefore, createdToolIds.text), after: findDrawing(captureAfter, createdToolIds.text), interaction: await readInteraction() }));

  for (const [name, first, second] of [['left', visiblePoint(.75, .48), visiblePoint(.35, .45)], ['equal', visiblePoint(.55, .44), visiblePoint(.55, .58)]]) {
    const before = await readDomain(); await page.getByTestId('drawing-tool-ray').click(); await page.mouse.click(first.x, first.y); await page.mouse.click(second.x, second.y);
    await page.waitForFunction(() => { const value = document.querySelector('[data-testid="chart-workspace"]')?.dataset.drawingInteractionState; const state = value && JSON.parse(value); return state?.tool === 'ray' && state?.preview?.anchors?.length === 1; });
    const interaction = await readInteraction();
    batch3(`hardening.ray-${name}-invalid-create`, JSON.stringify(await readDomain()) === JSON.stringify(before) && !!interaction.preview && interaction.tool === 'ray', JSON.stringify(interaction.preview)); await page.getByTestId('drawing-tool-select').click();
  }

  const rectangleProjection = (await readInteraction()).drawings.find(item => item.id === createdToolIds.rectangle); await page.mouse.click(currentPane.left + rectangleProjection.handles[0].x, currentPane.top + rectangleProjection.handles[0].y);
  await page.getByTestId('drawing-selection-toolbar').waitFor(); const inspectorBefore = await readDomain();
  await page.getByTestId('drawing-line-color').fill('#22c55e'); await page.getByTestId('drawing-line-width').fill('4'); await page.getByTestId('drawing-line-style').selectOption('dashed'); await page.getByTestId('drawing-fill-color').fill('#14532d'); await page.getByTestId('drawing-fill-opacity').fill('0.3'); await page.getByTestId('drawing-visible').uncheck(); await page.getByTestId('drawing-locked').check(); await page.getByTestId('apply-drawing-settings').click();
  await waitForRevision(inspectorBefore.revision + 1); const inspectorAfter = await readDomain(); const styledRectangle = findDrawing(inspectorAfter, createdToolIds.rectangle);
  batch3('hardening.inspector-style-visibility-lock-one-command', inspectorAfter.revision === inspectorBefore.revision + 1 && styledRectangle.style.lineColor === '#22c55e' && styledRectangle.style.lineWidth === 4 && styledRectangle.style.lineStyle === 'dashed' && styledRectangle.style.fillColor === '#14532d' && styledRectangle.style.fillOpacity === 0.3 && !styledRectangle.visible && styledRectangle.locked, JSON.stringify(styledRectangle));
  await page.getByTestId('undo-drawing').click(); await waitForRevision(inspectorAfter.revision + 1); const inspectorUndone = await readDomain(); await page.getByTestId('redo-drawing').click(); await waitForRevision(inspectorAfter.revision + 2); const inspectorRedone = await readDomain();
  batch3('hardening.inspector-style-undo-redo', JSON.stringify(findDrawing(inspectorUndone, createdToolIds.rectangle)) === JSON.stringify(findDrawing(inspectorBefore, createdToolIds.rectangle))
    && JSON.stringify({ ...findDrawing(inspectorRedone, createdToolIds.rectangle), order: 0 }) === JSON.stringify({ ...styledRectangle, order: 0 }), JSON.stringify({ undone: findDrawing(inspectorUndone, createdToolIds.rectangle), redone: findDrawing(inspectorRedone, createdToolIds.rectangle) }));
  await page.getByTestId('undo-drawing').click(); await waitForRevision(inspectorRedone.revision + 1);

  const textProjection = (await readInteraction()).drawings.find(item => item.id === createdToolIds.text); await page.mouse.click(currentPane.left + textProjection.handles[0].x, currentPane.top + textProjection.handles[0].y); await page.getByTestId('drawing-selection-toolbar').waitFor();
  const multilineBefore = await readDomain(); const multiline = 'Breakout thesis\nRetest held above support\nVolume confirms the setup with a deliberately long wrapped line for bounds';
  await page.getByTestId('drawing-text-input').fill(multiline); await page.getByTestId('drawing-text-color').fill('#facc15'); await page.getByTestId('drawing-font-size').fill('16'); await page.getByTestId('apply-drawing-settings').click(); await waitForRevision(multilineBefore.revision + 1);
  const multilineSnapshot = (await readInteraction()).drawings.find(item => item.id === createdToolIds.text);
  batch3('hardening.multiline-text-render-hit-bounds', findDrawing(await readDomain(), createdToolIds.text).geometry.text === multiline && findDrawing(await readDomain(), createdToolIds.text).style.textColor === '#facc15' && findDrawing(await readDomain(), createdToolIds.text).style.fontSize === 16 && multilineSnapshot.bounds
    && multilineSnapshot.bounds.right > multilineSnapshot.bounds.left && multilineSnapshot.bounds.bottom > multilineSnapshot.bounds.top + 32, JSON.stringify({ text: multiline, bounds: multilineSnapshot.bounds }));
  const multilineHit = { x: currentPane.left + multilineSnapshot.bounds.left + 20, y: currentPane.top + multilineSnapshot.bounds.top + 20 }; await page.mouse.click(multilineHit.x, multilineHit.y);
  batch3('hardening.multiline-text-hit-select', (await readInteraction()).selectedIds.includes(createdToolIds.text), JSON.stringify({ hit: multilineHit, bounds: multilineSnapshot.bounds }));
  await page.screenshot({ path: path.join(runDir, '14-multiline-text-selection-bounds.png'), fullPage: true });

  await createHardeningTool('ray'); const rayProjection = (await readInteraction()).drawings.find(item => item.id === createdToolIds.ray); await page.mouse.click(currentPane.left + rayProjection.handles[0].x, currentPane.top + rayProjection.handles[0].y); await page.getByTestId('drawing-selection-toolbar').waitFor();
  const invalidRayBefore = await readDomain(); const rayStartDate = findDrawing(invalidRayBefore, createdToolIds.ray).anchors[0].time; await page.getByTestId('drawing-anchor-1-time').fill(rayStartDate);
  const invalidDisabled = await page.getByTestId('apply-drawing-settings').isDisabled(); await page.getByTestId('cancel-drawing-settings').click();
  batch3('hardening.ray-invalid-inspector-retains-valid', invalidDisabled && JSON.stringify(await readDomain()) === JSON.stringify(invalidRayBefore), JSON.stringify(findDrawing(await readDomain(), createdToolIds.ray).anchors));
  const cancelSettingsBefore = await readDomain(); await page.getByTestId('drawing-line-color').fill('#ff0000'); await page.getByTestId('drawing-line-width').fill('7'); await page.getByTestId('cancel-drawing-settings').click();
  batch3('hardening.inspector-cancel-no-command', JSON.stringify(await readDomain()) === JSON.stringify(cancelSettingsBefore)
    && await page.getByTestId('drawing-line-color').inputValue() === findDrawing(cancelSettingsBefore, createdToolIds.ray).style.lineColor, JSON.stringify(findDrawing(cancelSettingsBefore, createdToolIds.ray).style));
  const keyboardBefore = await readDomain(); await page.getByTestId('drawing-line-width').focus(); await page.keyboard.press('Delete'); await page.keyboard.press('ArrowRight'); await page.keyboard.press('Escape');
  batch3('hardening.inspector-keyboard-isolation', JSON.stringify(await readDomain()) === JSON.stringify(keyboardBefore) && await page.getByTestId('drawing-selection-toolbar').count() === 1, 'Delete/ArrowRight/Escape while editing did not delete drawing or navigate replay');
  const wideInspectorGeometry = await page.getByTestId('drawing-selection-toolbar').evaluate(element => { const box = element.getBoundingClientRect(); const aside = element.closest('aside')?.getBoundingClientRect(); const values = [...element.querySelectorAll('input:not([type="checkbox"]),select,textarea')].map(node => ({ label: node.getAttribute('aria-label'), value: node.value, width: node.getBoundingClientRect().width })); return { box: { left: box.left, right: box.right, top: box.top, bottom: box.bottom }, aside: aside && { left: aside.left, right: aside.right, top: aside.top, bottom: aside.bottom }, values, contained: !!aside && box.left >= aside.left && box.right <= aside.right + 1 && box.width >= 220 }; });
  batch3('hardening.inspector-contained-readable-1440x1000', wideInspectorGeometry.contained && wideInspectorGeometry.values.every(item => item.width >= 90 && String(item.value).length > 0), JSON.stringify(wideInspectorGeometry));
  await page.getByTestId('drawing-selection-toolbar').evaluate(element => { element.scrollTop = 0; });
  await page.screenshot({ path: path.join(runDir, '15-two-anchor-inspector-1440x1000.png'), fullPage: true });

  const textCancelBefore = await readDomain(); await page.getByTestId('drawing-tool-text').click(); await page.mouse.click(...Object.values(visiblePoint(.74, .38))); await page.getByTestId('drawing-text-dialog').waitFor();
  await page.getByTestId('new-drawing-text').fill('   '); await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  batch3('text.empty-cancel', JSON.stringify(await readDomain()) === JSON.stringify(textCancelBefore), 'No empty text orphan');

  let delayedDrawingWrites = 0;
  await page.route('**/api/replay/sessions/*/drawings', async route => { if (route.request().method() === 'PUT' && delayedDrawingWrites < 2) { delayedDrawingWrites += 1; await new Promise(resolve => setTimeout(resolve, 350)); } await route.continue(); });
  const drawingRapidBefore = await readDomain(); await page.getByTestId('drawing-tool-horizontal').click(); await page.mouse.click(...Object.values(visiblePoint(.76, .34)));
  await page.getByTestId('drawing-tool-trendline').click(); await page.mouse.click(...Object.values(visiblePoint(.18, .72))); await page.mouse.click(...Object.values(visiblePoint(.30, .62)));
  await waitForRevision(drawingRapidBefore.revision + 2); await page.unroute('**/api/replay/sessions/*/drawings'); const drawingRapidAfter = await readDomain();
  const rapidBackend = await (await page.request.get(`${backendUrl}/api/replay/sessions/${sessionId}/drawings`)).json(); const rapidLocal = await page.evaluate(({ id, symbol }) => localStorage.getItem(`sumi:drawing-document:v1:${id}:${encodeURIComponent(symbol)}`), { id: sessionId, symbol: drawingRapidAfter.symbol });
  batch3('hardening.two-immediate-edits-two-durable-revisions', drawingRapidAfter.revision === drawingRapidBefore.revision + 2 && drawingRapidAfter.drawings.length === drawingRapidBefore.drawings.length + 2
    && drawingRapidAfter.drawings.at(-2).tool === 'horizontal' && drawingRapidAfter.drawings.at(-1).tool === 'trendline', JSON.stringify({ beforeRevision: drawingRapidBefore.revision, afterRevision: drawingRapidAfter.revision, added: drawingRapidAfter.drawings.slice(-2) }));
  batch3('hardening.rapid-edits-ui-local-backend-equality', rapidBackend.state_data === rapidLocal && JSON.stringify(JSON.parse(rapidBackend.state_data)) === JSON.stringify(drawingRapidAfter), JSON.stringify({ backend: rapidBackend.state_data, local: rapidLocal }));

  const runtimeCorpusResults = JSON.parse(await page.getByTestId('drawing-contract-corpus-state').textContent());
  const structuralCorpusPass = drawingStructuralCorpusResults.every(item => item.structural === item.expectedStructural);
  const semanticCorpusPass = runtimeCorpusResults.length === drawingCorpus.cases.length && runtimeCorpusResults.every(item => item.semantic === item.expectedSemantic);
  const matchingCorpusIds = JSON.stringify(runtimeCorpusResults.map(item => item.id)) === JSON.stringify(drawingStructuralCorpusResults.map(item => item.id));
  batch3('hardening.runtime-canonical-v1-contract', structuralCorpusPass && semanticCorpusPass && matchingCorpusIds,
    JSON.stringify({ draft: drawingCorpus.schemaDraft, structural: drawingStructuralCorpusResults, semantic: runtimeCorpusResults, supplemental: drawingCorpus.semanticInvariants }));
  batch3('second-closure.schema-runtime-corpus', structuralCorpusPass && semanticCorpusPass && matchingCorpusIds,
    JSON.stringify({ structural: drawingStructuralCorpusResults, semantic: runtimeCorpusResults }));
  const horizontalCompatibility = drawingRapidAfter.drawings.find(item => item.id === createdId); batch3('hardening.horizontal-v1-compatibility', horizontalCompatibility.tool === 'horizontal' && horizontalCompatibility.paneId === 'price'
    && horizontalCompatibility.anchors.length === 1 && horizontalCompatibility.geometry.kind === 'horizontal', JSON.stringify(horizontalCompatibility));

  const drawingRoute = `**/api/replay/sessions/${sessionId}/drawings`;
  const commitHorizontalUnderRoute = async fraction => {
    const before = await readDomain(); await page.getByTestId('drawing-tool-horizontal').click(); await page.mouse.click(...Object.values(visiblePoint(fraction, .36)));
    await waitForRevision(before.revision + 1); return { before, after: await readDomain() };
  };
  let commitThenErrorPut = false;
  await page.route(drawingRoute, async route => {
    if (route.request().method() !== 'PUT' || commitThenErrorPut) { await route.continue(); return; }
    commitThenErrorPut = true; const upstream = await route.fetch(); await route.fulfill({ response: upstream, headers: { ...upstream.headers(), 'x-sumi-uat-commit-then-error': '1' } });
  });
  const commitThenError = await commitHorizontalUnderRoute(.81); await page.unroute(drawingRoute);
  const commitThenErrorBackend = await (await page.request.get(`${backendUrl}/api/replay/sessions/${sessionId}/drawings`)).json();
  const commitThenErrorLocal = await readPersistedDrawing(commitThenError.after);
  batch3('second-closure.commit-then-error-intended', commitThenErrorPut && commitThenError.after.revision === commitThenError.before.revision + 1
    && commitThenErrorBackend.state_data === commitThenErrorLocal && JSON.stringify(JSON.parse(commitThenErrorBackend.state_data)) === JSON.stringify(commitThenError.after)
    && await page.getByTestId('drawing-persistence-status').textContent() === 'ready', JSON.stringify({ before: commitThenError.before.revision, after: commitThenError.after.revision, backend: commitThenErrorBackend.state_data }));

  let mismatchedEchoPut = false;
  await page.route(drawingRoute, async route => {
    if (route.request().method() !== 'PUT' || mismatchedEchoPut) { await route.continue(); return; }
    mismatchedEchoPut = true; const upstream = await route.fetch(); const response = await upstream.json();
    await route.fulfill({ response: upstream, contentType: 'application/json', body: JSON.stringify({ ...response, state_data: '{controlled-mismatched-echo}' }) });
  });
  const mismatchedEcho = await commitHorizontalUnderRoute(.84); await page.unroute(drawingRoute);
  const mismatchedEchoBackend = await (await page.request.get(`${backendUrl}/api/replay/sessions/${sessionId}/drawings`)).json();
  const mismatchedEchoLocal = await readPersistedDrawing(mismatchedEcho.after);
  batch3('second-closure.mismatched-echo-intended', mismatchedEchoPut && mismatchedEcho.after.revision === mismatchedEcho.before.revision + 1
    && mismatchedEchoBackend.state_data === mismatchedEchoLocal && JSON.stringify(JSON.parse(mismatchedEchoBackend.state_data)) === JSON.stringify(mismatchedEcho.after)
    && await page.getByTestId('drawing-persistence-status').textContent() === 'ready', JSON.stringify({ before: mismatchedEcho.before.revision, after: mismatchedEcho.after.revision, backend: mismatchedEchoBackend.state_data }));

  const divergentBefore = await readDomain(); const divergentPriorRaw = JSON.stringify(divergentBefore); let divergentRemoteRaw = null; let divergentPut = false;
  await page.route(drawingRoute, async route => {
    if (route.request().method() !== 'PUT' || divergentPut) { await route.continue(); return; }
    divergentPut = true; const upstream = await route.fetch(); const response = await upstream.json();
    divergentRemoteRaw = JSON.stringify({ ...divergentBefore, revision: divergentBefore.revision + 50 });
    await page.request.put(`${backendUrl}/api/replay/sessions/${sessionId}/drawings`, { data: { state_data: divergentRemoteRaw } });
    await route.fulfill({ response: upstream, contentType: 'application/json', body: JSON.stringify({ ...response, state_data: '{controlled-mismatched-echo}' }) });
  });
  await page.getByTestId('drawing-tool-horizontal').click(); await page.mouse.click(...Object.values(visiblePoint(.87, .38)));
  await page.waitForFunction(() => document.querySelector('[data-testid="drawing-persistence-status"]')?.textContent === 'indeterminate'); await page.unroute(drawingRoute);
  const divergentAfter = await readDomain(); const divergentBackend = await (await page.request.get(`${backendUrl}/api/replay/sessions/${sessionId}/drawings`)).json();
  const divergentLocal = await readPersistedDrawing(divergentAfter); const divergentRecovery = await page.evaluate(({ id, symbol }) => localStorage.getItem(`sumi:drawing-indeterminate:v1:${id}:${encodeURIComponent(symbol)}`), { id: sessionId, symbol: divergentAfter.symbol });
  const divergentEvidence = JSON.parse(divergentRecovery ?? 'null');
  batch3('second-closure.divergent-remote-blocked', divergentPut && JSON.stringify(divergentAfter) === JSON.stringify(divergentBefore) && divergentLocal === divergentPriorRaw
    && divergentBackend.state_data === divergentRemoteRaw && divergentEvidence.priorRaw === divergentPriorRaw && divergentEvidence.observedRaw === divergentRemoteRaw
    && JSON.parse(divergentEvidence.intendedRaw).revision === divergentBefore.revision + 1, JSON.stringify({ local: divergentLocal, backend: divergentBackend.state_data, recovery: divergentEvidence }));
  await page.request.put(`${backendUrl}/api/replay/sessions/${sessionId}/drawings`, { data: { state_data: divergentPriorRaw } });
  await page.evaluate(({ id, symbol }) => localStorage.removeItem(`sumi:drawing-indeterminate:v1:${id}:${encodeURIComponent(symbol)}`), { id: sessionId, symbol: divergentBefore.symbol });
  await page.reload(); await page.locator('header').getByText(/Session #/).waitFor(); await page.waitForFunction(() => document.querySelector('[data-testid="drawing-persistence-status"]')?.textContent === 'ready');

  const unknownBefore = await readDomain(); const unknownPriorRaw = JSON.stringify(unknownBefore); let unknownPut = false;
  await page.route(drawingRoute, async route => {
    if (route.request().method() === 'PUT' && !unknownPut) { unknownPut = true; const upstream = await route.fetch(); await route.fulfill({ response: upstream, headers: { ...upstream.headers(), 'x-sumi-uat-commit-then-error': '1' } }); return; }
    if (route.request().method() === 'GET' && unknownPut) { const upstream = await route.fetch(); await route.fulfill({ response: upstream, headers: { ...upstream.headers(), 'x-sumi-uat-reconciliation-unavailable': '1' } }); return; }
    await route.continue();
  });
  await page.getByTestId('drawing-tool-horizontal').click(); await page.mouse.click(...Object.values(visiblePoint(.89, .40)));
  await page.waitForFunction(() => document.querySelector('[data-testid="drawing-persistence-status"]')?.textContent === 'indeterminate'); await page.unroute(drawingRoute);
  const unknownAfter = await readDomain(); const unknownBackend = await (await page.request.get(`${backendUrl}/api/replay/sessions/${sessionId}/drawings`)).json();
  const unknownLocal = await readPersistedDrawing(unknownAfter); const unknownRecovery = JSON.parse(await page.evaluate(({ id, symbol }) => localStorage.getItem(`sumi:drawing-indeterminate:v1:${id}:${encodeURIComponent(symbol)}`), { id: sessionId, symbol: unknownAfter.symbol }) ?? 'null');
  batch3('second-closure.unavailable-remote-blocked', unknownPut && JSON.stringify(unknownAfter) === JSON.stringify(unknownBefore) && unknownLocal === unknownPriorRaw
    && unknownRecovery.priorRaw === unknownPriorRaw && unknownRecovery.observedRaw === null && JSON.parse(unknownBackend.state_data).revision === unknownBefore.revision + 1,
  JSON.stringify({ local: unknownLocal, backend: unknownBackend.state_data, recovery: unknownRecovery }));
  await page.request.put(`${backendUrl}/api/replay/sessions/${sessionId}/drawings`, { data: { state_data: unknownPriorRaw } });
  await page.evaluate(({ id, symbol }) => localStorage.removeItem(`sumi:drawing-indeterminate:v1:${id}:${encodeURIComponent(symbol)}`), { id: sessionId, symbol: unknownBefore.symbol });
  await page.reload(); await page.locator('header').getByText(/Session #/).waitFor(); await page.waitForFunction(() => document.querySelector('[data-testid="drawing-persistence-status"]')?.textContent === 'ready');
  batch3('second-closure.blocked-recovery-reload', JSON.stringify(await readDomain()) === JSON.stringify(unknownBefore), JSON.stringify(await readDomain()));

  const magnetSnapshot = await readInteraction();
  const magnetCandidate = [...magnetSnapshot.magnet.visibleCandles].reverse().find(candle => candle.x !== null && candle.prices.every(item => item.y !== null) && candle.prices.every(item => item.y > 20 && item.y < currentPane.height - 20));
  if (!magnetCandidate) throw new Error('No visible OHLC magnet candidate');
  const high = magnetCandidate.prices.find(item => item.field === 'high');
  await page.getByTestId('drawing-magnet-mode').selectOption('off'); const offBefore = await readDomain(); await page.getByTestId('drawing-tool-horizontal').click();
  await page.mouse.click(currentPane.left + magnetCandidate.x, currentPane.top + high.y + 4); await waitForRevision(offBefore.revision + 1); await page.getByTestId('drawing-persistence-status').filter({ hasText: 'ready' }).waitFor(); const offDrawing = (await readDomain()).drawings.at(-1);
  batch3('magnet.off-unsnapped', offDrawing.anchors[0].price !== high.price, JSON.stringify({ raw: offDrawing.anchors[0], candidate: high }));
  await page.getByTestId('drawing-magnet-mode').selectOption('ohlc'); const snapBefore = await readDomain(); await page.getByTestId('drawing-tool-horizontal').click();
  await page.mouse.click(currentPane.left + magnetCandidate.x, currentPane.top + high.y + 4); await waitForRevision(snapBefore.revision + 1); await page.getByTestId('drawing-persistence-status').filter({ hasText: 'ready' }).waitFor(); const snapped = (await readDomain()).drawings.at(-1);
  batch3('magnet.ohlc-snap', snapped.anchors[0].time === magnetCandidate.time && snapped.anchors[0].price === high.price, JSON.stringify({ snapped: snapped.anchors[0], candidate: magnetCandidate }));
  const outsideY = 4; const outsideBefore = await readDomain(); await page.getByTestId('drawing-tool-horizontal').click(); await page.mouse.click(currentPane.left + magnetCandidate.x, currentPane.top + outsideY); await waitForRevision(outsideBefore.revision + 1); await page.getByTestId('drawing-persistence-status').filter({ hasText: 'ready' }).waitFor(); const outside = (await readDomain()).drawings.at(-1);
  batch3('magnet.outside-threshold', !magnetCandidate.prices.some(item => item.price === outside.anchors[0].price), JSON.stringify({ outside: outside.anchors[0], candidate: magnetCandidate }));
  batch3('magnet.visible-only', magnetSnapshot.magnet.visibleCandles.length === candlePayload.length && magnetSnapshot.magnet.visibleCandles.at(-1).time === toDateKeyForUat(candlePayload.at(-1).timestamp), JSON.stringify({ visible: magnetSnapshot.magnet.visibleCandles.length, backend: candlePayload.length }));
  recordAction('drawing-magnet-lifecycle', 'off/OHLC/threshold/visible-only exercised through UI', 'drawing.magnet');

  const backendAfterTools = await (await page.request.get(`${backendUrl}/api/replay/sessions/${sessionId}/drawings`)).json();
  const localAfterTools = await page.evaluate(({ id, symbol }) => localStorage.getItem(`sumi:drawing-document:v1:${id}:${encodeURIComponent(symbol)}`), { id: sessionId, symbol: (await readDomain()).symbol });
  batch3('canonical-backend-local-equality', backendAfterTools.state_data === localAfterTools && JSON.stringify(JSON.parse(backendAfterTools.state_data)) === JSON.stringify(await readDomain()), JSON.stringify({ backend: backendAfterTools.state_data, local: localAfterTools }));
  await page.screenshot({ path: path.join(runDir, '10-all-tools-selected.png'), fullPage: true });

  const allToolsBeforeClear = await readDomain(); const allToolKinds = new Set(allToolsBeforeClear.drawings.map(item => item.tool));
  await page.getByTestId('clear-all-drawings').click(); batch3('clear-all-confirmation', (await readDomain()).drawings.length === allToolsBeforeClear.drawings.length && await page.getByTestId('confirm-clear-drawings').count() === 1, 'Confirmation required');
  await page.getByTestId('confirm-clear-drawings').click(); await waitForRevision(allToolsBeforeClear.revision + 1); batch3('clear-all-tools', (await readDomain()).drawings.length === 0 && ['horizontal', 'trendline', 'ray', 'rectangle', 'fibonacci-retracement', 'text'].every(tool => allToolKinds.has(tool)), [...allToolKinds].join(','));
  await page.getByTestId('undo-drawing').click(); await waitForRevision(allToolsBeforeClear.revision + 2); const allToolsRestored = await readDomain();
  batch3('undo-clear-all-tools-exact', JSON.stringify(allToolsRestored.drawings) === JSON.stringify(allToolsBeforeClear.drawings), JSON.stringify(allToolsRestored.drawings.map(item => ({ id: item.id, tool: item.tool }))));
  recordAction('drawing-all-tools-edit-history', `tools=${[...allToolKinds].sort().join(',')}; edit/delete/undo/redo exercised`, 'drawing.all-tools-edit-history');

  const allToolsBeforeReload = await readDomain(); await page.reload(); await page.locator('header').getByText(/Session #/).waitFor();
  await page.waitForFunction(() => document.querySelector('[data-testid="drawing-persistence-status"]')?.textContent === 'ready');
  const allToolsAfterReload = await readDomain(); batch3('all-tools-reload-equality-no-duplicates', JSON.stringify(allToolsAfterReload) === JSON.stringify(allToolsBeforeReload) && new Set(allToolsAfterReload.drawings.map(item => item.id)).size === allToolsAfterReload.drawings.length, JSON.stringify(allToolsAfterReload.drawings.map(item => ({ id: item.id, tool: item.tool }))));
  await page.screenshot({ path: path.join(runDir, '11-all-tools-reloaded.png'), fullPage: true });
  const stabilityPane = (await readInteraction()).pricePane; await page.mouse.move(stabilityPane.left + stabilityPane.width * .8, stabilityPane.top + stabilityPane.height * .85); await page.mouse.wheel(0, -300);
  await page.getByRole('button', { name: 'Next →', exact: true }).click(); await page.waitForTimeout(300); await page.getByRole('button', { name: '← Prev', exact: true }).click(); await page.waitForTimeout(300);
  const stableSnapshot = await readInteraction(); batch3('all-tools-zoom-replay-stability', stableSnapshot.drawings.length === allToolsAfterReload.drawings.length && stableSnapshot.drawings.every(item => item.anchors.every(anchor => anchor.x !== null && anchor.y !== null)), JSON.stringify(stableSnapshot.drawings.map(item => ({ id: item.id, tool: item.tool, anchors: item.anchors }))));
  await page.screenshot({ path: path.join(runDir, '12-all-tools-zoom-replay.png'), fullPage: true });
  const nonPriceAllBefore = await readDomain(); const stabilityBox = await workspace.boundingBox(); const lowerPaneY = Math.min(stabilityBox.y + stabilityBox.height - 15, stableSnapshot.pricePane.top + stableSnapshot.pricePane.height + 30);
  for (const tool of ['horizontal', 'trendline', 'ray', 'rectangle', 'fibonacci-retracement', 'text']) { await page.getByTestId(`drawing-tool-${tool}`).click(); await page.mouse.click(stableSnapshot.pricePane.left + stableSnapshot.pricePane.width * .5, lowerPaneY); }
  await page.getByTestId('drawing-tool-select').click(); batch3('all-tools-non-price-isolation', JSON.stringify(await readDomain()) === JSON.stringify(nonPriceAllBefore) && !(await readInteraction()).preview && await page.getByTestId('drawing-text-dialog').count() === 0, `lowerPaneY=${lowerPaneY}`);

  const indicatorResponsesBeforeLifecycle = indicatorResponses.length;
  const lifecycleIndicatorCycles = [];
  for (let cycle = 0; cycle < 10; cycle += 1) {
    await page.goto(`${frontendUrl}/analytics`); await page.goto(`${frontendUrl}/replay`); await page.locator('header').getByText(/Session #/).waitFor();
    await page.waitForTimeout(700);
    const document = await readIndicatorDomain(); const chart = await readIndicatorChart(); const runtime = await readIndicatorRuntime();
    lifecycleIndicatorCycles.push({ chart, layout: inspectIndicatorLayout(document, chart), runtime: inspectActiveRuntime(document, runtime) });
  }
  await page.waitForFunction(() => {
    const output = document.querySelector('[data-testid="drawing-domain-state"]');
    return output && JSON.parse(output.textContent || '{}').symbol === 'FPT';
  });
  const lifecycleBefore = await readDomain();
  await page.getByTestId('drawing-tool-horizontal').click();
  await page.getByTestId('drawing-tool-horizontal').waitFor();
  await page.waitForTimeout(300);
  const lifecyclePane = (await readInteraction()).pricePane;
  await page.mouse.click(lifecyclePane.left + lifecyclePane.width * 0.72, lifecyclePane.top + lifecyclePane.height * 0.42);
  await waitForRevision(lifecycleBefore.revision + 1);
  const lifecycleAfter = await readDomain();
  scoped('mount-unmount-10', lifecycleAfter.drawings.length === lifecycleBefore.drawings.length + 1 && lifecycleAfter.revision === lifecycleBefore.revision + 1, `10 cycles then one input: count ${lifecycleBefore.drawings.length}->${lifecycleAfter.drawings.length}; revision ${lifecycleBefore.revision}->${lifecycleAfter.revision}`);
  const lifecycleDrawingInteraction = await readInteraction(); batch3('hardening.ten-remount-one-gesture-response', lifecycleAfter.drawings.length === lifecycleBefore.drawings.length + 1
    && lifecycleAfter.revision === lifecycleBefore.revision + 1 && lifecycleDrawingInteraction.listenerCount === 6 && lifecycleDrawingInteraction.primitiveCount === 1,
  JSON.stringify({ before: { count: lifecycleBefore.drawings.length, revision: lifecycleBefore.revision }, after: { count: lifecycleAfter.drawings.length, revision: lifecycleAfter.revision }, listenerCount: lifecycleDrawingInteraction.listenerCount, primitiveCount: lifecycleDrawingInteraction.primitiveCount }));
  const lifecycleIndicatorChart = await readIndicatorChart();
  const lifecycleIndicatorResponses = indicatorResponses.slice(indicatorResponsesBeforeLifecycle);
  batch2('mount-unmount-10', lifecycleIndicatorChart.keys.length === beforeReloadIndicatorChart.keys.length
    && new Set(lifecycleIndicatorChart.keys).size === lifecycleIndicatorChart.keys.length
    && lifecycleIndicatorCycles.every(state => state.chart.keys.length === beforeReloadIndicatorChart.keys.length && new Set(state.chart.keys).size === state.chart.keys.length)
    && lifecycleIndicatorResponses.length === 40
    && lifecycleIndicatorResponses.every(response => response.status === 200),
  JSON.stringify({ responses: lifecycleIndicatorResponses.length, cycles: lifecycleIndicatorCycles.map(state => state.chart.keys), chart: lifecycleIndicatorChart }));
  batch2('layout-every-remount', lifecycleIndicatorCycles.every(state => state.layout.pass), JSON.stringify(lifecycleIndicatorCycles.map(state => state.layout)));
  batch2('runtime-every-remount-no-active-error', lifecycleIndicatorCycles.every(state => state.runtime.pass), JSON.stringify(lifecycleIndicatorCycles.map(state => state.runtime)));

  // Batch 4: one integrated, server-authoritative practice workflow. Every action below
  // uses the visible workstation; direct reads are evidence only.
  const batch4 = (id, pass, evidence) => check(`batch4.${id}`, pass, evidence);
  const readPractice = async () => {
    const value = await page.getByTestId('practice-workflow-state').textContent();
    if (!value) return null;
    try { return JSON.parse(value); } catch { return null; }
  };
  const readTradeMarkers = async () => JSON.parse(await page.getByTestId('trade-marker-state').textContent());
  const readJournal = async () => (await page.request.get(`${backendUrl}/api/replay/sessions/${sessionId}/journal`)).json();
  const waitPractice = async (predicate, description) => {
    let snapshot;
    for (let attempt = 0; attempt < 80; attempt += 1) {
      snapshot = await readPractice();
      if (snapshot && predicate(snapshot)) return snapshot;
      await page.waitForTimeout(100);
    }
    throw new Error(`Timed out waiting for practice state: ${description}; last=${JSON.stringify(snapshot)}`);
  };
  const openPracticeTab = async name => {
    await page.getByRole('tab', { name, exact: true }).click();
    await page.getByRole('tab', { name, exact: true }).waitFor();
  };
  const submitPracticeDecision = async (action, fields = {}) => {
    await page.getByRole('button', { name: action, exact: true }).click();
    const dialog = page.getByTestId('trade-decision-dialog');
    await dialog.waitFor();
    if (fields.quantity !== undefined) await dialog.getByLabel('Quantity').fill(String(fields.quantity));
    if (fields.orderType) await dialog.getByLabel('Order type').selectOption(fields.orderType);
    if (fields.limitPrice !== undefined) await dialog.getByLabel('Limit price').fill(String(fields.limitPrice));
    if (fields.setup) await dialog.getByLabel('Setup').selectOption(fields.setup);
    if (fields.reason) await dialog.getByLabel('Reason').fill(fields.reason);
    await dialog.getByRole('button', { name: `Submit ${action}`, exact: true }).click();
    return dialog;
  };
  const saveChecklist = async observation => {
    await openPracticeTab('Journal');
    await page.getByRole('button', { name: 'New observation', exact: true }).click();
    const dialog = page.getByTestId('practice-checklist-dialog');
    await dialog.getByLabel('Trend identified').check();
    await dialog.getByLabel('Risk defined').check();
    await dialog.getByLabel('Checklist observation').fill(observation);
    await dialog.getByRole('button', { name: 'Save checklist', exact: true }).click();
    await dialog.waitFor({ state: 'detached' });
    await page.getByText(observation, { exact: true }).waitFor();
  };
  const roundPrice = value => Math.round(value * 100) / 100;
  const demoFptCandle = index => {
    const amplitude = 3_600;
    const close = roundPrice(80_000 + index * 42 + Math.sin(index / 9) * amplitude + Math.sin(index / 31) * amplitude * 1.6);
    const previousClose = index === 0
      ? 80_000
      : roundPrice(80_000 + (index - 1) * 42 + Math.sin((index - 1) / 9) * amplitude + Math.sin((index - 1) / 31) * amplitude * 1.6);
    const open = roundPrice(previousClose + Math.sin(index / 4) * amplitude * 0.2);
    return {
      open,
      high: roundPrice(Math.max(open, close) + amplitude * 0.35 + Math.abs(Math.sin(index)) * amplitude * 0.25),
      low: roundPrice(Math.min(open, close) - amplitude * 0.35 - Math.abs(Math.sin(index / 2)) * amplitude * 0.2),
      close,
    };
  };
  const findIntermediateOnlyLimit = currentIndex => {
    const future = Array.from({ length: 5 }, (_, offset) => demoFptCandle(currentIndex + offset + 1));
    const destination = future[4];
    const referenceClose = demoFptCandle(currentIndex - 1).close;
    const floor = referenceClose * 0.93;
    const ceiling = referenceClose * 1.07;
    for (let offset = 0; offset < 4; offset += 1) {
      const candle = future[offset];
      const candidates = [candle.low, candle.high, candle.close, candle.open, roundPrice((candle.low + candle.high) / 2)];
      for (const price of candidates) {
        const earliestOffset = future.findIndex(item => item.low <= price && price <= item.high);
        const destinationEligible = destination.low <= price && price <= destination.high;
        if (earliestOffset === offset && !destinationEligible && price >= floor && price <= ceiling) {
          return { price, expectedOffset: offset + 1 };
        }
      }
    }
    throw new Error(`No deterministic intermediate-only LIMIT price at index ${currentIndex}`);
  };

  await page.setViewportSize({ width: 1440, height: 1000 });
  await openPracticeTab('Trade');
  const practiceStart = await waitPractice(state => state && state.current_index >= 0, 'initial Batch 4 snapshot');
  const wideWorkspace = await page.getByTestId('chart-workspace').boundingBox();
  const wideRail = await page.getByTestId('practice-rail').boundingBox();
  const practiceHeader = await page.locator('header').innerText();
  const tradeContext = await page.getByTestId('trade-context').innerText();
  batch4('T-01.integrated-wide-layout', !!wideWorkspace && !!wideRail && wideWorkspace.width > 700 && wideRail.width >= 250
    && await page.getByRole('tab', { name: 'Journal', exact: true }).count() === 1
    && await page.locator('.limited-workstation-warning').isHidden(),
  JSON.stringify({ chart: wideWorkspace, rail: wideRail }));
  batch4('R-02.exact-visible-context', practiceHeader.includes(`#${practiceStart.visible_bar}/${practiceStart.total_bars}`)
    && practiceHeader.includes('O:') && practiceHeader.includes('H:') && practiceHeader.includes('L:') && practiceHeader.includes('C:') && practiceHeader.includes('V:')
    && tradeContext.includes(`Bar ${practiceStart.visible_bar}/${practiceStart.total_bars}`) && tradeContext.includes(practiceStart.current_date.slice(0, 10)),
  JSON.stringify({ header: practiceHeader, rail: tradeContext, snapshot: { index: practiceStart.current_index, date: practiceStart.current_date, volume: practiceStart.current_volume } }));

  // Client validation must fail before transport.
  let decisionPostCount = 0;
  const decisionRoute = '**/api/replay/sessions/*/decisions';
  await page.route(decisionRoute, async route => {
    if (route.request().method() === 'POST') decisionPostCount += 1;
    await route.continue();
  });
  await page.getByRole('button', { name: 'BUY', exact: true }).click();
  let decisionDialog = page.getByTestId('trade-decision-dialog');
  await decisionDialog.getByLabel('Quantity').fill('0');
  await decisionDialog.getByRole('button', { name: 'Submit BUY', exact: true }).click();
  const validationMessage = await decisionDialog.getByRole('alert').innerText();
  batch4('G-02.validation-before-api', decisionPostCount === 0 && validationMessage.includes('greater than zero'), validationMessage);
  await decisionDialog.getByLabel('Quantity').fill('100');
  await decisionDialog.getByLabel('Setup').selectOption('Breakout');
  await decisionDialog.getByLabel('Reason').fill('Batch 4 market entry');
  const submitBuy = decisionDialog.getByRole('button', { name: 'Submit BUY', exact: true });
  await submitBuy.evaluate(element => { element.click(); element.click(); });
  await decisionDialog.waitFor({ state: 'detached' });
  const afterMarketBuy = await waitPractice(state => state.executions.length === practiceStart.executions.length + 1, 'market BUY execution');
  batch4('G-03.duplicate-submit-deduped', decisionPostCount === 1 && afterMarketBuy.decisions.length === practiceStart.decisions.length + 1
    && afterMarketBuy.orders.length === practiceStart.orders.length + 1 && afterMarketBuy.executions.length === practiceStart.executions.length + 1,
  JSON.stringify({ decisionPostCount, decisions: afterMarketBuy.decisions, orders: afterMarketBuy.orders, executions: afterMarketBuy.executions }));
  await page.unroute(decisionRoute);
  batch4('T-02.buy-position-pnl', afterMarketBuy.positions.length === 1 && afterMarketBuy.positions[0].quantity === 100
    && afterMarketBuy.positions[0].available_quantity === 0 && Number.isFinite(afterMarketBuy.positions[0].unrealized_pnl)
    && afterMarketBuy.current_cash < afterMarketBuy.initial_cash,
  JSON.stringify({ position: afterMarketBuy.positions[0], cash: afterMarketBuy.current_cash }));

  const beforeNonOrders = afterMarketBuy;
  await submitPracticeDecision('HOLD', { setup: 'Trend Follow', reason: 'Wait for confirmation' });
  await page.getByTestId('trade-decision-dialog').waitFor({ state: 'detached' });
  await submitPracticeDecision('SKIP', { reason: 'No clean setup' });
  await page.getByTestId('trade-decision-dialog').waitFor({ state: 'detached' });
  const afterNonOrders = await waitPractice(state => state.decisions.length === beforeNonOrders.decisions.length + 2, 'HOLD and SKIP');
  batch4('T-03.hold-skip-no-order-or-marker', afterNonOrders.orders.length === beforeNonOrders.orders.length
    && afterNonOrders.executions.length === beforeNonOrders.executions.length
    && ['HOLD', 'SKIP'].every(action => afterNonOrders.decisions.some(item => item.action === action)),
  JSON.stringify({ decisions: afterNonOrders.decisions, orders: afterNonOrders.orders, executions: afterNonOrders.executions }));

  const baseIndex = afterNonOrders.current_index;
  await saveChecklist('Batch 4 entry checklist');
  const journalAtEntry = await readJournal();
  batch4('T-04.context-checklist-created', journalAtEntry.length === 1 && journalAtEntry[0].note_type === 'practice_checklist'
    && JSON.parse(journalAtEntry[0].content).context.candleIndex === baseIndex,
  JSON.stringify(journalAtEntry));
  await openPracticeTab('Trade');

  expectingPracticeRejection = true;
  decisionDialog = await submitPracticeDecision('SELL', { quantity: 100, reason: 'Controlled pre-T+2 rejection' });
  const t2Message = await decisionDialog.getByRole('alert').innerText();
  expectingPracticeRejection = false;
  const afterRejectedSell = await readPractice();
  batch4('T-05.pre-t2-backend-rejection', t2Message.includes('T+2') && afterRejectedSell.decisions.length === afterNonOrders.decisions.length
    && afterRejectedSell.orders.length === afterNonOrders.orders.length && afterRejectedSell.executions.length === afterNonOrders.executions.length,
  JSON.stringify({ message: t2Message, available: afterRejectedSell.available_quantity }));
  batch4('G-05.expected-rejection-contained', expectedPracticeConsoleErrors.length === 1,
  JSON.stringify(expectedPracticeConsoleErrors));
  await page.screenshot({ path: path.join(runDir, '17-batch4-t2-rejection.png'), fullPage: true });
  await decisionDialog.getByRole('button', { name: 'Cancel', exact: true }).click();

  const crossedLimitFixture = findIntermediateOnlyLimit(baseIndex);
  decisionDialog = await submitPracticeDecision('BUY', { quantity: 100, orderType: 'LIMIT', limitPrice: crossedLimitFixture.price, setup: 'Pullback', reason: 'Batch 4 pending limit path' });
  await decisionDialog.waitFor({ state: 'detached' });
  const pendingLimit = await waitPractice(state => state.orders.some(order => order.order_type === 'LIMIT' && order.status === 'pending'), 'pending LIMIT');
  const limitOrder = pendingLimit.orders.find(order => order.order_type === 'LIMIT');
  batch4('T-06.limit-pending', !!limitOrder && limitOrder.status === 'pending' && pendingLimit.executions.length === afterNonOrders.executions.length,
  JSON.stringify(limitOrder));
  await page.screenshot({ path: path.join(runDir, '16-batch4-integrated-position-pending-journal.png'), fullPage: true });

  await page.getByRole('button', { name: '+5', exact: true }).click();
  let filledLimit = await waitPractice(state => state.current_index === pendingLimit.current_index + 5, '+5 LIMIT traversal');
  const limitExecutions = filledLimit.executions.filter(item => item.order_id === limitOrder.id);
  batch4('T-07.limit-filled-on-visible-candle', limitExecutions.length === 1 && filledLimit.orders.find(order => order.id === limitOrder.id)?.status === 'executed'
    && filledLimit.positions[0]?.quantity === 200,
  JSON.stringify({ order: filledLimit.orders.find(order => order.id === limitOrder.id), executions: limitExecutions, position: filledLimit.positions[0] }));

  const visibleAfterFive = await (await page.request.get(`${backendUrl}/api/replay/sessions/${sessionId}/candles`)).json();
  const crossedCandles = visibleAfterFive.slice(baseIndex + 1, baseIndex + 6);
  const actualEligibleOffset = crossedCandles.findIndex(candle => Number(candle.low) <= crossedLimitFixture.price && crossedLimitFixture.price <= Number(candle.high));
  const limitExecution = limitExecutions[0];
  const executionCandle = crossedCandles[actualEligibleOffset];
  const expectedExecutionPrice = Math.min(crossedLimitFixture.price, Number(executionCandle?.open));
  const expectedLimitMarker = {
    time: limitExecution?.execution_date.slice(0, 10),
    position: 'belowBar',
    color: '#00E676',
    shape: 'arrowUp',
    text: `BUY ${limitExecution?.quantity}`,
  };
  const filledMarkers = await readTradeMarkers();
  batch4('closure.R01.plus5-intermediate-fill-earliest', limitExecutions.length === 1
    && actualEligibleOffset === crossedLimitFixture.expectedOffset - 1
    && !(Number(crossedCandles[4].low) <= crossedLimitFixture.price && crossedLimitFixture.price <= Number(crossedCandles[4].high))
    && limitExecution?.execution_index === baseIndex + crossedLimitFixture.expectedOffset
    && limitExecution?.execution_date.slice(0, 10) === executionCandle?.timestamp.slice(0, 10),
  JSON.stringify({ fixture: crossedLimitFixture, actualEligibleOffset: actualEligibleOffset + 1, destination: crossedCandles[4], execution: limitExecution }));
  batch4('closure.R01.plus5-exact-lifecycle', limitExecutions.length === 1
    && Number(limitExecution?.execution_price) === expectedExecutionPrice
    && filledLimit.positions[0]?.quantity === 200
    && filledLimit.positions[0]?.available_quantity === 200
    && filledMarkers.filter(marker => JSON.stringify(marker) === JSON.stringify(expectedLimitMarker)).length === 1
    && Math.abs(filledLimit.current_cash - (pendingLimit.current_cash - Number(limitExecution?.net_amount))) < 0.01,
  JSON.stringify({ expectedExecutionPrice, execution: limitExecution, marker: expectedLimitMarker, markers: filledMarkers, cashBefore: pendingLimit.current_cash, cashAfter: filledLimit.current_cash, position: filledLimit.positions[0] }));
  await page.screenshot({ path: path.join(runDir, '21-batch4-closure-plus5-intermediate-fill.png'), fullPage: true });

  while (filledLimit.current_index < baseIndex + 2) {
    await page.getByRole('button', { name: 'Next →', exact: true }).click();
    filledLimit = await waitPractice(state => state.current_index > filledLimit.current_index, 'T+2 advance');
  }
  batch4('T-08.t2-available-later', filledLimit.available_quantity === 200 && filledLimit.positions[0]?.available_quantity === 200,
  JSON.stringify({ index: filledLimit.current_index, baseIndex, available: filledLimit.available_quantity, position: filledLimit.positions[0] }));
  decisionDialog = await submitPracticeDecision('CLOSE', { reason: 'Close after settlement' });
  await decisionDialog.waitFor({ state: 'detached' });
  const closedState = await waitPractice(state => state.positions.length === 0 && state.executions.length === filledLimit.executions.length + 1, 'settled CLOSE');
  batch4('T-09.close-realized-pnl', closedState.positions.length === 0 && closedState.trades.some(trade => trade.status === 'closed')
    && closedState.orders.at(-1)?.side === 'SELL' && closedState.executions.at(-1)?.side === 'SELL',
  JSON.stringify({ trades: closedState.trades, finalExecution: closedState.executions.at(-1), cash: closedState.current_cash }));
  recordAction('trade-complete-lifecycle', 'BUY/HOLD/SKIP/LIMIT/fill/T+2 rejection/CLOSE completed through UI', 'trade.lifecycle');
  await page.screenshot({ path: path.join(runDir, '18-batch4-settled-closed.png'), fullPage: true });

  const journalBeforeWrongDate = await readJournal();
  const expectedConsoleErrorsBeforeWrongDate = expectedPracticeConsoleErrors.length;
  expectingPracticeRejection = true;
  const wrongDateResponse = await page.evaluate(async ({ id, context }) => {
    const response = await fetch(`/api/replay/sessions/${id}/journal`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        note_type: 'practice_checklist',
        content: JSON.stringify({
          schemaVersion: 1,
          context: { ...context, date: '2099-12-31' },
          checks: {
            trendIdentified: true,
            setupConfirmed: false,
            entryTriggerDefined: false,
            riskDefined: true,
            exitPlanDefined: false,
            emotionChecked: true,
          },
          observation: 'Batch 4 rejected mismatched date',
        }),
      }),
    });
    return { status: response.status, body: await response.text() };
  }, {
    id: sessionId,
    context: { sessionId, symbol: closedState.symbol, candleIndex: closedState.current_index, date: closedState.current_date.slice(0, 10) },
  });
  await page.waitForTimeout(100);
  expectingPracticeRejection = false;
  const journalAfterWrongDate = await readJournal();
  batch4('closure.R02.mismatched-date-rejected-no-insert', wrongDateResponse.status === 409
    && JSON.stringify(journalAfterWrongDate.map(item => item.id)) === JSON.stringify(journalBeforeWrongDate.map(item => item.id)),
  JSON.stringify({ response: wrongDateResponse, before: journalBeforeWrongDate.map(item => item.id), after: journalAfterWrongDate.map(item => item.id) }));
  batch4('closure.R03.expected-date-rejection-contained', expectedPracticeConsoleErrors.length === expectedConsoleErrorsBeforeWrongDate + 1,
  JSON.stringify(expectedPracticeConsoleErrors));

  await saveChecklist('Batch 4 exit review');
  const finalJournal = await readJournal();
  recordAction('journal-checklist-lifecycle', `saved entries=${finalJournal.length}; authoritative checklist context retained`, 'journal.checklist');
  const finalIndex = closedState.current_index;
  const finalIndicatorDocument = await readIndicatorDomain();
  const finalDrawingDocument = await readDomain();
  const exitChecklist = finalJournal.find(item => item.content.includes('Batch 4 exit review'));
  batch4('closure.R02.authoritative-date-saves-and-reloads', !!exitChecklist
    && JSON.parse(exitChecklist.content).context.date === closedState.current_date.slice(0, 10), JSON.stringify(exitChecklist));
  await openPracticeTab('Trade');
  await page.getByRole('button', { name: '← Prev', exact: true }).click();
  const beforeCloseProjection = await waitPractice(state => state.current_index === finalIndex - 1, 'rewind before close');
  batch4('R-04.rewind-hides-future-close', beforeCloseProjection.historical && !beforeCloseProjection.can_trade
    && beforeCloseProjection.executions.length === closedState.executions.length - 1 && beforeCloseProjection.positions.length === 1
    && await page.getByTestId('historical-trade-block').count() === 1,
  JSON.stringify({ snapshot: beforeCloseProjection, block: await page.getByTestId('historical-trade-block').innerText() }));

  while ((await readPractice()).current_index > baseIndex) {
    const priorIndex = (await readPractice()).current_index;
    await page.getByRole('button', { name: '← Prev', exact: true }).click();
    await waitPractice(state => state.current_index === priorIndex - 1, 'rewind to decision bar');
  }
  const rewoundBase = await readPractice();
  const rewoundMarkers = await readTradeMarkers();
  const rewoundJournal = await readJournal();
  batch4('R-05.rewind-limit-projection', rewoundBase.current_index === baseIndex
    && rewoundBase.orders.find(order => order.id === limitOrder.id)?.status === 'pending'
    && !rewoundBase.executions.some(item => item.order_id === limitOrder.id)
    && rewoundBase.positions[0]?.quantity === 100,
  JSON.stringify({ order: rewoundBase.orders.find(order => order.id === limitOrder.id), executions: rewoundBase.executions, position: rewoundBase.positions[0] }));
  batch4('T-10.rewind-journal-context', rewoundJournal.length === 1 && rewoundJournal[0].id === journalAtEntry[0].id,
  JSON.stringify(rewoundJournal));

  await page.getByRole('button', { name: '+5', exact: true }).click();
  const restoredFinal = await waitPractice(state => state.current_index === finalIndex && state.executions.length === closedState.executions.length, 'forward restoration');
  const restoredJournal = await readJournal();
  batch4('R-06.forward-restores-exactly-once', JSON.stringify(restoredFinal.decisions.map(item => item.id)) === JSON.stringify(closedState.decisions.map(item => item.id))
    && JSON.stringify(restoredFinal.orders.map(item => item.id)) === JSON.stringify(closedState.orders.map(item => item.id))
    && JSON.stringify(restoredFinal.executions.map(item => item.id)) === JSON.stringify(closedState.executions.map(item => item.id))
    && new Set(restoredFinal.executions.map(item => item.id)).size === restoredFinal.executions.length,
  JSON.stringify({ decisions: restoredFinal.decisions.map(item => item.id), orders: restoredFinal.orders.map(item => item.id), executions: restoredFinal.executions.map(item => item.id) }));
  batch4('closure.R01.rewind-plus5-restores-limit-once', restoredFinal.executions.filter(item => item.id === limitExecution.id).length === 1
    && restoredFinal.executions.filter(item => item.order_id === limitOrder.id).length === 1
    && rewoundMarkers.filter(marker => JSON.stringify(marker) === JSON.stringify(expectedLimitMarker)).length === 0
    && (await readTradeMarkers()).filter(marker => JSON.stringify(marker) === JSON.stringify(expectedLimitMarker)).length === 1,
  JSON.stringify({ executions: restoredFinal.executions.filter(item => item.order_id === limitOrder.id), rewoundMarkers, restoredMarkers: await readTradeMarkers() }));
  batch4('T-11.forward-restores-journal', restoredJournal.length === finalJournal.length && restoredJournal.some(item => item.content.includes('Batch 4 exit review')), JSON.stringify(restoredJournal));

  await page.reload(); await page.locator('header').getByText(/Session #/).waitFor();
  const reloadedPractice = await waitPractice(state => state.current_index === finalIndex, 'reload/resume practice state');
  const reloadedJournal = await readJournal();
  batch4('G-04.reload-resume-full-state', JSON.stringify(reloadedPractice.executions.map(item => item.id)) === JSON.stringify(restoredFinal.executions.map(item => item.id))
    && reloadedJournal.length === restoredJournal.length && JSON.stringify(await readIndicatorDomain()) === JSON.stringify(finalIndicatorDocument)
    && JSON.stringify(await readDomain()) === JSON.stringify(finalDrawingDocument),
  JSON.stringify({ practice: { index: reloadedPractice.current_index, decisions: reloadedPractice.decisions.length, orders: reloadedPractice.orders.length, executions: reloadedPractice.executions.length }, journal: reloadedJournal.length }));
  batch4('closure.R01.reload-preserves-limit-once', reloadedPractice.executions.filter(item => item.id === limitExecution.id).length === 1
    && reloadedPractice.executions.filter(item => item.order_id === limitOrder.id).length === 1
    && (await readTradeMarkers()).filter(marker => JSON.stringify(marker) === JSON.stringify(expectedLimitMarker)).length === 1,
  JSON.stringify({ executions: reloadedPractice.executions.filter(item => item.order_id === limitOrder.id), markers: await readTradeMarkers() }));
  batch4('closure.R02.authoritative-date-present-after-reload', reloadedJournal.some(item => item.id === exitChecklist.id
    && JSON.parse(item.content).context.date === closedState.current_date.slice(0, 10)), JSON.stringify(reloadedJournal));
  await openPracticeTab('Journal');
  await page.getByText('Batch 4 exit review', { exact: true }).waitFor();
  await page.screenshot({ path: path.join(runDir, '19-batch4-reload-resume.png'), fullPage: true });

  // Batch 5: additive keyboard/accessibility evidence, exercised with real browser input.
  const batch5 = (id, pass, evidence) => check(`batch5.${id}`, pass, evidence);
  const tabSemantics = await page.getByRole('tab').evaluateAll(nodes => nodes.map(node => ({
    id: node.id, controls: node.getAttribute('aria-controls'), selected: node.getAttribute('aria-selected'), name: node.textContent?.trim(),
    panelLabel: document.getElementById(node.getAttribute('aria-controls') || '')?.getAttribute('aria-labelledby'),
  })));
  batch5('accessibility.tab-relations', tabSemantics.length === 4 && tabSemantics.every(item => item.id && item.controls && item.panelLabel === item.id)
    && tabSemantics.filter(item => item.selected === 'true').length === 1, JSON.stringify(tabSemantics));
  const tabIsolationBefore = { practice: await readPractice(), drawings: JSON.stringify(await readDomain()) };
  await page.getByRole('tab', { name: 'Trade', exact: true }).focus();
  await page.keyboard.press('ArrowRight');
  batch5('accessibility.tab-arrow-keyboard', await page.getByRole('tab', { name: 'Journal', exact: true }).getAttribute('aria-selected') === 'true'
    && await page.getByRole('tab', { name: 'Journal', exact: true }).evaluate(node => node === document.activeElement), 'Trade -> ArrowRight -> Journal');
  await page.keyboard.press('Home');
  const homeSelectedTrade = await page.getByRole('tab', { name: 'Trade', exact: true }).getAttribute('aria-selected') === 'true';
  await page.keyboard.press('End');
  const endSelectedDrawing = await page.getByRole('tab', { name: 'Drawing', exact: true }).getAttribute('aria-selected') === 'true';
  const tabIsolationAfter = { practice: await readPractice(), drawings: JSON.stringify(await readDomain()) };
  const tabStateIsolated = tabIsolationAfter.practice.current_index === tabIsolationBefore.practice.current_index
    && tabIsolationAfter.drawings === tabIsolationBefore.drawings && homeSelectedTrade && endSelectedDrawing;

  await openPracticeTab('Trade');
  const buyOpener = page.getByRole('button', { name: 'BUY', exact: true });
  await buyOpener.click();
  const tradeA11yDialog = page.getByTestId('trade-decision-dialog');
  const tradeNamed = await tradeA11yDialog.getAttribute('aria-label');
  const tradeFirstFocused = await tradeA11yDialog.getByLabel('Quantity').evaluate(node => node === document.activeElement);
  await page.keyboard.press('Shift+Tab');
  const tradeWrapped = await tradeA11yDialog.getByRole('button', { name: 'Submit BUY', exact: true }).evaluate(node => node === document.activeElement);
  await page.keyboard.press('Escape');
  batch5('accessibility.trade-dialog-focus', !!tradeNamed && tradeFirstFocused && tradeWrapped && await buyOpener.evaluate(node => node === document.activeElement),
    JSON.stringify({ tradeNamed, tradeFirstFocused, tradeWrapped }));

  await openPracticeTab('Journal');
  const journalOpener = page.getByRole('button', { name: 'New observation', exact: true });
  await journalOpener.click();
  const journalA11yDialog = page.getByTestId('practice-checklist-dialog');
  const journalFirstFocused = await journalA11yDialog.getByLabel('Trend identified').evaluate(node => node === document.activeElement);
  await page.keyboard.press('Shift+Tab');
  const journalWrapped = await journalA11yDialog.getByRole('button', { name: 'Save checklist', exact: true }).evaluate(node => node === document.activeElement);
  await page.keyboard.press('Escape');
  batch5('accessibility.journal-dialog-focus', journalFirstFocused && journalWrapped && await journalOpener.evaluate(node => node === document.activeElement),
    JSON.stringify({ journalFirstFocused, journalWrapped }));

  const indicatorOpener = page.getByTestId('open-add-indicator');
  await indicatorOpener.click();
  const indicatorA11yDialog = page.getByTestId('indicator-dialog');
  const indicatorNamed = await indicatorA11yDialog.getAttribute('aria-labelledby');
  const indicatorFirstFocused = await indicatorA11yDialog.evaluate(node => node.contains(document.activeElement) && document.activeElement?.getAttribute('aria-label') === 'Close indicator dialog');
  await page.keyboard.press('Shift+Tab');
  const indicatorWrapped = await indicatorA11yDialog.evaluate(node => node.contains(document.activeElement) && document.activeElement?.getAttribute('aria-label') !== 'Close indicator dialog');
  await page.keyboard.press('Escape');
  batch5('accessibility.indicator-dialog-focus', !!indicatorNamed && indicatorFirstFocused && indicatorWrapped && await indicatorOpener.evaluate(node => node === document.activeElement),
    JSON.stringify({ indicatorNamed, indicatorFirstFocused, indicatorWrapped }));

  // Buttons perform their own action once; global replay shortcuts must not double-handle the same key.
  const buttonDrawingBytes = JSON.stringify(await readDomain());
  const buttonStart = await readPractice();
  const nextButton = page.getByRole('button', { name: 'Next →', exact: true });
  await nextButton.focus(); await page.keyboard.press('Space');
  const afterButtonSpace = await waitPractice(state => state.current_index === buttonStart.current_index + 1, 'Space on Next button exactly once');
  await page.getByRole('button', { name: '← Prev', exact: true }).focus(); await page.keyboard.press('Enter');
  const afterButtonEnter = await waitPractice(state => state.current_index === buttonStart.current_index, 'Enter on Prev button exactly once');
  const buttonKeysIsolated = afterButtonSpace.current_index === buttonStart.current_index + 1
    && afterButtonEnter.current_index === buttonStart.current_index && JSON.stringify(await readDomain()) === buttonDrawingBytes;

  const isolatedEditableCases = [];
  const assertIsolatedKeys = async (label, locator, keys) => {
    const before = { index: (await readPractice()).current_index, drawingBytes: JSON.stringify(await readDomain()) };
    await locator.focus();
    for (const key of keys) await page.keyboard.press(key);
    const after = { index: (await readPractice()).current_index, drawingBytes: JSON.stringify(await readDomain()) };
    isolatedEditableCases.push({ label, pass: before.index === after.index && before.drawingBytes === after.drawingBytes, before, after });
  };
  await openPracticeTab('Trade'); await buyOpener.click();
  await assertIsolatedKeys('trade-dialog-form-input', page.getByTestId('trade-decision-dialog').getByLabel('Quantity'), ['ArrowRight', 'Delete']);
  await page.keyboard.press('Escape');
  await openPracticeTab('Journal'); await journalOpener.click();
  await assertIsolatedKeys('journal-dialog-textarea', page.getByTestId('practice-checklist-dialog').getByLabel('Checklist observation'), ['ArrowLeft', 'Space', 'Delete']);
  await page.keyboard.press('Escape');
  await indicatorOpener.click();
  await assertIsolatedKeys('indicator-dialog-input', page.getByTestId('indicator-search'), ['ArrowRight', 'Space', 'Backspace']);
  await page.keyboard.press('Escape');
  await selectDrawing(createdId); await openPracticeTab('Drawing');
  await assertIsolatedKeys('drawing-inspector-field', page.getByTestId('drawing-line-width'), ['ArrowRight', 'Delete', 'Escape']);
  const contenteditable = page.locator('[data-testid="batch5-contenteditable"]');
  await page.getByTestId('practice-rail').evaluate(node => { const editable = document.createElement('div'); editable.contentEditable = 'true'; editable.dataset.testid = 'batch5-contenteditable'; editable.textContent = 'editable closure fixture'; node.append(editable); });
  await assertIsolatedKeys('contenteditable', contenteditable, ['ArrowRight', 'Space', 'Delete', 'Escape']);
  await contenteditable.evaluate(node => node.remove());

  // Intended non-interactive workspace shortcuts each mutate exactly one domain action.
  const chartBackground = page.getByTestId('chart-workspace');
  await chartBackground.evaluate(node => { node.setAttribute('tabindex', '-1'); node.focus(); });
  const backgroundStart = await readPractice(); await page.keyboard.press('Space');
  const backgroundSpace = await waitPractice(state => state.current_index === backgroundStart.current_index + 1, 'background Space exactly once');
  await chartBackground.focus(); await page.keyboard.press('ArrowLeft');
  const backgroundArrow = await waitPractice(state => state.current_index === backgroundStart.current_index, 'background ArrowLeft exactly once');
  await selectDrawing(createdId);
  const keyboardSelectedId = (await readInteraction()).selectedIds[0];
  if (!keyboardSelectedId) throw new Error('No selected drawing available for intended background Delete');
  const beforeDelete = await readDomain(); await chartBackground.focus(); await page.keyboard.press('Delete');
  await waitForRevision(beforeDelete.revision + 1); const afterDelete = await readDomain(); const deleteExactlyOne = afterDelete.drawings.length === beforeDelete.drawings.length - 1 && !findDrawing(afterDelete, keyboardSelectedId);
  await page.getByTestId('undo-drawing').click(); await waitForRevision(afterDelete.revision + 1); const afterDeleteUndo = await readDomain();
  const restoredIds = JSON.stringify(afterDeleteUndo.drawings.map(item => item.id)) === JSON.stringify(beforeDelete.drawings.map(item => item.id));
  await page.getByTestId('drawing-tool-horizontal').click(); const escapeBefore = await readDomain(); await chartBackground.focus(); await page.keyboard.press('Escape');
  const escapeInteraction = await readInteraction(); const escapeExactlyOnce = escapeInteraction.tool === 'select' && JSON.stringify(await readDomain()) === JSON.stringify(escapeBefore);
  const intendedBackgroundExactlyOnce = backgroundSpace.current_index === backgroundStart.current_index + 1 && backgroundArrow.current_index === backgroundStart.current_index
    && deleteExactlyOne && restoredIds && escapeExactlyOnce;
  check('batch5.closure.R01.keyboard-isolation', tabStateIsolated && buttonKeysIsolated && isolatedEditableCases.every(item => item.pass) && intendedBackgroundExactlyOnce,
    JSON.stringify({ canonicalRedIndices: ['70->71 at iteration 5', '72->73 at iteration 15'], tabStateIsolated, buttonKeysIsolated, isolatedEditableCases, intendedBackgroundExactlyOnce,
      background: { start: backgroundStart.current_index, space: backgroundSpace.current_index, arrow: backgroundArrow.current_index, keyboardSelectedId, deleteExactlyOne, restoredIds, escapeExactlyOnce } }));

  const measureKeyboardFocus = async (name, locator) => {
    const beforeEvidence = await locator.evaluate(node => {
    const styleOf = () => { const style = getComputedStyle(node); return { outlineStyle: style.outlineStyle, outlineWidth: style.outlineWidth, outlineColor: style.outlineColor, boxShadow: style.boxShadow }; };
    const rgba = value => { if (!value || value === 'transparent') return { r: 0, g: 0, b: 0, a: 0 }; const parts = value.match(/[\d.]+/g)?.map(Number) ?? []; return { r: parts[0] ?? 0, g: parts[1] ?? 0, b: parts[2] ?? 0, a: parts.length > 3 ? parts[3] : 1 }; };
    const composite = (front, back) => ({ r: front.r * front.a + back.r * (1 - front.a), g: front.g * front.a + back.g * (1 - front.a), b: front.b * front.a + back.b * (1 - front.a), a: 1 });
    let adjacent = { r: 255, g: 255, b: 255, a: 1 }; const layers = [];
    for (let current = node.parentElement; current; current = current.parentElement) layers.push(rgba(getComputedStyle(current).backgroundColor));
    for (const layer of layers.reverse()) adjacent = composite(layer, adjacent);
    const luminance = color => [color.r, color.g, color.b].map(channel => { const value = channel / 255; return value <= .04045 ? value / 12.92 : ((value + .055) / 1.055) ** 2.4; }).reduce((sum, value, index) => sum + value * [.2126, .7152, .0722][index], 0);
    node.blur(); const before = styleOf();
    const focusable = [...document.querySelectorAll('button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])')];
    const index = focusable.indexOf(node); (focusable[index - 1] || document.body).focus();
    return { before, adjacent, focusFrom: index > 0 ? focusable[index - 1].getAttribute('data-testid') || focusable[index - 1].textContent?.trim() : 'body', luminance: luminance(adjacent) };
    });
    await page.keyboard.press('Tab');
    await page.waitForTimeout(300); // allow the declared 200ms focus transition to reach its measured final style
    const afterEvidence = await locator.evaluate((node, prior) => {
    const style = getComputedStyle(node); const after = { outlineStyle: style.outlineStyle, outlineWidth: style.outlineWidth, outlineColor: style.outlineColor, boxShadow: style.boxShadow };
    const parts = after.outlineColor.match(/[\d.]+/g)?.map(Number) ?? []; const outline = { r: parts[0] ?? 0, g: parts[1] ?? 0, b: parts[2] ?? 0 };
    const shadowParts = after.boxShadow.match(/rgba?\(([^)]+)\)/)?.[1].match(/[\d.]+/g)?.map(Number) ?? [];
    const shadow = shadowParts.length >= 3 && (shadowParts.length < 4 || shadowParts[3] > 0) ? { r: shadowParts[0], g: shadowParts[1], b: shadowParts[2] } : null;
    const luminance = color => [color.r, color.g, color.b].map(channel => { const value = channel / 255; return value <= .04045 ? value / 12.92 : ((value + .055) / 1.055) ** 2.4; }).reduce((sum, value, index) => sum + value * [.2126, .7152, .0722][index], 0);
    const outlineLum = luminance(outline); const contrast = (Math.max(outlineLum, prior.luminance) + .05) / (Math.min(outlineLum, prior.luminance) + .05);
    const shadowLum = shadow ? luminance(shadow) : null;
    const shadowContrast = shadowLum === null ? 1 : (Math.max(shadowLum, prior.luminance) + .05) / (Math.min(shadowLum, prior.luminance) + .05);
    return { after, keyboardFocused: document.activeElement === node, outline, shadow, contrast, shadowContrast, visibleContrast: Math.max(contrast, shadowContrast) };
    }, beforeEvidence);
    const focusChanged = JSON.stringify(beforeEvidence.before) !== JSON.stringify(afterEvidence.after);
    return { name, ...beforeEvidence, ...afterEvidence, focusChanged,
      pass: afterEvidence.keyboardFocused && focusChanged
        && ((afterEvidence.after.outlineStyle !== 'none' && parseFloat(afterEvidence.after.outlineWidth) >= 2) || afterEvidence.after.boxShadow !== 'none')
        && afterEvidence.visibleContrast >= 3 };
  };
  await openPracticeTab('Trade');
  const focusEvidence = [
    await measureKeyboardFocus('Replay Next', page.getByRole('button', { name: 'Next →', exact: true })),
    await measureKeyboardFocus('PracticeRail Trade tab', page.getByRole('tab', { name: 'Trade', exact: true })),
    await measureKeyboardFocus('Indicator dialog opener', indicatorOpener),
  ];
  const focusStyle = focusEvidence.find(item => item.name === 'Indicator dialog opener');
  const focusPass = focusEvidence.every(item => item.pass);
  await page.screenshot({ path: path.join(runDir, '23-batch5-focused-control.png'), fullPage: true });
  batch5('accessibility.visible-focus', focusPass, JSON.stringify(focusEvidence));
  const unnamedCore = await page.locator('header button, header select, [data-testid="drawing-toolbar"] button, [data-testid="drawing-toolbar"] select, [data-testid="practice-rail"] button, [data-testid="practice-rail"] input, [data-testid="practice-rail"] select, [data-testid="practice-rail"] textarea').evaluateAll(nodes => nodes.filter(node => {
    const text = node.getAttribute('aria-label') || node.getAttribute('title') || node.textContent?.trim();
    return !text;
  }).map(node => node.outerHTML.slice(0, 160)));
  batch5('accessibility.core-controls-named', unnamedCore.length === 0, JSON.stringify(unnamedCore));

  await page.setViewportSize({ width: 720, height: 500 });
  await page.evaluate(() => {
    document.documentElement.scrollTo(0, 0);
    document.body.scrollTo(0, 0);
    document.querySelector('.app-main')?.scrollTo(0, 0);
    document.querySelector('.replay-workspace')?.scrollTo(0, 0);
  });
  const zoomGeometry = await page.evaluate(() => {
    const geometry = selector => {
      const node = document.querySelector(selector);
      if (!node) return null;
      const style = getComputedStyle(node); const rect = node.getBoundingClientRect();
      return {
        selector,
        computedStyle: { display: style.display, position: style.position, flexDirection: style.flexDirection,
          overflowX: style.overflowX, overflowY: style.overflowY, width: style.width, height: style.height },
        bounds: { left: rect.left, right: rect.right, top: rect.top, bottom: rect.bottom, width: rect.width, height: rect.height },
        clientWidth: node.clientWidth, scrollWidth: node.scrollWidth, clientHeight: node.clientHeight, scrollHeight: node.scrollHeight,
        scrollLeft: node.scrollLeft, scrollTop: node.scrollTop,
      };
    };
    return {
      method: { mechanism: 'page.setViewportSize', cssViewport: { width: window.innerWidth, height: window.innerHeight },
        devicePixelRatio: window.devicePixelRatio, smallViewportMediaQuery: matchMedia('(max-width: 768px)').matches },
      document: { scrollWidth: document.documentElement.scrollWidth, clientWidth: document.documentElement.clientWidth, scrollHeight: document.documentElement.scrollHeight, clientHeight: document.documentElement.clientHeight, scrollLeft: document.documentElement.scrollLeft, scrollTop: document.documentElement.scrollTop },
      body: { scrollWidth: document.body.scrollWidth, clientWidth: document.body.clientWidth, scrollHeight: document.body.scrollHeight, clientHeight: document.body.clientHeight, scrollLeft: document.body.scrollLeft, scrollTop: document.body.scrollTop, overflowX: getComputedStyle(document.body).overflowX, overflowY: getComputedStyle(document.body).overflowY },
      elements: Object.fromEntries(['.sidebar', '.app-shell', '.app-main', '.replay-workspace', '.replay-workspace-body', '.replay-chart-region', '.replay-details-region']
        .map(selector => [selector, geometry(selector)])),
    };
  });
  const screenshotDimensions = buffer => ({ width: buffer.readUInt32BE(16), height: buffer.readUInt32BE(20) });
  const replayScreenshotPath = path.join(runDir, '22a-batch5-reflow-replay-720x500.png');
  await page.locator('.replay-chart-region').evaluate(node => node.scrollIntoView({ block: 'start', inline: 'nearest' }));
  await page.waitForTimeout(100);
  const replayScreenshot = { path: replayScreenshotPath, ...screenshotDimensions(await page.screenshot({ path: replayScreenshotPath })) };
  const keyboardReach = async (locator, activateKey) => {
    await page.evaluate(() => { document.activeElement?.blur(); document.body.tabIndex = -1; document.body.focus(); });
    let tabs = 0;
    while (tabs < 120 && !await locator.evaluate(node => node === document.activeElement)) { await page.keyboard.press('Tab'); tabs += 1; }
    const focused = await locator.evaluate(node => node === document.activeElement);
    const beforeSelected = await locator.getAttribute('aria-selected');
    if (focused && activateKey) await page.keyboard.press(activateKey);
    const box = await locator.boundingBox();
    const viewport = await page.evaluate(() => ({ width: window.innerWidth, height: window.innerHeight, devicePixelRatio: window.devicePixelRatio }));
    const intersects = !!box && !!viewport && box.x + box.width > 0 && box.y + box.height > 0 && box.x < viewport.width && box.y < viewport.height;
    return { focused, tabs, activated: activateKey ? (await locator.getAttribute('aria-selected') === 'true' || beforeSelected === null) : true, box, viewport, intersects };
  };
  const zoomTrade = await keyboardReach(page.getByRole('tab', { name: 'Trade', exact: true }), 'Enter');
  await page.keyboard.press('ArrowRight');
  const journalTab = page.getByRole('tab', { name: 'Journal', exact: true });
  const journalPanel = page.getByTestId('practice-tab-journal');
  await journalTab.evaluate(node => node.scrollIntoView({ block: 'start', inline: 'nearest' }));
  await page.waitForTimeout(100);
  const journalBox = await journalTab.boundingBox(); const journalPanelBox = await journalPanel.boundingBox();
  const journalViewport = await page.evaluate(() => ({ width: window.innerWidth, height: window.innerHeight, devicePixelRatio: window.devicePixelRatio }));
  const boxIntersects = (box, viewport) => !!box && box.x + box.width > 0 && box.y + box.height > 0 && box.x < viewport.width && box.y < viewport.height;
  const zoomJournal = {
    focused: await journalTab.evaluate(node => node === document.activeElement),
    selected: await journalTab.getAttribute('aria-selected') === 'true',
    panelActive: !await journalPanel.isHidden(),
    box: journalBox, panelBox: journalPanelBox, viewport: journalViewport,
    intersects: boxIntersects(journalBox, journalViewport), panelIntersects: boxIntersects(journalPanelBox, journalViewport),
    activated: await journalTab.getAttribute('aria-selected') === 'true' && !await journalPanel.isHidden(),
    path: 'body -> sequential Tab to selected Trade -> Enter -> ArrowRight to Journal',
  };
  const journalScreenshotPath = path.join(runDir, '22b-batch5-reflow-journal-720x500.png');
  const journalScreenshot = { path: journalScreenshotPath, ...screenshotDimensions(await page.screenshot({ path: journalScreenshotPath })) };
  const zoomReplay = await keyboardReach(page.getByRole('button', { name: 'Next →', exact: true }), null);
  const zoomAccess = { replay: zoomReplay, journal: zoomJournal, trade: zoomTrade, geometry: zoomGeometry,
    screenshots: { replay: replayScreenshot, journal: journalScreenshot } };
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.evaluate(() => document.documentElement.scrollTo(0, 0));
  const geometryElements = zoomGeometry.elements;
  const horizontalContainment = [zoomGeometry.document, zoomGeometry.body, geometryElements['.app-main'], geometryElements['.replay-workspace']]
    .every(item => item && item.scrollWidth <= item.clientWidth + 1);
  const verticalScrollPreserved = [zoomGeometry.document, zoomGeometry.body, geometryElements['.app-main'], geometryElements['.replay-workspace']]
    .some(item => item && item.scrollHeight > item.clientHeight + 1)
    && [geometryElements['.app-main'], geometryElements['.replay-workspace']]
      .some(item => item && item.computedStyle.overflowY !== 'hidden');
  const reflowPass = Object.values(geometryElements).every(Boolean)
    && geometryElements['.app-shell'].computedStyle.flexDirection === 'column'
    && geometryElements['.sidebar'].computedStyle.position === 'static'
    && geometryElements['.sidebar'].bounds.width >= zoomGeometry.method.cssViewport.width * 0.9
    && geometryElements['.sidebar'].bounds.left >= -1 && geometryElements['.sidebar'].bounds.right <= zoomGeometry.method.cssViewport.width + 1
    && geometryElements['.replay-workspace-body'].computedStyle.flexDirection === 'column'
    && geometryElements['.replay-details-region'].bounds.top >= geometryElements['.replay-chart-region'].bounds.bottom - 1
    && horizontalContainment && verticalScrollPreserved;
  const zoomPass = [zoomReplay, zoomTrade].every(item => item.focused && item.intersects && item.activated)
    && zoomJournal.focused && zoomJournal.selected && zoomJournal.panelActive && zoomJournal.intersects && zoomJournal.panelIntersects && zoomJournal.activated
    && replayScreenshot.width === 720 && replayScreenshot.height === 500
    && journalScreenshot.width === 720 && journalScreenshot.height === 500
    && zoomGeometry.method.cssViewport.width === 720 && zoomGeometry.method.cssViewport.height === 500
    && zoomGeometry.method.smallViewportMediaQuery && reflowPass;
  batch5('accessibility.zoom-200-core-access', zoomPass, JSON.stringify(zoomAccess));
  check('batch5.closure.R04.accessibility-evidence', focusPass && zoomPass, JSON.stringify({ focusEvidence, zoomAccess }));
  hardening.accessibility = { tabSemantics, focusStyle, focusEvidence, unnamedCore, zoomAccess };

  if (sustainedSeconds > 0) {
    recordAction('sustained-hardening-start', `target=${sustainedSeconds}s total elapsed practice time`);
    // Establish a genuinely long authoritative history through the real replay controls before stress measurement.
    while ((await readPractice()).current_index < 249) {
      const before = await readPractice();
      await page.getByRole('button', { name: '+5', exact: true }).click();
      await waitPractice(state => state.current_index === Math.min(state.total_bars - 1, before.current_index + 5), 'Batch 5 long-history advance');
    }
    const longHistoryPractice = await readPractice(); const longHistoryChart = await readIndicatorChart();
    hardening.longHistory = { currentIndex: longHistoryPractice.current_index, currentDate: longHistoryPractice.current_date,
      visibleCandleCount: longHistoryChart.candleInput.count, visibleMaxDate: longHistoryChart.candleInput.maxDate };
    recordAction('long-visible-history-established', JSON.stringify(hardening.longHistory), 'replay.long-history');
    // Exercise the many-drawing state through the public UI until at least 50 documents are persisted.
    while ((await readDomain()).drawings.length < 50) {
      const pane = (await readInteraction()).pricePane;
      const before = await readDomain();
      await page.getByTestId('drawing-tool-horizontal').click();
      const fraction = .12 + ((before.drawings.length * 7) % 72) / 100;
      await page.mouse.click(pane.left + pane.width * .55, pane.top + pane.height * fraction);
      await waitForRevision(before.revision + 1);
    }
    recordAction('many-drawing-state', `count=${(await readDomain()).drawings.length}`);

    const cdp = await context.newCDPSession(page);
    await cdp.send('Performance.enable');
    await cdp.send('HeapProfiler.enable');
    const collectHeap = async () => {
      await cdp.send('HeapProfiler.collectGarbage');
      const metrics = await cdp.send('Performance.getMetrics');
      return metrics.metrics.find(item => item.name === 'JSHeapUsedSize')?.value ?? null;
    };
    const baselineHeap = await collectHeap();
    const baselineDom = await page.locator('*').count();
    const metricSegments = [];
    const churnSamples = [];
    let routeRemounts = 0;
    const captureMetrics = async label => {
      const values = await page.evaluate(() => {
        const captured = structuredClone(window.__sumiBatch5Metrics || { rafGaps: [], longTasks: [] });
        if (window.__sumiBatch5Metrics) { window.__sumiBatch5Metrics.rafGaps = []; window.__sumiBatch5Metrics.longTasks = []; }
        return captured;
      });
      metricSegments.push({ label, ...values });
    };
    const waitIndicatorNetworkIdle = async label => {
      for (let attempt = 0; attempt < 100 && indicatorInflight.size; attempt += 1) await page.waitForTimeout(50);
      if (indicatorInflight.size) throw new Error(`${label}: indicator requests did not settle: ${JSON.stringify([...indicatorInflight.values()].map(item => item.url))}`);
    };
    const verifyNoFuture = async label => {
      let practice; let candles; let authoritative; let markersNow; let sample; let verdict = { pass: false, failures: [{ surface: 'uninitialized' }] };
      for (let attempt = 0; attempt < 150; attempt += 1) {
        practice = await readPractice();
        authoritative = await (await page.request.get(`${backendUrl}/api/replay/sessions/${sessionId}`)).json();
        candles = await (await page.request.get(`${backendUrl}/api/replay/sessions/${sessionId}/candles`)).json();
        markersNow = await readTradeMarkers();
        const chart = await readIndicatorChart();
        const indicatorDocumentNow = await readIndicatorDomain();
        const indicatorRuntimeNow = await readIndicatorRuntime();
        const interaction = await readInteraction();
        const drawingDocumentNow = await readDomain();
        const boundary = toDateKeyForUat(candles.at(-1)?.timestamp);
        const chartInstances = new Map(chart.instances.map(instance => [instance.id, instance]));
        sample = {
          at: new Date().toISOString(), label,
          authoritativeIndex: authoritative.current_index, authoritativeDate: boundary,
          practiceIndex: practice?.current_index ?? null,
          apiCandleCount: candles.length, apiMaxDate: toDateKeyForUat(candles.at(-1)?.timestamp),
          chartCandleCount: chart.candleInput?.count ?? null, chartMaxDate: chart.candleInput?.maxDate ?? null,
          indicators: indicatorDocumentNow.instances.filter(instance => instance.visible).map(instance => ({
            id: instance.id, definitionId: instance.definitionId,
            inputMaxDate: indicatorRuntimeNow[instance.id]?.inputMaxDate ?? null,
            responseMaxDate: indicatorRuntimeNow[instance.id]?.responseMaxDate ?? null,
            responseCount: indicatorRuntimeNow[instance.id]?.responseCount ?? null,
            seriesMaxDates: chartInstances.get(instance.id)?.seriesMaxDates ?? {},
          })),
          markerMaxDate: markersNow.map(marker => toDateKeyForUat(marker.time)).sort().at(-1) ?? null,
          visibleProviderDrawings: interaction.drawings.filter(drawing => drawing.providerVisible).map(drawing => ({ id: drawing.id, anchorDates: drawing.anchors.map(anchor => toDateKeyForUat(anchor.time)) })),
          retainedFutureDrawings: drawingDocumentNow.drawings.filter(drawing => drawing.anchors.some(anchor => toDateKeyForUat(anchor.time) > boundary)).map(drawing => ({
            id: drawing.id, anchorDates: drawing.anchors.map(anchor => toDateKeyForUat(anchor.time)),
            providerVisible: interaction.drawings.find(item => item.id === drawing.id)?.providerVisible ?? false,
          })),
        };
        verdict = validateFutureBoundary(sample);
        if (verdict.pass) break;
        await page.waitForTimeout(100);
      }
      const recorded = { ...sample, pass: verdict.pass, failures: verdict.failures };
      hardening.noFutureSamples.push(recorded);
      await writeFile(path.join(runDir, 'batch5-live.json'), JSON.stringify({ hardening, checks, runtimeErrors, indicatorRequestFailures, providerErrors }, null, 2));
      if (!verdict.pass) throw new Error(`Batch 5 future-data invariant failed: ${JSON.stringify(recorded)}`);
    };
    const navigateTimed = async (name, delta) => {
      const before = await readPractice(); const started = performance.now();
      await page.getByRole('button', { name, exact: true }).click();
      const expected = Math.max(0, Math.min(before.total_bars - 1, before.current_index + delta));
      await waitPractice(state => state.current_index === expected, `Batch 5 ${name}`);
      const elapsed = performance.now() - started;
      hardening.navigationMs.push({ name, elapsed, at: new Date().toISOString() });
      recordAction(`navigate-${name}`, `${before.current_index}->${expected}; ${elapsed.toFixed(1)}ms`, 'replay.navigation-autoplay');
    };

    let iteration = 0;
    while ((Date.now() - practiceSessionStartedMs) / 1000 < sustainedSeconds) {
      const mode = iteration % 10;
      if (mode === 0) await navigateTimed('Next →', 1);
      else if (mode === 1) await navigateTimed('← Prev', -1);
      else if (mode === 2) await navigateTimed('+5', 5);
      else if (mode === 3) await navigateTimed('-5', -5);
      else if (mode === 4) {
        const beforeAutoplay = await readPractice();
        await page.locator('header select').selectOption(iteration % 20 ? '200' : '100');
        await page.getByRole('button', { name: 'Auto-Play', exact: true }).click();
        await page.getByRole('button', { name: 'Pause', exact: true }).waitFor();
        let autoplayAdvanced = null;
        for (let attempt = 0; attempt < 30; attempt += 1) {
          autoplayAdvanced = await readPractice();
          if (autoplayAdvanced && autoplayAdvanced.current_index > beforeAutoplay.current_index) break;
          await page.waitForTimeout(100);
        }
        await page.getByRole('button', { name: 'Pause', exact: true }).click();
        await page.getByRole('button', { name: 'Auto-Play', exact: true }).waitFor();
        const afterAutoplay = await readPractice();
        recordAction('autoplay-pause', `speed changed; visible advance ${beforeAutoplay.current_index}->${afterAutoplay?.current_index ?? autoplayAdvanced?.current_index ?? 'unknown'}; exact pause control observed`, 'replay.navigation-autoplay');
      } else if (mode === 5) {
        const tab = page.getByRole('tab', { name: 'Trade', exact: true }); await tab.focus(); await page.keyboard.press('ArrowRight');
        recordAction('keyboard-tab-navigation', 'Trade -> Journal without replay movement', 'replay.navigation-autoplay');
      } else if (mode === 6) {
        const active = (await readIndicatorDomain()).instances.find(item => item.visible) || (await readIndicatorDomain()).instances[0];
        await instanceAction(active.id, 'toggle').click(); await instanceAction(active.id, 'toggle').click();
        recordAction('indicator-hide-show', active.id, 'indicator.lifecycle');
      } else if (mode === 7) {
        const magnet = page.getByTestId('drawing-magnet-mode'); await magnet.selectOption('ohlc'); await magnet.selectOption('off');
        recordAction('drawing-magnet-cycle', 'ohlc -> off', 'drawing.magnet');
      } else if (mode === 8) {
        const pane = (await readInteraction()).pricePane; await page.mouse.move(pane.left + pane.width * .5, pane.top + pane.height * .5); await page.mouse.wheel(0, -180); await page.mouse.wheel(0, 180);
        recordAction('chart-zoom-pan-input', 'wheel in/out on provider pane', 'replay.navigation-autoplay');
      } else {
        await captureMetrics(`before-reload-${iteration}`);
        const duplicateRequestCount = () => confirmedIndicatorDuplicates().length;
        const beforeChurn = { heap: await collectHeap(), dom: await page.locator('*').count(), provider: await readInteraction(), indicator: await readIndicatorChart(), duplicateRequests: duplicateRequestCount() };
        if (routeRemounts < 10) {
          await waitIndicatorNetworkIdle('before analytics route transition');
          await page.getByRole('link', { name: /Analytics/ }).click();
          await page.getByRole('heading', { name: 'Performance Analytics' }).waitFor();
          await waitIndicatorNetworkIdle('after replay unmount');
          await page.getByRole('link', { name: /Trading Lab/ }).click();
          await page.locator('header').getByText(/Session #/).waitFor();
          await waitPractice(state => state.current_index >= 0, 'Batch 5 route-remount resume');
          await waitIndicatorNetworkIdle('after replay route remount');
          routeRemounts += 1;
          recordAction('analytics-replay-route-remount', `remount=${routeRemounts}`, 'route.remount');
        }
        await page.setViewportSize(iteration % 20 === 9 ? { width: 1280, height: 800 } : { width: 1440, height: 1000 });
        const usableStarted = performance.now(); await page.reload(); await page.locator('header').getByText(/Session #/).waitFor();
        await waitPractice(state => state.current_index >= 0, 'Batch 5 reload resume');
        const usableElapsed = performance.now() - usableStarted; hardening.workspaceUsableMs.push({ elapsed: usableElapsed, viewport: await page.viewportSize(), at: new Date().toISOString() });
        await verifyNoFuture(`post-churn-${routeRemounts}`);
        const afterChurn = { heap: await collectHeap(), dom: await page.locator('*').count(), provider: await readInteraction(), indicator: await readIndicatorChart(), duplicateRequests: duplicateRequestCount() };
        churnSamples.push({ at: new Date().toISOString(), routeRemount: routeRemounts, before: beforeChurn, after: afterChurn,
          ownershipStable: beforeChurn.provider.primitiveCount === afterChurn.provider.primitiveCount && beforeChurn.provider.listenerCount === afterChurn.provider.listenerCount
            && beforeChurn.indicator.keys.length === afterChurn.indicator.keys.length && new Set(afterChurn.indicator.keys).size === afterChurn.indicator.keys.length,
          requestStable: beforeChurn.duplicateRequests === afterChurn.duplicateRequests });
        recordAction('reload-resume', `iteration=${iteration}; usable=${usableElapsed.toFixed(1)}ms`, 'reload.resume');
      }
      await verifyNoFuture(`iteration-${iteration}`);
      if (iteration % 20 === 0) await page.screenshot({ path: path.join(runDir, `batch5-sustained-${String(iteration).padStart(3, '0')}.png`), fullPage: true });
      if (iteration % 9 === 8) await captureMetrics(`segment-${iteration}`);
      iteration += 1;
      const remainingMs = sustainedSeconds * 1000 - (Date.now() - practiceSessionStartedMs);
      if (remainingMs > 0) await page.waitForTimeout(Math.min(sustainedActionIntervalMs, remainingMs));
    }
    await captureMetrics('final');
    const finalHeap = await collectHeap();
    const finalDom = await page.locator('*').count();
    const rafGaps = metricSegments.flatMap(segment => segment.rafGaps).filter(value => value < 10_000).sort((a, b) => a - b);
    const longTasks = metricSegments.flatMap(segment => segment.longTasks);
    const percentile = (values, p) => values.length ? values[Math.min(values.length - 1, Math.ceil(values.length * p) - 1)] : null;
    const nav = hardening.navigationMs.map(item => item.elapsed).sort((a, b) => a - b);
    const usable = hardening.workspaceUsableMs.map(item => item.elapsed).sort((a, b) => a - b);
    const indicatorTimingValues = indicatorLatencies.map(item => item.elapsed).sort((a, b) => a - b);
    const heapGrowth = finalHeap === null || baselineHeap === null ? null : finalHeap - baselineHeap;
    hardening.performance = {
      baselineHeap, finalHeap, heapGrowth, heapGrowthRatio: heapGrowth === null || !baselineHeap ? null : heapGrowth / baselineHeap,
      baselineDom, finalDom, domGrowth: finalDom - baselineDom, domGrowthRatio: (finalDom - baselineDom) / baselineDom,
      navigation: { samples: nav.length, median: percentile(nav, .5), p95: percentile(nav, .95), worst: nav.at(-1) ?? null },
      workspaceUsable: { samples: usable.length, median: percentile(usable, .5), p95: percentile(usable, .95), worst: usable.at(-1) ?? null, viewports: hardening.workspaceUsableMs.map(item => item.viewport) },
      indicatorRequests: {
        samples: indicatorTimingValues.length, median: percentile(indicatorTimingValues, .5), p95: percentile(indicatorTimingValues, .95), worst: indicatorTimingValues.at(-1) ?? null,
        duplicateInflight: confirmedIndicatorDuplicates().map(({ url, at }) => ({ url, at })),
        canceledOverlaps: indicatorOverlapCandidates.filter(candidate => candidate.requests.some(request => indicatorRequestStates.get(request) === 'canceled')).map(({ url, at }) => ({ url, at })),
        failed: indicatorRequestFailures, cancellations: indicatorRequestCancellations,
      },
      raf: { samples: rafGaps.length, p95: percentile(rafGaps, .95), worst: rafGaps.at(-1) ?? null },
      longTasks: { count: longTasks.length, worst: longTasks.length ? Math.max(...longTasks.map(item => item.duration)) : 0 },
      provider: await readInteraction(), indicator: await readIndicatorChart(), routeRemounts, churnSamples,
      metricSegments: metricSegments.map(({ label, rafGaps, longTasks }) => ({ label, rafSamples: rafGaps.length, longTasks })),
    };
    const durationSeconds = (Date.now() - practiceSessionStartedMs) / 1000;
    const perf = hardening.performance;
    batch5('sustained.minimum-1800-seconds', durationSeconds >= sustainedSeconds && (allowShortHardeningSmoke || durationSeconds >= 1800), JSON.stringify({ durationSeconds, allowShortHardeningSmoke, startedAt: new Date(practiceSessionStartedMs).toISOString(), endedAt: new Date().toISOString(), actions: hardening.timeline.length }));
    batch5('sustained.no-future-every-checkpoint', hardening.noFutureSamples.length > 0 && hardening.noFutureSamples.every(item => item.pass), JSON.stringify(hardening.noFutureSamples));
    batch5('performance.navigation-budget', perf.navigation.samples >= (allowShortHardeningSmoke ? 1 : 5) && perf.navigation.median <= 500 && perf.navigation.p95 <= 900 && perf.navigation.worst <= 1500, JSON.stringify({ ...perf.navigation, allowShortHardeningSmoke }));
    batch5('performance.workspace-usable-budget', perf.workspaceUsable.samples >= (allowShortHardeningSmoke ? 0 : 5)
      && (allowShortHardeningSmoke || new Set(perf.workspaceUsable.viewports.map(item => `${item.width}x${item.height}`)).size === 2)
      && (perf.workspaceUsable.samples === 0 || (perf.workspaceUsable.median <= 5000 && perf.workspaceUsable.worst <= 8000)), JSON.stringify({ ...perf.workspaceUsable, allowShortHardeningSmoke }));
    batch5('performance.indicator-request-budget', perf.indicatorRequests.samples >= 5 && perf.indicatorRequests.median <= 1500 && perf.indicatorRequests.p95 <= 2500
      && perf.indicatorRequests.worst <= 4000 && perf.indicatorRequests.duplicateInflight.length === 0 && perf.indicatorRequests.failed.length === 0, JSON.stringify(perf.indicatorRequests));
    batch5('performance.raf-budget', perf.raf.samples > 0 && perf.raf.p95 <= 100 && perf.raf.worst <= 500, JSON.stringify(perf.raf));
    batch5('performance.long-task-budget', perf.longTasks.worst <= 1000 && perf.longTasks.count <= Math.ceil(durationSeconds / 600) * 20, JSON.stringify(perf.longTasks));
    batch5('performance.heap-dom-budget', perf.heapGrowth !== null && perf.heapGrowth <= 20 * 1024 * 1024 && perf.heapGrowthRatio <= .35
      && perf.domGrowth <= 150 && perf.domGrowthRatio <= .20, JSON.stringify({ heapGrowth: perf.heapGrowth, heapGrowthRatio: perf.heapGrowthRatio, domGrowth: perf.domGrowth, domGrowthRatio: perf.domGrowthRatio }));
    batch5('performance.provider-ownership', perf.provider.primitiveCount === 1 && perf.provider.listenerCount === 6, JSON.stringify({ primitiveCount: perf.provider.primitiveCount, listenerCount: perf.provider.listenerCount }));
    const futureFieldsComplete = hardening.noFutureSamples.length > 0 && hardening.noFutureSamples.every(sample => sample.pass
      && Number.isInteger(sample.authoritativeIndex) && !!sample.authoritativeDate && Number.isInteger(sample.apiCandleCount) && !!sample.apiMaxDate
      && Number.isInteger(sample.chartCandleCount) && !!sample.chartMaxDate && Array.isArray(sample.indicators)
      && sample.indicators.every(indicator => indicator.inputMaxDate && indicator.responseMaxDate && Object.keys(indicator.seriesMaxDates).length > 0)
      && Array.isArray(sample.visibleProviderDrawings) && Array.isArray(sample.retainedFutureDrawings));
    check('batch5.closure.R02.full-future-boundary', futureFieldsComplete,
      JSON.stringify({ samples: hardening.noFutureSamples.length, fields: hardening.noFutureSamples.at(-1), negativeSelfTests: ['api', 'chart', 'indicator-input', 'indicator-response', 'indicator-chart', 'marker', 'drawing', 'retained-drawing-visible'] }));

    const auditFixture = { ...structuredClone(batch5ReturnedBaseline), passed: 273, failed: 0, blockingFailed: 0,
      checks: [...structuredClone(batch5ReturnedBaseline.checks), { id: 'batch5.closure.audit-fixture', pass: true }] };
    const validAudit = auditEvidence(batch5ReturnedBaseline, auditFixture);
    const failedAdditiveFixture = structuredClone(auditFixture); failedAdditiveFixture.checks.at(-1).pass = false; failedAdditiveFixture.passed = 272; failedAdditiveFixture.failed = 1; failedAdditiveFixture.blockingFailed = 1;
    const duplicateFixture = structuredClone(auditFixture); duplicateFixture.checks.push(structuredClone(duplicateFixture.checks[0])); duplicateFixture.passed += 1;
    const missingFixture = structuredClone(auditFixture); missingFixture.checks.shift(); missingFixture.passed -= 1;
    const changedFixture = structuredClone(auditFixture); changedFixture.checks[0].pass = false; changedFixture.passed -= 1; changedFixture.failed = 1;
    const auditNegativePass = [failedAdditiveFixture, duplicateFixture, missingFixture, changedFixture].every(fixture => !auditEvidence(batch5ReturnedBaseline, fixture).pass);
    check('batch5.closure.R03.fail-closed-evidence', validAudit.pass && auditNegativePass,
      JSON.stringify({ validAudit, negativeCases: ['failed-additive', 'duplicate-baseline', 'missing-baseline', 'changed-baseline'], manifestNegativeSelftest: 'scripts/batch5-evidence-negative-selftest.mjs' }));

    const requiredCategories = ['replay.navigation-autoplay', 'replay.long-history', 'indicator.lifecycle', 'drawing.all-tools-edit-history', 'drawing.magnet', 'trade.lifecycle', 'journal.checklist', 'route.remount', 'reload.resume'];
    const categories = Object.fromEntries(requiredCategories.map(category => [category, hardening.timeline.filter(item => item.category === category).length]));
    const windows = Array.from({ length: 6 }, (_, index) => ({ index, startSeconds: index * 300, endSeconds: (index + 1) * 300,
      meaningfulActions: hardening.timeline.filter(item => item.category && item.elapsedSeconds >= index * 300 && item.elapsedSeconds < (index + 1) * 300).length,
      categories: [...new Set(hardening.timeline.filter(item => item.category && item.elapsedSeconds >= index * 300 && item.elapsedSeconds < (index + 1) * 300).map(item => item.category))].sort() }));
    hardening.categoryCoverage = { requiredCategories, categories, windows };
    const churnPass = perf.routeRemounts === 10 && new Set(perf.churnSamples.map(item => item.routeRemount)).size === 10
      && perf.churnSamples.every(item => item.ownershipStable && item.requestStable);
    const sustainedScopePass = requiredCategories.every(category => categories[category] > 0) && windows.every(window => window.meaningfulActions > 0)
      && hardening.longHistory.currentIndex >= 249 && hardening.longHistory.visibleCandleCount >= 250
      && hardening.longHistory.visibleMaxDate === toDateKeyForUat(hardening.longHistory.currentDate) && churnPass;
    check('batch5.closure.R05.complete-sustained-scope', allowShortHardeningSmoke || sustainedScopePass,
      JSON.stringify({ allowShortHardeningSmoke, categoryCoverage: hardening.categoryCoverage, longHistory: hardening.longHistory, routeRemounts: perf.routeRemounts,
        churnSamples: perf.churnSamples.map(item => ({ routeRemount: item.routeRemount, ownershipStable: item.ownershipStable, requestStable: item.requestStable,
          heapBefore: item.before.heap, heapAfter: item.after.heap, domBefore: item.before.dom, domAfter: item.after.dom })) }));
    const workspaceStorage = await page.evaluate(() => Object.fromEntries(Object.keys(localStorage).sort().map(key => [key, localStorage.getItem(key)])));
    hardening.workspace = { sessionId, storage: workspaceStorage, practice: await readPractice(), indicators: await readIndicatorDomain(), drawings: await readDomain(), journal: await readJournal() };
    await writeFile(path.join(runDir, 'workspace-export.json'), JSON.stringify(hardening.workspace, null, 2));
    recordAction('sustained-hardening-finished', `duration=${durationSeconds.toFixed(1)}s`);
  }

  await selectDrawing(createdId);
  await openPracticeTab('Drawing');
  await page.setViewportSize({ width: 1280, height: 800 });
  const compactInteraction = await readInteraction();
  const compactDrawing = compactInteraction.drawings.find(drawing => drawing.id === createdId);
  scoped('compact-layout', (await page.getByTestId('chart-workspace').boundingBox())?.width > 600 && compactDrawing?.coordinate !== null && compactDrawing?.visible && compactDrawing.price === findDrawing(lifecycleAfter, createdId).anchors[0].price, JSON.stringify(compactDrawing));
  const compactIndicatorChart = await readIndicatorChart();
  const compactIndicatorDocument = await readIndicatorDomain();
  const compactLayout = inspectIndicatorLayout(compactIndicatorDocument, compactIndicatorChart);
  const compactRuntime = inspectActiveRuntime(compactIndicatorDocument, await readIndicatorRuntime());
  batch2('responsive-1280x800', compactIndicatorChart.keys.length === beforeReloadIndicatorChart.keys.length
    && new Set(compactIndicatorChart.keys).size === compactIndicatorChart.keys.length
    && compactLayout.pass, JSON.stringify({ chart: compactIndicatorChart, layout: compactLayout }));
  batch2('runtime-1280x800-no-active-error', compactRuntime.pass, JSON.stringify(compactRuntime));
  const compactPracticeRail = await page.getByTestId('practice-rail').boundingBox();
  const compactPracticeBody = await page.locator('.replay-workspace-body').boundingBox();
  batch4('T-01.integrated-compact-1280x800', !!compactPracticeRail && !!compactPracticeBody
    && compactPracticeRail.x >= compactPracticeBody.x && compactPracticeRail.x + compactPracticeRail.width <= compactPracticeBody.x + compactPracticeBody.width + 1
    && (await page.getByTestId('chart-workspace').boundingBox())?.width > 600,
  JSON.stringify({ rail: compactPracticeRail, body: compactPracticeBody, chart: await page.getByTestId('chart-workspace').boundingBox() }));
  const compactInspectorGeometry = await page.getByTestId('drawing-selection-toolbar').evaluate(element => { const box = element.getBoundingClientRect(); const aside = element.closest('aside')?.getBoundingClientRect(); const values = [...element.querySelectorAll('input:not([type="checkbox"]),select,textarea')].map(node => ({ value: node.value, width: node.getBoundingClientRect().width })); return { box: { left: box.left, right: box.right, top: box.top, bottom: box.bottom }, aside: aside && { left: aside.left, right: aside.right, top: aside.top, bottom: aside.bottom }, values, contained: !!aside && box.left >= aside.left && box.right <= aside.right + 1 && box.width >= 220 }; });
  batch3('hardening.inspector-contained-readable-1280x800', compactInspectorGeometry.contained && compactInspectorGeometry.values.every(item => item.width >= 90 && String(item.value).length > 0), JSON.stringify(compactInspectorGeometry));
  const compactChromeGeometry = await page.locator('[data-testid^="indicator-pane-chrome-"]').evaluateAll(nodes => nodes.map(node => {
    const box = node.getBoundingClientRect(); const parent = node.parentElement?.getBoundingClientRect();
    return { id: node.getAttribute('data-testid'), top: box.top, bottom: box.bottom, width: box.width, parentTop: parent?.top, parentBottom: parent?.bottom,
      contained: !!parent && box.top >= parent.top && box.bottom <= parent.bottom + 1 && box.width > 0 };
  }));
  batch2('compact-pane-chrome-geometry', compactChromeGeometry.every(item => item.contained), JSON.stringify(compactChromeGeometry));
  await page.screenshot({ path: path.join(runDir, '07-compact-1280x800.png'), fullPage: true });
  await openPracticeTab('Trade');
  batch4('T-01.compact-full-workstation', await page.locator('.limited-workstation-warning').isHidden()
    && !(await page.getByRole('button', { name: 'BUY', exact: true }).isDisabled()), '1280px retains full trade controls');
  await page.screenshot({ path: path.join(runDir, '20-batch4-compact-1280x800.png'), fullPage: true });
  await page.setViewportSize({ width: 1179, height: 800 });
  await page.waitForFunction(() => {
    const buy = [...document.querySelectorAll('button')].find(node => node.textContent?.trim() === 'BUY');
    return window.matchMedia('(max-width: 1179px)').matches && buy?.disabled;
  });
  const limitedMobileEvidence = {
    warningVisible: await page.locator('.limited-workstation-warning').isVisible(),
    buyDisabled: await page.getByRole('button', { name: 'BUY', exact: true }).isDisabled(),
    viewport: await page.evaluate(() => ({ innerWidth: window.innerWidth, innerHeight: window.innerHeight, devicePixelRatio: window.devicePixelRatio,
      limitedMedia: matchMedia('(max-width: 1179px)').matches })),
  };
  batch4('T-01.explicit-limited-mobile', limitedMobileEvidence.warningVisible && limitedMobileEvidence.buyDisabled, JSON.stringify(limitedMobileEvidence));
  const networkOriginList = [...networkOrigins].sort();
  batch5('privacy.loopback-only', networkOriginList.every(origin => origin === 'null' || origin.startsWith('http://127.0.0.1:')), JSON.stringify(networkOriginList));
  check('runtime.no-errors', runtimeErrors.length === 0, runtimeErrors.join('\n'));

  const result = {
    runId,
    frontendUrl,
    backendUrl,
    passed: checks.filter(item => item.pass).length,
    failed: checks.filter(item => !item.pass).length,
    blockingFailed: checks.filter(item => !item.pass && (item.id.startsWith('batch1.') || item.id.startsWith('batch2.') || item.id.startsWith('batch3.') || item.id.startsWith('batch4.') || item.id.startsWith('batch5.') || item.id.startsWith('drawings.') || item.id === 'runtime.no-errors')).length,
    checks,
    runtimeErrors,
    expectedPracticeConsoleErrors,
    indicatorResponses,
    indicatorRequestFailures,
    providerErrors,
    networkOrigins: networkOriginList,
    hardening,
  };
  await writeFile(path.join(runDir, 'results.json'), JSON.stringify(result, null, 2));
  console.log(JSON.stringify(result, null, 2));
  console.log(`Product UAT artifacts: ${runDir}`);
  if (result.blockingFailed > 0) process.exitCode = 1;
} catch (error) {
  const partial = {
    runId, frontendUrl, backendUrl, failedAt: new Date().toISOString(),
    error: error instanceof Error ? { name: error.name, message: error.message, stack: error.stack } : String(error),
    checks, runtimeErrors, expectedPracticeConsoleErrors, indicatorResponses, indicatorRequestFailures,
    providerErrors, networkOrigins: [...networkOrigins].sort(), hardening,
  };
  await writeFile(path.join(runDir, 'partial-results.json'), JSON.stringify(partial, null, 2));
  throw error;
} finally {
  await context.close();
  await browser.close();
}

async function launchBrowser() {
  const candidates = [
    { headless: true, channel: process.env.SUMI_BROWSER_CHANNEL || 'chrome' },
    { headless: true },
  ];
  const errors = [];
  for (const options of candidates) {
    try {
      return await chromium.launch(options);
    } catch (error) {
      errors.push(error instanceof Error ? error.message : String(error));
    }
  }
  throw new Error(`Unable to launch product UAT browser:\n${errors.join('\n---\n')}`);
}

function toDateKeyForUat(value) {
  return String(value).slice(0, 10);
}

function normalizeUrl(value) {
  const url = new URL(value);
  const entries = [...url.searchParams.entries()].sort(([leftKey, leftValue], [rightKey, rightValue]) => leftKey.localeCompare(rightKey) || leftValue.localeCompare(rightValue));
  url.search = '';
  for (const [key, item] of entries) url.searchParams.append(key, item);
  return url.toString();
}

function materializeCorpusCaseForUat(base, patches) {
  const document = structuredClone(base);
  const decode = part => part.replace(/~1/g, '/').replace(/~0/g, '~');
  const locate = pathValue => pathValue.split('/').slice(1).map(decode).reduce((value, part) => Array.isArray(value) ? value[Number(part)] : value[part], document);
  for (const patch of patches) {
    const parts = patch.path.split('/').slice(1).map(decode); const key = parts.pop();
    const parent = parts.reduce((value, part) => Array.isArray(value) ? value[Number(part)] : value[part], document);
    if (patch.op === 'remove') { if (Array.isArray(parent)) parent.splice(Number(key), 1); else delete parent[key]; continue; }
    const value = patch.valueFrom ? structuredClone(locate(patch.valueFrom)) : patch.repeat ? patch.repeat.value.repeat(patch.repeat.count) : structuredClone(patch.value);
    if (Array.isArray(parent)) parent[Number(key)] = value; else parent[key] = value;
  }
  return document;
}
