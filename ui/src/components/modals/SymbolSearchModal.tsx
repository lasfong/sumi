import React, { useEffect, useRef, useState } from 'react'
import { Search, X } from 'lucide-react'
import * as api from '../../api/tauri'
import { useWorkspaceStore } from '../../store/workspaceStore'
import type { SymbolSearchResult } from '../../types/market'

export function isFormulaExpressionQuery(raw: string): boolean {
  const t = raw.trim()
  if (!t) return false
  if (/[/*+()]/.test(t)) return true
  if (t.includes(' - ') || t.includes(' -') || t.includes('- ')) return true
  return (t.match(/:/g)?.length ?? 0) >= 2 && t.includes('-')
}

const SEARCH_TABS = [
  { id: 'all', label: 'Tất cả' },
  { id: 'hose', label: 'HOSE' },
  { id: 'hnx', label: 'HNX' },
  { id: 'upcom', label: 'UPCOM' },
  { id: 'derivatives', label: 'Phái sinh' },
  { id: 'crypto', label: 'Crypto' },
  { id: 'global', label: 'Quốc tế' },
] as const

export const SymbolSearchModal: React.FC = () => {
  const open = useWorkspaceStore((s) => s.symbolSearchOpen)
  const initialQuery = useWorkspaceStore((s) => s.symbolSearchInitialQuery)
  const closeSymbolSearch = useWorkspaceStore((s) => s.closeSymbolSearch)
  const setActiveTicker = useWorkspaceStore((s) => s.setActiveTicker)

  const dialogRef = useRef<HTMLDialogElement | null>(null)
  const inputRef = useRef<HTMLInputElement | null>(null)
  const [query, setQuery] = useState('')
  const [category, setCategory] = useState<string>('all')
  const [results, setResults] = useState<SymbolSearchResult[]>([])
  const [loading, setLoading] = useState(false)
  const [highlightIdx, setHighlightIdx] = useState(0)

  // Open/close native <dialog> via showModal() + Safari/WebKit light-dismiss fallback
  useEffect(() => {
    const dlg = dialogRef.current
    if (!dlg) return
    if (open) {
      setQuery(initialQuery)
      setHighlightIdx(0)
      if (!dlg.open) {
        dlg.showModal()
      }
      window.setTimeout(() => inputRef.current?.focus(), 0)
    } else if (dlg.open) {
      dlg.close()
    }
  }, [open, initialQuery])

  useEffect(() => {
    const dlg = dialogRef.current
    if (!dlg) return
    const onClose = () => {
      dlg.returnValue = ''
      closeSymbolSearch()
    }
    dlg.addEventListener('close', onClose)

    // Progressive enhancement fallback for light-dismiss where HTMLDialogElement.closedBy is absent (WebKit/Safari)
    let pressedOutside = false
    const isOutside = (e: PointerEvent) => {
      const r = dlg.getBoundingClientRect()
      return e.clientX < r.left || e.clientX > r.right || e.clientY < r.top || e.clientY > r.bottom
    }
    const onPointerDown = (e: PointerEvent) => {
      if (!('closedBy' in HTMLDialogElement.prototype)) {
        pressedOutside = isOutside(e)
      }
    }
    const onPointerUp = (e: PointerEvent) => {
      if (!('closedBy' in HTMLDialogElement.prototype) && pressedOutside && isOutside(e)) {
        dlg.close()
      }
      pressedOutside = false
    }
    dlg.addEventListener('pointerdown', onPointerDown)
    dlg.addEventListener('pointerup', onPointerUp)
    return () => {
      dlg.removeEventListener('close', onClose)
      dlg.removeEventListener('pointerdown', onPointerDown)
      dlg.removeEventListener('pointerup', onPointerUp)
    }
  }, [closeSymbolSearch])

  useEffect(() => {
    if (!open) return
    let active = true
    setLoading(true)
    const timer = window.setTimeout(() => {
      void api
        .searchSymbols(query, category, 50)
        .then((hits) => {
          if (active) {
            setResults(hits)
            setHighlightIdx(0)
          }
        })
        .finally(() => {
          if (active) setLoading(false)
        })
    }, 80)
    return () => {
      active = false
      window.clearTimeout(timer)
    }
  }, [open, query, category])

  const formulaCandidate = isFormulaExpressionQuery(query) ? query.trim().toUpperCase() : null

  const selectItem = (item: SymbolSearchResult) => {
    setActiveTicker(item.ticker)
    closeSymbolSearch()
  }

  const selectFormula = (formula: string) => {
    setActiveTicker(formula)
    closeSymbolSearch()
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setHighlightIdx((i) => Math.min(results.length - 1, i + 1))
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setHighlightIdx((i) => Math.max(0, i - 1))
    } else if (e.key === 'Enter') {
      e.preventDefault()
      if (formulaCandidate) {
        selectFormula(formulaCandidate)
      } else if (results[highlightIdx]) {
        selectItem(results[highlightIdx])
      }
    }
  }

  return (
    <dialog
      ref={dialogRef}
      className="tv-dialog"
      aria-labelledby="symbol-search-title"
      {...({ closedby: 'any' } as Record<string, string>)}
    >
      {/* Modal Header */}
      <div
        className="flex items-center justify-between px-4 py-3 shrink-0"
        style={{ borderBottom: '1px solid var(--tv-border)' }}
      >
        <h2 id="symbol-search-title" className="m-0 text-[15px] font-semibold">
          Tìm kiếm mã chứng khoán
        </h2>
        <button
          type="button"
          className="tv-icon-btn w-7 h-7"
          onClick={closeSymbolSearch}
          aria-label="Đóng hộp thoại tìm kiếm mã"
        >
          <X className="w-4 h-4" />
        </button>
      </div>

      {/* Search Input */}
      <div
        className="flex items-center gap-2 px-4 py-2.5 shrink-0"
        style={{ borderBottom: '1px solid var(--tv-border)' }}
      >
        <Search className="w-4 h-4 shrink-0" style={{ color: 'var(--tv-text-secondary)' }} />
        <input
          ref={inputRef}
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Nhập mã hoặc công thức (VD: FPT, SSI / VNINDEX, VN30F1! - VN30)..."
          className="flex-1 bg-transparent border-0 outline-none text-[14px]"
          style={{ color: 'var(--tv-text-primary)' }}
          autoFocus
        />
        {query && (
          <button
            type="button"
            className="text-[11px] px-2 py-0.5 rounded border-0 cursor-pointer"
            style={{ backgroundColor: 'var(--tv-hover)', color: 'var(--tv-text-secondary)' }}
            onClick={() => setQuery('')}
          >
            Xóa
          </button>
        )}
      </div>

      {/* Exchange Filter Tabs */}
      <div
        role="tablist"
        aria-label="Lọc theo sàn giao dịch"
        className="flex items-center gap-1 px-3 overflow-x-auto shrink-0"
        style={{ height: 36, borderBottom: '1px solid var(--tv-border)' }}
      >
        {SEARCH_TABS.map((t) => (
          <button
            key={t.id}
            type="button"
            role="tab"
            aria-selected={category === t.id}
            className="tv-tab text-[12px]"
            onClick={() => setCategory(t.id)}
          >
            {t.label}
          </button>
        ))}
      </div>

      {/* Results List */}
      <div className="flex-1 overflow-y-auto tv-scroll min-h-[280px] max-h-[420px]">
        {formulaCandidate && (
          <button
            type="button"
            className="w-full flex items-center justify-between gap-3 px-4 py-2.5 text-left border-0 border-b cursor-pointer"
            style={{
              borderColor: 'var(--tv-border)',
              backgroundColor: 'var(--tv-accent-soft)',
              color: 'var(--tv-text-primary)',
            }}
            onClick={() => selectFormula(formulaCandidate)}
          >
            <div className="min-w-0">
              <div className="font-bold text-[13px] text-[#2962FF] truncate">
                𝑓(x) {formulaCandidate}
              </div>
              <div className="text-[11px]" style={{ color: 'var(--tv-text-secondary)' }}>
                Mở biểu đồ Công thức Tổng hợp (Tỷ số / Chênh lệch OHLCV — hỗ trợ Ichimoku, Trendline, Replay)
              </div>
            </div>
            <span
              className="px-2 py-0.5 rounded text-[10px] font-bold shrink-0"
              style={{ backgroundColor: '#2962FF', color: '#FFFFFF' }}
            >
              EXPR · Enter
            </span>
          </button>
        )}
        {loading && results.length === 0 && !formulaCandidate ? (
          <div className="p-6 text-center text-[12px]" style={{ color: 'var(--tv-text-secondary)' }}>
            Đang tìm kiếm trên SQLite cục bộ và TradingView...
          </div>
        ) : results.length === 0 && !formulaCandidate ? (
          <div className="p-6 text-center text-[12px]" style={{ color: 'var(--tv-text-secondary)' }}>
            Không tìm thấy mã phù hợp với "{query}"
          </div>
        ) : (
          results.map((item, idx) => (
            <button
              key={item.ticker}
              type="button"
              className="w-full flex items-center justify-between gap-3 px-4 py-2 text-left border-0 border-b cursor-pointer"
              style={{
                borderColor: 'color-mix(in srgb, var(--tv-border) 50%, transparent)',
                backgroundColor: idx === highlightIdx ? 'var(--tv-hover)' : 'transparent',
                color: 'var(--tv-text-primary)',
              }}
              onMouseEnter={() => setHighlightIdx(idx)}
              onClick={() => selectItem(item)}
            >
              <div className="flex items-center gap-3 min-w-0">
                <span className="font-bold text-[13px] w-24 shrink-0">{item.symbol}</span>
                <span className="text-[12px] truncate" style={{ color: 'var(--tv-text-secondary)' }}>
                  {item.description}
                </span>
              </div>
              <div className="flex items-center gap-2 shrink-0">
                <span className="text-[10.5px] uppercase" style={{ color: 'var(--tv-text-secondary)' }}>
                  {item.market_type}
                </span>
                <span
                  className="px-2 py-0.5 rounded text-[10.5px] font-semibold"
                  style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}
                >
                  {item.exchange}
                </span>
              </div>
            </button>
          ))
        )}
      </div>

      {/* Footer Hint */}
      <div
        className="flex items-center justify-between px-4 py-2 text-[11px] shrink-0"
        style={{
          borderTop: '1px solid var(--tv-border)',
          color: 'var(--tv-text-secondary)',
        }}
      >
        <span>Mẹo: Gõ trực tiếp chữ cái bất kỳ trên biểu đồ để mở nhanh Tìm kiếm mã</span>
        <span>↑↓ chọn · Enter mở · Esc đóng</span>
      </div>
    </dialog>
  )
}
