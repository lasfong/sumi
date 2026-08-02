import React from 'react';
import type { MetricResult } from '../../types/analytics';

interface Props {
  metric?: MetricResult;
  format?: (value: number) => string;
  testId?: string;
}

export const MetricResultValue: React.FC<Props> = ({ metric, format = value => value.toFixed(2), testId }) => {
  const valid = metric?.status === 'valid' && metric.value !== null;
  return (
    <div data-testid={testId} data-metric-status={metric?.status || 'not_applicable'}>
      <div style={{ fontSize: '28px', fontWeight: 'bold', color: valid ? 'var(--text-main)' : 'var(--text-muted)' }}>
        {valid ? format(metric.value as number) : 'Unavailable'}
      </div>
      <div style={{ color: 'var(--text-muted)', fontSize: '12px', marginTop: '4px' }}>
        n={metric?.sample_size ?? 0}{metric?.period ? ` ${metric.period.replaceAll('_', ' ')}` : ''}
      </div>
      {!valid && metric?.reason && (
        <div style={{ color: 'var(--text-muted)', fontSize: '12px', marginTop: '4px' }}>{metric.reason}</div>
      )}
    </div>
  );
};
