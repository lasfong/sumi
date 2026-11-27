import React, { useEffect, useRef, useState } from 'react'
import { BellPlus, Calculator, Copy, GripVertical, Lock, Trash2, Unlock } from 'lucide-react'

export type DrawingLineStyleKind = 'solid' | 'dashed' | 'dotted'

export interface SelectedDrawingState {
  id: string
  name: string
  color: string
  size: number
  lineStyle: DrawingLineStyleKind
  lock: boolean
}

interface FloatingDrawingToolbarProps {
  selected: SelectedDrawingState
  boundsRef: React.RefObject<HTMLDivElement | null>
  onChangeColor: (color: string) => void
  onChangeSize: (size: number) => void
  onChangeLineStyle: (style: DrawingLineStyleKind) => void
  onToggleLock: () => void
  onCreateAlert?: () => void
  onOpenTradePlanner?: () => void
  onClone: () => void
  onDelete: () => void
}

const SWATCHES = [
  '#2962FF',
  '#089981',
  '#F23645',
  '#FF9800',
  '#FBC02D',
  '#E040FB',
  '#00BCD4',
  '#D1D4DC',
]

export const FloatingDrawingToolbar: React.FC<FloatingDrawingToolbarProps> = ({
  selected,
  boundsRef,
  onChangeColor,
  onChangeSize,
  onChangeLineStyle,
  onToggleLock,
  onCreateAlert,
  onOpenTradePlanner,
  onClone,
  onDelete,
}) => {
  const toolbarRef = useRef<HTMLDivElement | null>(null)
  const [pos, setPos] = useState<{ x: number; y: number }>({ x: 260, y: 360 })
  const hasUserDraggedRef = useRef(false)
  const [colorMenuOpen, setColorMenuOpen] = useState(false)
  const dragRef = useRef<{ active: boolean; offsetX: number; offsetY: number }>({
    active: false,
    offsetX: 0,
    offsetY: 0,
  })

  // Anchor bottom-center on initial selection so it NEVER overlaps the top-left OHLC & EMA legend (`BUG-036`)!
  useEffect(() => {
    const parent = boundsRef.current
    if (!parent) return
    const rect = parent.getBoundingClientRect()
    const barW = toolbarRef.current?.offsetWidth ?? 330
    setPos((prev) => {
      if (!hasUserDraggedRef.current) {
        return {
          x: Math.max(8, Math.round((rect.width - barW) / 2)),
          y: Math.max(64, Math.round(rect.height - 62)),
        }
      }
      return {
        x: Math.max(8, Math.min(prev.x, Math.max(8, rect.width - barW - 8))),
        y: Math.max(8, Math.min(prev.y, Math.max(8, rect.height - 56))),
      }
    })
  }, [boundsRef, selected.id])

  // Pointer dragging for repositioning anywhere on chart
  useEffect(() => {
    const onMove = (e: PointerEvent) => {
      if (!dragRef.current.active || !boundsRef.current) return
      const parentRect = boundsRef.current.getBoundingClientRect()
      const barW = toolbarRef.current?.offsetWidth ?? 330
      const barH = toolbarRef.current?.offsetHeight ?? 36
      const nextX = e.clientX - parentRect.left - dragRef.current.offsetX
      const nextY = e.clientY - parentRect.top - dragRef.current.offsetY
      hasUserDraggedRef.current = true
      setPos({
        x: Math.max(6, Math.min(nextX, Math.max(6, parentRect.width - barW - 6))),
        y: Math.max(6, Math.min(nextY, Math.max(6, parentRect.height - barH - 6))),
      })
    }
    const onUp = () => {
      dragRef.current.active = false
    }
    window.addEventListener('pointermove', onMove)
    window.addEventListener('pointerup', onUp)
    return () => {
      window.removeEventListener('pointermove', onMove)
      window.removeEventListener('pointerup', onUp)
    }
  }, [boundsRef])

  // Roving tabindex keyboard fallback only when native `focusGroup` is unsupported
  const hasNativeFocusGroup =
    typeof HTMLElement !== 'undefined' && 'focusGroup' in HTMLElement.prototype

  useEffect(() => {
    const el = toolbarRef.current
    if (!el || hasNativeFocusGroup) return
    const items = Array.from(el.querySelectorAll<HTMLElement>('[data-tb-item="true"]'))
    items.forEach((b, i) => {
      b.tabIndex = i === 0 ? 0 : -1
    })
    const onFocusIn = (e: FocusEvent) => {
      const target = e.target as HTMLElement | null
      if (target && items.includes(target)) {
        items.forEach((b) => {
          b.tabIndex = b === target ? 0 : -1
        })
      }
    }
    el.addEventListener('focusin', onFocusIn)
    return () => el.removeEventListener('focusin', onFocusIn)
  }, [hasNativeFocusGroup, selected.id])

  const handleToolbarKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    const el = toolbarRef.current
    if (!el || hasNativeFocusGroup) return
    const items = Array.from(el.querySelectorAll<HTMLElement>('[data-tb-item="true"]'))
    const idx = items.indexOf(document.activeElement as HTMLElement)
    if (idx === -1 || items.length === 0) return

    let next = -1
    if (e.key === 'ArrowRight') next = (idx + 1) % items.length
    else if (e.key === 'ArrowLeft') next = (idx - 1 + items.length) % items.length
    else if (e.key === 'Home') next = 0
    else if (e.key === 'End') next = items.length - 1
    else return

    e.preventDefault()
    items[idx].tabIndex = -1
    items[next].tabIndex = 0
    items[next].focus()
  }

  return (
    <div
      ref={toolbarRef}
      role="toolbar"
      aria-label="Tùy chỉnh hình vẽ đang chọn"
      onKeyDown={handleToolbarKeyDown}
      onMouseDown={(e) => e.stopPropagation()}
      onPointerDown={(e) => e.stopPropagation()}
      className="absolute z-30 flex items-center gap-1 px-1.5 py-1 rounded-md shadow-2xl select-none"
      style={{
        left: pos.x,
        top: pos.y,
        backgroundColor: 'var(--tv-bg-elevated)',
        border: '1px solid var(--tv-border)',
        color: 'var(--tv-text-primary)',
      }}
      {...({ focusgroup: 'toolbar wrap' } as Record<string, string>)}
    >
      {/* Drag Handle */}
      <div
        role="presentation"
        title="Kéo để di chuyển thanh công cụ vẽ"
        className="flex items-center justify-center w-4 h-6 cursor-grab active:cursor-grabbing opacity-60 hover:opacity-100"
        onPointerDown={(e) => {
          e.stopPropagation()
          const rect = toolbarRef.current?.getBoundingClientRect()
          if (!rect) return
          dragRef.current = {
            active: true,
            offsetX: e.clientX - rect.left,
            offsetY: e.clientY - rect.top,
          }
        }}
      >
        <GripVertical className="w-3.5 h-3.5" />
      </div>

      {/* Color Swatch Dropdown */}
      <div className="relative">
        <button
          type="button"
          data-tb-item="true"
          tabIndex={0}
          aria-label="Đổi màu hình vẽ"
          aria-expanded={colorMenuOpen}
          onClick={() => setColorMenuOpen((o) => !o)}
          className="tv-icon-btn w-7 h-7"
          title="Đổi màu đường vẽ"
        >
          <span
            className="w-4 h-4 rounded-sm block"
            style={{
              backgroundColor: selected.color,
              border: '1px solid rgba(255,255,255,0.35)',
            }}
          />
        </button>

        {colorMenuOpen && (
          <div
            className="absolute left-0 top-8 z-40 p-2 rounded-md shadow-2xl flex flex-col gap-2 w-44"
            style={{
              backgroundColor: 'var(--tv-bg-elevated)',
              border: '1px solid var(--tv-border)',
            }}
          >
            <div className="grid grid-cols-4 gap-1.5">
              {SWATCHES.map((hex) => (
                <button
                  key={hex}
                  type="button"
                  aria-label={`Màu ${hex}`}
                  onClick={() => {
                    onChangeColor(hex)
                    setColorMenuOpen(false)
                  }}
                  className="w-7 h-6 rounded cursor-pointer p-0"
                  style={{
                    backgroundColor: hex,
                    border:
                      selected.color.toLowerCase() === hex.toLowerCase()
                        ? '2px solid #FFFFFF'
                        : '1px solid rgba(255,255,255,0.2)',
                  }}
                />
              ))}
            </div>
            <label className="flex items-center justify-between text-[11px]" style={{ color: 'var(--tv-text-secondary)' }}>
              <span>Màu tùy chỉnh</span>
              <input
                type="color"
                value={selected.color}
                onChange={(e) => onChangeColor(e.target.value)}
                className="w-7 h-5 border-0 bg-transparent cursor-pointer p-0"
              />
            </label>
          </div>
        )}
      </div>

      <span role="separator" aria-orientation="vertical" className="tv-divider-v" />

      {/* Stroke Width Buttons (1..4px) */}
      <div role="group" aria-label="Độ dày nét" className="flex items-center gap-0.5">
        {[1, 2, 3, 4].map((w) => (
          <button
            key={w}
            type="button"
            data-tb-item="true"
            tabIndex={hasNativeFocusGroup ? undefined : -1}
            aria-pressed={Math.round(selected.size) === w}
            data-active={Math.round(selected.size) === w}
            onClick={() => onChangeSize(w)}
            className="tv-btn px-1.5 h-6 text-[11px] tabular-nums"
            title={`Độ dày nét ${w}px`}
          >
            {w}px
          </button>
        ))}
      </div>

      <span role="separator" aria-orientation="vertical" className="tv-divider-v" />

      {/* Line Style (Solid / Dashed / Dotted) */}
      <div role="group" aria-label="Kiểu đường nét" className="flex items-center gap-0.5">
        {(
          [
            { key: 'solid', label: '━━', title: 'Nét liền (Solid)' },
            { key: 'dashed', label: '╌╌', title: 'Nét đứt (Dashed)' },
            { key: 'dotted', label: '···', title: 'Nét chấm (Dotted)' },
          ] as const
        ).map((st) => (
          <button
            key={st.key}
            type="button"
            data-tb-item="true"
            tabIndex={hasNativeFocusGroup ? undefined : -1}
            aria-pressed={selected.lineStyle === st.key}
            data-active={selected.lineStyle === st.key}
            onClick={() => onChangeLineStyle(st.key)}
            className="tv-btn px-1.5 h-6 text-[11px]"
            title={st.title}
          >
            {st.label}
          </button>
        ))}
      </div>

      <span role="separator" aria-orientation="vertical" className="tv-divider-v" />

      {/* Lock / Unlock */}
      <button
        type="button"
        data-tb-item="true"
        tabIndex={hasNativeFocusGroup ? undefined : -1}
        aria-pressed={selected.lock}
        aria-label={selected.lock ? 'Mở khóa hình vẽ này' : 'Khóa hình vẽ này'}
        data-active={selected.lock}
        onClick={onToggleLock}
        className="tv-icon-btn w-7 h-7"
        title={selected.lock ? 'Mở khóa hình vẽ này' : 'Khóa hình vẽ này'}
      >
        {selected.lock ? <Lock aria-hidden="true" className="w-3.5 h-3.5" /> : <Unlock aria-hidden="true" className="w-3.5 h-3.5" />}
      </button>

      {/* Attach Alert to Drawing Line (v2.2) */}
      {onCreateAlert &&
        ['segment', 'rayLine', 'straightLine', 'horizontalStraightLine', 'horizontalRayLine', 'horizontalSegment'].includes(
          selected.name,
        ) && (
          <button
            type="button"
            data-tb-item="true"
            tabIndex={hasNativeFocusGroup ? undefined : -1}
            aria-label="Đặt cảnh báo cắt đường vẽ này"
            onClick={onCreateAlert}
            className="tv-icon-btn w-7 h-7 text-[#FBC02D]"
            title="Đặt cảnh báo giá cắt đường vẽ này"
          >
            <BellPlus aria-hidden="true" className="w-3.5 h-3.5" />
          </button>
        )}

      {/* Open Trade Planner directly from Long/Short Position R:R Tool (Wave 3) */}
      {onOpenTradePlanner && ['longPosition', 'shortPosition'].includes(selected.name) && (
        <button
          type="button"
          data-tb-item="true"
          tabIndex={hasNativeFocusGroup ? undefined : -1}
          aria-label="Mở Kế hoạch Lệnh (Trade Planner) từ khung R:R"
          onClick={onOpenTradePlanner}
          className="tv-btn h-6 px-2 text-[10.5px] font-semibold bg-[#2962FF] text-white"
          title="Tính khối lượng lô 100 & gửi thẳng vào Paper Broker từ khung R:R này"
        >
          <Calculator aria-hidden="true" className="w-3 h-3" />
          <span>Kế hoạch Lệnh</span>
        </button>
      )}

      {/* Clone */}
      <button
        type="button"
        data-tb-item="true"
        tabIndex={hasNativeFocusGroup ? undefined : -1}
        aria-label="Nhân bản hình vẽ"
        onClick={onClone}
        className="tv-icon-btn w-7 h-7"
        title="Nhân bản hình vẽ (Clone)"
      >
        <Copy aria-hidden="true" className="w-3.5 h-3.5" />
      </button>

      {/* Delete */}
      <button
        type="button"
        data-tb-item="true"
        tabIndex={hasNativeFocusGroup ? undefined : -1}
        aria-label="Xóa hình vẽ đang chọn"
        onClick={onDelete}
        className="tv-icon-btn w-7 h-7 hover:text-[#F23645]"
        title="Xóa hình vẽ đang chọn (Delete / Backspace)"
      >
        <Trash2 aria-hidden="true" className="w-3.5 h-3.5" />
      </button>
    </div>
  )
}
