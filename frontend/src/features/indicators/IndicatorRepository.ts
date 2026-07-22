import type { IndicatorDefinition } from '../../api/indicatorsApi';
import {
  addIndicator, approvedDefinitions, createIndicatorInstance, emptyIndicatorDocument,
  validateIndicatorDocument, type IndicatorDocumentV1,
} from './indicatorDomain';

export interface IndicatorStorage { getItem(key: string): string | null; setItem(key: string, value: string): void }

export class IndicatorRepository {
  private readonly storage: IndicatorStorage;

  constructor(storage: IndicatorStorage) { this.storage = storage; }
  private key(sessionId: number) { return `sumi:workspace:${sessionId}`; }

  load(sessionId: number, registry: IndicatorDefinition[]): IndicatorDocumentV1 {
    const definitions = approvedDefinitions(registry); const raw = this.storage.getItem(this.key(sessionId));
    if (!raw) return emptyIndicatorDocument(sessionId);
    try {
      const envelope = JSON.parse(raw) as Record<string, unknown>;
      if (validateIndicatorDocument(envelope.indicatorDocument, sessionId, definitions)) return structuredClone(envelope.indicatorDocument);
      if (!Array.isArray(envelope.indicators)) return emptyIndicatorDocument(sessionId);
      const definitionMap = new Map(definitions.map(definition => [definition.id, definition]));
      let document = emptyIndicatorDocument(sessionId);
      for (const rawInstance of envelope.indicators) {
        if (!rawInstance || typeof rawInstance !== 'object') continue;
        const legacy = rawInstance as Record<string, unknown>; const definition = definitionMap.get(String(legacy.name));
        if (!definition || !legacy.params || typeof legacy.params !== 'object') continue;
        try {
          const instance = createIndicatorInstance(definition, legacy.params as Record<string, unknown>, document.instances.length);
          if (typeof legacy.color === 'string' && /^#[0-9a-f]{6}$/i.test(legacy.color)) {
            const seriesKey = definition.id === 'volume' ? 'volume' : definition.id === 'macd' ? 'macd' : 'primary';
            instance.styles[seriesKey] = { color: legacy.color };
          }
          document = addIndicator(document, instance);
        } catch { /* Malformed legacy entries are isolated. */ }
      }
      if (document.instances.length) this.save(document);
      return document;
    } catch { return emptyIndicatorDocument(sessionId); }
  }

  save(document: IndicatorDocumentV1): void {
    const key = this.key(document.sessionId); let envelope: Record<string, unknown> = {};
    try { envelope = JSON.parse(this.storage.getItem(key) ?? '{}') as Record<string, unknown>; } catch { /* replace malformed envelope */ }
    this.storage.setItem(key, JSON.stringify({ ...envelope, version: 2, indicatorDocument: document }));
  }
}
