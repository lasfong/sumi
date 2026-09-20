import React, { useEffect, useMemo } from 'react';
import { Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { getSessionAnalytics } from '../../api/analyticsApi';
import type { PracticeWorkflowSnapshot } from '../../types';
import { formatVietnamesePercent, formatVnd } from '../../utils/formatters';

interface SessionDebriefModalProps {
  isOpen: boolean;
  onClose: () => void;
  sessionId: number;
  symbolName: string;
  practiceData?: PracticeWorkflowSnapshot;
}

export const SessionDebriefModal: React.FC<SessionDebriefModalProps> = ({
  isOpen,
  onClose,
  sessionId,
  symbolName,
  practiceData,
}) => {
  useEffect(() => {
    if (!isOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  const { data: analytics } = useQuery({
    queryKey: ['analytics', sessionId],
    queryFn: () => getSessionAnalytics(sessionId),
    enabled: isOpen && Boolean(sessionId),
  });

  // Local fallback calculations from practiceData if analytics query is pending
  const localStats = useMemo(() => {
    if (!practiceData) return null;
    const trades = practiceData.trades || [];
    const closedTrades = trades.filter(t => t.status === 'closed');
    const winningTrades = closedTrades.filter(t => (t.net_pnl ?? 0) > 0);
    const winRate = closedTrades.length > 0 ? winningTrades.length / closedTrades.length : 0;
    const totalPnl = closedTrades.reduce((sum, t) => sum + (t.net_pnl ?? 0), 0);
    const grossWin = winningTrades.reduce((sum, t) => sum + (t.net_pnl ?? 0), 0);
    const grossLoss = Math.abs(closedTrades.filter(t => (t.net_pnl ?? 0) < 0).reduce((sum, t) => sum + (t.net_pnl ?? 0), 0));
    const profitFactor = grossLoss > 0 ? grossWin / grossLoss : grossWin > 0 ? 99 : 0;

    // Discipline metric: % of trades with Stop Loss defined
    const tradesWithSl = trades.filter(t => t.initial_stop_loss != null && t.initial_stop_loss > 0).length;
    const slAdherenceRate = trades.length > 0 ? tradesWithSl / trades.length : 0;

    // Mistakes count from decisions
    const mistakesMap = new Map<string, number>();
    (practiceData.decisions || []).forEach(d => {
      if (d.mistake_tag && d.mistake_tag !== 'None') {
        mistakesMap.set(d.mistake_tag, (mistakesMap.get(d.mistake_tag) || 0) + 1);
      }
    });

    return {
      totalTrades: trades.length,
      closedTradesCount: closedTrades.length,
      winningTradesCount: winningTrades.length,
      losingTradesCount: closedTrades.length - winningTrades.length,
      winRate,
      totalPnl,
      profitFactor,
      slAdherenceRate,
      mistakes: Array.from(mistakesMap.entries()).map(([tag, count]) => ({ tag, count })),
    };
  }, [practiceData]);

  if (!isOpen) return null;

  const totalPnl = analytics?.total_net_pnl ?? localStats?.totalPnl ?? 0;
  const isProfitable = totalPnl >= 0;
  const winRate = analytics?.metrics?.win_rate?.value ?? localStats?.winRate ?? 0;
  const profitFactor = analytics?.metrics?.profit_factor?.value ?? localStats?.profitFactor ?? 0;
  const totalTrades = analytics?.total_trades ?? localStats?.totalTrades ?? 0;
  const closedTrades = localStats?.closedTradesCount ?? analytics?.total_trades ?? 0;
  const maxDrawdownPct = analytics?.max_drawdown_pct != null ? analytics.max_drawdown_pct / 100 : 0;
  const slAdherence = localStats?.slAdherenceRate ?? 1.0;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Tổng kết phiên luyện tập"
      data-testid="session-debrief-dialog"
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 2000,
        background: 'rgba(0, 0, 0, 0.75)',
        display: 'grid',
        placeItems: 'center',
        padding: 16,
        backdropFilter: 'blur(4px)',
      }}
      onClick={e => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className="glass-panel-solid"
        style={{
          width: 'min(720px, 95vw)',
          maxHeight: '90vh',
          overflowY: 'auto',
          padding: 24,
          background: 'rgba(19, 23, 34, 0.98)',
          border: '1px solid rgba(41, 98, 255, 0.4)',
          borderRadius: 12,
          boxShadow: '0 16px 48px rgba(0, 0, 0, 0.8)',
          display: 'grid',
          gap: 20,
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <span style={{ fontSize: 24 }}>📊</span>
            <div>
              <h2 style={{ margin: 0, fontSize: 17, fontWeight: 600 }}>
                Tổng kết phiên Replay · {symbolName}
              </h2>
              <span style={{ fontSize: 12, color: 'var(--text-muted)' }}>
                Session #{sessionId} · Đánh giá hiệu quả chiến lược và mức độ tuân thủ kỷ luật
              </span>
            </div>
          </div>
          <button
            onClick={onClose}
            style={{
              background: 'transparent',
              border: 'none',
              color: 'var(--text-muted)',
              fontSize: 20,
              cursor: 'pointer',
              padding: '4px 8px',
            }}
          >
            ✕
          </button>
        </div>

        {/* Primary Metrics 4-Box Grid */}
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(140px, 1fr))', gap: 12 }}>
          <div style={{ background: 'rgba(255, 255, 255, 0.03)', padding: 12, borderRadius: 8, border: '1px solid var(--border-color)' }}>
            <span style={{ fontSize: 11, color: 'var(--text-muted)', textTransform: 'uppercase' }}>Tổng P&amp;L</span>
            <div style={{ fontSize: 18, fontWeight: 700, color: isProfitable ? 'var(--color-buy)' : 'var(--color-sell)', marginTop: 4 }}>
              {isProfitable ? '+' : ''}{formatVnd(totalPnl)}
            </div>
          </div>

          <div style={{ background: 'rgba(255, 255, 255, 0.03)', padding: 12, borderRadius: 8, border: '1px solid var(--border-color)' }}>
            <span style={{ fontSize: 11, color: 'var(--text-muted)', textTransform: 'uppercase' }}>Win Rate (Tỷ lệ thắng)</span>
            <div style={{ fontSize: 18, fontWeight: 700, color: winRate >= 0.5 ? 'var(--color-buy)' : 'var(--text-main)', marginTop: 4 }}>
              {formatVietnamesePercent(winRate)}
            </div>
            <span style={{ fontSize: 10, color: 'var(--text-muted)' }}>
              {localStats?.winningTradesCount ?? 0} thắng / {localStats?.losingTradesCount ?? 0} thua
            </span>
          </div>

          <div style={{ background: 'rgba(255, 255, 255, 0.03)', padding: 12, borderRadius: 8, border: '1px solid var(--border-color)' }}>
            <span style={{ fontSize: 11, color: 'var(--text-muted)', textTransform: 'uppercase' }}>Profit Factor</span>
            <div style={{ fontSize: 18, fontWeight: 700, color: profitFactor >= 1.5 ? 'var(--color-buy)' : profitFactor >= 1.0 ? '#FFD166' : 'var(--color-sell)', marginTop: 4 }}>
              {profitFactor.toFixed(2)}
            </div>
            <span style={{ fontSize: 10, color: 'var(--text-muted)' }}>{closedTrades} đóng / {totalTrades} lệnh</span>
          </div>

          <div style={{ background: 'rgba(255, 255, 255, 0.03)', padding: 12, borderRadius: 8, border: '1px solid var(--border-color)' }}>
            <span style={{ fontSize: 11, color: 'var(--text-muted)', textTransform: 'uppercase' }}>Max Drawdown</span>
            <div style={{ fontSize: 18, fontWeight: 700, color: 'var(--color-sell)', marginTop: 4 }}>
              {formatVietnamesePercent(maxDrawdownPct)}
            </div>
          </div>
        </div>

        {/* Discipline & Psychology Section */}
        <div style={{ display: 'grid', gap: 12, background: 'rgba(41, 98, 255, 0.05)', padding: 14, borderRadius: 8, border: '1px solid rgba(41, 98, 255, 0.2)' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <span style={{ fontWeight: 600, fontSize: 13, color: '#58a6ff' }}>
              🛡️ Kỷ luật giao dịch &amp; Quản trị rủi ro
            </span>
            <span style={{ fontSize: 12, color: 'var(--text-muted)' }}>
              Tuân thủ Stop Loss:{' '}
              <strong style={{ color: slAdherence >= 0.8 ? 'var(--color-buy)' : '#FFD166' }}>
                {formatVietnamesePercent(slAdherence)}
              </strong>
            </span>
          </div>

          {/* Mistakes Tag Breakdown */}
          {localStats?.mistakes && localStats.mistakes.length > 0 ? (
            <div style={{ display: 'grid', gap: 6 }}>
              <span style={{ fontSize: 11, color: 'var(--text-muted)' }}>Các lỗi tâm lý đã ghi nhận trong phiên:</span>
              <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6 }}>
                {localStats.mistakes.map(m => (
                  <span
                    key={m.tag}
                    style={{
                      padding: '3px 8px',
                      background: 'rgba(255, 23, 68, 0.15)',
                      color: '#FF8A80',
                      border: '1px solid rgba(255, 23, 68, 0.3)',
                      borderRadius: 4,
                      fontSize: 11,
                      fontWeight: 500,
                    }}
                  >
                    ⚠️ {m.tag} ({m.count} lần)
                  </span>
                ))}
              </div>
            </div>
          ) : (
            <div style={{ fontSize: 12, color: '#00E676' }}>
              ✓ Xuất sắc! Chưa phát hiện vi phạm kỷ luật hoặc lỗi tâm lý trong phiên này.
            </div>
          )}
        </div>

        {/* Setup Distribution */}
        {analytics?.setup_performance && analytics.setup_performance.length > 0 && (
          <div style={{ display: 'grid', gap: 8 }}>
            <span style={{ fontWeight: 600, fontSize: 13, color: 'var(--text-main)' }}>
              🎯 Hiệu suất theo mẫu hình (Setup)
            </span>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: 8 }}>
              {analytics.setup_performance.map(setup => (
                <div
                  key={setup.setup_type}
                  style={{
                    padding: '8px 12px',
                    background: 'rgba(255, 255, 255, 0.02)',
                    border: '1px solid rgba(255, 255, 255, 0.06)',
                    borderRadius: 6,
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'center',
                    fontSize: 12,
                  }}
                >
                  <span style={{ fontWeight: 500 }}>{setup.setup_type}</span>
                  <div style={{ textAlign: 'right' }}>
                    <div style={{ color: setup.net_pnl >= 0 ? 'var(--color-buy)' : 'var(--color-sell)', fontWeight: 600 }}>
                      {formatVnd(setup.net_pnl)}
                    </div>
                    <div style={{ fontSize: 10, color: 'var(--text-muted)' }}>
                      {setup.trades} lệnh · Win {(setup.win_rate * 100).toFixed(0)}%
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Action Buttons */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', paddingTop: 8, borderTop: '1px solid var(--border-color)' }}>
          <div style={{ display: 'flex', gap: 8 }}>
            <Link
              to={`/analytics?session=${sessionId}`}
              style={{
                padding: '6px 12px',
                background: 'rgba(255, 255, 255, 0.06)',
                color: 'var(--text-main)',
                borderRadius: 6,
                fontSize: 12,
                textDecoration: 'none',
                border: '1px solid var(--border-color)',
              }}
            >
              📈 Mở trang Analytics chi tiết
            </Link>
            <Link
              to={`/journal?session=${sessionId}`}
              style={{
                padding: '6px 12px',
                background: 'rgba(255, 255, 255, 0.06)',
                color: 'var(--text-main)',
                borderRadius: 6,
                fontSize: 12,
                textDecoration: 'none',
                border: '1px solid var(--border-color)',
              }}
            >
              📖 Xem Nhật ký giao dịch
            </Link>
          </div>

          <button className="btn-primary" onClick={onClose} style={{ padding: '6px 16px' }}>
            Tiếp tục Replay (Esc)
          </button>
        </div>
      </div>
    </div>
  );
};
