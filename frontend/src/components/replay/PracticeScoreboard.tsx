import React, { useState } from 'react';
import type { PracticeWorkflowSnapshot } from '../../types';
import { resetPracticeSession } from '../../api/replayApi';

interface PracticeScoreboardProps {
  snapshot: PracticeWorkflowSnapshot;
  onResetPractice?: () => void;
}

export const PracticeScoreboard: React.FC<PracticeScoreboardProps> = ({ snapshot, onResetPractice }) => {
  const [isResetting, setIsResetting] = useState(false);

  // Closed trades
  const closedTrades = (snapshot.trades || []).filter(t => t.status === 'closed');
  const totalTrades = closedTrades.length;
  const wins = closedTrades.filter(t => t.result === 'win').length;
  const losses = closedTrades.filter(t => t.result === 'loss').length;
  const winRate = totalTrades > 0 ? (wins / totalTrades) * 100 : 0;

  // R multiples
  const tradesWithR = closedTrades.filter(t => t.r_multiple !== null && t.r_multiple !== undefined);
  const netR = tradesWithR.reduce((sum, t) => sum + (t.r_multiple || 0), 0);
  const avgRR = tradesWithR.length > 0 ? netR / tradesWithR.length : 0;

  // PnL %
  const tradesWithPnl = closedTrades.filter(t => t.pnl_percent !== null && t.pnl_percent !== undefined);
  const avgPnl = tradesWithPnl.length > 0 ? tradesWithPnl.reduce((sum, t) => sum + (t.pnl_percent || 0), 0) / tradesWithPnl.length : 0;

  const handleReset = async () => {
    if (!window.confirm('Bạn có chắc chắn muốn đặt lại kết quả luyện tập (xóa các lệnh đã đặt trên phiên này)?')) {
      return;
    }
    setIsResetting(true);
    try {
      await resetPracticeSession(snapshot.session_id);
      onResetPractice?.();
    } catch {
      // ignore
    } finally {
      setIsResetting(false);
    }
  };

  return (
    <div
      className="panel"
      data-testid="practice-scoreboard"
      style={{
        padding: '12px',
        display: 'grid',
        gap: '10px',
        background: 'rgba(19, 23, 34, 0.8)',
        border: '1px solid rgba(88, 166, 255, 0.2)',
        borderRadius: '8px',
      }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <span style={{ fontSize: '14px' }}>🎯</span>
          <strong style={{ fontSize: '13px', color: 'var(--text-main)' }}>Hiệu Quả Luyện Tập PTKT</strong>
        </div>
        <button
          type="button"
          data-testid="btn-reset-practice"
          onClick={handleReset}
          disabled={isResetting || totalTrades === 0}
          style={{
            padding: '3px 8px',
            fontSize: '11px',
            background: 'rgba(255, 23, 68, 0.1)',
            color: totalTrades > 0 ? 'var(--color-sell)' : 'var(--text-muted)',
            border: '1px solid rgba(255, 23, 68, 0.25)',
            borderRadius: '4px',
            cursor: totalTrades > 0 ? 'pointer' : 'default',
          }}
          title="Xóa chuỗi lệnh cũ để bắt đầu vòng luyện tập mới"
        >
          {isResetting ? 'Đang reset…' : 'Reset Luyện tập'}
        </button>
      </div>

      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(4, 1fr)',
          gap: '6px',
          textAlign: 'center',
        }}
      >
        <div style={{ background: 'rgba(255,255,255,0.03)', padding: '6px 4px', borderRadius: '4px' }}>
          <div style={{ fontSize: '10px', color: 'var(--text-muted)' }}>Tổng lệnh</div>
          <div style={{ fontSize: '14px', fontWeight: 700, marginTop: '2px' }}>{totalTrades}</div>
        </div>

        <div style={{ background: 'rgba(255,255,255,0.03)', padding: '6px 4px', borderRadius: '4px' }}>
          <div style={{ fontSize: '10px', color: 'var(--text-muted)' }}>Win Rate</div>
          <div
            style={{
              fontSize: '14px',
              fontWeight: 700,
              marginTop: '2px',
              color: totalTrades === 0 ? 'var(--text-muted)' : winRate >= 50 ? 'var(--color-buy)' : 'var(--color-sell)',
            }}
          >
            {totalTrades > 0 ? `${winRate.toFixed(1)}%` : '—'}
          </div>
          {totalTrades > 0 && (
            <div style={{ fontSize: '9px', color: 'var(--text-muted)' }}>
              {wins}W · {losses}L
            </div>
          )}
        </div>

        <div style={{ background: 'rgba(255,255,255,0.03)', padding: '6px 4px', borderRadius: '4px' }}>
          <div style={{ fontSize: '10px', color: 'var(--text-muted)' }}>Net R</div>
          <div
            style={{
              fontSize: '14px',
              fontWeight: 700,
              marginTop: '2px',
              color: netR > 0 ? 'var(--color-buy)' : netR < 0 ? 'var(--color-sell)' : 'var(--text-muted)',
            }}
          >
            {totalTrades > 0 ? `${netR >= 0 ? '+' : ''}${netR.toFixed(1)}R` : '—'}
          </div>
          {totalTrades > 0 && (
            <div style={{ fontSize: '9px', color: 'var(--text-muted)' }}>
              Avg {avgRR >= 0 ? '+' : ''}{avgRR.toFixed(2)}R
            </div>
          )}
        </div>

        <div style={{ background: 'rgba(255,255,255,0.03)', padding: '6px 4px', borderRadius: '4px' }}>
          <div style={{ fontSize: '10px', color: 'var(--text-muted)' }}>Hiệu suất TB</div>
          <div
            style={{
              fontSize: '14px',
              fontWeight: 700,
              marginTop: '2px',
              color: avgPnl > 0 ? 'var(--color-buy)' : avgPnl < 0 ? 'var(--color-sell)' : 'var(--text-muted)',
            }}
          >
            {totalTrades > 0 ? `${avgPnl >= 0 ? '+' : ''}${avgPnl.toFixed(1)}%` : '—'}
          </div>
        </div>
      </div>
    </div>
  );
};
