import React from 'react';
import { Search } from 'lucide-react';
import type { Candle, PracticeWorkflowSnapshot, ReplaySession, ReplaySourceContext } from '../../types';
import type { WebSocketStatus } from '../../hooks/useWebSocket';
import { SessionPicker } from '../common/SessionPicker';
import { ScannerSourceContext } from './ScannerSourceContext';
import { formatVietnameseDate, formatVietnameseNumber, formatVietnameseVolume } from '../../utils/formatters';

export interface ReplayHeaderProps {
  sessionId: number;
  symbolName: string;
  sessionStatus?: string;
  sessionData?: ReplaySession;
  sourceContext?: ReplaySourceContext;
  currentCandle: Candle | null;
  candleCount: number;
  practiceData?: PracticeWorkflowSnapshot;
  targetTimeframe?: string;
  onChangeTimeframe: (timeframe: string | undefined) => void;
  onResumeSession: (sessionId: number) => void;
  onOpenSymbolSwitcher: () => void;
  onOpenDebrief: () => void;
  onOpenShortcuts: () => void;
  playSpeed: number;
  onChangeSpeed: (speed: number) => void;
  isRightPanelOpen: boolean;
  onToggleRightPanel: () => void;
  onClearSession: () => void;
  wsStatus?: WebSocketStatus;
  wsReconnectAttempt?: number;
}

export const ReplayHeader: React.FC<ReplayHeaderProps> = ({
  sessionId,
  symbolName,
  sessionStatus,
  sessionData,
  sourceContext,
  currentCandle,
  candleCount,
  practiceData,
  targetTimeframe,
  onChangeTimeframe,
  onResumeSession,
  onOpenSymbolSwitcher,
  onOpenDebrief,
  onOpenShortcuts,
  playSpeed,
  onChangeSpeed,
  isRightPanelOpen,
  onToggleRightPanel,
  onClearSession,
  wsStatus,
  wsReconnectAttempt = 0,
}) => {
  const isCompleted = String(sessionStatus || '').toLowerCase() === 'completed';

  return (
    <header
      className="replay-header"
      style={{
        height: '42px',
        minHeight: '42px',
        padding: '0 12px',
        background: 'var(--bg-header)',
        backdropFilter: 'var(--backdrop-blur)',
        display: 'flex',
        flexWrap: 'nowrap',
        gap: '12px',
        justifyContent: 'space-between',
        alignItems: 'center',
        borderBottom: '1px solid var(--border-color)',
        boxShadow: '0 2px 10px rgba(0,0,0,0.4)',
        zIndex: 10,
        overflowX: 'auto',
        overflowY: 'hidden',
        whiteSpace: 'nowrap',
      }}
    >
      <div
        style={{
          display: 'flex',
          flexWrap: 'nowrap',
          alignItems: 'center',
          gap: '8px',
          flex: '1 1 auto',
          minWidth: 0,
          overflow: 'hidden',
          whiteSpace: 'nowrap',
        }}
      >
        <h2 style={{ margin: 0, fontSize: '15px', fontWeight: 600, whiteSpace: 'nowrap' }}>Sumi Replay</h2>
        <button
          type="button"
          data-testid="symbol-switcher-button"
          title="Đổi mã cổ phiếu (Phím tắt: /)"
          onClick={onOpenSymbolSwitcher}
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: 5,
            padding: '2px 8px',
            background: 'rgba(41, 98, 255, 0.15)',
            color: '#58A6FF',
            borderRadius: '4px',
            fontWeight: 700,
            fontSize: '13px',
            border: '1px solid rgba(41, 98, 255, 0.4)',
            cursor: 'pointer',
            transition: 'all 0.15s ease',
          }}
        >
          <Search size={12} />
          <span>{symbolName}</span>
          <span style={{ fontSize: '10px', opacity: 0.7 }}>▾</span>
        </button>
        {sessionData && (
          <div style={{ display: 'flex', gap: '5px', fontSize: '12px', color: 'var(--text-muted)', alignItems: 'center' }}>
            <select
              value={targetTimeframe ?? sessionData.timeframe}
              onChange={(e) => onChangeTimeframe(e.target.value === sessionData.timeframe ? undefined : e.target.value)}
              style={{
                background: 'rgba(255,255,255,0.05)',
                padding: '2px 6px',
                borderRadius: '4px',
                border: '1px solid rgba(255,255,255,0.1)',
                color: 'var(--text-main)',
                outline: 'none',
                cursor: 'pointer',
                fontSize: '12px',
              }}
            >
              {['5m', '15m', '1H', '1D', '1W', '1M'].map((tf) => (
                <option key={tf} value={tf} style={{ background: '#1e222d' }}>
                  {tf}
                </option>
              ))}
            </select>
            <span style={{ background: 'rgba(255,255,255,0.05)', padding: '2px 5px', borderRadius: '4px', textTransform: 'capitalize', fontSize: '11px' }}>
              {sessionData.adjustment_type}
            </span>
            <span style={{ background: 'rgba(255,255,255,0.05)', padding: '2px 5px', borderRadius: '4px', textTransform: 'capitalize', fontSize: '11px' }}>
              {sourceContext?.replay_intent?.replace('_', ' ') || sessionData.mode.replace('_', ' ')}
            </span>
          </div>
        )}
        <SessionPicker
          selectedSessionId={sessionId}
          onSelectSession={onResumeSession}
          compact
        />
        <button
          type="button"
          data-testid="open-add-indicator"
          aria-label="Add indicator"
          onClick={() => window.dispatchEvent(new CustomEvent('sumi:indicator-add-request'))}
          style={{
            padding: '3px 8px',
            background: 'rgba(41, 98, 255, 0.1)',
            color: '#58a6ff',
            borderRadius: '4px',
            fontSize: '11px',
            fontWeight: 500,
            border: '1px solid rgba(41, 98, 255, 0.3)',
            display: 'inline-flex',
            alignItems: 'center',
            gap: '4px',
            cursor: 'pointer',
          }}
        >
          <span style={{ fontWeight: 700, fontSize: '12px', fontStyle: 'italic', fontFamily: 'serif' }}>fx</span>
          <span>Indicators</span>
        </button>
        <button
          type="button"
          data-testid="open-session-debrief"
          aria-label="Session debrief"
          title="Tổng kết phiên (D)"
          onClick={onOpenDebrief}
          style={{
            padding: '3px 7px',
            background: 'rgba(255,255,255,0.05)',
            color: 'var(--text-main)',
            borderRadius: '4px',
            fontSize: '11px',
            border: '1px solid var(--border-color)',
            cursor: 'pointer',
            display: 'inline-flex',
            alignItems: 'center',
            gap: '4px',
          }}
        >
          <span>📊</span>
          <span>Debrief</span>
        </button>
        <button
          type="button"
          data-testid="open-keyboard-shortcuts"
          aria-label="Keyboard shortcuts"
          title="Phím tắt hệ thống (?)"
          onClick={onOpenShortcuts}
          style={{
            padding: '3px 6px',
            background: 'rgba(255,255,255,0.05)',
            color: 'var(--text-muted)',
            borderRadius: '4px',
            fontSize: '11px',
            border: '1px solid var(--border-color)',
            cursor: 'pointer',
            display: 'inline-flex',
            alignItems: 'center',
          }}
        >
          ⌨️
        </button>
        {sessionStatus && (
          <span
            data-testid="replay-session-status-badge"
            style={{
              padding: '2px 6px',
              background: isCompleted ? 'rgba(255, 209, 102, 0.15)' : 'rgba(255, 255, 255, 0.05)',
              color: isCompleted ? '#FFD166' : 'var(--text-muted)',
              borderRadius: '4px',
              fontSize: '11px',
              textTransform: 'uppercase',
              border: `1px solid ${isCompleted ? 'rgba(255, 209, 102, 0.4)' : 'var(--border-color)'}`,
            }}
          >
            {sessionStatus}
          </span>
        )}
        {(wsStatus === 'OFFLINE' || wsStatus === 'RECONNECTING') && (
          <span
            role="status"
            data-testid="replay-ws-status-badge"
            style={{
              padding: '2px 6px',
              background: 'rgba(255, 123, 114, 0.15)',
              color: '#FF7B72',
              borderRadius: '4px',
              fontSize: '11px',
              border: '1px solid rgba(255, 123, 114, 0.4)',
            }}
          >
            {wsStatus === 'OFFLINE'
              ? 'Offline — waiting for network'
              : `Reconnecting (${wsReconnectAttempt})…`}
          </span>
        )}
        {sourceContext && <ScannerSourceContext context={sourceContext} />}

        {currentCandle && (
          <div
            style={{
              display: 'flex',
              flexWrap: 'nowrap',
              gap: '8px',
              fontSize: '11px',
              fontFamily: 'monospace',
              alignItems: 'center',
            }}
          >
            <span data-testid="replay-bar-context" style={{ color: 'var(--text-muted)' }}>
              Bar:{' '}
              <span style={{ color: 'white' }}>
                #{practiceData?.visible_bar ?? candleCount}/{practiceData?.total_bars ?? '…'}
              </span>
            </span>
            <span data-testid="current-candle-date" style={{ color: 'var(--text-muted)' }}>
              Date: <span style={{ color: 'white' }}>{formatVietnameseDate(currentCandle.timestamp)}</span>{' '}
              <span style={{ fontSize: '10px' }}>(Asia/Ho_Chi_Minh)</span>
            </span>
            <span style={{ color: 'var(--text-muted)' }}>
              O: <span style={{ color: 'white' }}>{formatVietnameseNumber(currentCandle.open, 2)}</span>
            </span>
            <span style={{ color: 'var(--text-muted)' }}>
              H: <span style={{ color: 'white' }}>{formatVietnameseNumber(currentCandle.high, 2)}</span>
            </span>
            <span style={{ color: 'var(--text-muted)' }}>
              L: <span style={{ color: 'white' }}>{formatVietnameseNumber(currentCandle.low, 2)}</span>
            </span>
            <span style={{ color: 'var(--text-muted)' }}>
              C:{' '}
              <span
                style={{
                  color:
                    currentCandle.close >= currentCandle.open
                      ? 'var(--color-buy)'
                      : 'var(--color-sell)',
                  fontWeight: 600,
                }}
              >
                {formatVietnameseNumber(currentCandle.close, 2)}
              </span>
            </span>
            <span style={{ color: 'var(--text-muted)' }}>
              V: <span style={{ color: 'white' }}>{formatVietnameseVolume(currentCandle.volume)}</span>
            </span>
            <span
              data-testid="replay-readiness-badge"
              style={{
                padding: '2px 5px',
                borderRadius: '4px',
                fontSize: '10px',
                background: 'rgba(0, 230, 118, 0.1)',
                color: 'var(--color-buy)',
                border: '1px solid rgba(0, 230, 118, 0.2)',
                fontFamily: 'sans-serif',
              }}
            >
              Local Market Ready
            </span>
          </div>
        )}
      </div>

      <div
        style={{
          display: 'flex',
          flexWrap: 'nowrap',
          gap: '8px',
          alignItems: 'center',
          flexShrink: 0,
          whiteSpace: 'nowrap',
          marginLeft: 'auto',
          background: 'var(--bg-header)',
          paddingLeft: '8px',
          zIndex: 2,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '5px' }}>
          <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>Speed:</span>
          <select
            aria-label="Playback speed"
            value={playSpeed}
            onChange={(e) => onChangeSpeed(Number(e.target.value))}
            style={{
              background: 'var(--bg-panel)',
              color: 'white',
              border: '1px solid var(--border-color)',
              borderRadius: '4px',
              padding: '3px 6px',
              fontSize: '11px',
              cursor: 'pointer',
            }}
          >
            <option value={1000}>1x</option>
            <option value={500}>2x</option>
            <option value={200}>5x</option>
            <option value={100}>10x</option>
          </select>
        </div>

        <div style={{ width: '1px', height: '16px', background: 'var(--border-color)' }}></div>

        <button
          type="button"
          data-testid="toggle-panel-button"
          onClick={onToggleRightPanel}
          style={{
            background: isRightPanelOpen ? 'rgba(41, 98, 255, 0.2)' : 'var(--bg-panel)',
            color: isRightPanelOpen ? '#58A6FF' : 'var(--text-muted)',
            border: `1px solid ${isRightPanelOpen ? 'rgba(41, 98, 255, 0.5)' : 'var(--border-color)'}`,
            borderRadius: '4px',
            fontSize: '11px',
            padding: '3px 8px',
            cursor: 'pointer',
            whiteSpace: 'nowrap',
            display: 'flex',
            alignItems: 'center',
            gap: '4px',
          }}
          title="Bật/tắt bảng điều khiển lệnh & phân tích (Right Panel)"
        >
          ◫ Panel
        </button>

        <div style={{ width: '1px', height: '16px', background: 'var(--border-color)' }}></div>

        <button
          onClick={onClearSession}
          style={{
            background: 'var(--bg-panel)',
            color: 'var(--text-muted)',
            border: '1px solid var(--border-color)',
            borderRadius: '4px',
            fontSize: '11px',
            padding: '3px 8px',
            cursor: 'pointer',
            whiteSpace: 'nowrap',
          }}
        >
          New Session
        </button>
      </div>
    </header>
  );
};
