import {
  createDrawing, emptyDrawingDocument, normalizeDrawingOrder, parseDrawingDocument, validateDrawingDocument,
  type SumiDrawing, type SumiDrawingAnchor, type SumiDrawingDocumentV1,
} from './drawingDomain';

export class DrawingRevisionConflict extends Error {}
export class DrawingPersistenceConflict extends Error {}
export interface DrawingStorage { getItem(key: string): string | null; setItem(key: string, value: string): void; removeItem(key: string): void }
export interface DrawingHydration { document: SumiDrawingDocumentV1; remoteRaw: string; migrated: boolean; conflict: string | null }

const isRecord = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value);
const isAnchor = (value: unknown): value is SumiDrawingAnchor => isRecord(value) && typeof value.time === 'string'
  && /^\d{4}-\d{2}-\d{2}$/.test(value.time) && typeof value.price === 'number' && Number.isFinite(value.price) && value.price > 0;

export class DrawingRepository {
  private readonly storage: DrawingStorage;
  constructor(storage: DrawingStorage) { this.storage = storage; }
  private legacyKey(sessionId: number): string { return `sumi:drawing-document:v1:${sessionId}`; }
  identityKey(sessionId: number, symbol: string): string { return `${this.legacyKey(sessionId)}:${encodeURIComponent(symbol)}`; }
  backupKey(sessionId: number, symbol: string): string { return `sumi:drawing-legacy-backup:v1:${sessionId}:${encodeURIComponent(symbol)}`; }
  quarantineKey(sessionId: number, symbol: string): string { return `sumi:drawing-quarantine:v1:${sessionId}:${encodeURIComponent(symbol)}`; }
  indeterminateKey(sessionId: number, symbol: string): string { return `sumi:drawing-indeterminate:v1:${sessionId}:${encodeURIComponent(symbol)}`; }
  raw(sessionId: number, symbol: string): string | null { return this.storage.getItem(this.identityKey(sessionId, symbol)); }
  restore(sessionId: number, symbol: string, raw: string | null): void { const key = this.identityKey(sessionId, symbol); if (raw === null) this.storage.removeItem(key); else this.storage.setItem(key, raw); }
  private preserveLegacy(document: SumiDrawingDocumentV1): void {
    const key = this.identityKey(document.sessionId, document.symbol); const serialized = JSON.stringify(document); const existing = this.storage.getItem(key);
    if (!existing) { this.storage.setItem(key, serialized); return; } if (existing === serialized) return;
    const base = `${this.legacyKey(document.sessionId)}:collision-backup:${encodeURIComponent(document.symbol)}:${document.revision}`;
    let collision = base; let suffix = 1; while (this.storage.getItem(collision) !== null) { collision = `${base}:${suffix}`; suffix += 1; }
    this.storage.setItem(collision, serialized);
  }
  load(sessionId: number, symbol?: string): SumiDrawingDocumentV1 | null {
    if (symbol) { const identity = parseDrawingDocument(this.storage.getItem(this.identityKey(sessionId, symbol))); if (identity?.sessionId === sessionId && identity.symbol === symbol) return identity; }
    const legacy = parseDrawingDocument(this.storage.getItem(this.legacyKey(sessionId))); if (!legacy || legacy.sessionId !== sessionId || (symbol && legacy.symbol !== symbol)) return null;
    if (symbol) this.storage.setItem(this.identityKey(sessionId, symbol), JSON.stringify(legacy)); return legacy;
  }
  save(document: SumiDrawingDocumentV1, expectedRevision: number): SumiDrawingDocumentV1 {
    if (!validateDrawingDocument(document)) throw new TypeError('Invalid Sumi drawing document');
    const key = this.identityKey(document.sessionId, document.symbol); let current = parseDrawingDocument(this.storage.getItem(key));
    const oldKey = this.legacyKey(document.sessionId); const legacy = parseDrawingDocument(this.storage.getItem(oldKey));
    if (!current && legacy?.sessionId === document.sessionId) { if (legacy.symbol === document.symbol) current = legacy; else { this.preserveLegacy(legacy); this.storage.removeItem(oldKey); } }
    const actual = current?.revision ?? 0; if (actual !== expectedRevision) throw new DrawingRevisionConflict(`Expected revision ${expectedRevision}, found ${actual}`);
    const saved = { ...structuredClone(document), revision: expectedRevision + 1 }; this.storage.setItem(key, JSON.stringify(saved));
    if (legacy?.symbol === document.symbol) this.storage.removeItem(oldKey); return saved;
  }
  put(document: SumiDrawingDocumentV1): void { if (!validateDrawingDocument(document)) throw new TypeError('Invalid Sumi drawing document'); this.storage.setItem(this.identityKey(document.sessionId, document.symbol), JSON.stringify(document)); }
  remove(sessionId: number, symbol?: string): void { this.storage.removeItem(symbol ? this.identityKey(sessionId, symbol) : this.legacyKey(sessionId)); }
  preserveIndeterminate(sessionId: number, symbol: string, evidence: { priorRaw: string; intendedRaw: string; observedRaw: string | null; reason: string }): void {
    this.storage.setItem(this.indeterminateKey(sessionId, symbol), JSON.stringify(evidence));
  }
  hydrate(sessionId: number, symbol: string, remoteRaw: string | null | undefined): DrawingHydration {
    const local = this.load(sessionId, symbol); const remote = parseDrawingDocument(remoteRaw); const emptyRaw = remoteRaw ?? '[]';
    if (remote && (remote.sessionId !== sessionId || remote.symbol !== symbol)) return { document: local ?? emptyDrawingDocument(sessionId, symbol), remoteRaw: emptyRaw, migrated: false, conflict: 'Backend drawing identity does not match this replay session.' };
    if (remote && local) {
      if (JSON.stringify(remote) === JSON.stringify(local)) return { document: local, remoteRaw: emptyRaw, migrated: false, conflict: null };
      return { document: local, remoteRaw: emptyRaw, migrated: false, conflict: 'Local and backend canonical drawings diverged; neither copy was overwritten.' };
    }
    if (remote) { this.put(remote); return { document: remote, remoteRaw: emptyRaw, migrated: false, conflict: null }; }
    if (local && remoteRaw && remoteRaw !== '[]') {
      this.preserveRawBackup(sessionId, symbol, remoteRaw);
      return { document: local, remoteRaw: emptyRaw, migrated: false, conflict: 'Local canonical drawings and backend legacy drawings both contain state; neither copy was overwritten.' };
    }
    const migrated = this.migrateLegacy(sessionId, symbol, remoteRaw);
    if (migrated) return { document: migrated, remoteRaw: emptyRaw, migrated: true, conflict: null };
    if (remoteRaw && remoteRaw !== '[]' && this.storage.getItem(this.quarantineKey(sessionId, symbol)) !== null) {
      return { document: local ?? emptyDrawingDocument(sessionId, symbol), remoteRaw: emptyRaw, migrated: false,
        conflict: 'Backend drawings are malformed or ambiguous and were quarantined; reconciliation is required before editing.' };
    }
    return { document: local ?? emptyDrawingDocument(sessionId, symbol), remoteRaw: emptyRaw, migrated: false, conflict: null };
  }
  private preserveRawBackup(sessionId: number, symbol: string, raw: string): void {
    const key = this.backupKey(sessionId, symbol); if (this.storage.getItem(key) === null) this.storage.setItem(key, raw);
  }
  private migrateLegacy(sessionId: number, symbol: string, raw: string | null | undefined): SumiDrawingDocumentV1 | null {
    if (!raw || raw === '[]') return null; this.preserveRawBackup(sessionId, symbol, raw);
    let parsed: unknown; try { parsed = JSON.parse(raw); } catch { this.quarantine(sessionId, symbol, raw, 'Invalid JSON'); return null; }
    if (!Array.isArray(parsed)) { this.quarantine(sessionId, symbol, raw, 'Legacy payload is not an array'); return null; }
    if (parsed.length === 0) return null;
    const drawings: SumiDrawing[] = []; const invalid: unknown[] = []; const ids = new Set<string>();
    for (const item of parsed) {
      if (!isRecord(item) || typeof item.type !== 'string') { invalid.push(item); continue; }
      if (item.type === 'cursor') continue;
      const points = Array.isArray(item.points) ? item.points : []; const anchors = points.filter(isAnchor);
      const required = item.type === 'horizontal' ? 1 : 2;
      if (!['horizontal', 'trendline', 'fibonacci'].includes(item.type) || anchors.length !== required || typeof item.id !== 'string'
        || !/^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(item.id) || ids.has(item.id)) { invalid.push(item); continue; }
      const tool = item.type === 'fibonacci' ? 'fibonacci-retracement' : item.type as 'horizontal' | 'trendline';
      const drawing = createDrawing(tool, anchors, drawings.length); drawing.id = item.id; if (typeof item.color === 'string' && item.color) drawing.style.lineColor = item.color;
      drawings.push(drawing); ids.add(item.id);
    }
    if (invalid.length) { this.quarantine(sessionId, symbol, raw, 'Malformed or ambiguous legacy records'); return null; }
    if (!drawings.length) return null;
    return { schemaVersion: 1, revision: 0, sessionId, symbol, drawings: normalizeDrawingOrder(drawings) };
  }
  private quarantine(sessionId: number, symbol: string, raw: string, reason: string): void {
    const key = this.quarantineKey(sessionId, symbol); if (this.storage.getItem(key) === null) this.storage.setItem(key, JSON.stringify({ source: 'backend-state_data', reason, raw }));
  }
}
