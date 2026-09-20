import React from 'react';
import { Eye, EyeOff, Settings, X } from 'lucide-react';
import type { Candle } from '../../types';
import type { IndicatorInstanceV1 } from '../../features/indicators/indicatorDomain';
import type { IndicatorRuntimeState } from './IndicatorManager';
import { formatIndicatorParams } from '../../features/indicators/indicatorDomain';
import { formatVietnameseDate, formatVietnameseNumber } from '../../utils/formatters';

interface ChartLegendOverlayProps {
  symbol: string;
  timeframe?: string;
  currentCandle: Candle | null;
  previousCandle?: Candle | null;
  indicators: IndicatorInstanceV1[];
  runtime: Record<string, IndicatorRuntimeState>;
  onToggle: (id: string) => void;
  onSettings: (instance: IndicatorInstanceV1) => void;
  onRemove: (id: string) => void;
}

const formatVal = (val: number | null | undefined): string => {
  if (val === null || val === undefined) return '—';
  return val.toLocaleString(undefined, { maximumFractionDigits: 2 });
};

export const ChartLegendOverlay: React.FC<ChartLegendOverlayProps> = ({
  symbol,
  timeframe = '1D',
  currentCandle,
  indicators,
  runtime,
  onToggle,
  onSettings,
  onRemove,
}) => {
  const isBullish = currentCandle ? currentCandle.close >= currentCandle.open : true;
  const changeValue = currentCandle ? currentCandle.close - currentCandle.open : 0;
  const changePercent = currentCandle && currentCandle.open > 0 ? (changeValue / currentCandle.open) * 100 : 0;
  const changeColor = isBullish ? '#089981' : '#f23645';

  return (
    <div
      style={{
        position: 'absolute',
        top: 10,
        left: 14,
        zIndex: 5,
        pointerEvents: 'none',
        display: 'flex',
        flexDirection: 'column',
        gap: 5,
        fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
        maxWidth: 'calc(100% - 100px)',
      }}
    >
      {/* 1. Symbol & Bar OHLC Header */}
      {currentCandle && (
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            flexWrap: 'wrap',
            gap: 8,
            fontSize: '12px',
            color: '#b2b5be',
            lineHeight: '1.2',
            textShadow: '0 1px 3px rgba(0,0,0,0.8)',
            pointerEvents: 'auto',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            <span style={{ fontWeight: 700, color: '#f0f3fa', fontSize: '13px', letterSpacing: '0.3px' }}>
              {symbol}
            </span>
            <span style={{ color: '#787b86', fontSize: '11px', background: 'rgba(255,255,255,0.06)', padding: '1px 5px', borderRadius: 3 }}>
              {timeframe}
            </span>
            <span style={{ color: '#00E5FF', fontSize: '10px', background: 'rgba(0, 229, 255, 0.1)', padding: '1px 5px', borderRadius: 3, fontWeight: 600 }}>
              x1,000 ₫
            </span>
            <span
              style={{ color: '#787b86', fontSize: '11px', fontFamily: 'monospace' }}
            >
              {formatVietnameseDate(currentCandle.timestamp)}
            </span>
          </div>

          <div style={{ display: 'flex', gap: 7, fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace' }}>
            <span>
              O: <strong style={{ color: '#f0f3fa', fontWeight: 600 }}>{formatVietnameseNumber(currentCandle.open, 2)}</strong>
            </span>
            <span>
              H: <strong style={{ color: '#f0f3fa', fontWeight: 600 }}>{formatVietnameseNumber(currentCandle.high, 2)}</strong>
            </span>
            <span>
              L: <strong style={{ color: '#f0f3fa', fontWeight: 600 }}>{formatVietnameseNumber(currentCandle.low, 2)}</strong>
            </span>
            <span>
              C:{' '}
              <strong style={{ color: changeColor, fontWeight: 700 }}>
                {formatVietnameseNumber(currentCandle.close, 2)}
              </strong>
            </span>
            <span style={{ color: changeColor, fontWeight: 600 }}>
              {changeValue >= 0 ? `+${formatVietnameseNumber(changeValue, 2)}` : formatVietnameseNumber(changeValue, 2)} ({changePercent >= 0 ? `+${changePercent.toFixed(2)}` : changePercent.toFixed(2)}%)
            </span>
            <span style={{ color: '#787b86' }}>
              Vol: <span style={{ color: '#b2b5be' }}>{currentCandle.volume.toLocaleString()}</span>
            </span>
          </div>
        </div>
      )}

      {/* 2. Active Indicators List */}
      <div
        data-testid="active-indicator-list"
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: 3,
          pointerEvents: 'auto',
        }}
      >
        {indicators.map((instance) => {
          const state = runtime[instance.id] ?? { status: 'idle', values: {} };
          const mainColor = Object.values(instance.styles)[0]?.color ?? '#2962FF';
          const paramsStr = formatIndicatorParams(instance);

          return (
            <div
              key={instance.id}
              data-testid={`indicator-instance-${instance.id}`}
              data-card-id={`indicator-card-${instance.definitionId}`}
              data-definition={instance.definitionId}
              data-pane={instance.paneId}
              data-visible={instance.visible}
              className="indicator-legend-row"
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: 6,
                fontSize: '11px',
                color: instance.visible ? '#b2b5be' : '#5d606b',
                background: 'rgba(19, 23, 34, 0.65)',
                backdropFilter: 'blur(6px)',
                padding: '2px 7px',
                borderRadius: '4px',
                width: 'fit-content',
                maxWidth: '100%',
                border: '1px solid rgba(255,255,255,0.04)',
                userSelect: 'none',
              }}
            >
              {/* Color dot */}
              <span
                style={{
                  width: 7,
                  height: 7,
                  borderRadius: '50%',
                  background: mainColor,
                  opacity: instance.visible ? 1 : 0.4,
                  flexShrink: 0,
                }}
              />

              {/* Title & Params */}
              <span
                data-testid={`indicator-card-${instance.definitionId}`}
                style={{ fontWeight: 600, color: instance.visible ? '#d1d4dc' : '#787b86' }}
              >
                {instance.label}
                {paramsStr && (
                  <span style={{ color: '#787b86', fontSize: '10px', marginLeft: 4 }}>
                    ({paramsStr})
                  </span>
                )}
              </span>

              {/* Real-time Values */}
              <div
                data-testid={`indicator-values-${instance.id}`}
                style={{
                  display: 'inline-flex',
                  gap: 5,
                  fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
                  color: mainColor,
                  fontWeight: 500,
                  fontSize: '11px',
                  marginLeft: 4,
                }}
              >
                {Object.keys(state.values).length > 0 ? (
                  Object.entries(state.values).map(([name, val]) => (
                    <span key={name}>
                      {name !== 'value' && `${name}: `}{formatVal(val)}
                    </span>
                  ))
                ) : (
                  <span style={{ color: state.status === 'error' ? '#f23645' : '#787b86', fontSize: '10px' }}>
                    {state.error ?? (instance.visible ? 'warming' : 'hidden')}
                  </span>
                )}
              </div>

              {/* Action Buttons (Hover Controls) */}
              <div
                className="legend-actions"
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 2,
                  marginLeft: 4,
                }}
              >
                <button
                  type="button"
                  data-testid={`toggle-indicator-${instance.id}`}
                  title={instance.visible ? 'Ẩn chỉ báo' : 'Hiện chỉ báo'}
                  aria-label={`${instance.visible ? 'Hide' : 'Show'} ${instance.label}`}
                  onClick={() => onToggle(instance.id)}
                  style={{
                    background: 'transparent',
                    border: 'none',
                    padding: 2,
                    cursor: 'pointer',
                    color: '#787b86',
                    display: 'flex',
                    alignItems: 'center',
                  }}
                >
                  {instance.visible ? <Eye size={12} /> : <EyeOff size={12} />}
                </button>

                <button
                  type="button"
                  data-testid={`indicator-settings-${instance.id}`}
                  title="Cài đặt thông số"
                  aria-label={`Settings for ${instance.label}`}
                  onClick={() => onSettings(instance)}
                  style={{
                    background: 'transparent',
                    border: 'none',
                    padding: 2,
                    cursor: 'pointer',
                    color: '#787b86',
                    display: 'flex',
                    alignItems: 'center',
                  }}
                >
                  <Settings size={12} />
                </button>

                <button
                  type="button"
                  data-testid={`remove-indicator-${instance.id}`}
                  title="Xóa chỉ báo"
                  aria-label={`Remove ${instance.label}`}
                  onClick={() => onRemove(instance.id)}
                  style={{
                    background: 'transparent',
                    border: 'none',
                    padding: 2,
                    cursor: 'pointer',
                    color: '#787b86',
                    display: 'flex',
                    alignItems: 'center',
                  }}
                >
                  <X size={12} />
                </button>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
