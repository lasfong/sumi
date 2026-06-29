import { describe, expect, it } from 'vitest';
import { sortDateKeys, toDateKey, unixSecondsToDateKey } from './date';

describe('date utilities', () => {
  it('normalizes supported chart date formats to yyyy-mm-dd', () => {
    expect(toDateKey('2023-01-04')).toBe('2023-01-04');
    expect(toDateKey('2023-01-04T17:00:00.000Z')).toBe('2023-01-04');
    expect(toDateKey('2023-01-04 00:00:00')).toBe('2023-01-04');
    expect(toDateKey(null)).toBeNull();
  });

  it('converts websocket unix seconds to a chart date key', () => {
    expect(unixSecondsToDateKey(Date.UTC(2023, 0, 4) / 1000)).toBe('2023-01-04');
  });

  it('sorts date keys chronologically for yyyy-mm-dd strings', () => {
    expect(['2023-01-05', '2023-01-04'].sort(sortDateKeys)).toEqual(['2023-01-04', '2023-01-05']);
  });
});
