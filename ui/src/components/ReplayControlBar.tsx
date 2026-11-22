import React, { useEffect } from 'react'
import { Pause, Play, Scissors, SkipBack, SkipForward, X } from 'lucide-react'
import { useWorkspaceStore } from '../store/workspaceStore'

const SPEED_OPTIONS = [0.5, 1, 2, 3, 5, 10]

export const ReplayControlBar: React.FC = () => {
  const replay = useWorkspaceStore((s) => s.replay)
  const replaySelectingCut = useWorkspaceStore((s) => s.replaySelectingCut)
  const replayPlaying = useWorkspaceStore((s) => s.replayPlaying)

  const setReplaySelectingCut = useWorkspaceStore((s) => s.setReplaySelectingCut)
  const stepReplay = useWorkspaceStore((s) => s.stepReplay)
  const setReplaySpeed = useWorkspaceStore((s) => s.setReplaySpeed)
  const setReplayPlaying = useWorkspaceStore((s) => s.setReplayPlaying)
  const stopReplay = useWorkspaceStore((s) => s.stopReplay)

  // Deterministic playback timer driven by ReplaySpeed (0.5x..10x bars/sec)
  useEffect(() => {
    if (!replay?.active || !replayPlaying) return
    const speed = Math.max(0.5, Math.min(10, replay.speedMultiplier || 1))
    const intervalMs = Math.max(100, Math.round(1000 / speed))
    let busy = false
    const timer = window.setInterval(() => {
      if (busy) return
      busy = true
      void stepReplay(1).finally(() => {
        busy = false
      })
    }, intervalMs)
    return () => window.clearInterval(timer)
  }, [replay?.active, replay?.speedMultiplier, replayPlaying, stepReplay])

  if (!replay?.active) return null

  const cutoffLabel = replay.cutoffTimestampMs
    ? new Date(replay.cutoffTimestampMs).toLocaleString('vi-VN', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
      })
    : '—'

  return (
    <div
      className="flex items-center justify-between gap-2 px-3 py-1 shrink-0"
      style={{
        backgroundColor: 'color-mix(in srgb, var(--tv-accent) 14%, var(--tv-bg-panel))',
        borderBottom: '1px solid var(--tv-accent)',
      }}
    >
      <div className="flex items-center gap-2">
        <span className="px-2 py-0.5 rounded text-[10.5px] font-bold tracking-wide bg-[#2962FF] text-white">
          BAR REPLAY NHÂN QUẢ
        </span>
        <span className="text-[11.5px] font-semibold tabular-nums">
          {replay.ticker} ({replay.interval})
        </span>
        <span className="text-[11px] tabular-nums" style={{ color: 'var(--tv-text-secondary)' }}>
          Nến {replay.cursorIndex + 1} / {replay.totalBars} · {cutoffLabel}
        </span>
      </div>

      <div className="flex items-center gap-1">
        <button
          type="button"
          className="tv-btn text-[11.5px]"
          data-active={replaySelectingCut}
          onClick={() => {
            setReplayPlaying(false)
            setReplaySelectingCut(!replaySelectingCut)
          }}
          title="Chọn cây nến bất kỳ trên biểu đồ để cắt điểm phát lại (Jump to Bar)"
        >
          <Scissors className="w-3.5 h-3.5" />
          <span>Chọn điểm cắt</span>
        </button>

        <div className="tv-divider-v" />

        <button
          type="button"
          className="tv-icon-btn"
          onClick={() => {
            setReplayPlaying(false)
            void stepReplay(-1)
          }}
          title="Lùi 1 nến (Rewind nhân quả — khôi phục sổ lệnh)"
        >
          <SkipBack className="w-4 h-4" />
        </button>

        <button
          type="button"
          className="tv-btn px-3 font-semibold bg-[#2962FF] text-white hover:bg-[#1E53E5]"
          onClick={() => setReplayPlaying(!replayPlaying)}
          disabled={replay.status === 'Completed'}
          title={replayPlaying ? 'Tạm dừng (Space)' : 'Phát tự động (Space)'}
        >
          {replayPlaying ? <Pause className="w-3.5 h-3.5" /> : <Play className="w-3.5 h-3.5" />}
          <span>{replayPlaying ? 'Tạm dừng' : 'Phát'}</span>
        </button>

        <button
          type="button"
          className="tv-icon-btn"
          onClick={() => {
            setReplayPlaying(false)
            void stepReplay(1)
          }}
          disabled={replay.status === 'Completed'}
          title="Tiến +1 nến (Step Forward)"
        >
          <SkipForward className="w-4 h-4" />
        </button>

        <div className="tv-divider-v" />

        <label className="flex items-center gap-1 text-[11px]" style={{ color: 'var(--tv-text-secondary)' }}>
          <span>Tốc độ:</span>
          <select
            aria-label="Tốc độ Bar Replay"
            value={replay.speedMultiplier}
            onChange={(e) => void setReplaySpeed(Number(e.target.value))}
            className="rounded px-1.5 py-0.5 text-[11.5px] tabular-nums outline-none cursor-pointer"
            style={{
              backgroundColor: 'var(--tv-bg-input)',
              color: 'var(--tv-text-primary)',
              border: '1px solid var(--tv-border)',
            }}
          >
            {SPEED_OPTIONS.map((sp) => (
              <option key={sp} value={sp}>
                {sp}x
              </option>
            ))}
          </select>
        </label>

        <div className="tv-divider-v" />

        <button
          type="button"
          className="tv-btn text-[11.5px] text-[#F23645]"
          onClick={() => void stopReplay()}
          title="Thoát Bar Replay và trở về dữ liệu trực tiếp"
        >
          <X className="w-3.5 h-3.5" />
          <span>Thoát</span>
        </button>
      </div>
    </div>
  )
}
