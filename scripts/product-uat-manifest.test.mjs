import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

import {
  loadProductUatManifest,
  reconcileProductUatAssertions,
  validateProductUatManifest,
  writeProductUatResult,
} from './product-uat-manifest.mjs';


const manifestPath = new URL('./fixtures/product-uat-v3-baseline.json', import.meta.url);
const loadFixture = async () => JSON.parse(await readFile(manifestPath, 'utf8'));
const rejectsInvalid = async mutate => {
  const manifest = await loadFixture();
  mutate(manifest);
  assert.throws(() => validateProductUatManifest(manifest), /Invalid product UAT manifest/);
};

test('missing and malformed manifests fail closed', async () => {
  const temp = await mkdtemp(path.join(os.tmpdir(), 'sumi-pro00-manifest-'));
  try {
    await assert.rejects(loadProductUatManifest(path.join(temp, 'missing.json')), /manifest is required/);
    const malformed = path.join(temp, 'malformed.json');
    await writeFile(malformed, '{"schema_version":');
    await assert.rejects(loadProductUatManifest(malformed), /malformed JSON/);
  } finally {
    await rm(temp, { recursive: true, force: true });
  }
});

test('empty, duplicate, removed, and renamed accepted IDs fail closed', async () => {
  await rejectsInvalid(manifest => { manifest.assertions = []; });
  await rejectsInvalid(manifest => { manifest.assertions.push({ ...manifest.assertions[0] }); });
  await rejectsInvalid(manifest => { manifest.assertions.splice(10, 1); });
  await rejectsInvalid(manifest => { manifest.assertions[10].id += '-renamed'; });
});

test('invalid fields and blocking downgrade fail closed', async () => {
  await rejectsInvalid(manifest => { delete manifest.assertions[0].blocking; });
  await rejectsInvalid(manifest => { manifest.assertions[0].acceptance_ids = []; });
  const manifest = await loadFixture();
  const actual = manifest.assertions.map(item => ({ id: item.id, pass: true, blocking: item.blocking }));
  actual[0].blocking = false;
  const reconciliation = reconcileProductUatAssertions(manifest, actual);
  assert.equal(reconciliation.pass, false);
  assert.deepEqual(reconciliation.blocking_mismatch_ids, [actual[0].id]);
});

test('additive later-batch assertion is valid and actual removal/duplication fails', async () => {
  const manifest = await loadFixture();
  const addition = {
    id: 'pro01.fixture-additive',
    blocking: true,
    acceptance_ids: ['R-01', 'PRO-INT-10'],
  };
  manifest.assertions.push(addition);
  validateProductUatManifest(manifest);
  const actual = manifest.assertions.map(item => ({ id: item.id, pass: true, blocking: item.blocking }));
  assert.equal(reconcileProductUatAssertions(manifest, actual).pass, true);
  assert.equal(reconcileProductUatAssertions(manifest, actual.slice(1)).pass, false);
  assert.equal(reconcileProductUatAssertions(manifest, [...actual, actual[0]]).pass, false);
});

test('later professional batches remain additive to the sealed V3 baseline', async () => {
  const manifest = await loadFixture();
  manifest.assertions.push({
    id: 'pro01.fixture-additive',
    blocking: true,
    acceptance_ids: ['PRO-BT-03'],
  });
  validateProductUatManifest(manifest);
});

test('removing or renaming a sealed PRO-00 assertion fails closed', async () => {
  await rejectsInvalid(manifest => {
    const index = manifest.assertions.findIndex(item => item.id.startsWith('pro00.'));
    manifest.assertions.splice(index, 1);
  });
  await rejectsInvalid(manifest => {
    const item = manifest.assertions.find(assertion => assertion.id.startsWith('pro00.'));
    item.id = `${item.id}-renamed`;
  });
});

test('failed UAT results are retained as machine-readable evidence', async () => {
  const temp = await mkdtemp(path.join(os.tmpdir(), 'sumi-pro00-result-'));
  const resultPath = path.join(temp, 'failed-results.json');
  try {
    await writeProductUatResult(resultPath, {
      passed: 0,
      failed: 1,
      blockingFailed: 1,
      reconciliation: { pass: false, missing_ids: ['pro00.fixture'] },
    });
    const retained = JSON.parse(await readFile(resultPath, 'utf8'));
    assert.equal(retained.blockingFailed, 1);
    assert.deepEqual(retained.reconciliation.missing_ids, ['pro00.fixture']);
  } finally {
    await rm(temp, { recursive: true, force: true });
  }
});
