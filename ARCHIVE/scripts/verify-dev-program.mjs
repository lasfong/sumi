// Read-only structural checkpoint validation. Not a substitute for product/reviewer gates.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const allowedTask = new Set(['pending', 'running', 'review', 'accepted', 'blocked']);
const allowedProgram = new Set(['ready_for_bootstrap', 'running', 'paused', 'needs_review_capability', 'partial_blocked', 'complete']);

export function validate(state, exists) {
  assert.equal(state.schemaVersion, 1, 'Unsupported schema');
  assert(allowedProgram.has(state.status), 'Unknown program status');
  assert(Array.isArray(state.tasks) && state.tasks.length > 0, 'Missing tasks');
  assert(Array.isArray(state.ownerQuestions), 'Missing ownerQuestions');
  const ids = new Map();
  for (const task of state.tasks) {
    assert(typeof task.id === 'string' && !ids.has(task.id), 'Missing/duplicate task ID');
    ids.set(task.id, task);
    assert(allowedTask.has(task.status), `Unknown status: ${task.id}`);
    assert(Array.isArray(task.dependsOn), `Missing dependencies: ${task.id}`);
    assert(Array.isArray(task.evidence), `Missing evidence list: ${task.id}`);
    assert(new Set(task.dependsOn).size === task.dependsOn.length, `Duplicate dependency: ${task.id}`);
    for (const evidence of task.evidence) {
      assert(typeof evidence === 'string' && evidence.length && !path.isAbsolute(evidence) && !evidence.split(/[\\/]/).includes('..'), 'Evidence must be a project-relative path');
      assert(exists(evidence), `Missing evidence: ${evidence}`);
    }
    if (task.status === 'accepted') assert(task.evidence.length, `Accepted without evidence: ${task.id}`);
    if (task.status === 'blocked') assert(typeof task.blocker === 'string' && task.blocker.trim(), `Missing blocker: ${task.id}`);
  }
  assert(ids.has('BOOTSTRAP') && ids.has('P10-SEAL-02'), 'Missing bootstrap/final seal');
  const visited = new Set(), active = new Set();
  function visit(id) {
    assert(ids.has(id), `Unknown dependency: ${id}`);
    assert(!active.has(id), `Dependency cycle: ${id}`);
    if (visited.has(id)) return;
    active.add(id);
    for (const dependency of ids.get(id).dependsOn) visit(dependency);
    active.delete(id); visited.add(id);
  }
  for (const task of state.tasks) {
    visit(task.id);
    if (['accepted', 'running', 'review'].includes(task.status)) {
      assert(task.dependsOn.every(id => ids.get(id).status === 'accepted'), `Unmet dependency: ${task.id}`);
    }
  }
  assert(state.tasks.filter(t => ['running', 'review'].includes(t.status)).length <= 1, 'Multiple active production batches');
  if (ids.get('BOOTSTRAP').status === 'accepted') {
    assert(typeof state.workingRoot === 'string' && state.workingRoot, 'Missing working root');
    assert(typeof state.reviewerMechanism === 'string' && state.reviewerMechanism, 'Missing independent reviewer mechanism');
    assert(typeof state.bootstrapEvidence === 'string' && exists(state.bootstrapEvidence), 'Missing bootstrap evidence');
  }
  if (state.status === 'running' || state.status === 'complete') assert.equal(ids.get('BOOTSTRAP').status, 'accepted', 'Bootstrap not accepted');
  if (state.status === 'complete') {
    assert(state.tasks.every(t => t.status === 'accepted'), 'Incomplete task in complete program');
    assert.equal(state.ownerQuestions.length, 0, 'Unresolved owner questions');
  }
  return state.tasks.filter(t => t.status === 'pending' && t.dependsOn.every(id => ids.get(id).status === 'accepted')).map(t => t.id);
}

function selfTest() {
  const make = () => ({schemaVersion:1,status:'ready_for_bootstrap',ownerQuestions:[],tasks:[
    {id:'BOOTSTRAP',status:'pending',dependsOn:[],evidence:[]},
    {id:'P10-SEAL-02',status:'pending',dependsOn:['BOOTSTRAP'],evidence:[]}
  ]});
  assert.deepEqual(validate(make(), () => true), ['BOOTSTRAP']);
  const cases = [
    s => { s.tasks.push({...s.tasks[0]}); },
    s => { s.tasks[0].dependsOn = ['P10-SEAL-02']; },
    s => { s.tasks[0].dependsOn = ['unknown']; },
    s => { s.tasks[1].status = 'accepted'; },
    s => { s.tasks[1].status = 'running'; },
    s => { s.tasks[0].status = 'blocked'; },
    s => { s.tasks[0].evidence = ['../secret']; },
    s => { s.status = 'complete'; },
    s => { s.status = 'running'; }
  ];
  for (const mutate of cases) { const state = make(); mutate(state); assert.throws(() => validate(state, () => true)); }
  const missing = make(); missing.tasks[0].evidence = ['missing.md'];
  assert.throws(() => validate(missing, () => false));
  const completed = make();
  completed.status = 'complete'; completed.workingRoot = 'local-test-root';
  completed.reviewerMechanism = 'independent-test-context'; completed.bootstrapEvidence = 'bootstrap.md';
  for (const task of completed.tasks) { task.status = 'accepted'; task.evidence = ['review.md']; }
  assert.deepEqual(validate(completed, () => true), []);
  completed.ownerQuestions = ['unresolved'];
  assert.throws(() => validate(completed, () => true));
  console.log(`Self-test PASS: ${cases.length + 4} cases`);
}

try {
  if (process.argv.includes('--self-test')) selfTest();
  else {
    for (const required of ['AGENTS.md','PLANS.md','docs/dev-program/START.md','docs/dev-program/ROADMAP.md','docs/research/SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md']) {
      assert(fs.existsSync(path.join(root, required)), `Missing source: ${required}`);
    }
    const state = JSON.parse(fs.readFileSync(path.join(root, 'docs/dev-program/STATE.json'), 'utf8'));
    const roadmap = fs.readFileSync(path.join(root, 'docs/research/SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md'), 'utf8');
    const expected = [...roadmap.matchAll(/^#### `(P(?:[1-9]|10)-[A-Z]+-\d+)`/gm)].map(match => match[1]);
    assert(expected.length > 0, 'Could not identify canonical roadmap task cards');
    for (const id of expected) assert(state.tasks.some(t => t.id === id), `Roadmap task omitted: ${id}`);
    const ready = validate(state, p => fs.existsSync(path.join(root, p)));
    console.log(JSON.stringify({validation:'PASS',status:state.status,dependencyReady:ready,note:'Dependency readiness is not semantic/data authorization. No product tests executed.'}, null, 2));
  }
} catch (error) {
  console.error(`DEV program validation FAILED: ${error.message}`);
  process.exitCode = 1;
}
