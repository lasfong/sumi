import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';


const REQUIRED_MANIFEST_KEYS = [
  'schema_version',
  'baseline_name',
  'acceptance_contract_revision',
  'accepted_baseline_count',
  'accepted_baseline_ids_sha256',
  'assertions',
];

const sha256 = value => createHash('sha256').update(value).digest('hex');
const idsHash = ids => sha256([...ids].sort().join('\n'));
const SEALED_PRO00_COUNT = 10;
const SEALED_PRO00_IDS_SHA256 = '63ecd633a6b84d60282871745ae5b552e1211dcaff9d22d7d217dd3521bb6408';

export function validateProductUatManifest(manifest) {
  const errors = [];
  if (!manifest || typeof manifest !== 'object' || Array.isArray(manifest)) {
    throw new Error('Product UAT manifest must be a JSON object');
  }
  for (const key of REQUIRED_MANIFEST_KEYS) {
    if (!(key in manifest)) errors.push(`missing manifest field: ${key}`);
  }
  if (manifest.schema_version !== 1) errors.push('schema_version must be 1');
  if (manifest.baseline_name !== 'sumi-v3-accepted-regression-baseline') errors.push('baseline_name is not recognized');
  if (typeof manifest.acceptance_contract_revision !== 'string' || !manifest.acceptance_contract_revision) {
    errors.push('acceptance_contract_revision must be a non-empty string');
  }
  if (!Number.isInteger(manifest.accepted_baseline_count) || manifest.accepted_baseline_count < 1) {
    errors.push('accepted_baseline_count must be a positive integer');
  }
  if (!/^[0-9a-f]{64}$/.test(manifest.accepted_baseline_ids_sha256 || '')) {
    errors.push('accepted_baseline_ids_sha256 must be a lowercase SHA-256');
  }
  if (!Array.isArray(manifest.assertions) || manifest.assertions.length === 0) {
    errors.push('assertions must be a non-empty array');
  }

  const assertions = Array.isArray(manifest.assertions) ? manifest.assertions : [];
  const ids = [];
  assertions.forEach((assertion, index) => {
    if (!assertion || typeof assertion !== 'object' || Array.isArray(assertion)) {
      errors.push(`assertions[${index}] must be an object`);
      return;
    }
    if (typeof assertion.id !== 'string' || !/^[a-zA-Z0-9][a-zA-Z0-9._:-]+$/.test(assertion.id)) {
      errors.push(`assertions[${index}].id is invalid`);
    } else {
      ids.push(assertion.id);
    }
    if (typeof assertion.blocking !== 'boolean') errors.push(`assertions[${index}].blocking must be boolean`);
    if (!Array.isArray(assertion.acceptance_ids) || assertion.acceptance_ids.length === 0
      || assertion.acceptance_ids.some(id => typeof id !== 'string' || !id)) {
      errors.push(`assertions[${index}].acceptance_ids must contain at least one ID`);
    }
  });
  const duplicateIds = [...new Set(ids.filter((id, index) => ids.indexOf(id) !== index))].sort();
  if (duplicateIds.length) errors.push(`duplicate assertion IDs: ${duplicateIds.join(', ')}`);

  const acceptedBaselineIds = ids.filter(id => !/^pro\d{2}\./.test(id));
  if (acceptedBaselineIds.length !== manifest.accepted_baseline_count) {
    errors.push(`accepted baseline count mismatch: expected ${manifest.accepted_baseline_count}, found ${acceptedBaselineIds.length}`);
  }
  const actualBaselineHash = idsHash(acceptedBaselineIds);
  if (actualBaselineHash !== manifest.accepted_baseline_ids_sha256) {
    errors.push(`accepted baseline ID hash mismatch: expected ${manifest.accepted_baseline_ids_sha256}, found ${actualBaselineHash}`);
  }
  const pro00Ids = ids.filter(id => id.startsWith('pro00.'));
  if (pro00Ids.length !== SEALED_PRO00_COUNT || idsHash(pro00Ids) !== SEALED_PRO00_IDS_SHA256) {
    errors.push('sealed PRO-00 assertion set was removed, renamed, or changed');
  }

  if (errors.length) throw new Error(`Invalid product UAT manifest:\n- ${errors.join('\n- ')}`);
  return manifest;
}

export async function loadProductUatManifest(manifestPath) {
  let bytes;
  try {
    bytes = await readFile(manifestPath);
  } catch (error) {
    throw new Error(`Product UAT manifest is required: ${String(error?.message || error)}`);
  }
  let parsed;
  try {
    parsed = JSON.parse(bytes.toString('utf8'));
  } catch (error) {
    throw new Error(`Product UAT manifest is malformed JSON: ${String(error?.message || error)}`);
  }
  const manifest = validateProductUatManifest(parsed);
  return {
    manifest,
    metadata: {
      path: path.resolve(manifestPath),
      schema_version: manifest.schema_version,
      baseline_name: manifest.baseline_name,
      acceptance_contract_revision: manifest.acceptance_contract_revision,
      sha256: sha256(bytes),
      baseline_assertion_count: manifest.accepted_baseline_count,
      declared_assertion_count: manifest.assertions.length,
    },
  };
}

export function reconcileProductUatAssertions(manifest, actualChecks) {
  validateProductUatManifest(manifest);
  const expectedById = new Map(manifest.assertions.map(item => [item.id, item]));
  const actualIds = actualChecks.map(item => item.id);
  const actualById = new Map(actualChecks.map(item => [item.id, item]));
  const duplicate = [...new Set(actualIds.filter((id, index) => actualIds.indexOf(id) !== index))].sort();
  const missing = manifest.assertions.map(item => item.id).filter(id => !actualById.has(id)).sort();
  const unexpected = [...new Set(actualIds.filter(id => !expectedById.has(id)))].sort();
  const blockingMismatch = actualChecks
    .filter(item => expectedById.has(item.id) && item.blocking !== expectedById.get(item.id).blocking)
    .map(item => item.id)
    .sort();
  const failedBlocking = actualChecks
    .filter(item => item.pass !== true && expectedById.get(item.id)?.blocking === true)
    .map(item => item.id)
    .sort();
  const assertionToAcceptance = Object.fromEntries(
    manifest.assertions.map(item => [item.id, item.acceptance_ids]),
  );
  return {
    pass: missing.length === 0
      && unexpected.length === 0
      && duplicate.length === 0
      && blockingMismatch.length === 0
      && failedBlocking.length === 0,
    baseline_count: manifest.accepted_baseline_count,
    expected_count: manifest.assertions.length,
    actual_count: actualChecks.length,
    missing_ids: missing,
    unexpected_ids: unexpected,
    duplicate_ids: duplicate,
    blocking_mismatch_ids: blockingMismatch,
    failed_blocking_ids: failedBlocking,
    assertion_to_acceptance: assertionToAcceptance,
  };
}

export async function writeProductUatResult(resultPath, result) {
  await mkdir(path.dirname(resultPath), { recursive: true });
  await writeFile(resultPath, JSON.stringify(result, null, 2));
}
