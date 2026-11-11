import { pathToFileURL } from 'node:url';
import { readFile, writeFile } from 'node:fs/promises';

const BLOCKING_PREFIXES = ['batch1.', 'batch2.', 'batch3.', 'batch4.', 'batch5.', 'drawings.'];
const isBlocking = id => BLOCKING_PREFIXES.some(prefix => id.startsWith(prefix)) || id === 'runtime.no-errors';

export function auditEvidence(baseline, current) {
  const baselineIds = baseline.checks?.map(item => item.id) ?? [];
  const currentChecks = current.checks ?? [];
  const currentById = Map.groupBy(currentChecks, item => item.id);
  const missing = baselineIds.filter(id => !currentById.has(id));
  const duplicateBaseline = baselineIds.filter(id => (currentById.get(id) || []).length !== 1);
  const changedPass = (baseline.checks ?? []).filter(item => currentById.get(item.id)?.[0]?.pass !== item.pass).map(item => item.id);
  const duplicateCurrent = [...currentById].filter(([, items]) => items.length !== 1).map(([id]) => id);
  const additive = currentChecks.filter(item => !baselineIds.includes(item.id));
  const invalidAdditive = additive.filter(item => !item.id.startsWith('batch5.closure.') || !isBlocking(item.id)).map(item => item.id);
  const failedAdditive = additive.filter(item => item.pass !== true).map(item => item.id);
  const baselineNotPassing = (baseline.checks ?? []).filter(item => item.pass !== true).map(item => item.id);
  const resultCountsMatch = current.passed === currentChecks.filter(item => item.pass === true).length
    && current.failed === currentChecks.filter(item => item.pass !== true).length
    && current.blockingFailed === currentChecks.filter(item => item.pass !== true && isBlocking(item.id)).length;
  const errors = {
    runtime: current.runtimeErrors ?? null,
    provider: current.providerErrors ?? null,
    indicator: current.indicatorRequestFailures ?? null,
    expectedPractice: current.expectedPracticeConsoleErrors ?? null,
  };
  const expectedRejectionsValid = Array.isArray(errors.expectedPractice) && errors.expectedPractice.length === 2
    && errors.expectedPractice.some(item => item.includes('400')) && errors.expectedPractice.some(item => item.includes('409'));
  const errorPolicyPass = Array.isArray(errors.runtime) && errors.runtime.length === 0
    && Array.isArray(errors.provider) && errors.provider.length === 0
    && Array.isArray(errors.indicator) && errors.indicator.length === 0 && expectedRejectionsValid;
  const pass = baselineIds.length === 272 && new Set(baselineIds).size === 272 && baselineNotPassing.length === 0
    && missing.length === 0 && duplicateBaseline.length === 0 && changedPass.length === 0 && duplicateCurrent.length === 0
    && invalidAdditive.length === 0 && failedAdditive.length === 0 && additive.length > 0
    && current.blockingFailed === 0 && current.failed === 0 && resultCountsMatch && errorPolicyPass;
  return {
    pass, baselineCount: baselineIds.length, currentCount: currentChecks.length, additiveCount: additive.length,
    missing, duplicateBaseline, changedPass, duplicateCurrent, invalidAdditive, failedAdditive, baselineNotPassing,
    resultCountsMatch, errorPolicyPass, errors,
    additive: additive.map(item => ({ id: item.id, pass: item.pass, blocking: isBlocking(item.id) })),
  };
}

async function main() {
  const [baselinePath, currentPath, outputPath] = process.argv.slice(2);
  if (!baselinePath || !currentPath || !outputPath) throw new Error('usage: node batch5-evidence-audit.mjs BASELINE CURRENT OUTPUT');
  const result = auditEvidence(JSON.parse(await readFile(baselinePath, 'utf8')), JSON.parse(await readFile(currentPath, 'utf8')));
  await writeFile(outputPath, JSON.stringify(result, null, 2));
  console.log(JSON.stringify(result, null, 2));
  if (!result.pass) process.exitCode = 1;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) await main();
