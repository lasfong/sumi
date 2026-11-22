import React, { useEffect, useRef, useState } from 'react'
import { ChevronRight, Eye, EyeOff, Lock, Magnet, Trash2, Unlock } from 'lucide-react'
import { ALL_DRAWING_TOOLS_COUNT, DRAWING_TOOL_GROUPS, DrawingToolIcon } from '../chart/drawingIcons'
import { useWorkspaceStore } from '../store/workspaceStore'

export const LeftDrawingRail: React.FC = () => {
  const activeDrawingTool = useWorkspaceStore((s) => s.activeDrawingTool)
  const magnetMode = useWorkspaceStore((s) => s.magnetMode)
  const drawingsLocked = useWorkspaceStore((s) => s.drawingsLocked)
  const drawingsHidden = useWorkspaceStore((s) => s.drawingsHidden)

  const setActiveDrawingTool = useWorkspaceStore((s) => s.setActiveDrawingTool)
  const setMagnetMode = useWorkspaceStore((s) => s.setMagnetMode)
  const toggleDrawingsLocked = useWorkspaceStore((s) => s.toggleDrawingsLocked)
  const toggleDrawingsHidden = useWorkspaceStore((s) => s.toggleDrawingsHidden)
  const triggerClearDrawings = useWorkspaceStore((s) => s.triggerClearDrawings)

  // Remember last selected tool per group just like TradingView Supercharts
  const [groupChoice, setGroupChoice] = useState<Record<string, string>>(() =>
    Object.fromEntries(DRAWING_TOOL_GROUPS.map((g) => [g.id, g.defaultTool])),
  )
  const [openGroupId, setOpenGroupId] = useState<string | null>(null)
  const [popoverTop, setPopoverTop] = useState<number>(48)
  const railRef = useRef<HTMLElement | null>(null)

  useEffect(() => {
    const onPointerDown = (e: PointerEvent) => {
      if (railRef.current && !railRef.current.contains(e.target as Node)) {
        setOpenGroupId(null)
      }
    }
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setOpenGroupId(null)
        setActiveDrawingTool(null)
      }
    }
    document.addEventListener('pointerdown', onPointerDown)
    document.addEventListener('keydown', onKeyDown)
    return () => {
      document.removeEventListener('pointerdown', onPointerDown)
      document.removeEventListener('keydown', onKeyDown)
    }
  }, [setActiveDrawingTool])

  const openGroup = DRAWING_TOOL_GROUPS.find((g) => g.id === openGroupId) ?? null

  const cycleMagnet = () => {
    if (magnetMode === 'normal') setMagnetMode('weak_magnet')
    else if (magnetMode === 'weak_magnet') setMagnetMode('strong_magnet')
    else setMagnetMode('normal')
  }

  return (
    <aside
      ref={railRef}
      aria-label="Thanh công cụ vẽ biểu đồ"
      className="relative z-20 flex flex-col items-center py-1.5 gap-1 shrink-0 select-none"
      style={{
        width: 42,
        backgroundColor: 'var(--tv-bg-panel)',
        borderRight: '1px solid var(--tv-border)',
      }}
    >
      {/* Crosshair / Cursor reset */}
      <button
        type="button"
        className="tv-icon-btn w-8 h-8"
        data-active={activeDrawingTool === null}
        onClick={() => {
          setActiveDrawingTool(null)
          setOpenGroupId(null)
        }}
        title="Con trỏ chữ thập (Esc)"
      >
        <DrawingToolIcon name="cursor" />
      </button>

      <div className="w-6 h-px my-0.5" style={{ backgroundColor: 'var(--tv-border)' }} />

      {/* Grouped Drawing Tools (37+ tools total) */}
      {DRAWING_TOOL_GROUPS.map((group) => {
        const currentTool = groupChoice[group.id] ?? group.defaultTool
        const isGroupActive = group.items.some((it) => it.key === activeDrawingTool)
        return (
          <div key={group.id} className="relative flex items-center justify-center w-full group">
            <button
              type="button"
              className="tv-icon-btn w-8 h-8"
              data-active={isGroupActive}
              onClick={() => {
                setActiveDrawingTool(currentTool)
                setOpenGroupId(null)
              }}
              title={`${group.labelVi} — ${group.items.find((i) => i.key === currentTool)?.labelVi ?? currentTool}`}
            >
              <DrawingToolIcon name={currentTool} />
            </button>
            {group.items.length > 1 && (
              <button
                type="button"
                aria-label={`Mở nhóm ${group.labelVi}`}
                className="absolute right-0.5 bottom-0.5 w-3 h-3 flex items-center justify-center border-0 bg-transparent p-0 cursor-pointer opacity-60 hover:opacity-100"
                style={{ color: 'var(--tv-text-secondary)' }}
                onClick={(e) => {
                  const rect = (e.currentTarget.parentElement as HTMLElement).getBoundingClientRect()
                  setPopoverTop(Math.min(rect.top, window.innerHeight - 320))
                  setOpenGroupId(openGroupId === group.id ? null : group.id)
                }}
              >
                <ChevronRight className="w-2.5 h-2.5" />
              </button>
            )}
          </div>
        )
      })}

      <div className="w-6 h-px my-1" style={{ backgroundColor: 'var(--tv-border)' }} />

      {/* Magnet Mode (Off / Weak / Strong) */}
      <button
        type="button"
        className="tv-icon-btn w-8 h-8 relative"
        data-active={magnetMode !== 'normal'}
        onClick={cycleMagnet}
        title={
          magnetMode === 'normal'
            ? 'Chế độ Hút nến (Magnet): TẮT — Nhấp để bật Hút yếu'
            : magnetMode === 'weak_magnet'
              ? 'Chế độ Hút nến (Magnet): HÚT YẾU — Nhấp để bật Hút mạnh'
              : 'Chế độ Hút nến (Magnet): HÚT MẠNH — Nhấp để tắt'
        }
      >
        <Magnet className="w-4 h-4" />
        {magnetMode === 'strong_magnet' && (
          <span className="absolute top-0.5 right-0.5 w-1.5 h-1.5 rounded-full bg-[#2962FF]" />
        )}
      </button>

      {/* Lock all drawings */}
      <button
        type="button"
        className="tv-icon-btn w-8 h-8"
        data-active={drawingsLocked}
        onClick={toggleDrawingsLocked}
        title={drawingsLocked ? 'Mở khóa toàn bộ hình vẽ' : 'Khóa toàn bộ hình vẽ'}
      >
        {drawingsLocked ? <Lock className="w-4 h-4" /> : <Unlock className="w-4 h-4" />}
      </button>

      {/* Show / Hide all drawings */}
      <button
        type="button"
        className="tv-icon-btn w-8 h-8"
        data-active={drawingsHidden}
        onClick={toggleDrawingsHidden}
        title={drawingsHidden ? 'Hiện toàn bộ hình vẽ' : 'Ẩn toàn bộ hình vẽ'}
      >
        {drawingsHidden ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
      </button>

      {/* Delete all drawings */}
      <button
        type="button"
        className="tv-icon-btn w-8 h-8 hover:text-[#F23645]"
        onClick={triggerClearDrawings}
        title="Xóa toàn bộ hình vẽ trên mã đang xem"
      >
        <Trash2 className="w-4 h-4" />
      </button>

      {/* Flyout Popover for Active Group */}
      {openGroup && (
        <div
          className="fixed left-[46px] w-[324px] rounded-md p-1.5 shadow-2xl z-50"
          style={{
            top: Math.max(42, popoverTop),
            backgroundColor: 'var(--tv-bg-elevated)',
            border: '1px solid var(--tv-border)',
          }}
        >
          <div
            className="flex items-center justify-between gap-2 px-2 py-1 text-[10px] font-semibold uppercase tracking-wider"
            style={{ color: 'var(--tv-text-secondary)' }}
          >
            <span className="truncate">{openGroup.labelVi}</span>
            <span className="whitespace-nowrap shrink-0">{ALL_DRAWING_TOOLS_COUNT} công cụ</span>
          </div>
          <div className="max-h-[340px] overflow-y-auto tv-scroll space-y-0.5">
            {openGroup.items.map((item) => {
              const selected = activeDrawingTool === item.key
              return (
                <button
                  key={item.key}
                  type="button"
                  className="w-full flex items-center gap-2.5 px-2 py-1.5 rounded text-left text-[12px] border-0 cursor-pointer hover:bg-[var(--tv-hover)]"
                  style={{
                    backgroundColor: selected ? 'var(--tv-accent-soft)' : 'transparent',
                    color: selected ? 'var(--tv-accent)' : 'var(--tv-text-primary)',
                  }}
                  onClick={() => {
                    setGroupChoice((prev) => ({ ...prev, [openGroup.id]: item.key }))
                    setActiveDrawingTool(item.key)
                    setOpenGroupId(null)
                  }}
                >
                  <DrawingToolIcon name={item.key} className="w-4 h-4 shrink-0" />
                  <span className="flex-1 whitespace-nowrap">{item.labelVi}</span>
                  {item.shortcut && (
                    <span className="text-[10px] whitespace-nowrap shrink-0" style={{ color: 'var(--tv-text-secondary)' }}>
                      {item.shortcut}
                    </span>
                  )}
                </button>
              )
            })}
          </div>
        </div>
      )}
    </aside>
  )
}
