import assert from 'node:assert/strict';
import test from 'node:test';
import { NegativeOperationTracker } from './negative-operation-tracker.mjs';

test('NegativeOperationTracker: event outside active operation window returns runtime_error', () => {
  const tracker = new NegativeOperationTracker();
  const res = tracker.classifyConsoleMessage(
    'Failed to load resource: 404',
    'http://127.0.0.1:15173/api/replay/sessions/999999'
  );
  assert.equal(res.action, 'runtime_error');
  assert.match(res.reason, /outside active matching operation window/);
});

test('NegativeOperationTracker: wrong operation name throws on endOperation', () => {
  const tracker = new NegativeOperationTracker();
  tracker.startOperation('invalid-journal-session-999999', {
    expectedEndpoint: '/api/replay/sessions/999999',
    expectedStatus: 404,
  });
  assert.throws(
    () => tracker.endOperation('wrong-operation-name'),
    /active operation is 'invalid-journal-session-999999'/
  );
});

test('NegativeOperationTracker: wrong status (200 instead of 404) fails pass condition', () => {
  const tracker = new NegativeOperationTracker();
  tracker.startOperation('invalid-journal-session-999999', {
    expectedEndpoint: '/api/replay/sessions/999999',
    expectedStatus: 404,
  });

  tracker.classifyResponse('http://127.0.0.1:18000/api/replay/sessions/999999', 200);
  const snap = tracker.endOperation('invalid-journal-session-999999');

  assert.equal(snap.hasExpectedResponse, false);
  assert.equal(snap.allResponsesMatch, false);
  assert.equal(snap.pass, false);
});

test('NegativeOperationTracker: missing expected response fails pass condition', () => {
  const tracker = new NegativeOperationTracker();
  tracker.startOperation('invalid-journal-session-999999', {
    expectedEndpoint: '/api/replay/sessions/999999',
    expectedStatus: 404,
  });

  const snap = tracker.endOperation('invalid-journal-session-999999');
  assert.equal(snap.capturedResponseCount, 0);
  assert.equal(snap.hasExpectedResponse, false);
  assert.equal(snap.pass, false);
});

test('NegativeOperationTracker: valid negative operation captures response and console error and passes', () => {
  const tracker = new NegativeOperationTracker();
  tracker.startOperation('invalid-journal-session-999999', {
    expectedEndpoint: '/api/replay/sessions/999999',
    expectedStatus: 404,
    pattern: /999999/,
  });

  const consoleRes = tracker.classifyConsoleMessage(
    'Failed to load resource: status 404',
    'http://127.0.0.1:15173/api/replay/sessions/999999'
  );
  assert.equal(consoleRes.action, 'captured');

  const apiRes = tracker.classifyResponse('http://127.0.0.1:18000/api/replay/sessions/999999', 404);
  assert.notEqual(apiRes, null);

  const snap = tracker.endOperation('invalid-journal-session-999999');
  assert.equal(snap.name, 'invalid-journal-session-999999');
  assert.equal(snap.hasExpectedResponse, true);
  assert.equal(snap.allResponsesMatch, true);
  assert.equal(snap.pass, true);
  assert.equal(snap.capturedConsoleErrorCount, 1);
});

test('NegativeOperationTracker: forbidden endpoint request (validating/fetching persisted session ID 5) fails pass condition', () => {
  const tracker = new NegativeOperationTracker();
  tracker.startOperation('malformed-session-syntax-abc', {
    forbiddenEndpoints: ['/api/replay/sessions/5'],
    allowNoResponses: true,
  });

  tracker.classifyResponse('http://127.0.0.1:18000/api/replay/sessions/5', 200);
  const snap = tracker.endOperation('malformed-session-syntax-abc');

  assert.equal(snap.forbiddenResponseCount, 1);
  assert.equal(snap.hasForbiddenResponse, true);
  assert.equal(snap.pass, false);
});

test('NegativeOperationTracker: malformed syntax with allowNoResponses and zero forbidden requests passes cleanly', () => {
  const tracker = new NegativeOperationTracker();
  tracker.startOperation('malformed-session-syntax-abc', {
    forbiddenEndpoints: ['/api/replay/sessions/5', '/api/replay/sessions/abc'],
    allowNoResponses: true,
  });

  const snap = tracker.endOperation('malformed-session-syntax-abc');

  assert.equal(snap.forbiddenResponseCount, 0);
  assert.equal(snap.hasForbiddenResponse, false);
  assert.equal(snap.hasExpectedResponse, true);
  assert.equal(snap.allResponsesMatch, true);
  assert.equal(snap.pass, true);
});
