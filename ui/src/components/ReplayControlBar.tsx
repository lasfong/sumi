import React, { useEffect, useRef, useState } from 'react'
import {
  Award,
  Bookmark,
  Calendar,
  Dices,
  Eye,
  EyeOff,
  FolderOpen,
  Pause,
  Play,
  Scissors,
  SkipBack,
  SkipForward,
  Trash2,
  X,
} from 'lucide-react'
import { useWorkspaceStore } from '../store/workspaceStore'

const SPEED_OPTIONS = [0.5, 1, 2, 3, 5, 10, 20]

export const ReplayControlBar: React.FC = () => {
  const replay = useWorkspaceStore((s) => s.replay)
  const replaySelectingCut = useWorkspaceStore((s) => s.replaySelectingCut)
  const replayPlaying = useWorkspaceStore((s) => s.replayPlaying)
  const replayBlindMode = useWorkspaceStore((s) => s.replayBlindMode)
  const replaySessions = useWorkspaceStore((s) => s.replaySessions)

  const setReplaySelectingCut = useWorkspaceStore((s) => s.setReplaySelectingCut)
  const setReplayBlindMode = useWorkspaceStore((s) => s.setReplayBlindMode)
  const startReplay = useWorkspaceStore((s) => s.startReplay)
  const startRandomBlindDrill = useWorkspaceStore((s) => s.startRandomBlindDrill)
  const revealBlindDrill = useWorkspaceStore((s) => s.revealBlindDrill)
  const stepReplay = useWorkspaceStore((s) => s.stepReplay)
  const jumpReplay = useWorkspaceStore((s) => s.jumpReplay)
  const setReplaySpeed = useWorkspaceStore((s) => s.setReplaySpeed)
  const setReplayPlaying = useWorkspaceStore((s) => s.setReplayPlaying)
  const stopReplay = useWorkspaceStore((s) => s.stopReplay)
  const saveCurrentReplaySession = useWorkspaceStore((s) => s.saveCurrentReplaySession)
  const resumeSavedReplaySession = useWorkspaceStore((s) => s.resumeSavedReplaySession)
  const deleteSavedReplaySession = useWorkspaceStore((s) => s.deleteSavedReplaySession)
  const setDebriefModalOpen = useWorkspaceStore((s) => s.setDebriefModalOpen)

  const [sessionsMenuOpen, setSessionsMenuOpen] = useState(false)
  const [sessionNameDraft, setSessionNameDraft] = useState('')
  const [jumpDateVal, setJumpDateVal] = useState('')
  const menuRef = useRef<HTMLDivElement | null>(null)

  useEffect(() => {
    const onPointerDown = (e: PointerEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setSessionsMenuOpen(false)
      }
    }
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setSessionsMenuOpen(false)
    }
    document.addEventListener('pointerdown', onPointerDown)
    document.addEventListener('keydown', onKeyDown)
    return () => {
      document.removeEventListener('pointerdown', onPointerDown)
      document.removeEventListener('keydown', onKeyDown)
    }
  }, [])

  // Deterministic playback timer driven by ReplaySpeed (0.5x..20x bars/sec, fixes `BUG-028`)
  useEffect(() => {
    if (!replay?.active || !replayPlaying) return
    const speed = Math.max(0.5, Math.min(20, replay.speedMultiplier || 1))
    const intervalMs = Math.max(50, Math.round(1000 / speed))
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

  const renderSavedSessionsPopover = (allowSave: boolean) => (
    <div ref={menuRef} className="relative">
      <button
        type="button"
        className="tv-btn text-[11px]"
        data-active={sessionsMenuOpen}
        onClick={() => setSessionsMenuOpen((v) => !v)}
        title="Lưu hoặc mở lại phiên luyện tập Bar Replay (bảo toàn vị trí nến & sổ lệnh giấy)"
      >
        {allowSave ? <Bookmark className="w-3.5 h-3.5" aria-hidden="true" /> : <FolderOpen className="w-3.5 h-3.5" aria-hidden="true" />}
        <span>{allowSave ? `Lưu / Phiên (${replaySessions.length})` : `Phiên đã lưu (${replaySessions.length})`}</span>
      </button>
      {sessionsMenuOpen && (
        <div
          className="absolute right-0 top-[30px] w-80 rounded-md p-2.5 shadow-2xl z-50 space-y-2"
          style={{
            backgroundColor: 'var(--tv-bg-elevated)',
            border: '1px solid var(--tv-border)',
          }}
        >
          <div className="text-[11px] font-semibold uppercase tracking-wide" style={{ color: 'var(--tv-text-secondary)' }}>
            Phiên Bar Replay đã lưu ({replaySessions.length})
          </div>
          {allowSave && (
            <div className="flex gap-1">
              <input
                type="text"
                value={sessionNameDraft}
                onChange={(e) => setSessionNameDraft(e.target.value)}
                placeholder={`${replay?.ticker ?? 'FPT'} (${(replay?.trackedIntervals ?? [replay?.interval ?? '1D']).join('+')})...`}
                className="flex-1 px-2 py-1 rounded text-[11.5px] outline-none"
                style={{
                  backgroundColor: 'var(--tv-bg-input)',
                  border: '1px solid var(--tv-border)',
                  color: 'var(--tv-text-primary)',
                }}
              />
              <button
                type="button"
                className="px-2.5 py-1 rounded text-[11px] font-semibold bg-[#2962FF] text-white border-0 cursor-pointer"
                onClick={() => {
                  const defaultLabel = `${replay?.ticker ?? 'Replay'} (${(replay?.trackedIntervals ?? [replay?.interval ?? '1D']).join('+')})`
                  void saveCurrentReplaySession(sessionNameDraft.trim() || defaultLabel)
                  setSessionNameDraft('')
                }}
              >
                Lưu
              </button>
            </div>
          )}
          <div className="max-h-56 overflow-y-auto tv-scroll space-y-1">
            {replaySessions.length === 0 ? (
              <div className="text-[11px] py-3 text-center" style={{ color: 'var(--tv-text-secondary)' }}>
                Chưa có phiên luyện tập nào được lưu.
              </div>
            ) : (
              replaySessions.map((sess) => {
                const dt = new Date(sess.cursorTimestampMs).toLocaleString('vi-VN', {
                  day: '2-digit',
                  month: '2-digit',
                  year: 'numeric',
                  hour: '2-digit',
                  minute: '2-digit',
                })
                return (
                  <div
                    key={sess.id}
                    className="flex items-center justify-between gap-1.5 px-2 py-1.5 rounded hover:bg-[var(--tv-hover)]"
                  >
                    <button
                      type="button"
                      className="flex-1 min-w-0 text-left border-0 bg-transparent p-0 cursor-pointer"
                      onClick={() => {
                        void resumeSavedReplaySession(sess.id)
                        setSessionsMenuOpen(false)
                      }}
                    >
                      <div className="text-[11.5px] font-semibold truncate text-[var(--tv-text-primary)]">
                        {sess.name}
                      </div>
                      <div className="text-[10px] truncate tabular-nums" style={{ color: 'var(--tv-text-secondary)' }}>
                        {sess.ticker} · {sess.trackedIntervals.join(' + ')} · {dt}
                      </div>
                    </button>
                    <button
                      type="button"
                      aria-label={`Xóa phiên ${sess.name}`}
                      className="border-0 bg-transparent p-1 cursor-pointer text-[var(--tv-text-secondary)] hover:text-[#F23645]"
                      onClick={() => void deleteSavedReplaySession(sess.id)}
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                )
              })
            )}
          </div>
        </div>
      )}
    </div>
  )

  // Pre-cut interactive scissor selection strip (TradingView flow when clicking "Phát lại")
  if (!replay?.active && replaySelectingCut) {
    return (
      <div
        className="flex items-center justify-between gap-2 px-3 py-1 shrink-0"
        style={{
          backgroundColor: 'color-mix(in srgb, var(--tv-accent) 16%, var(--tv-bg-panel))',
          borderBottom: '1px solid var(--tv-accent)',
        }}
      >
        <div className="flex items-center gap-2 text-[11.5px]">
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[10.5px] font-bold tracking-wide bg-[#2962FF] text-white">
            <Scissors className="w-3 h-3" />
            CHỌN ĐIỂM CẮT REPLAY
          </span>
          <span>Di chuyển đường rèm màu xanh trên biểu đồ và nhấp chuột vào vị trí quá khứ muốn cắt nến.</span>
        </div>
        <div className="flex items-center gap-1.5">
          {renderSavedSessionsPopover(false)}
          <button
            type="button"
            className="tv-btn text-[11.5px] font-semibold px-2.5"
            style={{ backgroundColor: 'rgba(251, 192, 45, 0.2)', color: '#FBC02D', border: '1px solid #FBC02D' }}
            onClick={() => void startRandomBlindDrill()}
            title="Chọn ngẫu nhiên 1 mã QMV60 lịch sử, chuẩn hóa giá về 100.00 và ẩn danh tính + ngày tháng!"
          >
            <Dices className="w-3.5 h-3.5" />
            <span>Luyện Mù Ngẫu Nhiên</span>
          </button>
          <button
            type="button"
            className="tv-btn text-[11.5px] bg-[#2962FF] text-white hover:bg-[#1E53E5] px-2.5"
            onClick={() => void startReplay(null)}
            title="Cắt nhanh lùi 60 cây nến từ hiện tại"
          >
            Cắt nhanh (-60 nến)
          </button>
          <button
            type="button"
            className="tv-btn text-[11.5px] text-[#F23645]"
            onClick={() => setReplaySelectingCut(false)}
          >
            <X className="w-3.5 h-3.5" />
            <span>Hủy (Esc)</span>
          </button>
        </div>
      </div>
    )
  }

  if (!replay?.active) return null

  const isBlindDrillUnrevealed = Boolean(replay.blindDrill?.enabled && !replay.blindDrill.revealed)
  const isBlindDrillRevealed = Boolean(replay.blindDrill?.enabled && replay.blindDrill.revealed)

  const cutoffLabel = isBlindDrillUnrevealed
    ? `Phiên #${replay.cursorIndex + 1} (Giá chuẩn hóa 100.00)`
    : replay.cutoffTimestampMs
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
        backgroundColor: isBlindDrillUnrevealed
          ? 'color-mix(in srgb, #FBC02D 14%, var(--tv-bg-panel))'
          : 'color-mix(in srgb, var(--tv-accent) 14%, var(--tv-bg-panel))',
        borderBottom: `1px solid ${isBlindDrillUnrevealed ? '#FBC02D' : 'var(--tv-accent)'}`,
      }}
    >
      <div className="flex items-center gap-2 min-w-0">
        <span
          className="px-2 py-0.5 rounded text-[10.5px] font-bold tracking-wide shrink-0"
          style={{
            backgroundColor: isBlindDrillUnrevealed ? '#FBC02D' : '#2962FF',
            color: isBlindDrillUnrevealed ? '#111827' : '#FFFFFF',
          }}
        >
          {isBlindDrillUnrevealed ? 'LUYỆN MÙ NGẪU NHIÊN' : 'BAR REPLAY NHÂN QUẢ'}
        </span>
        <span className="text-[11.5px] font-semibold tabular-nums truncate">
          {replay.ticker} ({(replay.trackedIntervals && replay.trackedIntervals.length > 0 ? replay.trackedIntervals : [replay.interval]).join(' + ')})
        </span>
        <span className="text-[11px] tabular-nums hidden md:inline" style={{ color: 'var(--tv-text-secondary)' }}>
          Nến {replay.cursorIndex + 1} / {replay.totalBars} · {cutoffLabel}
        </span>
        {isBlindDrillRevealed && replay.blindDrill?.cutDate && (
          <span className="px-1.5 py-0.5 rounded text-[10px] font-semibold bg-[#089981]/20 text-[#089981] shrink-0">
            Mã thật: {replay.blindDrill.realTicker} ({replay.blindDrill.cutDate})
          </span>
        )}
      </div>

      <div className="flex items-center gap-1 shrink-0">
        {isBlindDrillUnrevealed ? (
          <button
            type="button"
            className="tv-btn text-[11px] font-semibold px-2"
            style={{ backgroundColor: '#089981', color: '#FFFFFF' }}
            onClick={() => void revealBlindDrill()}
            title="Kết thúc phần ẩn danh: lật mở mã cổ phiếu thật, ngày lịch sử và chấm điểm Debrief"
          >
            <Eye className="w-3.5 h-3.5" aria-hidden="true" />
            <span>Lật Mở Mã Thật</span>
          </button>
        ) : (
          <>
            <button
              type="button"
              className="tv-btn text-[11px]"
              style={{ color: '#FBC02D' }}
              onClick={() => void startRandomBlindDrill()}
              title="Bắt đầu ván Luyện Mù Ngẫu Nhiên mới (ẩn tên mã + chuẩn hóa 100.00)"
            >
              <Dices className="w-3.5 h-3.5" aria-hidden="true" />
              <span className="hidden xl:inline">Mù ngẫu nhiên</span>
            </button>
            <button
              type="button"
              className="tv-btn text-[11px]"
              data-active={replayBlindMode}
              aria-pressed={replayBlindMode}
              onClick={() => setReplayBlindMode(!replayBlindMode)}
              title="Chế độ Luyện tập mù (Blind Mode): ẩn giá Live ở Watchlist, Chi tiết mã và hình vẽ tương lai"
            >
              {replayBlindMode ? <EyeOff className="w-3.5 h-3.5" aria-hidden="true" /> : <Eye className="w-3.5 h-3.5" aria-hidden="true" />}
              <span>{replayBlindMode ? 'Luyện mù: BẬT' : 'Luyện mù: TẮT'}</span>
            </button>

            {/* Jump-to-Date picker (Wave 5) */}
            <label
              className="hidden lg:inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10.5px]"
              style={{ backgroundColor: 'var(--tv-bg-input)', border: '1px solid var(--tv-border)', color: 'var(--tv-text-secondary)' }}
              title="Nhảy thẳng đến ngày giao dịch bất kỳ trong quá khứ"
            >
              <Calendar className="w-3 h-3 shrink-0" />
              <input
                type="date"
                aria-label="Nhảy tới ngày Bar Replay"
                value={jumpDateVal}
                onChange={(e) => {
                  const v = e.target.value
                  setJumpDateVal(v)
                  const ms = Date.parse(`${v}T15:00:00Z`)
                  if (Number.isFinite(ms)) {
                    setReplayPlaying(false)
                    void jumpReplay(ms)
                  }
                }}
                className="bg-transparent border-0 outline-none text-[10.5px] tabular-nums w-[108px]"
                style={{ color: 'var(--tv-text-primary)' }}
              />
            </label>
          </>
        )}

        {renderSavedSessionsPopover(!isBlindDrillUnrevealed)}

        <div className="tv-divider-v" />
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
          <span className="hidden lg:inline">Chọn điểm cắt</span>
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
          <span className="hidden xl:inline">Tốc độ:</span>
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

        <button
          type="button"
          className="tv-btn text-[11px]"
          style={{ color: '#FBC02D' }}
          onClick={() => setDebriefModalOpen(true)}
          title="Xem báo cáo Tổng kết phiên luyện tập (SL Adherence, Chi phí sai lầm, Coaching Callouts)"
        >
          <Award className="w-3.5 h-3.5" aria-hidden="true" />
          <span className="hidden xl:inline">Tổng kết</span>
        </button>

        <button
          type="button"
          className="tv-btn text-[11.5px] text-[#F23645] ml-1"
          onClick={() => void stopReplay()}
          title="Thoát chế độ Bar Replay và khôi phục dữ liệu đầy đủ"
        >
          <X className="w-3.5 h-3.5" />
          <span>Thoát</span>
        </button>
      </div>
    </div>
  )
}
