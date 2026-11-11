import { describe, expect, it } from 'vitest';
import Ajv2020 from 'ajv/dist/2020.js';
import valid from '../__fixtures__/valid-horizontal-document.json';
import duplicate from '../__fixtures__/invalid-duplicate-document.json';
import allTools from '../__fixtures__/valid-all-tools-document.json';
import future from '../__fixtures__/invalid-future-document.json';
import schema from '../../../../../docs/decision-packs/sumi-drawing-document-v1.schema.json';
import { DrawingCommandHistory } from '../DrawingCommandHistory';
import { DRAWING_CONTRACT_CORPUS, materializeDrawingContractCase } from '../drawingContractCorpus';
import { DrawingRepository, DrawingRevisionConflict } from '../DrawingRepository';
import { createDrawing, parseDrawingDocument, validateDrawingDocument, validateDrawingDocumentSemantics, type SumiDrawingDocumentV1 } from '../drawingDomain';

describe('Sumi drawing document v1', () => {
  it('validates and round-trips the valid fixture', () => {
    expect(validateDrawingDocument(valid)).toBe(true);
    expect(parseDrawingDocument(JSON.stringify(valid))).toEqual(valid);
  });
  it('validates one shared corpus with Draft 2020-12 structure and supplemental Sumi semantics', () => {
    const ajv = new Ajv2020({ allErrors: true, strict: true });
    ajv.addFormat('uuid', /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i);
    const validateStructure = ajv.compile(schema);
    const results = DRAWING_CONTRACT_CORPUS.cases.map(item => {
      const document = materializeDrawingContractCase(item);
      return {
        id: item.id,
        structural: validateStructure(document), expectedStructural: item.expectedStructural,
        semantic: validateDrawingDocumentSemantics(document, item.identity), expectedSemantic: item.expectedSemantic,
      };
    });
    expect(results.map(item => ({ id: item.id, structural: item.structural, semantic: item.semantic }))).toEqual(
      results.map(item => ({ id: item.id, structural: item.expectedStructural, semantic: item.expectedSemantic })),
    );
    expect(DRAWING_CONTRACT_CORPUS.semanticInvariants).toEqual([
      'ray-strictly-rightward', 'unique-drawing-ids', 'contiguous-drawing-order', 'workspace-identity',
    ]);
  });
  it('preserves an existing Horizontal v1 document byte-semantically', () => {
    const serialized = JSON.stringify(valid); expect(JSON.stringify(parseDrawingDocument(serialized))).toBe(serialized);
  });
  it('rejects duplicate ids and broken order invariants', () => {
    expect(validateDrawingDocument(duplicate)).toBe(false);
    expect(validateDrawingDocument({ ...valid, drawings: [{ ...valid.drawings[0], order: 2 }] })).toBe(false);
  });
  it('rejects provider-native and invalid semantic fields', () => {
    expect(validateDrawingDocument({ ...valid, providerId: 'raw' })).toBe(false);
    expect(validateDrawingDocument({ ...valid, drawings: [{ ...valid.drawings[0], anchors: [{ time: 'bad', price: -1 }] }] })).toBe(false);
  });
  it('validates one strict canonical fixture containing every required tool', () => {
    expect(validateDrawingDocument(allTools)).toBe(true);
    expect((allTools.drawings as Array<{ tool: string }>).map(item => item.tool)).toEqual([
      'horizontal', 'trendline', 'ray', 'rectangle', 'fibonacci-retracement', 'text',
    ]);
  });
  it('rejects future versions, unknown fields, excessive text, invalid styles and fib semantics', () => {
    expect(validateDrawingDocument(future)).toBe(false);
    expect(validateDrawingDocument({ ...allTools, unknown: true })).toBe(false);
    expect(validateDrawingDocument({ ...allTools, drawings: allTools.drawings.map((item, index) => index === 5 ? { ...item, geometry: { kind: 'text', text: 'x'.repeat(2001) } } : item) })).toBe(false);
    expect(validateDrawingDocument({ ...allTools, drawings: allTools.drawings.map((item, index) => index === 0 ? { ...item, style: { ...item.style, lineWidth: 9 } } : item) })).toBe(false);
    expect(validateDrawingDocument({ ...allTools, drawings: allTools.drawings.map((item, index) => index === 4 ? { ...item, geometry: { ...item.geometry, direction: 'up', levels: [] } } : item) })).toBe(false);
    expect(validateDrawingDocument({ ...valid, drawings: [{ ...valid.drawings[0], paneId: 'volume' }] })).toBe(false);
    expect(validateDrawingDocument({ ...allTools, drawings: allTools.drawings.map((item, index) => index === 5 ? { ...item, geometry: { kind: 'text', text: ' \n ' } } : item) })).toBe(false);
    expect(validateDrawingDocument({ ...allTools, drawings: allTools.drawings.map((item, index) => index === 2 ? { ...item, anchors: [item.anchors[1], item.anchors[0]] } : item) })).toBe(false);
  });
});

describe('DrawingRepository and DrawingCommandHistory', () => {
  it('increments revisions and rejects stale writes', () => {
    const storage = new Map<string, string>();
    const adapter = { getItem: (k: string) => storage.get(k) ?? null, setItem: (k: string, v: string) => { storage.set(k, v); }, removeItem: (k: string) => { storage.delete(k); } };
    const repository = new DrawingRepository(adapter);
    const saved = repository.save(valid as SumiDrawingDocumentV1, 0);
    expect(saved.revision).toBe(1);
    expect(repository.load(7, 'FPT')).toEqual(saved);
    expect(() => repository.save(valid as SumiDrawingDocumentV1, 0)).toThrow(DrawingRevisionConflict);
  });
  it('isolates a new symbol, saves it, and preserves the old identity', () => {
    const storage = new Map<string, string>();
    const adapter = { getItem: (k: string) => storage.get(k) ?? null, setItem: (k: string, v: string) => { storage.set(k, v); }, removeItem: (k: string) => { storage.delete(k); } };
    const repository = new DrawingRepository(adapter);
    const oldFpt = { ...(valid as SumiDrawingDocumentV1), revision: 5 };
    storage.set('sumi:drawing-document:v1:7', JSON.stringify(oldFpt));
    expect(repository.load(7, 'SSI')).toBeNull();
    const newSsi = { ...(valid as SumiDrawingDocumentV1), symbol: 'SSI', drawings: [], revision: 0 };
    const savedSsi = repository.save(newSsi, 0);
    expect(savedSsi).toMatchObject({ symbol: 'SSI', revision: 1, drawings: [] });
    expect(repository.load(7, 'SSI')).toEqual(savedSsi);
    expect(repository.load(7, 'FPT')).toEqual(oldFpt);
    expect(JSON.parse(storage.get('sumi:drawing-document:v1:7:FPT') ?? 'null')).toEqual(oldFpt);
  });
  it('loads and safely promotes an existing same-symbol legacy document', () => {
    const storage = new Map<string, string>();
    const adapter = { getItem: (k: string) => storage.get(k) ?? null, setItem: (k: string, v: string) => { storage.set(k, v); }, removeItem: (k: string) => { storage.delete(k); } };
    const repository = new DrawingRepository(adapter);
    const oldFpt = { ...(valid as SumiDrawingDocumentV1), revision: 4 };
    storage.set('sumi:drawing-document:v1:7', JSON.stringify(oldFpt));
    expect(repository.load(7, 'FPT')).toEqual(oldFpt);
    expect(JSON.parse(storage.get('sumi:drawing-document:v1:7:FPT') ?? 'null')).toEqual(oldFpt);
    expect(repository.load(7, 'SSI')).toBeNull(); expect(repository.load(8, 'FPT')).toBeNull();
  });
  it('keeps same-identity compare-and-swap conflicts after identity promotion', () => {
    const storage = new Map<string, string>();
    const adapter = { getItem: (k: string) => storage.get(k) ?? null, setItem: (k: string, v: string) => { storage.set(k, v); }, removeItem: (k: string) => { storage.delete(k); } };
    const repository = new DrawingRepository(adapter);
    const saved = repository.save(valid as SumiDrawingDocumentV1, 0);
    expect(repository.load(7, 'FPT')).toEqual(saved);
    expect(() => repository.save(valid as SumiDrawingDocumentV1, 0)).toThrow(DrawingRevisionConflict);
  });
  it('undoes and redoes one domain command', () => {
    const history = new DrawingCommandHistory();
    const before = { ...(valid as SumiDrawingDocumentV1), drawings: [] };
    history.commit({ kind: 'create', before, after: valid as SumiDrawingDocumentV1 });
    expect(history.peekUndo()?.before.drawings).toHaveLength(0);
    history.acceptUndo();
    expect(history.peekRedo()?.after.drawings).toHaveLength(1);
  });
  it('migrates valid legacy backend records, backs up raw data, skips cursor, and is idempotent', () => {
    const storage = new Map<string, string>();
    const adapter = { getItem: (k: string) => storage.get(k) ?? null, setItem: (k: string, v: string) => { storage.set(k, v); }, removeItem: (k: string) => { storage.delete(k); } };
    const repository = new DrawingRepository(adapter);
    const raw = JSON.stringify([
      { id: '00000000-0000-4000-8000-000000000099', type: 'cursor', color: '#fff', points: [] },
      { id: '00000000-0000-4000-8000-000000000001', type: 'horizontal', color: '#123456', points: [{ time: '2026-07-01', price: 100 }] },
      { id: '00000000-0000-4000-8000-000000000005', type: 'fibonacci', color: '#654321', points: [{ time: '2026-07-01', price: 90 }, { time: '2026-07-05', price: 120 }] },
    ]);
    const first = repository.hydrate(7, 'FPT', raw);
    expect(first.migrated).toBe(true); expect(first.document.drawings.map(item => item.tool)).toEqual(['horizontal', 'fibonacci-retracement']);
    expect(storage.get(repository.backupKey(7, 'FPT'))).toBe(raw);
    const second = repository.hydrate(7, 'FPT', JSON.stringify(first.document));
    expect(second.document.drawings).toHaveLength(2); expect(second.migrated).toBe(false);
  });
  it('quarantines malformed legacy records and isolates session/symbol canonical state', () => {
    const storage = new Map<string, string>();
    const adapter = { getItem: (k: string) => storage.get(k) ?? null, setItem: (k: string, v: string) => { storage.set(k, v); }, removeItem: (k: string) => { storage.delete(k); } };
    const repository = new DrawingRepository(adapter);
    const malformed = JSON.stringify([{ id: 'bad', type: 'trendline', points: [{ time: 'bad', price: -1 }] }]);
    const blocked = repository.hydrate(7, 'FPT', malformed); expect(blocked.document.drawings).toEqual([]); expect(blocked.conflict).toContain('quarantined');
    expect(storage.get(repository.quarantineKey(7, 'FPT'))).toContain('Malformed or ambiguous');
    expect(storage.get(repository.backupKey(7, 'FPT'))).toBe(malformed);
    expect(repository.raw(7, 'FPT')).toBeNull();
    const wrongIdentity = { ...(allTools as unknown as SumiDrawingDocumentV1), sessionId: 8, symbol: 'SSI' };
    expect(repository.hydrate(7, 'FPT', JSON.stringify(wrongIdentity)).conflict).toContain('identity');
  });
  it('never chooses a divergent valid canonical copy merely because its revision is higher', () => {
    const storage = new Map<string, string>();
    const adapter = { getItem: (k: string) => storage.get(k) ?? null, setItem: (k: string, v: string) => { storage.set(k, v); }, removeItem: (k: string) => { storage.delete(k); } };
    const repository = new DrawingRepository(adapter); const local = { ...(valid as SumiDrawingDocumentV1), revision: 9 }; repository.put(local);
    const remote = { ...(valid as SumiDrawingDocumentV1), revision: 10, drawings: [] };
    const hydration = repository.hydrate(7, 'FPT', JSON.stringify(remote));
    expect(hydration.document).toEqual(local); expect(hydration.conflict).toContain('diverged'); expect(repository.load(7, 'FPT')).toEqual(local);
  });
  it('creates and validates risk-reward drawing contract', () => {
    const rr = createDrawing('risk-reward', [
      { time: '2026-07-01', price: 100 },
      { time: '2026-07-01', price: 90 },
      { time: '2026-07-01', price: 125 },
    ], 0);
    expect(rr.tool).toBe('risk-reward');
    if (rr.tool !== 'risk-reward') throw new Error('type mismatch');
    expect(rr.geometry.direction).toBe('long');
    expect(rr.geometry.riskRewardRatio).toBe(2.5);

    const doc: SumiDrawingDocumentV1 = {
      schemaVersion: 1,
      revision: 1,
      sessionId: 7,
      symbol: 'FPT',
      drawings: [rr],
    };
    expect(validateDrawingDocument(doc)).toBe(true);
  });
});
