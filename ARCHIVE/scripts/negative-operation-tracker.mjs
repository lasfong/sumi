/**
 * Deterministic Negative Operation Tracker & Classifier for Product UAT.
 * Manages operation-scoped windows, captures API network responses and console events,
 * and produces fail-closed semantic evidence without relying on raw console event counts.
 */
export class NegativeOperationTracker {
  constructor() {
    this.activeOperation = null;
    this.completedOperations = new Map();
  }

  startOperation(name, options = {}) {
    if (this.activeOperation) {
      throw new Error(`Cannot start negative operation '${name}': operation '${this.activeOperation.name}' is already active.`);
    }
    this.activeOperation = {
      name,
      expectedEndpoint: options.expectedEndpoint || null, // e.g. '/api/replay/sessions/999999'
      expectedStatus: options.expectedStatus ?? 404,
      pattern: options.pattern || null,
      forbiddenEndpoints: options.forbiddenEndpoints || [], // e.g. ['/api/replay/sessions/5']
      allowNoResponses: options.allowNoResponses ?? false,
      capturedResponses: [],
      forbiddenResponses: [],
      capturedConsoleErrors: [],
      startTime: Date.now(),
    };
  }

  endOperation(name) {
    if (!this.activeOperation || this.activeOperation.name !== name) {
      const activeName = this.activeOperation ? this.activeOperation.name : 'none';
      throw new Error(`Cannot end operation '${name}': active operation is '${activeName}'.`);
    }
    const op = this.activeOperation;

    const hasForbiddenResponse = op.forbiddenResponses.length > 0;

    const hasExpectedResponse = op.expectedEndpoint
      ? op.capturedResponses.some(res => res.url.includes(op.expectedEndpoint) && res.status === op.expectedStatus)
      : op.allowNoResponses ? !hasForbiddenResponse : op.capturedResponses.length > 0;

    const allResponsesMatch = op.expectedEndpoint
      ? op.capturedResponses.length > 0 && op.capturedResponses.every(res => res.url.includes(op.expectedEndpoint) && res.status === op.expectedStatus)
      : !hasForbiddenResponse;

    const pass = hasExpectedResponse && allResponsesMatch && !hasForbiddenResponse;

    const snapshot = {
      name: op.name,
      expectedEndpoint: op.expectedEndpoint,
      expectedStatus: op.expectedStatus,
      forbiddenEndpoints: op.forbiddenEndpoints.map(f => typeof f === 'string' ? f : String(f)),
      allowNoResponses: op.allowNoResponses,
      capturedResponseCount: op.capturedResponses.length,
      capturedResponses: op.capturedResponses,
      forbiddenResponseCount: op.forbiddenResponses.length,
      forbiddenResponses: op.forbiddenResponses,
      capturedConsoleErrorCount: op.capturedConsoleErrors.length,
      capturedConsoleErrors: op.capturedConsoleErrors,
      hasExpectedResponse,
      allResponsesMatch,
      hasForbiddenResponse,
      pass,
    };

    this.completedOperations.set(name, snapshot);
    this.activeOperation = null;
    return snapshot;
  }

  classifyConsoleMessage(text, url = '') {
    const detail = `console: ${text} [${url}]`;
    const matchesPattern = (this.activeOperation?.pattern && this.activeOperation.pattern.test(text + url))
      || text.includes('999999') || url.includes('999999');

    if (!matchesPattern) {
      return { action: 'unmatched', detail };
    }

    if (this.activeOperation && this.activeOperation.pattern && this.activeOperation.pattern.test(text + url)) {
      this.activeOperation.capturedConsoleErrors.push(detail);
      return { action: 'captured', detail, operationName: this.activeOperation.name };
    }

    // Event matches negative pattern (e.g. 999999) but occurred outside active matching operation window
    return {
      action: 'runtime_error',
      detail,
      reason: 'Console event matches negative pattern but occurred outside active matching operation window',
    };
  }

  classifyResponse(url, status) {
    if (!this.activeOperation) return null;
    const op = this.activeOperation;

    const isForbidden = (op.forbiddenEndpoints || []).some(forbidden => {
      if (typeof forbidden === 'string') return url.includes(forbidden);
      if (forbidden instanceof RegExp) return forbidden.test(url);
      return false;
    });

    if (isForbidden) {
      const record = { url, status, timestamp: Date.now(), forbidden: true };
      op.forbiddenResponses.push(record);
      return record;
    }

    const matchesEndpoint = op.expectedEndpoint
      ? url.includes(op.expectedEndpoint)
      : op.pattern ? op.pattern.test(url) : false;

    if (matchesEndpoint) {
      const record = { url, status, timestamp: Date.now(), forbidden: false };
      op.capturedResponses.push(record);
      return record;
    }
    return null;
  }

  getSnapshot(name) {
    return this.completedOperations.get(name) || null;
  }
}
