import { beforeEach, describe, expect, it } from 'vitest';
import type { IndicatorDefinition } from '../../../api/indicatorsApi';
import { IndicatorRepository } from '../IndicatorRepository';
import {
  addIndicator, approvedDefinitions, createIndicatorInstance, emptyIndicatorDocument, moveIndicator,
  removeIndicator, toggleIndicator, updateIndicator, validateIndicatorDocument, validateIndicatorParams,
} from '../indicatorDomain';

const registry: IndicatorDefinition[] = [
  { id: 'ema', label: 'EMA', category: 'Trend', pane: 'main', description: 'EMA', params: [{ name: 'length', type: 'int', default: 20, minimum: 1, maximum: 500 }, { name: 'offset', type: 'int', default: 0, minimum: -100, maximum: 100 }] },
  { id: 'rsi', label: 'Relative Strength Index', category: 'Momentum', pane: 'oscillator', description: 'RSI', params: [{ name: 'length', type: 'int', default: 14, minimum: 1, maximum: 200 }, { name: 'offset', type: 'int', default: 0, minimum: null, maximum: null }] },
  { id: 'macd', label: 'MACD', category: 'Momentum', pane: 'oscillator', description: 'MACD', params: [{ name: 'fast', type: 'int', default: 12, minimum: 1, maximum: 200 }, { name: 'slow', type: 'int', default: 26, minimum: 2, maximum: 400 }, { name: 'signal', type: 'int', default: 9, minimum: 1, maximum: 200 }, { name: 'offset', type: 'int', default: 0, minimum: null, maximum: null }] },
  { id: 'cci', label: 'Commodity Channel Index', category: 'Momentum', pane: 'oscillator', description: 'CCI', params: [{ name: 'length', type: 'int', default: 20, minimum: 1, maximum: 200 }, { name: 'offset', type: 'int', default: 0, minimum: null, maximum: null }] },
  { id: 'sma', label: 'Simple Moving Average', category: 'Trend', pane: 'main', description: 'SMA', params: [{ name: 'length', type: 'int', default: 20, minimum: 1, maximum: 500 }, { name: 'offset', type: 'int', default: 0, minimum: -100, maximum: 100 }] },
  { id: 'bbands', label: 'Bollinger Bands', category: 'Volatility', pane: 'main', description: 'Bollinger Bands', params: [{ name: 'length', type: 'int', default: 20, minimum: 1, maximum: 500 }, { name: 'std', type: 'float', default: 2.0, minimum: 0.1, maximum: 10 }] },
  { id: 'atr', label: 'Average True Range', category: 'Volatility', pane: 'oscillator', description: 'ATR', params: [{ name: 'length', type: 'int', default: 14, minimum: 1, maximum: 200 }] },
  { id: 'volume_sma', label: 'Volume Moving Average', category: 'Volume', pane: 'oscillator', description: 'Volume SMA', params: [{ name: 'length', type: 'int', default: 20, minimum: 1, maximum: 500 }] },
  { id: 'mfi', label: 'Money Flow Index', category: 'Oscillators', pane: 'oscillator', description: 'MFI', params: [{ name: 'length', type: 'int', default: 14, minimum: 1, maximum: 200 }] },
  { id: 'stoch', label: 'Stochastic Oscillator', category: 'Oscillators', pane: 'oscillator', description: 'Stoch', params: [{ name: 'k', type: 'int', default: 14, minimum: 1, maximum: 200 }, { name: 'd', type: 'int', default: 3, minimum: 1, maximum: 50 }, { name: 'smooth_k', type: 'int', default: 3, minimum: 1, maximum: 50 }] },
  { id: 'adx', label: 'Average Directional Index', category: 'Trend', pane: 'oscillator', description: 'ADX', params: [{ name: 'length', type: 'int', default: 14, minimum: 1, maximum: 200 }] },
  { id: 'relative_strength', label: 'Relative Strength (vs VNINDEX)', category: 'Oscillators', pane: 'oscillator', description: 'RS', params: [{ name: 'length', type: 'int', default: 20, minimum: 1, maximum: 500 }, { name: 'benchmark', type: 'str', default: 'VNINDEX', minimum: null, maximum: null }] },
  { id: 'kc', label: 'Keltner Channels', category: 'Volatility', pane: 'main', description: 'KC', params: [{ name: 'length', type: 'int', default: 20, minimum: 1, maximum: 500 }, { name: 'scalar', type: 'float', default: 2.0, minimum: 0.1, maximum: 20 }] },
  { id: 'psar', label: 'Parabolic SAR', category: 'Trend', pane: 'main', description: 'PSAR', params: [{ name: 'af0', type: 'float', default: 0.02, minimum: 0.001, maximum: 1 }, { name: 'af', type: 'float', default: 0.02, minimum: 0.001, maximum: 1 }, { name: 'max_af', type: 'float', default: 0.2, minimum: 0.001, maximum: 1 }] },
  { id: 'supertrend', label: 'SuperTrend', category: 'Trend', pane: 'main', description: 'SuperTrend', params: [{ name: 'length', type: 'int', default: 7, minimum: 1, maximum: 200 }, { name: 'multiplier', type: 'float', default: 3.0, minimum: 0.1, maximum: 20 }] },
  { id: 'ichimoku', label: 'Ichimoku Cloud', category: 'Trend', pane: 'main', description: 'Ichimoku', params: [{ name: 'tenkan', type: 'int', default: 9, minimum: 1, maximum: 200 }] },
];
const definitions = approvedDefinitions(registry);
const byId = (id: string) => definitions.find(item => item.id === id)!;
const ids = [
  '00000000-0000-4000-8000-000000000001' as ReturnType<typeof crypto.randomUUID>,
  '00000000-0000-4000-8000-000000000002' as ReturnType<typeof crypto.randomUUID>,
  '00000000-0000-4000-8000-000000000003' as ReturnType<typeof crypto.randomUUID>,
  '00000000-0000-4000-8000-000000000004' as ReturnType<typeof crypto.randomUUID>,
];

describe('indicator domain', () => {
  it('creates duplicate definitions as independent stable instances and panes', () => {
    const first = createIndicatorInstance(byId('rsi'), {}, 0, ids[0]);
    const second = createIndicatorInstance(byId('rsi'), { length: 21 }, 1, ids[1]);
    expect(first.id).not.toBe(second.id);
    expect(first.paneId).toBe(`indicator:${ids[0]}`);
    expect(second.paneId).toBe(`indicator:${ids[1]}`);
    expect(first.params).toEqual({ length: 14, offset: 0 });
    expect(second.params).toEqual({ length: 21, offset: 0 });
  });

  it('filters out unreleased backend definitions like ichimoku from approved definitions', () => {
    const idsInApproved = definitions.map(def => def.id);
    expect(idsInApproved).toContain('sma');
    expect(idsInApproved).toContain('bbands');
    expect(idsInApproved).toContain('atr');
    expect(idsInApproved).toContain('volume_sma');
    expect(idsInApproved).toContain('volume');
    expect(idsInApproved).toContain('mfi');
    expect(idsInApproved).toContain('stoch');
    expect(idsInApproved).toContain('adx');
    expect(idsInApproved).toContain('relative_strength');
    expect(idsInApproved).toContain('kc');
    expect(idsInApproved).toContain('psar');
    expect(idsInApproved).toContain('supertrend');
    expect(idsInApproved).not.toContain('ichimoku');
  });

  it('correctly assigns price, volume, and oscillator placements for all indicators', () => {
    const smaInst = createIndicatorInstance(byId('sma'), {}, 0, ids[0]);
    const bbandsInst = createIndicatorInstance(byId('bbands'), {}, 1, ids[1]);
    const atrInst = createIndicatorInstance(byId('atr'), {}, 2, ids[2]);
    const vmaInst = createIndicatorInstance(byId('volume_sma'), {}, 3, ids[3]);
    const mfiInst = createIndicatorInstance(byId('mfi'), {}, 4, ids[0]);
    const stochInst = createIndicatorInstance(byId('stoch'), {}, 5, ids[1]);
    const adxInst = createIndicatorInstance(byId('adx'), {}, 6, ids[2]);
    const rsInst = createIndicatorInstance(byId('relative_strength'), {}, 7, ids[3]);
    const kcInst = createIndicatorInstance(byId('kc'), {}, 8, ids[0]);
    const psarInst = createIndicatorInstance(byId('psar'), {}, 9, ids[1]);
    const stInst = createIndicatorInstance(byId('supertrend'), {}, 10, ids[2]);

    expect(smaInst.placement).toBe('price');
    expect(smaInst.paneId).toBe('price');
    expect(bbandsInst.placement).toBe('price');
    expect(bbandsInst.paneId).toBe('price');
    expect(kcInst.placement).toBe('price');
    expect(kcInst.paneId).toBe('price');
    expect(psarInst.placement).toBe('price');
    expect(psarInst.paneId).toBe('price');
    expect(stInst.placement).toBe('price');
    expect(stInst.paneId).toBe('price');
    expect(atrInst.placement).toBe('oscillator');
    expect(atrInst.paneId).toBe(`indicator:${ids[2]}`);
    expect(vmaInst.placement).toBe('volume');
    expect(vmaInst.paneId).toBe('volume');
    expect(mfiInst.placement).toBe('oscillator');
    expect(mfiInst.paneId).toBe(`indicator:${ids[0]}`);
    expect(stochInst.placement).toBe('oscillator');
    expect(stochInst.paneId).toBe(`indicator:${ids[1]}`);
    expect(adxInst.placement).toBe('oscillator');
    expect(adxInst.paneId).toBe(`indicator:${ids[2]}`);
    expect(rsInst.placement).toBe('oscillator');
    expect(rsInst.paneId).toBe(`indicator:${ids[3]}`);
  });

  it('validates type and registry ranges before an instance can be added', () => {
    expect(validateIndicatorParams(byId('ema'), { length: '21', offset: 0 }).params).toEqual({ length: 21, offset: 0 });
    expect(validateIndicatorParams(byId('ema'), { length: 0, offset: 0 }).errors.length).toContain('1–500');
    expect(validateIndicatorParams(byId('ema'), { length: 2.5, offset: 0 }).errors.length).toContain('must be int');
  });

  it('supports apply, cancel-by-immutability, visibility, removal and deterministic order', () => {
    const a = createIndicatorInstance(byId('ema'), {}, 0, ids[0]);
    const b = createIndicatorInstance(byId('cci'), {}, 1, ids[1]);
    const initial = addIndicator(addIndicator(emptyIndicatorDocument(7), a), b);
    const draft = { params: { length: 55 }, styles: a.styles };
    expect(initial.instances[0].params).toEqual({ length: 20, offset: 0 }); // cancelling leaves source untouched
    const applied = updateIndicator(initial, a.id, draft);
    expect(applied.instances[0].params).toEqual({ length: 55 });
    expect(toggleIndicator(applied, a.id).instances[0].visible).toBe(false);
    expect(moveIndicator(applied, b.id, -1).instances.map(item => [item.id, item.order])).toEqual([[b.id, 0], [a.id, 1]]);
    expect(removeIndicator(applied, a.id).instances).toEqual([{ ...b, order: 0 }]);
  });

  it('rejects malformed, cross-session and duplicate-id documents', () => {
    const item = createIndicatorInstance(byId('rsi'), {}, 0, ids[0]);
    const valid = addIndicator(emptyIndicatorDocument(9), item);
    expect(validateIndicatorDocument(valid, 9, definitions)).toBe(true);
    expect(validateIndicatorDocument(valid, 10, definitions)).toBe(false);
    expect(validateIndicatorDocument({ ...valid, instances: [item, { ...item, order: 1 }] }, 9, definitions)).toBe(false);
    expect(validateIndicatorDocument({ ...valid, instances: [{ ...item, params: { ...item.params, providerBlob: 1 } }] }, 9, definitions)).toBe(false);
  });
});

describe('IndicatorRepository', () => {
  beforeEach(() => window.localStorage.clear());

  it('round-trips canonical session state while preserving unrelated envelope data', () => {
    window.localStorage.setItem('sumi:workspace:4', JSON.stringify({ drawings: ['keep-me'] }));
    const repository = new IndicatorRepository(window.localStorage);
    const document = addIndicator(emptyIndicatorDocument(4), createIndicatorInstance(byId('macd'), {}, 0, ids[0]));
    repository.save(document);
    expect(repository.load(4, registry)).toEqual(document);
    expect(JSON.parse(window.localStorage.getItem('sumi:workspace:4') ?? '{}').drawings).toEqual(['keep-me']);
  });

  it('promotes valid legacy entries and isolates malformed or unknown entries', () => {
    window.localStorage.setItem('sumi:workspace:5', JSON.stringify({ indicators: [
      { name: 'ema', params: { length: 34, offset: 0 }, color: '#abcdef' },
      { name: 'unknown', params: {} }, { name: 'rsi', params: { length: 0 } }, null,
    ] }));
    const loaded = new IndicatorRepository(window.localStorage).load(5, registry);
    expect(loaded.instances).toHaveLength(1);
    expect(loaded.instances[0]).toMatchObject({ definitionId: 'ema', params: { length: 34, offset: 0 }, styles: { primary: { color: '#abcdef' } } });
  });

  it('does not leak indicator state across sessions and fails closed on corrupt storage', () => {
    const repository = new IndicatorRepository(window.localStorage);
    repository.save(addIndicator(emptyIndicatorDocument(6), createIndicatorInstance(byId('cci'), {}, 0, ids[0])));
    window.localStorage.setItem('sumi:workspace:7', '{bad json');
    expect(repository.load(7, registry)).toEqual(emptyIndicatorDocument(7));
    expect(repository.load(8, registry)).toEqual(emptyIndicatorDocument(8));
  });
});
