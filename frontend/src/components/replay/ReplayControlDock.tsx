import React from 'react';
import { Play, Pause, ChevronLeft, ChevronRight, ChevronsLeft, ChevronsRight, RotateCcw } from 'lucide-react';

interface ReplayControlDockProps {
  isPlaying: boolean;
  playSpeed: number;
  onTogglePlay: () => void;
  onChangeSpeed: (speed: number) => void;
  onStepPrev: (steps: number) => void;
  onStepNext: (steps: number) => void;
  onNewSession?: () => void;
  navigationPending?: boolean;
  currentBar?: number;
  totalBars?: number;
}

export const ReplayControlDock: React.FC<ReplayControlDockProps> = ({
  isPlaying,
  playSpeed,
  onTogglePlay,
  onChangeSpeed,
  onStepPrev,
  onStepNext,
  onNewSession,
  navigationPending = false,
  currentBar = 0,
  totalBars = 0,
}) => {
  const progressPercent = totalBars > 0 ? Math.min(100, (currentBar / totalBars) * 100) : 0;

  return (
    <div
      className="replay-control-dock"
      style={{
        position: 'absolute',
        bottom: 24,
        left: '50%',
        transform: 'translateX(-50%)',
        zIndex: 10,
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        background: 'rgba(19, 23, 34, 0.90)',
        backdropFilter: 'blur(16px)',
        WebkitBackdropFilter: 'blur(16px)',
        border: '1px solid rgba(255, 255, 255, 0.12)',
        borderRadius: '32px',
        padding: '6px 14px',
        boxShadow: '0 8px 32px rgba(0, 0, 0, 0.55), 0 0 0 1px rgba(255,255,255,0.05)',
        userSelect: 'none',
      }}
    >
      {/* 1. Bar Progress / Reset */}
      {onNewSession && (
        <button
          type="button"
          onClick={onNewSession}
          title="Tạo phiên Replay mới (New Session)"
          style={{
            background: 'rgba(255, 255, 255, 0.05)',
            border: 'none',
            color: '#787b86',
            borderRadius: '50%',
            width: 28,
            height: 28,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            cursor: 'pointer',
            transition: 'all 0.15s ease',
          }}
          onMouseEnter={(e) => (e.currentTarget.style.color = '#f0f3fa')}
          onMouseLeave={(e) => (e.currentTarget.style.color = '#787b86')}
        >
          <RotateCcw size={14} />
        </button>
      )}

      {/* Mini Progress Indicator */}
      <div
        data-testid="replay-dock-bar-context"
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: 2,
          paddingRight: 6,
          borderRight: '1px solid rgba(255, 255, 255, 0.1)',
        }}
      >
        <span
          style={{
            fontSize: '11px',
            fontFamily: 'ui-monospace, monospace',
            color: '#b2b5be',
            whiteSpace: 'nowrap',
          }}
        >
          #{currentBar} <span style={{ color: '#5d606b' }}>/ {totalBars || '…'}</span>
        </span>
        <div
          style={{
            width: 48,
            height: 3,
            background: 'rgba(255,255,255,0.1)',
            borderRadius: 2,
            overflow: 'hidden',
          }}
        >
          <div
            style={{
              width: `${progressPercent}%`,
              height: '100%',
              background: '#2962ff',
              transition: 'width 0.2s linear',
            }}
          />
        </div>
      </div>

      {/* 2. Step Backward (-5, -1) */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 2 }}>
        <button
          type="button"
          aria-label="-5"
          onClick={() => onStepPrev(5)}
          disabled={navigationPending || isPlaying}
          title="Lùi 5 nến (-5)"
          style={{
            background: 'transparent',
            border: 'none',
            color: navigationPending || isPlaying ? '#434651' : '#b2b5be',
            cursor: navigationPending || isPlaying ? 'not-allowed' : 'pointer',
            padding: '4px 6px',
            borderRadius: 4,
            display: 'flex',
            alignItems: 'center',
            gap: 2,
            fontSize: '11px',
            fontFamily: 'monospace',
            fontWeight: 600,
          }}
        >
          <ChevronsLeft size={14} />
          <span>-5</span>
        </button>

        <button
          type="button"
          aria-label="← Prev"
          onClick={() => onStepPrev(1)}
          disabled={navigationPending || isPlaying}
          title="Lùi 1 nến (← Prev)"
          style={{
            background: 'transparent',
            border: 'none',
            color: navigationPending || isPlaying ? '#434651' : '#b2b5be',
            cursor: navigationPending || isPlaying ? 'not-allowed' : 'pointer',
            padding: '4px 8px',
            borderRadius: 4,
            display: 'flex',
            alignItems: 'center',
            fontSize: '12px',
          }}
        >
          <ChevronLeft size={16} />
          <span style={{ fontSize: '11px', fontWeight: 500 }}>← Prev</span>
        </button>
      </div>

      {/* 3. Central PLAY / PAUSE Button */}
      <button
        type="button"
        aria-label={isPlaying ? 'Pause' : 'Auto-Play'}
        onClick={onTogglePlay}
        title={isPlaying ? 'Dừng phát (Pause - Spacebar)' : 'Tự động chạy (Auto-Play - Spacebar)'}
        style={{
          background: isPlaying
            ? 'linear-gradient(135deg, #f23645, #d32f2f)'
            : 'linear-gradient(135deg, #089981, #00796b)',
          color: '#ffffff',
          border: 'none',
          borderRadius: '24px',
          padding: '6px 16px',
          fontSize: '12px',
          fontWeight: 700,
          letterSpacing: '0.3px',
          display: 'flex',
          alignItems: 'center',
          gap: 6,
          cursor: 'pointer',
          boxShadow: isPlaying
            ? '0 0 16px rgba(242, 54, 69, 0.45)'
            : '0 0 16px rgba(8, 153, 129, 0.45)',
          transition: 'all 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
        }}
      >
        {isPlaying ? (
          <>
            <Pause size={14} fill="currentColor" />
            <span>Pause</span>
          </>
        ) : (
          <>
            <Play size={14} fill="currentColor" />
            <span>Auto-Play</span>
          </>
        )}
      </button>

      {/* 4. Step Forward (+1, +5) */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 2 }}>
        <button
          type="button"
          aria-label="Next →"
          onClick={() => onStepNext(1)}
          disabled={navigationPending || isPlaying}
          title="Tiến 1 nến (Next →)"
          style={{
            background: 'transparent',
            border: 'none',
            color: navigationPending || isPlaying ? '#434651' : '#b2b5be',
            cursor: navigationPending || isPlaying ? 'not-allowed' : 'pointer',
            padding: '4px 8px',
            borderRadius: 4,
            display: 'flex',
            alignItems: 'center',
            fontSize: '12px',
          }}
        >
          <span style={{ fontSize: '11px', fontWeight: 500 }}>Next →</span>
          <ChevronRight size={16} />
        </button>

        <button
          type="button"
          aria-label="+5"
          onClick={() => onStepNext(5)}
          disabled={navigationPending || isPlaying}
          title="Tiến 5 nến (+5)"
          style={{
            background: 'transparent',
            border: 'none',
            color: navigationPending || isPlaying ? '#434651' : '#b2b5be',
            cursor: navigationPending || isPlaying ? 'not-allowed' : 'pointer',
            padding: '4px 6px',
            borderRadius: 4,
            display: 'flex',
            alignItems: 'center',
            gap: 2,
            fontSize: '11px',
            fontFamily: 'monospace',
            fontWeight: 600,
          }}
        >
          <span>+5</span>
          <ChevronsRight size={14} />
        </button>
      </div>

      {/* 5. Speed Selector */}
      <div style={{ paddingLeft: 6, borderLeft: '1px solid rgba(255, 255, 255, 0.1)' }}>
        <select
          aria-label="Tốc độ phát"
          value={playSpeed}
          onChange={(e) => onChangeSpeed(Number(e.target.value))}
          style={{
            background: 'rgba(255, 255, 255, 0.06)',
            color: '#d1d4dc',
            border: '1px solid rgba(255, 255, 255, 0.1)',
            borderRadius: '12px',
            fontSize: '11px',
            fontWeight: 600,
            padding: '3px 8px',
            outline: 'none',
            cursor: 'pointer',
          }}
        >
          <option value={2000} style={{ background: '#1e222d' }}>0.5x</option>
          <option value={1000} style={{ background: '#1e222d' }}>1x</option>
          <option value={500} style={{ background: '#1e222d' }}>2x</option>
          <option value={200} style={{ background: '#1e222d' }}>5x</option>
          <option value={100} style={{ background: '#1e222d' }}>10x</option>
        </select>
      </div>
    </div>
  );
};
