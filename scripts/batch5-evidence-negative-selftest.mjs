import { spawnSync } from 'node:child_process';
import { mkdtemp, mkdir, readFile, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { validateFutureBoundary } from './batch5-closure-contract.mjs';
import { EXPECTED_NEGATIVE_CASES } from './batch5-manifest.mjs';

const root = path.resolve(new URL('..', import.meta.url).pathname);
const auditCli = path.join(root, 'scripts/batch5-evidence-audit.mjs');
const manifestCli = path.join(root, 'scripts/batch5-manifest.mjs');
const baselinePath = path.join(root, 'test-results/batch5-hardening/2026-07-19T01-05-23Z/product-uat/2026-07-19T01-05-26-343Z/results.json');
const baseline = JSON.parse(await readFile(baselinePath, 'utf8'));
const tempRoot = await mkdtemp(path.join(os.tmpdir(), 'sumi-batch5-negative.'));
const results = [];
const run = (label, args, options = {}) => {
  const child = spawnSync(process.execPath, args, { encoding: 'utf8', ...options });
  const pass = child.status !== 0;
  results.push({ label, pass, exitCode: child.status, stderr: child.stderr.trim().slice(0, 500) });
  if (!pass) throw new Error(`${label} unexpectedly exited zero`);
};
const productWithClosure = () => ({ ...structuredClone(baseline), passed: 273, failed: 0, blockingFailed: 0,
  checks: [...structuredClone(baseline.checks), { id: 'batch5.closure.fixture', pass: true, evidence: 'fixture' }] });

for (const [label, mutate] of [
  ['audit.failed-additive', value => { value.checks.at(-1).pass = false; value.passed = 272; value.failed = 1; value.blockingFailed = 1; }],
  ['audit.duplicate-baseline', value => { value.checks.push(structuredClone(value.checks[0])); value.passed += 1; }],
  ['audit.missing-baseline', value => { value.checks.splice(0, 1); value.passed -= 1; }],
  ['audit.changed-baseline', value => { value.checks[0].pass = false; value.passed -= 1; value.failed = 1; }],
  ['audit.nonzero-blocking', value => { value.blockingFailed = 1; }],
]) {
  const dir = path.join(tempRoot, label); await mkdir(dir, { recursive: true });
  const current = productWithClosure(); mutate(current);
  const currentPath = path.join(dir, 'current.json'); await writeFile(currentPath, JSON.stringify(current));
  run(label, [auditCli, baselinePath, currentPath, path.join(dir, 'audit.json')]);
}

async function manifestFixture(label, mutate) {
  const dir = path.join(tempRoot, label); await mkdir(dir, { recursive: true });
  const files = {
    product: productWithClosure(), workspace: { fixture: true },
    fresh: { source: { tables: ['alembic_version'] } },
    copyPre: { applicationSchema: { pass: true } }, copyPost: { source: { tables: ['alembic_version'] } },
    recovery: { semanticEqual: true, source: { semanticSha256: 'same' }, backup: { semanticSha256: 'same' } },
    restored: { source: { semanticSha256: 'same' } },
    restore: { pass: true, comparisons: { practice: true, indicators: true, drawings: true, journal: true }, runtimeErrors: [] },
  };
  const names = { product: 'product.json', workspace: 'workspace.json', fresh: 'fresh-migration.json', copyPre: 'production-copy-pre-snapshot.json',
    copyPost: 'production-copy-snapshot.json', recovery: 'database-recovery.json', restored: 'restored-snapshot.json', restore: 'restore-results.json' };
  for (const [key, name] of Object.entries(names)) await writeFile(path.join(dir, name), JSON.stringify(files[key]));
  await writeFile(path.join(dir, 'backup.db'), 'backup fixture'); await writeFile(path.join(dir, 'restored.db'), 'restored fixture');
  await writeFile(path.join(dir, 'negative-selftest.json'), JSON.stringify({ pass: true,
    cases: EXPECTED_NEGATIVE_CASES.map(label => ({ label, pass: true })) }));
  const auditPath = path.join(dir, 'baseline-audit.json');
  const audit = spawnSync(process.execPath, [auditCli, baselinePath, path.join(dir, names.product), auditPath], { encoding: 'utf8' });
  if (audit.status !== 0) throw new Error(`valid fixture audit failed for ${label}: ${audit.stderr}`);
  const env = { ...process.env, SUMI_BATCH5_ARTIFACT_DIR: dir, SUMI_BATCH5_PRODUCT_RESULTS: path.join(dir, names.product),
    SUMI_BATCH5_WORKSPACE_EXPORT: path.join(dir, names.workspace), SUMI_BATCH5_BACKUP_DB: path.join(dir, 'backup.db'),
    SUMI_BATCH5_RESTORED_DB: path.join(dir, 'restored.db'), SUMI_BATCH5_PRODUCTION_DB: path.join(root, 'backend/sumi.db'),
    SUMI_BATCH5_PRODUCTION_SHA_BEFORE: '60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d',
    SUMI_BATCH5_HEAD: '108aa5dc0e26994607836e2b3b33f482e3791b4e', SUMI_BATCH5_TAG_TARGET: '812675ce37d30ddfafc11c6eeca299b5cd8a3c9e' };
  await mutate({ dir, files, names, env });
  run(label, [manifestCli], { env });
}

await manifestFixture('manifest.missing-artifact', async ({ dir, env }) => { env.SUMI_BATCH5_WORKSPACE_EXPORT = path.join(dir, 'absent.json'); });
await manifestFixture('manifest.recovery-failure', async ({ dir, files, names }) => { files.recovery.semanticEqual = false; await writeFile(path.join(dir, names.recovery), JSON.stringify(files.recovery)); });
await manifestFixture('manifest.restore-failure', async ({ dir, files, names }) => { files.restore.pass = false; await writeFile(path.join(dir, names.restore), JSON.stringify(files.restore)); });
await manifestFixture('manifest.migration-failure', async ({ dir, files, names }) => { files.fresh.source.tables = []; await writeFile(path.join(dir, names.fresh), JSON.stringify(files.fresh)); });
await manifestFixture('manifest.database-mismatch', async ({ env }) => { env.SUMI_BATCH5_PRODUCTION_SHA_BEFORE = 'changed'; });
await manifestFixture('manifest.provenance-mismatch', async ({ env }) => { env.SUMI_BATCH5_HEAD = 'changed'; });
await manifestFixture('manifest.external-existing-artifact', async ({ env }) => { env.SUMI_BATCH5_WORKSPACE_EXPORT = baselinePath; });

const validBoundary = { authoritativeIndex: 249, authoritativeDate: '2023-12-15', practiceIndex: 249,
  apiCandleCount: 250, apiMaxDate: '2023-12-15', chartCandleCount: 250, chartMaxDate: '2023-12-15',
  indicators: [{ id: 'rsi', inputMaxDate: '2023-12-15', responseMaxDate: '2023-12-15', seriesMaxDates: { primary: '2023-12-15' } }],
  markerMaxDate: '2023-12-14', visibleProviderDrawings: [{ id: 'past', anchorDates: ['2023-12-14'] }],
  retainedFutureDrawings: [{ id: 'future', providerVisible: false }] };
for (const [label, mutate, surface] of [
  ['future.api', value => { value.apiMaxDate = '2023-12-18'; }, 'api.maxDate'],
  ['future.chart', value => { value.chartCandleCount += 1; }, 'chart.count'],
  ['future.indicator-input', value => { value.indicators[0].inputMaxDate = '2023-12-18'; }, 'indicator.rsi.inputMaxDate'],
  ['future.indicator-response', value => { value.indicators[0].responseMaxDate = '2023-12-18'; }, 'indicator.rsi.responseMaxDate'],
  ['future.indicator-chart', value => { value.indicators[0].seriesMaxDates.primary = '2023-12-18'; }, 'indicator.rsi.chart.primary'],
  ['future.marker', value => { value.markerMaxDate = '2023-12-18'; }, 'markers.maxDate'],
  ['future.drawing', value => { value.visibleProviderDrawings[0].anchorDates = ['2023-12-18']; }, 'drawing.past.anchor'],
  ['future.retained-drawing-visible', value => { value.retainedFutureDrawings[0].providerVisible = true; }, 'drawing.future.retained-future-visible'],
]) {
  const fixture = structuredClone(validBoundary); mutate(fixture); const verdict = validateFutureBoundary(fixture);
  const pass = !verdict.pass && verdict.failures.some(item => item.surface === surface);
  results.push({ label, pass, failures: verdict.failures });
  if (!pass) throw new Error(`${label} did not fail on ${surface}`);
}

console.log(JSON.stringify({ pass: results.every(item => item.pass), cases: results }, null, 2));
