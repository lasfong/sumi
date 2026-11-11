import { createRequire } from 'node:module';
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';

const require = createRequire(new URL('../frontend/package.json', import.meta.url));
const { chromium } = require('playwright');
const url = process.env.SUMI_DEEPENTROPY_SPIKE_URL || 'http://127.0.0.1:41731';
const runId = new Date().toISOString().replace(/[:.]/g, '-');
const runDir = path.resolve('test-results/drawing-provider-spike/deepentropy', runId);
await mkdir(runDir, { recursive: true });

const browser = await chromium.launch({ headless: true, channel: process.env.SUMI_BROWSER_CHANNEL || 'chrome' });
const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
const page = await context.newPage();
const checks = [];
const runtimeErrors = [];
const check = (id, pass, evidence) => checks.push({ id, pass, evidence });
page.on('pageerror', error => runtimeErrors.push(`pageerror: ${error.message}`));
page.on('console', message => {
  if (message.type() === 'error') runtimeErrors.push(`console: ${message.text()}`);
});

const status = () => page.getByTestId('status').innerText();
const drawings = async () => JSON.parse(await page.getByTestId('serialized').innerText());
const chartBox = async () => {
  const box = await page.getByTestId('chart').boundingBox();
  if (!box) throw new Error('Chart has no bounding box');
  return box;
};
const point = (box, x, y) => ({ x: box.x + box.width * x, y: box.y + box.height * y });

try {
  await page.goto(url);
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  const box = await chartBox();

  const requiredTools = ['cursor', 'horizontal-line', 'trend-line', 'ray', 'rectangle', 'fib-retracement', 'text-annotation'];
  for (const tool of requiredTools) {
    check(`D-01.tool-${tool}`, await page.getByTestId(`tool-${tool}`).isVisible(), `tool-${tool} visible`);
  }

  await page.getByTestId('tool-trend-line').click();
  await page.mouse.click(...Object.values(point(box, 0.25, 0.35)));
  await page.keyboard.press('Escape');
  check('D-02.escape-cancel', /tool=cursor/.test(await status()) && (await drawings()).length === 0, await status());

  const placements = [
    ['horizontal-line', [[0.30, 0.28]]],
    ['trend-line', [[0.25, 0.36], [0.52, 0.52]]],
    ['ray', [[0.34, 0.66], [0.68, 0.44]]],
    ['rectangle', [[0.18, 0.23], [0.40, 0.45]]],
    ['fib-retracement', [[0.52, 0.24], [0.76, 0.67]]],
    ['text-annotation', [[0.74, 0.31]]],
  ];
  for (const [tool, anchors] of placements) {
    await page.getByTestId(`tool-${tool}`).click();
    for (const [x, y] of anchors) {
      const p = point(box, x, y);
      await page.mouse.click(p.x, p.y);
    }
    await page.waitForTimeout(80);
  }
  const created = await drawings();
  check('D-01.required-tool-create', created.length === 6, created.map(item => item.type).join(', '));
  check('D-03.selected-with-handles', /selected=spike-6/.test(await status()), await status());

  await page.getByTestId('tool-cursor').click();
  const textBefore = (await drawings()).find(item => item.type === 'text-annotation').anchors[0];
  const textAnchor = point(box, 0.74, 0.31);
  await page.mouse.move(textAnchor.x, textAnchor.y);
  await page.mouse.down();
  await page.mouse.move(textAnchor.x + 44, textAnchor.y + 32, { steps: 5 });
  await page.mouse.up();
  const textAfter = (await drawings()).find(item => item.type === 'text-annotation').anchors[0];
  check('D-04.anchor-edit', JSON.stringify(textBefore) !== JSON.stringify(textAfter), `${JSON.stringify(textBefore)} -> ${JSON.stringify(textAfter)}`);

  const trendBefore = (await drawings()).find(item => item.type === 'trend-line').anchors;
  const trendMid = point(box, 0.385, 0.44);
  await page.mouse.click(trendMid.x, trendMid.y);
  const selectedTrend = !/selected=none/.test(await status());
  await page.mouse.move(trendMid.x, trendMid.y);
  await page.mouse.down();
  await page.mouse.move(trendMid.x + 40, trendMid.y + 25, { steps: 5 });
  await page.mouse.up();
  const trendAfter = (await drawings()).find(item => item.type === 'trend-line').anchors;
  const wholeMove = JSON.stringify(trendBefore) !== JSON.stringify(trendAfter);
  check('D-03.canvas-select', selectedTrend, await status());
  check('D-04.whole-drawing-move', wholeMove, `${JSON.stringify(trendBefore)} -> ${JSON.stringify(trendAfter)}`);

  const countBeforeDelete = (await drawings()).length;
  await page.keyboard.press('Delete');
  check('D-05.keyboard-delete', (await drawings()).length === countBeforeDelete - 1, await status());
  await page.getByTestId('undo').click();
  const undoRestored = (await drawings()).length === countBeforeDelete;
  await page.getByTestId('redo').click();
  const redoDeleted = (await drawings()).length === countBeforeDelete - 1;
  await page.getByTestId('undo').click();
  check('D-06.adapter-undo-redo-delete', undoRestored && redoDeleted, `undo=${undoRestored} redo=${redoDeleted}`);

  const stableCount = (await drawings()).length;
  for (const action of ['pan', 'zoom', 'advance', 'resize']) {
    await page.getByTestId(action).click();
    await page.waitForTimeout(100);
  }
  check('D-07.pan-zoom-resize-replay', (await drawings()).length === stableCount, await status());

  const providerState = await drawings();
  const textState = providerState.find(item => item.type === 'text-annotation');
  const fibState = providerState.find(item => item.type === 'fib-retracement');
  check('D-08.base-json-export', providerState.length === stableCount && providerState.every(item => item.id && item.type && item.anchors && item.style && item.options), JSON.stringify(providerState));
  check('D-08.semantic-properties-export', Boolean(textState?.options?.text && fibState?.options?.levels), `text options=${JSON.stringify(textState?.options)} fib options=${JSON.stringify(fibState?.options)}`);
  check('D-09.fibonacci-create-direction-edit', Boolean(fibState && fibState.anchors.length === 2), JSON.stringify(fibState));
  check('D-10.magnet-configured', false, 'Provider SnapConfig exists but applySnap returns the anchor unchanged; spike exposes no working magnet mode.');

  await page.getByTestId('save').click();
  await page.screenshot({ path: path.join(runDir, '01-required-tools.png'), fullPage: true });
  await page.reload();
  await page.waitForTimeout(250);
  check('D-08.reload-roundtrip-count', (await drawings()).length === stableCount, await status());
  for (let cycle = 0; cycle < 10; cycle += 1) {
    await page.getByTestId('mount-toggle').click();
    await page.getByTestId('unmounted').waitFor();
    await page.getByTestId('mount-toggle').click();
    await page.getByTestId('chart').waitFor();
  }
  check('D-11.mount-unmount-errors', runtimeErrors.length === 0, runtimeErrors.join('\n'));
  await page.screenshot({ path: path.join(runDir, '02-reload-lifecycle.png'), fullPage: true });
  check('runtime.no-errors', runtimeErrors.length === 0, runtimeErrors.join('\n'));
} finally {
  const result = {
    runId,
    provider: 'deepentropy/lightweight-charts-drawing',
    packageVersion: '0.1.1',
    packageTagRevision: '778f1e5cf3d62c2499dd4c686a00ab66bb01c44f',
    auditedHeadRevision: '5f2afc335028d6a188ce0a50361056518c84cf72',
    url,
    viewport: { width: 1440, height: 1000 },
    passed: checks.filter(item => item.pass).length,
    failed: checks.filter(item => !item.pass).length,
    checks,
    runtimeErrors,
  };
  await writeFile(path.join(runDir, 'results.json'), JSON.stringify(result, null, 2));
  console.log(JSON.stringify(result, null, 2));
  console.log(`Deepentropy spike artifacts: ${runDir}`);
  await context.close();
  await browser.close();
}
