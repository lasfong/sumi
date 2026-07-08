import { describe, expect, it } from 'vitest';
import { SumiDrawingAdapter } from '../SumiDrawingAdapter';
import { WorkspacePersistence } from '../WorkspacePersistence';

describe('SumiDrawingAdapter', () => {
  const drawing = {
    id: 'trend-1',
    type: 'trendline' as const,
    color: '#fff',
    points: [
      { time: '2026-01-01' as const, price: 10 },
      { time: '2026-01-02' as const, price: 12 },
    ],
  };

  it('round-trips drawings for backend persistence', () => {
    expect(SumiDrawingAdapter.deserialize(SumiDrawingAdapter.serialize([drawing]))).toEqual([drawing]);
  });

  it('rejects malformed drawing records', () => {
    expect(SumiDrawingAdapter.deserialize('[{"id":"bad","type":"trendline","points":[]}]')).toEqual([]);
  });

  it('round-trips workspace metadata', () => {
    const indicators = [
      { name: 'ema', pane: 'main' as const, params: { length: 20 } },
      { name: 'rsi', pane: 'oscillator' as const, params: { length: 14 } },
    ];
    const value = WorkspacePersistence.serialize({ drawings: [drawing], indicators });
    expect(WorkspacePersistence.deserialize(value)).toEqual({ drawings: [drawing], indicators });
  });
});
