import { createHash } from 'node:crypto';
import { pathToFileURL } from 'node:url';
import { lstat, readFile, realpath, writeFile } from 'node:fs/promises';
import path from 'node:path';

const EXPECTED_HEAD = '108aa5dc0e26994607836e2b3b33f482e3791b4e';
const EXPECTED_TAG = '812675ce37d30ddfafc11c6eeca299b5cd8a3c9e';
const EXPECTED_PRODUCTION_SHA = '60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d';
export const EXPECTED_NEGATIVE_CASES = [
  'audit.failed-additive',
  'audit.duplicate-baseline',
  'audit.missing-baseline',
  'audit.changed-baseline',
  'audit.nonzero-blocking',
  'manifest.missing-artifact',
  'manifest.recovery-failure',
  'manifest.restore-failure',
  'manifest.migration-failure',
  'manifest.database-mismatch',
  'manifest.provenance-mismatch',
  'manifest.external-existing-artifact',
  'future.api',
  'future.chart',
  'future.indicator-input',
  'future.indicator-response',
  'future.indicator-chart',
  'future.marker',
  'future.drawing',
  'future.retained-drawing-visible',
];
const shaBuffer = value => createHash('sha256').update(value).digest('hex');
const isInside = (root, candidate) => {
  const relative = path.relative(root, candidate);
  return relative !== '' && !relative.startsWith(`..${path.sep}`) && relative !== '..' && !path.isAbsolute(relative);
};

async function readContainedRegularFile(root, suppliedPath) {
  if (typeof suppliedPath !== 'string' || suppliedPath.length === 0) throw new Error('all canonical evidence paths are required');
  if (suppliedPath.split(/[\\/]+/).includes('..')) throw new Error(`canonical evidence path traversal is forbidden: ${suppliedPath}`);
  const lexicalRoot = path.resolve(root);
  const candidate = path.resolve(lexicalRoot, suppliedPath);
  if (!isInside(lexicalRoot, candidate)) throw new Error(`canonical evidence path is outside artifact root: ${suppliedPath}`);
  const metadata = await lstat(candidate);
  if (metadata.isSymbolicLink() || !metadata.isFile()) throw new Error(`canonical evidence path is not a regular non-symlink file: ${suppliedPath}`);
  const canonicalRoot = await realpath(lexicalRoot);
  const canonical = await realpath(candidate);
  if (!isInside(canonicalRoot, canonical)) throw new Error(`canonical evidence path escapes artifact root: ${suppliedPath}`);
  const buffer = await readFile(canonical);
  return { buffer, canonical, relativePath: path.relative(canonicalRoot, canonical) };
}

function validateNegativeSelftest(value) {
  const labels = Array.isArray(value?.cases) ? value.cases.map(item => item?.label) : [];
  const unique = new Set(labels);
  const expected = new Set(EXPECTED_NEGATIVE_CASES);
  return value?.pass === true
    && labels.length === EXPECTED_NEGATIVE_CASES.length
    && unique.size === EXPECTED_NEGATIVE_CASES.length
    && labels.every(label => expected.has(label))
    && EXPECTED_NEGATIVE_CASES.every(label => unique.has(label))
    && value.cases.every(item => item?.pass === true);
}

export async function verifyManifestFiles(artifactDir, manifest) {
  const root = path.resolve(artifactDir);
  const results = {};
  for (const [key, entry] of Object.entries(manifest.files ?? {})) {
    if (path.isAbsolute(entry.path)) throw new Error(`manifest file path must be bundle-relative: ${entry.path}`);
    const evidence = await readContainedRegularFile(root, entry.path);
    const actualSha256 = shaBuffer(evidence.buffer);
    if (actualSha256 !== entry.sha256) throw new Error(`manifest checksum mismatch for ${key}`);
    results[key] = { path: evidence.relativePath, insideArtifactRoot: true, regularFile: true, symlink: false, sha256Matches: true };
  }
  return results;
}

export async function createManifest(env = process.env) {
  const artifactDir = env.SUMI_BATCH5_ARTIFACT_DIR;
  if (!artifactDir) throw new Error('SUMI_BATCH5_ARTIFACT_DIR is required');
  const root = path.resolve(artifactDir);
  const cited = {
    productUat: env.SUMI_BATCH5_PRODUCT_RESULTS,
    workspaceExport: env.SUMI_BATCH5_WORKSPACE_EXPORT,
    databaseBackup: env.SUMI_BATCH5_BACKUP_DB,
    restoredDatabase: env.SUMI_BATCH5_RESTORED_DB,
    freshMigration: path.join(root, 'fresh-migration.json'),
    productionCopyPreMigration: path.join(root, 'production-copy-pre-snapshot.json'),
    productionCopyMigration: path.join(root, 'production-copy-snapshot.json'),
    databaseRecovery: path.join(root, 'database-recovery.json'),
    restoredSnapshot: path.join(root, 'restored-snapshot.json'),
    restoredBrowser: path.join(root, 'restore-results.json'),
    baselineAudit: path.join(root, 'baseline-audit.json'),
    negativeSelftest: path.join(root, 'negative-selftest.json'),
  };
  const evidence = Object.fromEntries(await Promise.all(Object.entries(cited).map(async ([key, file]) => [key, await readContainedRegularFile(root, file)])));
  const parsed = key => JSON.parse(evidence[key].buffer.toString('utf8'));
  const product = parsed('productUat');
  const fresh = parsed('freshMigration');
  const copyPre = parsed('productionCopyPreMigration');
  const copyPost = parsed('productionCopyMigration');
  const recovery = parsed('databaseRecovery');
  const restored = parsed('restoredSnapshot');
  const restoreBrowser = parsed('restoredBrowser');
  const audit = parsed('baselineAudit');
  const negativeSelftest = parsed('negativeSelftest');
  const productionDb = env.SUMI_BATCH5_PRODUCTION_DB;
  const productionAfterSha = shaBuffer(await readFile(productionDb));
  const productChecks = product.checks ?? [];
  const productPass = productChecks.length === product.passed && product.failed === 0 && product.blockingFailed === 0
    && productChecks.every(item => item.pass === true) && (product.runtimeErrors ?? []).length === 0
    && (product.providerErrors ?? []).length === 0 && (product.indicatorRequestFailures ?? []).length === 0;
  const freshMigrationPass = fresh.source?.tables?.includes('alembic_version') === true;
  const copyMigrationPass = copyPre.applicationSchema?.pass === true && copyPost.source?.tables?.includes('alembic_version');
  const recoveryPass = recovery.semanticEqual === true && recovery.source?.semanticSha256 === recovery.backup?.semanticSha256
    && restored.source?.semanticSha256 === recovery.backup?.semanticSha256;
  const restorePass = restoreBrowser.pass === true && Object.values(restoreBrowser.comparisons ?? {}).every(value => value === true)
    && (restoreBrowser.runtimeErrors ?? []).length === 0;
  const provenancePass = env.SUMI_BATCH5_HEAD === EXPECTED_HEAD && env.SUMI_BATCH5_TAG_TARGET === EXPECTED_TAG;
  const productionPass = env.SUMI_BATCH5_PRODUCTION_SHA_BEFORE === EXPECTED_PRODUCTION_SHA
    && productionAfterSha === EXPECTED_PRODUCTION_SHA;
  const negativeSelftestPass = validateNegativeSelftest(negativeSelftest);
  const manifest = {
    schemaVersion: 3, generatedAt: new Date().toISOString(), localOnly: true,
    artifactRoot: '.',
    commands: [
      'DATABASE_URL=sqlite:////temporary/fresh.db ../.venv/bin/python -m alembic upgrade head',
      'DATABASE_URL=sqlite:////temporary/fresh.db ../.venv/bin/python scripts/seed_demo.py',
      'SUMI_BATCH5_DURATION_SECONDS=1800 node scripts/batch5-hardening-uat.mjs',
      'python backend/scripts/batch5_recovery_verify.py --source fresh.db --backup backup.db',
      'validate application schema; stamp/upgrade a temporary legacy copy only',
      'node scripts/batch5-restore-uat.mjs',
      'node scripts/batch5-evidence-audit.mjs BASELINE CURRENT OUTPUT',
      'node scripts/batch5-evidence-negative-selftest.mjs',
    ],
    files: Object.fromEntries(Object.entries(evidence).map(([key, value]) => [key, { path: value.relativePath, sha256: shaBuffer(value.buffer) }])),
    containment: Object.fromEntries(Object.entries(evidence).map(([key, value]) => [key, { path: value.relativePath, insideArtifactRoot: true, regularFile: true, symlink: false }])),
    verdicts: { productPass, auditPass: audit.pass === true, freshMigrationPass, copyMigrationPass, recoveryPass, restorePass, negativeSelftestPass, productionPass, provenancePass },
    product: { passed: product.passed, failed: product.failed, blockingFailed: product.blockingFailed, checkCount: productChecks.length,
      runtimeErrors: product.runtimeErrors, providerErrors: product.providerErrors, indicatorRequestFailures: product.indicatorRequestFailures,
      expectedPracticeConsoleErrors: product.expectedPracticeConsoleErrors },
    negativeSelftest: { pass: negativeSelftestPass, expectedCount: EXPECTED_NEGATIVE_CASES.length, caseCount: negativeSelftest.cases?.length ?? 0, expectedCases: EXPECTED_NEGATIVE_CASES },
    baselineAudit: audit, freshMigration: fresh, productionCopyPreMigration: copyPre, productionCopyMigration: copyPost,
    databaseRecovery: recovery, restoredSnapshot: restored, restore: restoreBrowser,
    productionDatabase: { path: productionDb, provenanceOnly: true, canonicalArtifact: false, expectedSha256: EXPECTED_PRODUCTION_SHA, beforeSha256: env.SUMI_BATCH5_PRODUCTION_SHA_BEFORE, afterSha256: productionAfterSha },
    provenance: { expectedHead: EXPECTED_HEAD, head: env.SUMI_BATCH5_HEAD, expectedProtectedTagTarget: EXPECTED_TAG, protectedTagTarget: env.SUMI_BATCH5_TAG_TARGET },
  };
  manifest.pass = Object.values(manifest.verdicts).every(Boolean);
  return manifest;
}

async function main() {
  const artifactDir = process.env.SUMI_BATCH5_ARTIFACT_DIR;
  const manifest = await createManifest();
  const manifestPath = path.join(artifactDir, 'manifest.json');
  await writeFile(manifestPath, JSON.stringify(manifest, null, 2));
  manifest.postWriteVerification = await verifyManifestFiles(artifactDir, JSON.parse(await readFile(manifestPath, 'utf8')));
  await writeFile(manifestPath, JSON.stringify(manifest, null, 2));
  console.log(JSON.stringify(manifest, null, 2));
  if (!manifest.pass) process.exitCode = 1;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) await main();
