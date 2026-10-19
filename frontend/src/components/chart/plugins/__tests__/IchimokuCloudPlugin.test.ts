import { describe, expect, it } from 'vitest';
import { IchimokuCloudPlugin } from '../IchimokuCloudPlugin';

describe('SQA-012 — IchimokuCloudPlugin Autoscale & Whitespace Isolation', () => {
  const plugin = new IchimokuCloudPlugin();

  it('treats incomplete or warmup bars as whitespace', () => {
    // Both null
    expect(plugin.isWhitespace({ time: '2024-01-01', spanA: null, spanB: null })).toBe(true);
    // Span A present, Span B null (e.g. between bar 26 and 52)
    expect(plugin.isWhitespace({ time: '2024-01-02', spanA: 95000, spanB: null })).toBe(true);
    // Span A null, Span B present
    expect(plugin.isWhitespace({ time: '2024-01-03', spanA: null, spanB: 94000 })).toBe(true);
    // Undefined
    expect(plugin.isWhitespace({ time: '2024-01-04' })).toBe(true);
    // NaN
    expect(plugin.isWhitespace({ time: '2024-01-05', spanA: NaN, spanB: 94000 })).toBe(true);
    expect(plugin.isWhitespace({ time: '2024-01-06', spanA: 95000, spanB: NaN })).toBe(true);
  });

  it('treats complete numeric bars as valid data (not whitespace)', () => {
    expect(plugin.isWhitespace({ time: '2024-01-07', spanA: 95000, spanB: 94000 })).toBe(false);
  });

  it('never injects 0 or non-finite values into autoscale priceValueBuilder', () => {
    // When both are null/missing, priceValueBuilder returns empty array
    const emptyValues = plugin.priceValueBuilder({ time: '2024-01-01', spanA: null, spanB: null });
    expect(emptyValues).toEqual([]);
    expect(emptyValues).not.toContain(0);

    // When one is missing, returns only the finite value
    const singleValues = plugin.priceValueBuilder({ time: '2024-01-02', spanA: 98500, spanB: null });
    expect(singleValues).toEqual([98500, 98500, 98500]);
    expect(singleValues).not.toContain(0);

    // When both are present, returns both finite values
    const fullValues = plugin.priceValueBuilder({ time: '2024-01-03', spanA: 99000, spanB: 97000 });
    expect(fullValues).toEqual([99000, 97000, 99000]);
    expect(fullValues).not.toContain(0);
  });
});
