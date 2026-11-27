import React, { useEffect, useRef, useState } from 'react'
import { Eye, EyeOff, RotateCcw, Trash2, X } from 'lucide-react'
import { findIndicatorMeta } from '../../chart/indicators/catalog'
import { INDICATOR_COLOR_PALETTE, useWorkspaceStore } from '../../store/workspaceStore'

const WIDTH_OPTIONS = [1, 1.5, 2, 3]

interface LineStyleDraft {
  color: string
  size: number
}

function extractLineStyles(styles: Record<string, unknown> | null | undefined, count: number): LineStyleDraft[] {
  const rawLines = Array.isArray((styles as { lines?: unknown } | null)?.lines)
    ? ((styles as { lines: Array<{ color?: string; size?: number }> }).lines)
    : []
  const n = Math.max(1, count)
  return Array.from({ length: n }, (_, i) => ({
    color: rawLines[i]?.color ?? INDICATOR_COLOR_PALETTE[i % INDICATOR_COLOR_PALETTE.length],
    size: typeof rawLines[i]?.size === 'number' ? rawLines[i].size! : 1.6,
  }))
}

export const IndicatorSettingsModal: React.FC = () => {
  const editingIndicator = useWorkspaceStore((s) => s.editingIndicator)
  const panes = useWorkspaceStore((s) => s.panes)
  const closeIndicatorSettings = useWorkspaceStore((s) => s.closeIndicatorSettings)
  const updateIndicatorInActivePane = useWorkspaceStore((s) => s.updateIndicatorInActivePane)
  const removeIndicatorFromActivePane = useWorkspaceStore((s) => s.removeIndicatorFromActivePane)

  const dialogRef = useRef<HTMLDialogElement | null>(null)

  const targetPane = editingIndicator
    ? panes.find((p) => p.slotId === editingIndicator.slotId)
    : undefined
  const indicator = editingIndicator && targetPane
    ? targetPane.indicators.find((ind) => ind.id === editingIndicator.indicatorId)
    : undefined
  const meta = indicator ? findIndicatorMeta(indicator.name) : undefined

  const [draftParams, setDraftParams] = useState<string[]>([])
  const [draftLines, setDraftLines] = useState<LineStyleDraft[]>([])
  const [draftVisible, setDraftVisible] = useState(true)

  const expectedLineCount = (() => {
    if (!indicator) return 1
    // Multi-series indicators (`MA`, `EMA`, `RSI`, `VOL`, `QMV_FLOW`) match their param count;
    // fixed-figure indicators (`BOLL` 3, `ICHIMOKU` 5, `RS_VNINDEX` 2, `VWAP` 5, `MACD` 2) use `lineLabels.length` (`BUG-019`)!
    if (['MA', 'EMA', 'RSI', 'VOL', 'QMV_FLOW'].includes(indicator.name)) {
      return Math.max(1, indicator.calcParams.length)
    }
    return Math.max(1, meta?.lineLabels?.length ?? indicator.calcParams.length ?? 1)
  })()

  useEffect(() => {
    if (!indicator) return
    setDraftParams(indicator.calcParams.map(String))
    setDraftLines(extractLineStyles(indicator.styles, expectedLineCount))
    setDraftVisible(indicator.visible)
  }, [indicator?.id, indicator, expectedLineCount])

  useEffect(() => {
    const dlg = dialogRef.current
    if (!dlg) return
    if (editingIndicator && indicator) {
      if (!dlg.open) dlg.showModal()
    } else if (dlg.open) {
      dlg.close()
    }
  }, [editingIndicator, indicator])

  useEffect(() => {
    const dlg = dialogRef.current
    if (!dlg) return
    const onClose = () => {
      dlg.returnValue = ''
      closeIndicatorSettings()
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
  }, [closeIndicatorSettings])

  const handleApply = (e: React.FormEvent) => {
    e.preventDefault()
    if (!editingIndicator || !indicator) return
    const parsedParams =
      draftParams.length > 0
        ? draftParams.map((v, idx) => {
            const n = Number(v)
            if (!Number.isFinite(n)) return indicator.calcParams[idx] ?? 14
            if (indicator.name === 'AFC') {
              if (idx === 0) return Math.max(1, Math.min(20, Math.round(n)))
              if (idx === 1) return Math.max(10, Math.min(250, Math.round(n)))
              if (idx === 2) return Math.round(n) <= 0 ? 0 : 1
            }
            return n > 0 ? n : (indicator.calcParams[idx] ?? 14)
          })
        : []
    const nextStyles: Record<string, unknown> = {
      ...(indicator.styles ?? {}),
      lines: draftLines.map((ln) => ({
        style: 'solid',
        smooth: false,
        color: ln.color,
        size: ln.size,
      })),
    }
    updateIndicatorInActivePane(
      indicator.id,
      {
        calcParams: parsedParams,
        visible: draftVisible,
        styles: nextStyles,
      },
      editingIndicator.slotId,
    )
    closeIndicatorSettings()
  }

  const handleReset = () => {
    if (!meta) return
    setDraftParams(meta.defaultParams.map(String))
    const resetCount = Math.max(1, meta.lineLabels?.length ?? meta.defaultParams.length)
    setDraftLines(extractLineStyles(null, resetCount))
    setDraftVisible(true)
  }

  const handleDelete = () => {
    if (!editingIndicator || !indicator) return
    removeIndicatorFromActivePane(indicator.id, editingIndicator.slotId)
    closeIndicatorSettings()
  }

  return (
    <dialog
      ref={dialogRef}
      className="tv-dialog"
      style={{ maxWidth: 'min(92vw, 440px)' }}
      aria-labelledby="ind-settings-title"
      {...({ closedby: 'any' } as Record<string, string>)}
    >
      {indicator && editingIndicator && (
        <form onSubmit={handleApply} className="flex flex-col">
          {/* Header */}
          <div
            className="flex items-center justify-between px-4 py-3"
            style={{ borderBottom: '1px solid var(--tv-border)' }}
          >
            <div className="min-w-0">
              <h2 id="ind-settings-title" className="m-0 text-[14px] font-semibold truncate">
                Cài đặt chỉ báo: {meta?.titleVi ?? indicator.name}
              </h2>
              <div className="text-[11px]" style={{ color: 'var(--tv-text-secondary)' }}>
                {indicator.name} · {targetPane?.ticker} ({targetPane?.interval})
              </div>
            </div>
            <button
              type="button"
              className="tv-icon-btn w-7 h-7"
              onClick={closeIndicatorSettings}
              aria-label="Đóng cài đặt chỉ báo"
            >
              <X className="w-4 h-4" />
            </button>
          </div>

          {/* Body */}
          <div className="p-4 space-y-4 overflow-y-auto tv-scroll max-h-[65vh]">
            {/* Parameters */}
            {draftParams.length > 0 && (
              <div className="space-y-2">
                <div className="text-[11px] font-semibold uppercase tracking-wide" style={{ color: 'var(--tv-text-secondary)' }}>
                  Tham số tính toán (Inputs)
                </div>
                <div className="grid grid-cols-2 gap-2.5">
                  {draftParams.map((val, idx) => (
                    <label key={idx} className="flex flex-col gap-1 text-[11.5px]">
                      <span style={{ color: 'var(--tv-text-secondary)' }}>
                        {meta?.paramLabels[idx] ?? `Chu kỳ ${idx + 1}`}
                      </span>
                      <input
                        type="number"
                        step={indicator.name === 'AFC' ? '1' : 'any'}
                        min={indicator.name === 'AFC' && idx === 2 ? '0' : '0.1'}
                        max={indicator.name === 'AFC' && idx === 2 ? '1' : undefined}
                        value={val}
                        onChange={(e) => {
                          const next = [...draftParams]
                          next[idx] = e.target.value
                          setDraftParams(next)
                        }}
                        className="px-2.5 py-1.5 rounded text-[12.5px] tabular-nums outline-none"
                        style={{
                          backgroundColor: 'var(--tv-bg-input)',
                          color: 'var(--tv-text-primary)',
                          border: '1px solid var(--tv-border)',
                        }}
                      />
                    </label>
                  ))}
                </div>
              </div>
            )}

            {/* Line Colors & Thickness */}
            <div className="space-y-2">
              <div className="text-[11px] font-semibold uppercase tracking-wide" style={{ color: 'var(--tv-text-secondary)' }}>
                Màu sắc & Độ dày đường (Style)
              </div>
              <div className="space-y-2">
                {draftLines.map((ln, idx) => (
                  <div
                    key={idx}
                    className="flex items-center justify-between gap-2 px-2.5 py-1.5 rounded"
                    style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}
                  >
                    <span className="text-[11.5px] font-medium">
                      {meta?.lineLabels?.[idx] ?? meta?.paramLabels[idx] ?? `Đường #${idx + 1}`}
                    </span>
                    <div className="flex items-center gap-2">
                      {/* Quick Swatches */}
                      <div className="flex items-center gap-1">
                        {INDICATOR_COLOR_PALETTE.slice(0, 6).map((hex) => (
                          <button
                            key={hex}
                            type="button"
                            aria-label={`Chọn màu ${hex}`}
                            onClick={() => {
                              const next = [...draftLines]
                              next[idx] = { ...next[idx], color: hex }
                              setDraftLines(next)
                            }}
                            className="w-4 h-4 rounded-full cursor-pointer p-0"
                            style={{
                              backgroundColor: hex,
                              border: ln.color.toLowerCase() === hex.toLowerCase() ? '2px solid #FFFFFF' : '1px solid rgba(255,255,255,0.2)',
                            }}
                          />
                        ))}
                      </div>
                      <input
                        type="color"
                        aria-label={`Màu tùy chỉnh đường ${idx + 1}`}
                        value={ln.color}
                        onChange={(e) => {
                          const next = [...draftLines]
                          next[idx] = { ...next[idx], color: e.target.value }
                          setDraftLines(next)
                        }}
                        className="w-6 h-6 rounded border-0 bg-transparent cursor-pointer p-0"
                      />
                      <select
                        aria-label={`Độ dày đường ${idx + 1}`}
                        value={ln.size}
                        onChange={(e) => {
                          const next = [...draftLines]
                          next[idx] = { ...next[idx], size: Number(e.target.value) }
                          setDraftLines(next)
                        }}
                        className="px-1.5 py-0.5 rounded text-[11px] outline-none"
                        style={{
                          backgroundColor: 'var(--tv-bg-input)',
                          color: 'var(--tv-text-primary)',
                          border: '1px solid var(--tv-border)',
                        }}
                      >
                        {WIDTH_OPTIONS.map((w) => (
                          <option key={w} value={w}>
                            {w}px
                          </option>
                        ))}
                      </select>
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* Visibility Toggle */}
            <div className="flex items-center justify-between pt-1">
              <button
                type="button"
                className="tv-btn text-[11.5px]"
                data-active={!draftVisible}
                onClick={() => setDraftVisible((v) => !v)}
              >
                {draftVisible ? <Eye className="w-3.5 h-3.5" /> : <EyeOff className="w-3.5 h-3.5" />}
                <span>{draftVisible ? 'Đang hiển thị' : 'Đang ẩn trên biểu đồ'}</span>
              </button>
              <button
                type="button"
                className="tv-btn text-[11.5px]"
                onClick={handleReset}
                title="Khôi phục tham số mặc định"
              >
                <RotateCcw className="w-3.5 h-3.5" />
                <span>Mặc định</span>
              </button>
            </div>
          </div>

          {/* Footer */}
          <div
            className="flex items-center justify-between gap-2 px-4 py-2.5"
            style={{ borderTop: '1px solid var(--tv-border)', backgroundColor: 'var(--tv-bg-elevated)' }}
          >
            <button
              type="button"
              className="flex items-center gap-1 px-2.5 py-1.5 rounded text-[11.5px] font-medium border cursor-pointer text-[#F23645]"
              style={{ backgroundColor: 'transparent', borderColor: 'rgba(242, 54, 69, 0.45)' }}
              onClick={handleDelete}
            >
              <Trash2 className="w-3.5 h-3.5" />
              <span>Xóa chỉ báo</span>
            </button>

            <div className="flex items-center gap-2">
              <button
                type="button"
                className="px-3 py-1.5 rounded text-[12px] border cursor-pointer"
                style={{
                  backgroundColor: 'transparent',
                  borderColor: 'var(--tv-border)',
                  color: 'var(--tv-text-primary)',
                }}
                onClick={closeIndicatorSettings}
              >
                Hủy
              </button>
              <button
                type="submit"
                className="px-4 py-1.5 rounded text-[12px] font-semibold bg-[#2962FF] hover:bg-[#1E53E5] text-white border-0 cursor-pointer"
              >
                Áp dụng
              </button>
            </div>
          </div>
        </form>
      )}
    </dialog>
  )
}
