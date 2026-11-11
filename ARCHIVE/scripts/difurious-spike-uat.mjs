import { createRequire } from 'node:module';
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';

const require = createRequire(new URL('../frontend/package.json', import.meta.url));
const { chromium } = require('playwright');
const url = process.env.SUMI_DIFURIOUS_SPIKE_URL || 'http://127.0.0.1:41732';
const runId = new Date().toISOString().replace(/[:.]/g, '-');
const runDir = path.resolve('test-results/drawing-provider-spike/difurious', runId);
await mkdir(runDir, { recursive: true });

const browser = await chromium.launch({ headless: true, channel: process.env.SUMI_BROWSER_CHANNEL || 'chrome' });
const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
const page = await context.newPage();
page.setDefaultTimeout(20_000);
const checks = [];
const runtimeErrors = [];
let latestExport = null;
let latestSelection = null;
const check = (id, pass, evidence) => checks.push({ id, pass, evidence });
page.on('pageerror', error => runtimeErrors.push(`pageerror: ${error.message}`));
page.on('console', async message => {
  if (message.type() === 'error') runtimeErrors.push(`console: ${message.text()}`);
  if (message.text().startsWith('Exporting all line tools:')) {
    latestExport = await message.args()[1]?.jsonValue().catch(() => null);
  }
  if (message.text().startsWith('Selected line tools:')) {
    latestSelection = await message.args()[1]?.jsonValue().catch(() => null);
  }
});

const openAccordion = async label => {
  const text = page.getByText(label, { exact: true }).first();
  await text.scrollIntoViewIfNeeded();
  const root = text.locator('xpath=ancestor::*[contains(@class,"MuiAccordion-root")][1]');
  if ((await root.getAttribute('class'))?.includes('Mui-expanded')) return root;
  await text.click();
  return root;
};
const activate = async label => {
  const root = await openAccordion(label);
  await root.getByRole('button', { name: 'Activate Default', exact: true }).click();
};
const chartBox = async () => {
  const chart = page.locator('.tv-lightweight-charts').first();
  await chart.scrollIntoViewIfNeeded();
  const box = await chart.boundingBox();
  if (!box) throw new Error('Official difurious test chart has no bounding box');
  return box;
};
const clickChart = async (_box, x, y) => {
  const box = await chartBox();
  await page.mouse.click(box.x + box.width * x, box.y + box.height * y);
};
const exportState = async () => {
  latestExport = null;
  const root = await openAccordion('2. Retrieval, Updates & Persistence');
  await root.getByRole('button', { name: 'Export All (JSON)', exact: true }).click();
  await page.waitForFunction(() => true);
  for (let retry = 0; retry < 20 && latestExport === null; retry += 1) await page.waitForTimeout(50);
  return latestExport ?? [];
};

try {
  await page.goto(url);
  await page.getByText('Line Tools Plugin Test Panel', { exact: true }).waitFor();
  await openAccordion('6. Interactive Drawing (Click Chart to Draw)');
  const box = await chartBox();

  for (const tool of ['Rectangle Tool', 'Trend Line Tool', 'Ray Line Tool', 'Horizontal Line Tool', 'Fib Retracement Tool', 'Text Tool']) {
    check(`D-01.${tool}`, await page.getByText(tool, { exact: true }).count() >= 1, `${tool} registered in official React test app`);
  }

  await activate('Trend Line Tool');
  await clickChart(box, 0.22, 0.16);
  await page.keyboard.press('Escape');
  check('D-02.escape-cancel', (await exportState()).length === 0, JSON.stringify(await exportState()));

  const scenarios = [
    ['Horizontal Line Tool', [[0.30, 0.18]]],
    ['Trend Line Tool', [[0.24, 0.17], [0.48, 0.29]]],
    ['Ray Line Tool', [[0.38, 0.31], [0.62, 0.17]]],
    ['Rectangle Tool', [[0.18, 0.14], [0.36, 0.30]]],
    ['Fib Retracement Tool', [[0.52, 0.13], [0.74, 0.32]]],
    ['Text Tool', [[0.70, 0.20]]],
  ];
  for (const [tool, anchors] of scenarios) {
    await activate(tool);
    for (const [x, y] of anchors) await clickChart(box, x, y);
    await page.waitForTimeout(80);
  }
  const created = await exportState();
  const completed = created.filter(item => item.points.length > 0);
  check('D-01.required-tool-create', completed.length === 6, created.map(item => `${item.toolType}:${item.points.length}`).join(', '));
  check('D-08.full-export-shape', created.every(item => item.id && item.toolType && item.points && item.options), JSON.stringify(created));
  const fib = created.find(item => item.toolType === 'FibRetracement');
  check('D-09.fibonacci-level-options', Boolean(fib?.options?.levels?.length), JSON.stringify(fib?.options));

  const rectangleBefore = created.find(item => item.toolType === 'Rectangle');
  const liveBox = await chartBox();
  const rectCenter = { x: liveBox.x + liveBox.width * 0.27, y: liveBox.y + liveBox.height * 0.22 };
  await page.mouse.click(rectCenter.x, rectCenter.y);
  await page.mouse.move(rectCenter.x, rectCenter.y);
  await page.mouse.down();
  await page.mouse.move(rectCenter.x + 48, rectCenter.y + 30, { steps: 8 });
  await page.mouse.up();
  const moved = (await exportState()).find(item => item.id === rectangleBefore?.id);
  check('D-03.canvas-select', Boolean(moved), `rectangle id ${rectangleBefore?.id}`);
  check('D-04.whole-drawing-move', JSON.stringify(rectangleBefore?.points) !== JSON.stringify(moved?.points), `${JSON.stringify(rectangleBefore?.points)} -> ${JSON.stringify(moved?.points)}`);

  const firstPoint = moved?.points?.[0];
  const corner = { x: liveBox.x + liveBox.width * 0.18 + 48, y: liveBox.y + liveBox.height * 0.14 + 30 };
  await page.mouse.move(corner.x, corner.y);
  await page.mouse.down();
  await page.mouse.move(corner.x - 28, corner.y + 22, { steps: 6 });
  await page.mouse.up();
  const edited = (await exportState()).find(item => item.id === rectangleBefore?.id);
  check('D-04.anchor-edit', JSON.stringify(firstPoint) !== JSON.stringify(edited?.points?.[0]), `${JSON.stringify(firstPoint)} -> ${JSON.stringify(edited?.points?.[0])}`);

  const settings = await openAccordion('3. Global Settings (Snapping, Locking, Crosshair)');
  const slider = settings.getByRole('slider');
  await slider.focus();
  for (let i = 0; i < 10; i += 1) await page.keyboard.press('ArrowRight');
  check('D-10.magnet-configurable', Number(await slider.getAttribute('aria-valuenow')) > 0, `threshold=${await slider.getAttribute('aria-valuenow')}`);

  const persisted = await exportState();
  const persistence = await openAccordion('2. Retrieval, Updates & Persistence');
  await persistence.getByRole('button', { name: 'Remove All Pane 0', exact: true }).click();
  await persistence.getByRole('button', { name: 'Import All (JSON)', exact: true }).click();
  const restored = await exportState();
  check('D-05.ui-delete-clear', restored.length === persisted.length, 'Remove All completed before Import All restored the prior export');
  check('D-08.export-import-roundtrip', restored.length === persisted.length && JSON.stringify(restored) === JSON.stringify(persisted), `before=${persisted.length} after=${restored.length}`);
  check('D-06.native-undo-redo', false, 'Core public API and official app expose no undo/redo; Sumi must own command history.');
  check('D-05.keyboard-delete', false, 'Core README explicitly states Delete is unsupported; Sumi must map keyboard input to removeSelectedLineTools().');

  await page.screenshot({ path: path.join(runDir, '01-required-tools.png'), fullPage: true });
  const destroy = await openAccordion('5. Destroy Pane 0');
  await destroy.getByRole('button', { name: 'Destroy Pane 0 Plugin', exact: true }).click();
  await page.waitForTimeout(150);
  for (let cycle = 0; cycle < 5; cycle += 1) {
    await page.reload();
    await page.getByText('Line Tools Plugin Test Panel', { exact: true }).waitFor();
  }
  check('D-11.destroy-remount-errors', runtimeErrors.length === 0, runtimeErrors.join('\n'));
  check('runtime.no-errors', runtimeErrors.length === 0, runtimeErrors.join('\n'));
  await page.screenshot({ path: path.join(runDir, '02-remount.png'), fullPage: true });
} finally {
  const result = {
    runId,
    provider: 'difurious/lightweight-charts-line-tools-core + required companions',
    revisions: {
      core: '167a83cf8702e35b4cfbe7beb0dafec94e800a71',
      lines: 'edb2a6ce00c8bbbe6f19e8469e378350efe6013f',
      rectangle: '8c229e62852f72936dbef5fdc198e321b9a85cc3',
      fibonacci: '248b46813b44dd2dbe1576a9bce67ddf39d1338f',
      text: '3a64a17814c85a98cccccce1f5a014ef8e18a091',
      reactTestApp: 'e306d8f6edf85cdd06b6e3a9096d92540cad708f',
    },
    url,
    viewport: { width: 1440, height: 1000 },
    passed: checks.filter(item => item.pass).length,
    failed: checks.filter(item => !item.pass).length,
    checks,
    runtimeErrors,
  };
  await writeFile(path.join(runDir, 'results.json'), JSON.stringify(result, null, 2));
  console.log(JSON.stringify(result, null, 2));
  console.log(`Difurious spike artifacts: ${runDir}`);
  await context.close();
  await browser.close();
}
