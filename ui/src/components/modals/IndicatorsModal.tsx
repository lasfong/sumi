import React, { useEffect, useMemo, useRef, useState } from 'react'
import { ChevronDown, ChevronRight, Eye, EyeOff, Plus, Search, Settings, Sliders, Trash2, X } from 'lucide-react'
import { INDICATOR_CATALOG, findIndicatorMeta, type IndicatorCatalogEntry } from '../../chart/indicators/catalog'
import { INDICATOR_COLOR_PALETTE, useWorkspaceStore } from '../../store/workspaceStore'

const CATEGORIES = [
  'Tất cả',
  'Yumeko QMV Thật',
  'QMV Cốt lõi',
  'Xu hướng',
  'Dao động',
  'Khối lượng',
  'Biến động',
] as const

export const IndicatorsModal: React.FC = () => {
  const open = useWorkspaceStore((s) => s.indicatorsModalOpen)
  const setIndicatorsModalOpen = useWorkspaceStore((s) => s.setIndicatorsModalOpen)
  const panes = useWorkspaceStore((s) => s.panes)
  const activeSlotId = useWorkspaceStore((s) => s.activeSlotId)
  const addIndicatorToActivePane = useWorkspaceStore((s) => s.addIndicatorToActivePane)
  const updateIndicatorInActivePane = useWorkspaceStore((s) => s.updateIndicatorInActivePane)
  const removeIndicatorFromActivePane = useWorkspaceStore((s) => s.removeIndicatorFromActivePane)
  const openIndicatorSettings = useWorkspaceStore((s) => s.openIndicatorSettings)

  const activePane = panes.find((p) => p.slotId === activeSlotId) ?? panes[0]
  const dialogRef = useRef<HTMLDialogElement | null>(null)
  const searchRef = useRef<HTMLInputElement | null>(null)

  const [query, setQuery] = useState('')
  const [category, setCategory] = useState<(typeof CATEGORIES)[number]>('Tất cả')
  const [showAux, setShowAux] = useState(false)
  const [editingId, setEditingId] = useState<string | null>(null)
  const [draftParams, setDraftParams] = useState<string[]>([])

  useEffect(() => {
    const dlg = dialogRef.current
    if (!dlg) return
    if (open) {
      if (!dlg.open) dlg.showModal()
      window.setTimeout(() => searchRef.current?.focus(), 0)
    } else if (dlg.open) {
      dlg.close()
    }
  }, [open])

  useEffect(() => {
    const dlg = dialogRef.current
    if (!dlg) return
    const onClose = () => {
      dlg.returnValue = ''
      setIndicatorsModalOpen(false)
      setEditingId(null)
    }
    dlg.addEventListener('close', onClose)

    let pressedOutside = false
    const isOutside = (e: PointerEvent) => {
      const r = dlg.getBoundingClientRect()
      return e.clientX < r.left || e.clientX > r.right || e.clientY < r.top || e.clientY > r.bottom
    }
    const onPointerDown = (e: PointerEvent) => {
      if (!('closedBy' in HTMLDialogElement.prototype)) pressedOutside = isOutside(e)
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
  }, [setIndicatorsModalOpen])

  const filteredCatalog = useMemo(() => {
    const q = query.trim().toLowerCase()
    return INDICATOR_CATALOG.filter((item) => {
      if (category !== 'Tất cả' && item.category !== category) return false
      if (!q) return true
      return (
        item.name.toLowerCase().includes(q) ||
        item.titleVi.toLowerCase().includes(q) ||
        item.description.toLowerCase().includes(q)
      )
    })
  }, [query, category])

  const yumekoItems = useMemo(() => filteredCatalog.filter((i) => i.tier === 'yumeko'), [filteredCatalog])
  const coreItems = useMemo(() => filteredCatalog.filter((i) => i.tier === 'core'), [filteredCatalog])
  const auxItems = useMemo(() => filteredCatalog.filter((i) => i.tier === 'aux'), [filteredCatalog])
  const effectiveShowAux = showAux || query.trim().length > 0

  const startEdit = (id: string, params: number[]) => {
    setEditingId(id)
    setDraftParams(params.map(String))
  }

  const commitEdit = (id: string) => {
    const targetInd = activePane.indicators.find((x) => x.id === id)
    const parsed = draftParams
      .map((s, idx) => {
        const n = Number(s)
        if (!Number.isFinite(n)) return null
        if (targetInd?.name === 'AFC') {
          if (idx === 0) return Math.max(1, Math.min(20, Math.round(n)))
          if (idx === 1) return Math.max(10, Math.min(250, Math.round(n)))
          if (idx === 2) return Math.round(n) <= 0 ? 0 : 1
        }
        return n > 0 ? n : null
      })
      .filter((n): n is number => n !== null)
    if (parsed.length > 0) {
      updateIndicatorInActivePane(id, { calcParams: parsed })
    }
    setEditingId(null)
  }

  const renderCatalogRow = (item: IndicatorCatalogEntry) => (
    <div
      key={item.name}
      className="flex items-center justify-between gap-2 px-2.5 py-2 rounded hover:bg-[var(--tv-hover)]"
    >
      <div className="min-w-0">
        <div className="flex flex-wrap items-center gap-1.5">
          <span className="font-semibold text-[12.5px]">{item.titleVi}</span>
          <span
            className="text-[10px] px-1.5 py-0 rounded font-medium"
            style={{
              backgroundColor: item.category === 'QMV Cốt lõi' ? 'var(--tv-accent-soft)' : 'var(--tv-bg-elevated)',
              color: item.category === 'QMV Cốt lõi' ? 'var(--tv-accent)' : 'var(--tv-text-secondary)',
            }}
          >
            {item.isMain ? 'Chính' : 'Phụ'} · {item.name}
          </span>
          {item.tier === 'yumeko' && (
            <span
              className="text-[9.5px] px-1.5 py-0 rounded font-semibold"
              style={{ backgroundColor: 'rgba(8, 153, 129, 0.16)', color: '#089981' }}
              title="Đọc trực tiếp Chỉ Đọc từ Yumeko v3.0 SQLite (QMV Blackbox thật); tự động fallback OHLCV Proxy khi máy mới chưa có Yumeko DB"
            >
              ✓ Yumeko QMV Thật
            </span>
          )}
          {item.isOhlcvProxy && (
            <span
              className="text-[9.5px] px-1.5 py-0 rounded font-semibold"
              style={{ backgroundColor: 'rgba(251, 192, 45, 0.16)', color: '#FBC02D' }}
              title="Tính toán nhân quả từ dữ liệu OHLCV của Sumi (không phải Blackbox nội bộ trên QMV Webapp)"
            >
              OHLCV Proxy
            </span>
          )}
        </div>
        <div className="text-[11px] truncate" style={{ color: 'var(--tv-text-secondary)' }}>
          {item.description}
        </div>
      </div>
      <button
        type="button"
        className="flex items-center gap-1 px-2.5 py-1 rounded text-[11px] font-semibold bg-[#2962FF] hover:bg-[#1E53E5] text-white border-0 cursor-pointer shrink-0"
        onClick={() =>
          addIndicatorToActivePane({
            name: item.name,
            paneId: item.isMain ? 'candle_pane' : `pane_${item.name.toLowerCase()}`,
            isMain: item.isMain,
            calcParams: [...item.defaultParams],
            visible: true,
          })
        }
      >
        <Plus className="w-3.5 h-3.5" />
        <span>Thêm</span>
      </button>
    </div>
  )

  return (
    <dialog
      ref={dialogRef}
      className="tv-dialog"
      style={{ maxWidth: 'min(94vw, 880px)' }}
      aria-labelledby="indicators-modal-title"
      {...({ closedby: 'any' } as Record<string, string>)}
    >
      {/* Header */}
      <div
        className="flex items-center justify-between px-4 py-3 shrink-0"
        style={{ borderBottom: '1px solid var(--tv-border)' }}
      >
        <div>
          <h2 id="indicators-modal-title" className="m-0 text-[15px] font-semibold">
            Bộ Chỉ Báo Kỹ Thuật QMV (3 Yumeko Thật · 11 Cốt lõi · 5 Phụ trợ)
          </h2>
          <div className="text-[11px]" style={{ color: 'var(--tv-text-secondary)' }}>
            Đang áp dụng cho khung {activePane.ticker} ({activePane.interval}) — Tham số chuẩn QMV V7 & KT01
          </div>
        </div>
        <button
          type="button"
          className="tv-icon-btn w-7 h-7"
          onClick={() => setIndicatorsModalOpen(false)}
          aria-label="Đóng hộp thoại Chỉ báo"
        >
          <X className="w-4 h-4" />
        </button>
      </div>

      {/* Search */}
      <div
        className="flex items-center gap-2 px-4 py-2 shrink-0"
        style={{ borderBottom: '1px solid var(--tv-border)' }}
      >
        <Search className="w-4 h-4 shrink-0" style={{ color: 'var(--tv-text-secondary)' }} />
        <input
          ref={searchRef}
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Tìm chỉ báo QMV (VD: AFC, MA 20/50/200, Ichimoku 6-18-36, QMV_FLOW, RS_VNINDEX, CCI, RSI, MACD)..."
          className="flex-1 bg-transparent border-0 outline-none text-[13px]"
          style={{ color: 'var(--tv-text-primary)' }}
          autoFocus
        />
      </div>

      {/* Category Tabs */}
      <div
        role="tablist"
        aria-label="Nhóm chỉ báo QMV"
        className="flex items-center gap-1 px-3 overflow-x-auto shrink-0"
        style={{ height: 34, borderBottom: '1px solid var(--tv-border)' }}
      >
        {CATEGORIES.map((cat) => (
          <button
            key={cat}
            type="button"
            role="tab"
            aria-selected={category === cat}
            className="tv-tab text-[12px]"
            onClick={() => setCategory(cat)}
          >
            {cat}
          </button>
        ))}
      </div>

      {/* Two-Column Body: Catalog Left + Active Indicators & Parameters Right */}
      <div className="grid grid-cols-1 md:grid-cols-[1fr_290px] flex-1 min-h-0 overflow-hidden max-h-[480px]">
        {/* Catalog List */}
        <div
          className="overflow-y-auto tv-scroll p-2 space-y-1"
          style={{ borderRight: '1px solid var(--tv-border)' }}
        >
          {yumekoItems.length > 0 && (
            <div className="pb-1.5 mb-1 border-b" style={{ borderColor: 'var(--tv-border)' }}>
              <div className="px-2.5 pt-1 pb-0.5 text-[10.5px] font-bold uppercase tracking-wider text-[#089981]">
                Yumeko QMV Blackbox — Dữ liệu Thật ({yumekoItems.length})
              </div>
              {yumekoItems.map(renderCatalogRow)}
            </div>
          )}

          {coreItems.length > 0 && (
            <>
              <div className="px-2.5 pt-1 pb-0.5 text-[10.5px] font-bold uppercase tracking-wider text-[#2962FF]">
                Trụ cột Kỹ thuật QMV ({coreItems.length})
              </div>
              {coreItems.map(renderCatalogRow)}
            </>
          )}

          {auxItems.length > 0 && (
            <div className="pt-2 mt-1 border-t" style={{ borderColor: 'var(--tv-border)' }}>
              <button
                type="button"
                onClick={() => setShowAux((v) => !v)}
                className="w-full flex items-center justify-between px-2.5 py-1.5 rounded text-[11px] font-semibold bg-transparent border-0 cursor-pointer hover:bg-[var(--tv-hover)]"
                style={{ color: 'var(--tv-text-secondary)' }}
              >
                <span className="flex items-center gap-1">
                  {effectiveShowAux ? <ChevronDown className="w-3.5 h-3.5" /> : <ChevronRight className="w-3.5 h-3.5" />}
                  <span>Phụ trợ Định lượng — Nâng cao ({auxItems.length})</span>
                </span>
                <span className="text-[10px] font-normal">ATR · VRVP · VWAP · OBV · ADX</span>
              </button>
              {effectiveShowAux && <div className="mt-1 space-y-1">{auxItems.map(renderCatalogRow)}</div>}
            </div>
          )}
        </div>

        {/* Active Indicators & Parameter Tuning */}
        <div className="flex flex-col overflow-y-auto tv-scroll p-2.5 space-y-2">
          <div className="text-[11px] font-semibold uppercase tracking-wide" style={{ color: 'var(--tv-text-secondary)' }}>
            Đang bật trên biểu đồ ({activePane.indicators.length})
          </div>
          <div className="text-[10px]" style={{ color: 'var(--tv-text-muted)' }}>
            Mẹo QMV: Dùng menu Mẫu chỉ báo trên thanh công cụ để chuyển nhanh giữa 4 bộ thiết lập QMV chuẩn.
          </div>
          {activePane.indicators.length === 0 ? (
            <div className="text-[11.5px] py-4 text-center" style={{ color: 'var(--tv-text-secondary)' }}>
              Chưa bật chỉ báo nào
            </div>
          ) : (
            activePane.indicators.map((ind, indIdx) => {
              const meta = findIndicatorMeta(ind.name)
              const isEditing = editingId === ind.id
              const lineColor =
                (ind.styles as { lines?: Array<{ color?: string }> } | null | undefined)?.lines?.[0]?.color ??
                INDICATOR_COLOR_PALETTE[indIdx % INDICATOR_COLOR_PALETTE.length]
              return (
                <div
                  key={ind.id}
                  className="p-2 rounded space-y-1.5"
                  style={{
                    backgroundColor: 'var(--tv-bg-elevated)',
                    border: '1px solid var(--tv-border)',
                  }}
                >
                  <div className="flex items-center justify-between gap-1">
                    <div className="min-w-0 flex items-center gap-1.5">
                      <span
                        className="w-2.5 h-2.5 rounded-full shrink-0"
                        style={{ backgroundColor: lineColor }}
                      />
                      <div className="min-w-0">
                        <div className="font-semibold text-[12px] truncate">
                          {ind.name}
                          {ind.calcParams.length > 0 && (
                            <span className="font-normal ml-1 tabular-nums" style={{ color: 'var(--tv-text-secondary)' }}>
                              ({ind.calcParams.join(', ')})
                            </span>
                          )}
                        </div>
                        <div className="text-[10px]" style={{ color: 'var(--tv-text-secondary)' }}>
                          {ind.isMain ? 'Chồng trên nến' : 'Khung phụ'}
                        </div>
                      </div>
                    </div>
                    <div className="flex items-center gap-0.5 shrink-0">
                      <button
                        type="button"
                        className="tv-icon-btn w-6 h-6"
                        onClick={() => {
                          setIndicatorsModalOpen(false)
                          openIndicatorSettings(activePane.slotId, ind.id)
                        }}
                        title="Cài đặt tham số & màu sắc"
                      >
                        <Settings className="w-3.5 h-3.5" />
                      </button>
                      {ind.calcParams.length > 0 && (
                        <button
                          type="button"
                          className="tv-icon-btn w-6 h-6"
                          onClick={() => (isEditing ? setEditingId(null) : startEdit(ind.id, ind.calcParams))}
                          title="Chỉnh nhanh tham số"
                        >
                          <Sliders className="w-3.5 h-3.5" />
                        </button>
                      )}
                      <button
                        type="button"
                        className="tv-icon-btn w-6 h-6"
                        onClick={() => updateIndicatorInActivePane(ind.id, { visible: !ind.visible })}
                        title={ind.visible ? 'Ẩn chỉ báo' : 'Hiện chỉ báo'}
                      >
                        {ind.visible ? <Eye className="w-3.5 h-3.5" /> : <EyeOff className="w-3.5 h-3.5" />}
                      </button>
                      <button
                        type="button"
                        className="tv-icon-btn w-6 h-6 hover:text-[#F23645]"
                        onClick={() => removeIndicatorFromActivePane(ind.id)}
                        title="Xóa chỉ báo"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </div>

                  {isEditing && (
                    <div className="pt-1.5 border-t space-y-1.5" style={{ borderColor: 'var(--tv-border)' }}>
                      <div className="grid grid-cols-2 gap-1.5">
                        {draftParams.map((val, idx) => (
                          <label key={idx} className="flex flex-col gap-0.5 text-[10px]" style={{ color: 'var(--tv-text-secondary)' }}>
                            <span>{meta?.paramLabels[idx] ?? `P${idx + 1}`}</span>
                            <input
                              type="number"
                              step="any"
                              value={val}
                              onChange={(e) => {
                                const next = [...draftParams]
                                next[idx] = e.target.value
                                setDraftParams(next)
                              }}
                              className="px-1.5 py-0.5 rounded text-[11.5px] tabular-nums outline-none"
                              style={{
                                backgroundColor: 'var(--tv-bg-input)',
                                color: 'var(--tv-text-primary)',
                                border: '1px solid var(--tv-border)',
                              }}
                            />
                          </label>
                        ))}
                      </div>
                      <button
                        type="button"
                        className="w-full py-1 rounded text-[11px] font-semibold bg-[#2962FF] text-white border-0 cursor-pointer"
                        onClick={() => commitEdit(ind.id)}
                      >
                        Áp dụng tham số
                      </button>
                    </div>
                  )}
                </div>
              )
            })
          )}
        </div>
      </div>
    </dialog>
  )
}
