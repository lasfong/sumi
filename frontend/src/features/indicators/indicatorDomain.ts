import type { IndicatorDefinition, IndicatorParamDefinition } from '../../api/indicatorsApi';

export type IndicatorValue = string | number | boolean;
export type IndicatorPlacement = 'price' | 'oscillator' | 'volume';
export interface IndicatorSeriesStyle { color: string; lineStyle?: 'solid' | 'dashed' | 'dotted' }

export const SUPPORTED_INDICATORS = [
  'ema', 'rsi', 'macd', 'cci', 'volume', 'sma', 'bbands', 'atr', 'volume_sma',
  'mfi', 'stoch', 'adx', 'relative_strength',
  'kc', 'psar', 'supertrend',
] as const;
export type SupportedIndicatorId = (typeof SUPPORTED_INDICATORS)[number];

export interface IndicatorInstanceV1 {
  id: string;
  definitionId: SupportedIndicatorId;
  label: string;
  params: Record<string, IndicatorValue>;
  placement: IndicatorPlacement;
  paneId: 'price' | 'volume' | `indicator:${string}`;
  visible: boolean;
  styles: Record<string, IndicatorSeriesStyle>;
  order: number;
}

export interface IndicatorDocumentV1 {
  schemaVersion: 1;
  sessionId: number;
  instances: IndicatorInstanceV1[];
}

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

export const VOLUME_DEFINITION: IndicatorDefinition = {
  id: 'volume', label: 'Volume', category: 'Volume', pane: 'oscillator', params: [],
  description: 'Raw traded volume from visible replay candles.',
};

export const approvedDefinitions = (definitions: IndicatorDefinition[]): IndicatorDefinition[] => {
  const approved = definitions.filter(definition => (SUPPORTED_INDICATORS as readonly string[]).includes(definition.id));
  return [...approved, VOLUME_DEFINITION];
};

export const emptyIndicatorDocument = (sessionId: number): IndicatorDocumentV1 => ({ schemaVersion: 1, sessionId, instances: [] });

const placementFor = (definition: IndicatorDefinition): IndicatorPlacement =>
  definition.id === 'volume' || definition.id === 'volume_sma' ? 'volume' : definition.pane === 'main' ? 'price' : 'oscillator';

const defaultStyles = (definitionId: string): Record<string, IndicatorSeriesStyle> => {
  if (definitionId === 'macd') return {
    macd: { color: '#58A6FF' }, signal: { color: '#FF8A00' }, histogram: { color: '#00E676' },
  };
  if (definitionId === 'volume') return { volume: { color: '#58A6FF' } };
  if (definitionId === 'rsi') return { primary: { color: '#B388FF' } };
  if (definitionId === 'cci') return { primary: { color: '#FFD166' } };
  if (definitionId === 'sma') return { primary: { color: '#FFD166' } };
  if (definitionId === 'bbands') return {
    upper: { color: '#00E5FF' }, middle: { color: '#FFD166' }, lower: { color: '#00E5FF' },
  };
  if (definitionId === 'kc') return {
    upper: { color: '#00E5FF' }, middle: { color: '#FFD166' }, lower: { color: '#00E5FF' },
  };
  if (definitionId === 'psar') return {
    sar: { color: '#E040FB' }, primary: { color: '#E040FB' },
  };
  if (definitionId === 'supertrend') return {
    supertrend: { color: '#26A69A' }, bull: { color: '#26A69A' }, bear: { color: '#EF5350' }, primary: { color: '#26A69A' },
  };
  if (definitionId === 'atr') return { primary: { color: '#E040FB' } };
  if (definitionId === 'volume_sma') return { primary: { color: '#FF8A00' } };
  if (definitionId === 'mfi') return { primary: { color: '#26A69A' } };
  if (definitionId === 'stoch') return {
    k: { color: '#58A6FF' }, d: { color: '#FF8A00' },
  };
  if (definitionId === 'adx') return {
    adx: { color: '#FFD166' }, dmp: { color: '#26A69A' }, dmn: { color: '#EF5350' },
  };
  if (definitionId === 'relative_strength') return { primary: { color: '#00E5FF' } };
  return { primary: { color: '#58A6FF' } };
};

export const defaultsFor = (definition: IndicatorDefinition): Record<string, IndicatorValue> =>
  definition.params.reduce<Record<string, IndicatorValue>>((result, param) => {
    if (param.default !== null) result[param.name] = param.default;
    return result;
  }, {});

const coerceParam = (param: IndicatorParamDefinition, raw: unknown): IndicatorValue | null => {
  if (param.type === 'bool') {
    if (typeof raw === 'boolean') return raw;
    if (raw === 'true' || raw === 'false') return raw === 'true';
    return null;
  }
  if (param.type === 'string' || param.type === 'str') return typeof raw === 'string' ? raw : (raw !== null && raw !== undefined ? String(raw) : null);
  if (raw === '' || raw === null || raw === undefined) return null;
  const value = Number(raw);
  if (!Number.isFinite(value) || (param.type === 'int' && !Number.isInteger(value))) return null;
  if (param.minimum !== null && value < param.minimum) return null;
  if (param.maximum !== null && value > param.maximum) return null;
  return value;
};

export const validateIndicatorParams = (
  definition: IndicatorDefinition, input: Record<string, unknown>,
): { params: Record<string, IndicatorValue>; errors: Record<string, string> } => {
  const params: Record<string, IndicatorValue> = {}; const errors: Record<string, string> = {};
  for (const param of definition.params) {
    const raw = input[param.name] ?? param.default;
    const value = coerceParam(param, raw);
    if (value === null) {
      const range = [param.minimum, param.maximum].some(item => item !== null)
        ? ` (${param.minimum ?? '−∞'}–${param.maximum ?? '∞'})` : '';
      errors[param.name] = `${param.name} must be ${param.type}${range}`;
    } else params[param.name] = value;
  }
  return { params, errors };
};

export const createIndicatorInstance = (
  definition: IndicatorDefinition, input: Record<string, unknown>, order: number, id = crypto.randomUUID(),
): IndicatorInstanceV1 => {
  const validated = validateIndicatorParams(definition, input);
  if (Object.keys(validated.errors).length) throw new TypeError(Object.values(validated.errors).join(', '));
  const placement = placementFor(definition);
  const definitionId = definition.id as IndicatorInstanceV1['definitionId'];
  return {
    id, definitionId, label: definition.label, params: validated.params, placement,
    paneId: placement === 'price' ? 'price' : placement === 'volume' ? 'volume' : `indicator:${id}`,
    visible: true, styles: defaultStyles(definitionId), order,
  };
};

const isColor = (value: unknown): value is string => typeof value === 'string' && /^#[0-9a-f]{6}$/i.test(value);

export const validateIndicatorDocument = (
  value: unknown, sessionId: number, definitions: IndicatorDefinition[],
): value is IndicatorDocumentV1 => {
  if (!value || typeof value !== 'object') return false;
  const doc = value as Record<string, unknown>;
  if (doc.schemaVersion !== 1 || doc.sessionId !== sessionId || !Array.isArray(doc.instances)) return false;
  const definitionMap = new Map(definitions.map(definition => [definition.id, definition]));
  const ids = new Set<string>();
  return doc.instances.every((raw, order) => {
    if (!raw || typeof raw !== 'object') return false;
    const instance = raw as Record<string, unknown>; const id = String(instance.id ?? '');
    const definition = definitionMap.get(String(instance.definitionId));
    if (!UUID.test(id) || ids.has(id) || !definition || instance.order !== order || typeof instance.visible !== 'boolean') return false;
    ids.add(id);
    const placement = placementFor(definition);
    const paneId = placement === 'price' ? 'price' : placement === 'volume' ? 'volume' : `indicator:${id}`;
    if (instance.label !== definition.label || instance.placement !== placement || instance.paneId !== paneId) return false;
    if (!instance.params || typeof instance.params !== 'object') return false;
    const parameterNames = definition.params.map(param => param.name).sort();
    if (JSON.stringify(Object.keys(instance.params).sort()) !== JSON.stringify(parameterNames)) return false;
    if (Object.keys(validateIndicatorParams(definition, instance.params as Record<string, unknown>).errors).length) return false;
    if (!instance.styles || typeof instance.styles !== 'object') return false;
    return Object.values(instance.styles as Record<string, unknown>).every(style => {
      if (!style || typeof style !== 'object') return false;
      const candidate = style as Record<string, unknown>;
      return isColor(candidate.color) && (candidate.lineStyle === undefined || ['solid', 'dashed', 'dotted'].includes(String(candidate.lineStyle)));
    });
  });
};

const normalize = (document: IndicatorDocumentV1, instances: IndicatorInstanceV1[]): IndicatorDocumentV1 => ({
  ...document, instances: instances.map((instance, order) => ({ ...instance, order })),
});

export const addIndicator = (document: IndicatorDocumentV1, instance: IndicatorInstanceV1) => normalize(document, [...document.instances, instance]);
export const removeIndicator = (document: IndicatorDocumentV1, id: string) => normalize(document, document.instances.filter(item => item.id !== id));
export const toggleIndicator = (document: IndicatorDocumentV1, id: string) => normalize(document, document.instances.map(item => item.id === id ? { ...item, visible: !item.visible } : item));
export const updateIndicator = (document: IndicatorDocumentV1, id: string, update: Pick<IndicatorInstanceV1, 'params' | 'styles'>) =>
  normalize(document, document.instances.map(item => item.id === id ? { ...item, ...update } : item));
export const moveIndicator = (document: IndicatorDocumentV1, id: string, direction: -1 | 1) => {
  const index = document.instances.findIndex(item => item.id === id); const target = index + direction;
  if (index < 0 || target < 0 || target >= document.instances.length) return document;
  const instances = [...document.instances]; [instances[index], instances[target]] = [instances[target], instances[index]];
  return normalize(document, instances);
};

export const formatIndicatorParams = (instance: IndicatorInstanceV1): string =>
  Object.entries(instance.params).filter(([name]) => name !== 'offset').map(([name, value]) => `${name}=${value}`).join(', ') || 'No parameters';
