import React, { useEffect, useRef } from 'react'
import {
  BellPlus,
  RotateCcw,
  Scissors,
  ShieldAlert,
  Target,
  Trash2,
  TrendingDown,
  TrendingUp,
} from 'lucide-react'

export interface ChartContextMenuState {
  x: number
  y: number
  price: number | null
  timestamp: number | null
  priceLabel: string
  hasSelectedDrawing: boolean
}

interface ChartContextMenuProps {
  menu: ChartContextMenuState
  ticker: string
  hasOpenPosition?: boolean
  onClose: () => void
  onResetView: () => void
  onReplayCutHere: () => void
  onCreateAlertAtPrice: () => void
  onBuyLimitAtPrice: () => void
  onSellLimitAtPrice: () => void
  onSetStopLossAtPrice?: () => void
  onSetTakeProfitAtPrice?: () => void
  onDeleteSelectedDrawing: () => void
  onClearAllDrawings: () => void
}

export const ChartContextMenu: React.FC<ChartContextMenuProps> = ({
  menu,
  ticker,
  hasOpenPosition = false,
  onClose,
  onResetView,
  onReplayCutHere,
  onCreateAlertAtPrice,
  onBuyLimitAtPrice,
  onSellLimitAtPrice,
  onSetStopLossAtPrice,
  onSetTakeProfitAtPrice,
  onDeleteSelectedDrawing,
  onClearAllDrawings,
}) => {
  const menuRef = useRef<HTMLDivElement | null>(null)
  const hasNativeFocusGroup =
    typeof HTMLElement !== 'undefined' && 'focusGroup' in HTMLElement.prototype

  // Focus first item on open + dismiss on outside click or Escape + fallback arrow navigation
  useEffect(() => {
    const el = menuRef.current
    if (!el) return
    const first = el.querySelector<HTMLElement>('button:not([disabled])')
    first?.focus()

    const onPointerDown = (e: PointerEvent) => {
      if (!el.contains(e.target as Node)) {
        onClose()
      }
    }
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault()
        onClose()
        return
      }
      if (hasNativeFocusGroup) return
      const items = Array.from(el.querySelectorAll<HTMLElement>('button:not([disabled])'))
      const idx = items.indexOf(document.activeElement as HTMLElement)
      if (items.length === 0) return
      if (e.key === 'ArrowDown') {
        e.preventDefault()
        items[(idx + 1) % items.length].focus({ preventScroll: true })
      } else if (e.key === 'ArrowUp') {
        e.preventDefault()
        items[(idx - 1 + items.length) % items.length].focus({ preventScroll: true })
      } else if (e.key === 'Home') {
        e.preventDefault()
        items[0].focus({ preventScroll: true })
      } else if (e.key === 'End') {
        e.preventDefault()
        items[items.length - 1].focus({ preventScroll: true })
      }
    }
    document.addEventListener('pointerdown', onPointerDown)
    document.addEventListener('keydown', onKeyDown)
    return () => {
      document.removeEventListener('pointerdown', onPointerDown)
      document.removeEventListener('keydown', onKeyDown)
    }
  }, [hasNativeFocusGroup, onClose])

  // Clamp coordinates cleanly inside viewport
  const menuW = 264
  const menuH = menu.hasSelectedDrawing ? 258 : 226
  const left = Math.max(8, Math.min(menu.x, (typeof window !== 'undefined' ? window.innerWidth : 1280) - menuW - 8))
  const top = Math.max(8, Math.min(menu.y, (typeof window !== 'undefined' ? window.innerHeight : 800) - menuH - 8))

  const itemCls =
    'w-full flex items-center justify-between gap-2 px-2.5 py-1.5 rounded text-left text-[12px] border-0 bg-transparent text-[var(--tv-text-primary)] hover:bg-[var(--tv-hover)] focus-visible:bg-[var(--tv-hover)] outline-none cursor-pointer disabled:opacity-45 disabled:cursor-not-allowed'

  const menuRole = hasNativeFocusGroup ? undefined : 'menu'
  const itemRole = hasNativeFocusGroup ? undefined : 'menuitem'

  return (
    <div
      ref={menuRef}
      role={menuRole}
      aria-label={`Menu ngữ cảnh biểu đồ ${ticker}`}
      onContextMenu={(e) => e.preventDefault()}
      onMouseDown={(e) => e.stopPropagation()}
      className="fixed z-50 w-[264px] rounded-md p-1.5 shadow-2xl space-y-0.5 select-none"
      style={{
        left,
        top,
        backgroundColor: 'var(--tv-bg-elevated)',
        border: '1px solid var(--tv-border)',
      }}
      {...({ focusgroup: 'menu nomemory' } as Record<string, string>)}
    >
      <button
        type="button"
        role={itemRole}
        className={itemCls}
        onClick={() => {
          onResetView()
          onClose()
        }}
      >
        <span className="flex items-center gap-2">
          <RotateCcw aria-hidden="true" className="w-3.5 h-3.5" style={{ color: 'var(--tv-text-secondary)' }} />
          <span>Đặt lại khung nhìn (Reset View)</span>
        </span>
        <span aria-hidden="true" className="text-[10px]" style={{ color: 'var(--tv-text-secondary)' }}>
          Alt+R
        </span>
      </button>

      <button
        type="button"
        role={itemRole}
        disabled={menu.timestamp === null}
        className={itemCls}
        onClick={() => {
          onReplayCutHere()
          onClose()
        }}
      >
        <span className="flex items-center gap-2">
          <Scissors aria-hidden="true" className="w-3.5 h-3.5 text-[#2962FF]" />
          <span>Cắt nến Replay tại vị trí này</span>
        </span>
      </button>

      <hr className="my-1 border-0 h-px" style={{ backgroundColor: 'var(--tv-border)' }} />

      <button
        type="button"
        role={itemRole}
        disabled={menu.price === null}
        className={itemCls}
        onClick={() => {
          onCreateAlertAtPrice()
          onClose()
        }}
      >
        <span className="flex items-center gap-2 truncate">
          <BellPlus aria-hidden="true" className="w-3.5 h-3.5 text-[#FBC02D] shrink-0" />
          <span className="truncate">Cảnh báo giá tại {menu.priceLabel}</span>
        </span>
      </button>

      <button
        type="button"
        role={itemRole}
        disabled={menu.price === null}
        className={itemCls}
        onClick={() => {
          onBuyLimitAtPrice()
          onClose()
        }}
      >
        <span className="flex items-center gap-2 truncate">
          <TrendingUp aria-hidden="true" className="w-3.5 h-3.5 text-[#089981] shrink-0" />
          <span className="truncate">Đặt lệnh MUA (LO) @ {menu.priceLabel}</span>
        </span>
      </button>

      <button
        type="button"
        role={itemRole}
        disabled={menu.price === null}
        className={itemCls}
        onClick={() => {
          onSellLimitAtPrice()
          onClose()
        }}
      >
        <span className="flex items-center gap-2 truncate">
          <TrendingDown aria-hidden="true" className="w-3.5 h-3.5 text-[#F23645] shrink-0" />
          <span className="truncate">Đặt lệnh BÁN (LO) @ {menu.priceLabel}</span>
        </span>
      </button>

      {hasOpenPosition && onSetStopLossAtPrice && onSetTakeProfitAtPrice && (
        <>
          <button
            type="button"
            role={itemRole}
            disabled={menu.price === null}
            className={itemCls}
            onClick={() => {
              onSetStopLossAtPrice()
              onClose()
            }}
          >
            <span className="flex items-center gap-2 truncate">
              <ShieldAlert aria-hidden="true" className="w-3.5 h-3.5 text-[#F23645] shrink-0" />
              <span className="truncate">Đặt Cắt lỗ (SL) @ {menu.priceLabel}</span>
            </span>
          </button>
          <button
            type="button"
            role={itemRole}
            disabled={menu.price === null}
            className={itemCls}
            onClick={() => {
              onSetTakeProfitAtPrice()
              onClose()
            }}
          >
            <span className="flex items-center gap-2 truncate">
              <Target aria-hidden="true" className="w-3.5 h-3.5 text-[#089981] shrink-0" />
              <span className="truncate">Đặt Chốt lời (TP) @ {menu.priceLabel}</span>
            </span>
          </button>
        </>
      )}

      <hr className="my-1 border-0 h-px" style={{ backgroundColor: 'var(--tv-border)' }} />

      {menu.hasSelectedDrawing && (
        <button
          type="button"
          role={itemRole}
          className={`${itemCls} text-[#F23645]`}
          onClick={() => {
            onDeleteSelectedDrawing()
            onClose()
          }}
        >
          <span className="flex items-center gap-2">
            <Trash2 aria-hidden="true" className="w-3.5 h-3.5" />
            <span>Xóa hình vẽ đang chọn</span>
          </span>
          <span aria-hidden="true" className="text-[10px] opacity-75">Del</span>
        </button>
      )}

      <button
        type="button"
        role={itemRole}
        className={itemCls}
        onClick={() => {
          onClearAllDrawings()
          onClose()
        }}
      >
        <span className="flex items-center gap-2">
          <Trash2 aria-hidden="true" className="w-3.5 h-3.5" style={{ color: 'var(--tv-text-secondary)' }} />
          <span>Xóa toàn bộ hình vẽ</span>
        </span>
      </button>
    </div>
  )
}
