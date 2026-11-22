import React, { useCallback, useEffect, useRef, useState } from 'react'
import { dispose, init } from 'klinecharts'
import type { Chart, KLineData, Overlay, OverlayEvent } from 'klinecharts'
import * as api from '../api/tauri'
import type {
  ChartPaneConfig,
  GridLayoutKind,
  IntervalKey,
  SavedDrawingRecord,
  VnBandLevels,
} from '../types/market'
import { useWorkspaceStore } from '../store/workspaceStore'
import { crosshairBus } from './crosshairBus'
import { toHeikinAshi } from './heikinAshi'
import { formatBarCountdown, intervalToKLinePeriod } from './periods'
import { buildIndicatorTooltipFeatures, buildSuperchartsStyles } from './themes'
import { registerAllCustomIndicators } from './indicators/registerAll'
import { registerAllCustomOverlays } from './overlays/registerAll'
import {
  FloatingDrawingToolbar,
  type DrawingLineStyleKind,
  type SelectedDrawingState,
} from '../components/FloatingDrawingToolbar'
import { ChartContextMenu, type ChartContextMenuState } from '../components/ChartContextMenu'

interface KLineViewportProps {
  pane: ChartPaneConfig
  isActive: boolean
  isMultiPane: boolean
  gridLayout: GridLayoutKind
  onActivate: () => void
}

type UndoEntry =
  | { kind: 'created'; record: SavedDrawingRecord }
  | { kind: 'deleted'; record: SavedDrawingRecord }
  | { kind: 'modified'; before: SavedDrawingRecord }

const MULTI_PANE_INTERVALS: IntervalKey[] = ['15m', '1h', '4h', '1D', '1W']

function guessPrecision(ticker: string): { pricePrecision: number; volumePrecision: number } {
  const upper = ticker.toUpperCase()
  const cachedQuote = useWorkspaceStore.getState().quotesMap[upper]
  if (cachedQuote && Number.isFinite(cachedQuote.pricePrecision)) {
    const isCrypto = upper.startsWith('BINANCE:') || upper.startsWith('OKX:') || upper.startsWith('BYBIT:')
    return { pricePrecision: cachedQuote.pricePrecision, volumePrecision: isCrypto ? 4 : 0 }
  }
  if (upper.startsWith('HOSE:VNINDEX') || upper.startsWith('HOSE:VN30') || upper.startsWith('HNX:HNX')) {
    return { pricePrecision: 2, volumePrecision: 0 }
  }
  if (upper.includes('VN30F')) {
    return { pricePrecision: 1, volumePrecision: 0 }
  }
  if (upper.startsWith('HOSE:') || upper.startsWith('HNX:') || upper.startsWith('UPCOM:') || upper.endsWith('USDVND')) {
    return { pricePrecision: 0, volumePrecision: 0 }
  }
  if (/(EURUSD|GBPUSD|AUDUSD|NZDUSD|USDCAD|USDCHF)/.test(upper)) {
    return { pricePrecision: 5, volumePrecision: 0 }
  }
  if (/(XRP|ADA|DOGE|TRX|SEI|ARB)/.test(upper)) {
    return { pricePrecision: 4, volumePrecision: 4 }
  }
  if (upper.startsWith('BINANCE:') || upper.startsWith('OKX:') || upper.startsWith('BYBIT:')) {
    return { pricePrecision: 2, volumePrecision: 4 }
  }
  return { pricePrecision: 2, volumePrecision: 0 }
}

function overlayToRecord(ov: Overlay, ticker: string): SavedDrawingRecord {
  return {
    id: ov.id,
    ticker,
    name: ov.name,
    paneId: ov.paneId || 'candle_pane',
    groupId: 'user_drawings',
    points: ov.points.map((p) => ({
      timestamp: p.timestamp,
      dataIndex: p.dataIndex,
      value: p.value,
    })),
    styles: (ov.styles as Record<string, unknown> | null) ?? null,
    extendData: ov.extendData ?? null,
    lock: ov.lock,
    visible: ov.visible,
    mode: ov.mode,
    updatedAtMs: Date.now(),
  }
}

function extractSelectedState(ov: Overlay): SelectedDrawingState {
  const styles = (ov.styles ?? {}) as {
    line?: { color?: string; size?: number; style?: string; dashedValue?: number[] }
    rect?: { borderColor?: string; borderSize?: number; borderStyle?: string; borderDashedValue?: number[] }
    polygon?: { borderColor?: string; borderSize?: number }
    circle?: { borderColor?: string; borderSize?: number }
  }
  const color =
    styles.line?.color ??
    styles.rect?.borderColor ??
    styles.polygon?.borderColor ??
    styles.circle?.borderColor ??
    '#2962FF'
  const size = styles.line?.size ?? styles.rect?.borderSize ?? 2
  const rawStyle = styles.line?.style ?? styles.rect?.borderStyle ?? 'solid'
  const dash = styles.line?.dashedValue ?? styles.rect?.borderDashedValue ?? []
  let lineStyle: DrawingLineStyleKind = 'solid'
  if (rawStyle === 'dashed') {
    lineStyle = dash[0] !== undefined && dash[0] <= 2 ? 'dotted' : 'dashed'
  }
  return {
    id: ov.id,
    name: ov.name,
    color,
    size,
    lineStyle,
    lock: ov.lock,
  }
}

function buildDrawingStylesPatch(
  color: string,
  size: number,
  lineStyle: DrawingLineStyleKind,
): Record<string, unknown> {
  const klineStyle = lineStyle === 'solid' ? 'solid' : 'dashed'
  const dashedValue = lineStyle === 'dotted' ? [2, 3] : [6, 4]
  // Soft translucent fill matching the chosen stroke color
  const fillColor = hexToRgba(color, 0.16)
  return {
    line: { style: klineStyle, color, size, dashedValue },
    rect: {
      style: 'stroke_fill',
      color: fillColor,
      borderColor: color,
      borderSize: size,
      borderStyle: klineStyle,
      borderDashedValue: dashedValue,
    },
    polygon: {
      style: 'stroke_fill',
      color: fillColor,
      borderColor: color,
      borderSize: size,
      borderStyle: klineStyle,
      borderDashedValue: dashedValue,
    },
    circle: {
      style: 'stroke_fill',
      color: fillColor,
      borderColor: color,
      borderSize: size,
      borderStyle: klineStyle,
      borderDashedValue: dashedValue,
    },
    arc: { style: klineStyle, color, size, dashedValue },
    text: { backgroundColor: color, borderColor: color, color: '#FFFFFF' },
    point: { color, activeColor: color },
  }
}

function hexToRgba(hex: string, alpha: number): string {
  const clean = hex.replace('#', '')
  if (clean.length !== 6) return `rgba(41, 98, 255, ${alpha})`
  const r = parseInt(clean.slice(0, 2), 16)
  const g = parseInt(clean.slice(2, 4), 16)
  const b = parseInt(clean.slice(4, 6), 16)
  if (Number.isNaN(r) || Number.isNaN(g) || Number.isNaN(b)) return `rgba(41, 98, 255, ${alpha})`
  return `rgba(${r}, ${g}, ${b}, ${alpha})`
}

function formatScissorDate(ts: number, interval: IntervalKey): string {
  const d = new Date(ts)
  if (['1D', '1W', '1M'].includes(interval)) {
    return d.toLocaleDateString('vi-VN', { year: 'numeric', month: '2-digit', day: '2-digit' })
  }
  return d.toLocaleString('vi-VN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}

export const KLineViewport: React.FC<KLineViewportProps> = ({
  pane,
  isActive,
  isMultiPane,
  gridLayout,
  onActivate,
}) => {
  const rootRef = useRef<HTMLDivElement | null>(null)
  const containerRef = useRef<HTMLDivElement | null>(null)
  const chartRef = useRef<Chart | null>(null)
  const paneRef = useRef<ChartPaneConfig>(pane)
  paneRef.current = pane

  const isActiveRef = useRef(isActive)
  isActiveRef.current = isActive

  // Scissor Curtain DOM refs (zero React re-renders during 120Hz mouse sweeping!)
  const curtainLineRef = useRef<HTMLDivElement | null>(null)
  const curtainMaskRef = useRef<HTMLDivElement | null>(null)
  const curtainBadgeRef = useRef<HTMLDivElement | null>(null)
  const curtainHoverTsRef = useRef<number | null>(null)

  const subscribeCallbackRef = useRef<((data: KLineData) => void) | null>(null)
  const lastHaBarRef = useRef<{ open: number; close: number } | undefined>(undefined)
  const appliedPaneRef = useRef<{ ticker: string; interval: string }>({
    ticker: pane.ticker,
    interval: pane.interval,
  })
  const precisionSyncedTickerRef = useRef<string | null>(null)
  const createdIndicatorIdsRef = useRef<Map<string, { klineId: string; paneId: string; signature: string }>>(
    new Map(),
  )
  const inProgressDrawingIdRef = useRef<string | null>(null)
  const undoStackRef = useRef<UndoEntry[]>([])
  const historyLimitRef = useRef<number>(600)
  const pendingScrollTsRef = useRef<number | null>(null)

  const [vnBand, setVnBand] = useState<VnBandLevels | null>(null)
  const [dataSourceBadge, setDataSourceBadge] = useState<string>('')
  const [loading, setLoading] = useState<boolean>(false)
  const [dataReadyNonce, setDataReadyNonce] = useState<number>(0)
  const [selectedDrawing, setSelectedDrawing] = useState<SelectedDrawingState | null>(null)
  const [contextMenu, setContextMenu] = useState<ChartContextMenuState | null>(null)

  const theme = useWorkspaceStore((s) => s.theme)
  const activeDrawingTool = useWorkspaceStore((s) => s.activeDrawingTool)
  const magnetMode = useWorkspaceStore((s) => s.magnetMode)
  const drawingsLocked = useWorkspaceStore((s) => s.drawingsLocked)
  const drawingsHidden = useWorkspaceStore((s) => s.drawingsHidden)
  const clearDrawingsNonce = useWorkspaceStore((s) => s.clearDrawingsNonce)
  const exportPngNonce = useWorkspaceStore((s) => s.exportPngNonce)
  const scrollToTimestampCmd = useWorkspaceStore((s) => s.scrollToTimestampCmd)
  const backtestOverlay = useWorkspaceStore((s) => s.backtestOverlay)
  const replay = useWorkspaceStore((s) => s.replay)
  const replaySelectingCut = useWorkspaceStore((s) => s.replaySelectingCut)
  const replayResetNonce = useWorkspaceStore((s) => s.replayResetNonce)
  const replaySteppedBar = useWorkspaceStore((s) => s.replaySteppedBar)
  const broker = useWorkspaceStore((s) => s.broker)

  const setActiveInterval = useWorkspaceStore((s) => s.setActiveInterval)
  const setActiveDrawingTool = useWorkspaceStore((s) => s.setActiveDrawingTool)
  const setReplaySelectingCut = useWorkspaceStore((s) => s.setReplaySelectingCut)
  const startReplay = useWorkspaceStore((s) => s.startReplay)
  const jumpReplay = useWorkspaceStore((s) => s.jumpReplay)
  const updateIndicatorInActivePane = useWorkspaceStore((s) => s.updateIndicatorInActivePane)
  const removeIndicatorFromActivePane = useWorkspaceStore((s) => s.removeIndicatorFromActivePane)
  const openIndicatorSettings = useWorkspaceStore((s) => s.openIndicatorSettings)
  const createNewAlert = useWorkspaceStore((s) => s.createNewAlert)
  const submitQuickLimitOrder = useWorkspaceStore((s) => s.submitQuickLimitOrder)
  const pushToast = useWorkspaceStore((s) => s.pushToast)

  const replaySelectingCutRef = useRef(replaySelectingCut)
  replaySelectingCutRef.current = replaySelectingCut
  const replayActiveRef = useRef(Boolean(replay?.active))
  replayActiveRef.current = Boolean(replay?.active)
  const selectedDrawingRef = useRef(selectedDrawing)
  selectedDrawingRef.current = selectedDrawing

  const scheduleSafeResize = useCallback(() => {
    window.requestAnimationFrame(() => {
      chartRef.current?.resize()
      window.requestAnimationFrame(() => {
        chartRef.current?.resize()
      })
    })
  }, [])

  const persistOverlayToDb = useCallback((event: OverlayEvent<unknown>) => {
    const ov = event.overlay
    if (ov.groupId !== 'user_drawings') return
    const record = overlayToRecord(ov, paneRef.current.ticker)
    void api.upsertDrawing(record)
  }, [])

  const attachDrawingCallbacks = useCallback(
    (isNew = false) => ({
      onSelected: (ev: OverlayEvent<unknown>) => {
        if (ev.overlay.groupId !== 'user_drawings') return
        setSelectedDrawing(extractSelectedState(ev.overlay))
      },
      onDeselected: (ev: OverlayEvent<unknown>) => {
        if (selectedDrawingRef.current?.id === ev.overlay.id) {
          setSelectedDrawing(null)
        }
      },
      onPressedMoveEnd: (ev: OverlayEvent<unknown>) => {
        persistOverlayToDb(ev)
        if (selectedDrawingRef.current?.id === ev.overlay.id) {
          setSelectedDrawing(extractSelectedState(ev.overlay))
        }
      },
      onRemoved: (ev: OverlayEvent<unknown>) => {
        if (ev.overlay.groupId === 'user_drawings') {
          void api.deleteDrawing(paneRef.current.ticker, ev.overlay.id)
          if (selectedDrawingRef.current?.id === ev.overlay.id) {
            setSelectedDrawing(null)
          }
        }
      },
      ...(isNew
        ? {
            onDrawEnd: (ev: OverlayEvent<unknown>) => {
              inProgressDrawingIdRef.current = null
              const rec = overlayToRecord(ev.overlay, paneRef.current.ticker)
              undoStackRef.current = [...undoStackRef.current.slice(-29), { kind: 'created', record: rec }]
              void api.upsertDrawing(rec)
              setActiveDrawingTool(null)
              setSelectedDrawing(extractSelectedState(ev.overlay))
            },
          }
        : {}),
    }),
    [persistOverlayToDb, setActiveDrawingTool],
  )

  const recreateOverlayFromRecord = useCallback(
    (rec: SavedDrawingRecord) => {
      const chart = chartRef.current
      if (!chart) return
      chart.createOverlay({
        id: rec.id,
        name: rec.name,
        groupId: 'user_drawings',
        paneId: rec.paneId || 'candle_pane',
        points: rec.points,
        styles: rec.styles ?? undefined,
        extendData: rec.extendData ?? undefined,
        lock: rec.lock || useWorkspaceStore.getState().drawingsLocked,
        visible: useWorkspaceStore.getState().drawingsHidden ? false : rec.visible,
        mode: (rec.mode as 'normal' | 'weak_magnet' | 'strong_magnet') || useWorkspaceStore.getState().magnetMode,
        ...attachDrawingCallbacks(false),
      })
    },
    [attachDrawingCallbacks],
  )

  const handleResetView = useCallback(() => {
    const chart = chartRef.current
    if (!chart) return
    chart.overrideYAxis({
      paneId: 'candle_pane',
      name: paneRef.current.priceScale,
      gap: { top: 0.12, bottom: 0.12 },
    })
    chart.setBarSpace(8)
    chart.scrollToRealTime(200)
    chart.resize()
  }, [])

  const handleAutoFitYAxis = useCallback(() => {
    const chart = chartRef.current
    if (!chart) return
    chart.overrideYAxis({
      paneId: 'candle_pane',
      name: paneRef.current.priceScale,
      gap: { top: 0.12, bottom: 0.12 },
    })
    chart.resize()
  }, [])

  const deleteSelectedDrawingById = useCallback((id: string) => {
    const chart = chartRef.current
    if (!chart) return
    const [existing] = chart.getOverlays({ id, groupId: 'user_drawings' })
    if (existing) {
      const snap = overlayToRecord(existing, paneRef.current.ticker)
      undoStackRef.current = [...undoStackRef.current.slice(-29), { kind: 'deleted', record: snap }]
    }
    chart.removeOverlay({ id, groupId: 'user_drawings' })
    void api.deleteDrawing(paneRef.current.ticker, id)
    setSelectedDrawing(null)
  }, [])

  const handleUndoDrawing = useCallback(() => {
    const chart = chartRef.current
    const stack = undoStackRef.current
    if (!chart || stack.length === 0) return
    const entry = stack.pop()
    if (!entry) return

    if (entry.kind === 'created') {
      chart.removeOverlay({ id: entry.record.id, groupId: 'user_drawings' })
      void api.deleteDrawing(paneRef.current.ticker, entry.record.id)
      setSelectedDrawing(null)
      pushToast('info', 'Đã hoàn tác hình vừa vẽ (Ctrl/Cmd+Z)')
    } else if (entry.kind === 'deleted') {
      recreateOverlayFromRecord(entry.record)
      void api.upsertDrawing(entry.record)
      pushToast('info', 'Đã khôi phục hình vừa xóa (Ctrl/Cmd+Z)')
    } else if (entry.kind === 'modified') {
      chart.overrideOverlay({
        id: entry.before.id,
        styles: entry.before.styles ?? undefined,
        lock: entry.before.lock,
        points: entry.before.points,
      })
      void api.upsertDrawing(entry.before)
      const [restored] = chart.getOverlays({ id: entry.before.id, groupId: 'user_drawings' })
      if (restored) setSelectedDrawing(extractSelectedState(restored))
      pushToast('info', 'Đã hoàn tác chỉnh sửa hình vẽ')
    }
  }, [pushToast, recreateOverlayFromRecord])

  // 1. Mount KLineChart v10 with strict order: register -> init -> setSymbol -> setPeriod -> setDataLoader
  useEffect(() => {
    const el = containerRef.current
    if (!el) return

    registerAllCustomOverlays()
    registerAllCustomIndicators()

    const initialPane = paneRef.current
    const initialTheme = useWorkspaceStore.getState().theme
    const prec = guessPrecision(initialPane.ticker)

    const chart = init(el, {
      timezone:
        initialPane.ticker.startsWith('HOSE:') ||
        initialPane.ticker.startsWith('HNX:') ||
        initialPane.ticker.startsWith('UPCOM:')
          ? 'Asia/Ho_Chi_Minh'
          : 'Etc/UTC',
      styles: buildSuperchartsStyles(initialTheme, initialPane.chartStyle, initialPane.showCountdown),
      layout: {
        barSpaceLimit: { min: 2, max: 46 },
        pane: { height: 112, minHeight: 54 },
        yAxis: { position: 'right', inside: false, scrollZoomEnabled: true },
      },
      thousandsSeparator: { sign: ',' },
      formatter: {
        formatExtendText: ({ data }) => {
          if (!paneRef.current.showCountdown) return ''
          if (replayActiveRef.current) return 'REPLAY'
          return formatBarCountdown(data.timestamp, paneRef.current.interval)
        },
      },
    })

    if (!chart) return
    chartRef.current = chart
    crosshairBus.register(initialPane.slotId, chart)
    const streamId = `stream_${initialPane.slotId}`

    chart.setSymbol({
      ticker: initialPane.ticker,
      pricePrecision: prec.pricePrecision,
      volumePrecision: prec.volumePrecision,
    })
    chart.setPeriod(intervalToKLinePeriod(initialPane.interval))

    chart.setDataLoader({
      getBars: async ({ type, timestamp, callback }) => {
        const cur = paneRef.current
        if (type === 'init') setLoading(true)
        try {
          const queryMode: 'init' | 'forward' | 'backward' =
            type === 'forward' || type === 'backward' ? type : 'init'
          const page = await api.getBars({
            ticker: cur.ticker,
            interval: cur.interval,
            mode: queryMode,
            timestamp: timestamp ?? null,
            limit: queryMode === 'init' ? historyLimitRef.current : 400,
          })
          setVnBand(page.vnBand)
          setDataSourceBadge(
            page.replayActive
              ? 'REPLAY'
              : page.source === 'cache_hit'
                ? 'SQLite'
                : page.source === 'offline_cache'
                  ? 'Offline'
                  : 'Live',
          )

          let bars: KLineData[] = page.bars
          if (cur.chartStyle === 'heikin_ashi') {
            bars = toHeikinAshi(page.bars)
            const last = bars[bars.length - 1]
            lastHaBarRef.current = last ? { open: last.open, close: last.close } : undefined
          } else {
            lastHaBarRef.current = undefined
          }

          callback(bars, { forward: page.hasOlder, backward: page.hasNewer })

          if (type === 'init') {
            setDataReadyNonce((n) => n + 1)
            // Guarantee post-layout paint & non-blank Y-axis in multi-chart grids (fixes BUG-006!)
            scheduleSafeResize()

            // If a trade row requested a deep historical timestamp, scroll immediately now that bars exist.
            if (pendingScrollTsRef.current !== null) {
              const targetTs = pendingScrollTsRef.current
              pendingScrollTsRef.current = null
              window.requestAnimationFrame(() => {
                chartRef.current?.scrollToTimestamp(targetTs, 240)
              })
            }

            // Synchronize authoritative precision once per ticker without loops
            if (page.symbolInfo && chartRef.current) {
              const realPricePrec = page.symbolInfo.price_precision
              const realVolPrec = page.symbolInfo.volume_precision
              const curSym = chartRef.current.getSymbol()
              if (
                curSym &&
                curSym.ticker === cur.ticker &&
                precisionSyncedTickerRef.current !== cur.ticker &&
                (curSym.pricePrecision !== realPricePrec || curSym.volumePrecision !== realVolPrec)
              ) {
                precisionSyncedTickerRef.current = cur.ticker
                queueMicrotask(() => {
                  if (chartRef.current && paneRef.current.ticker === cur.ticker) {
                    chartRef.current.setSymbol({
                      ticker: cur.ticker,
                      pricePrecision: realPricePrec,
                      volumePrecision: realVolPrec,
                    })
                  }
                })
              } else {
                precisionSyncedTickerRef.current = cur.ticker
              }
            }
          }
        } catch {
          callback([], { forward: false, backward: false })
        } finally {
          if (type === 'init') setLoading(false)
        }
      },
      subscribeBar: ({ callback }) => {
        subscribeCallbackRef.current = callback
        const cur = paneRef.current
        if (replayActiveRef.current) return
        void api.subscribeBars(streamId, cur.ticker, cur.interval, (payload) => {
          if (payload.ticker !== paneRef.current.ticker || payload.interval !== paneRef.current.interval) {
            return
          }
          if (replayActiveRef.current) return
          let bar: KLineData = payload.bar
          if (paneRef.current.chartStyle === 'heikin_ashi') {
            const [ha] = toHeikinAshi([bar], lastHaBarRef.current)
            if (ha) {
              bar = ha
              lastHaBarRef.current = { open: ha.open, close: ha.close }
            }
          }
          callback(bar)
        })
      },
      unsubscribeBar: () => {
        subscribeCallbackRef.current = null
        void api.unsubscribeBars(streamId)
      },
    })

    // Zero-React-re-render Crosshair sync via CrosshairBus (fixes BUG-001!)
    chart.subscribeAction('onCrosshairChange', (data) => {
      if (crosshairBus.isSuppressed(paneRef.current.slotId)) return
      const payload = data as { kLineData?: KLineData } | null
      crosshairBus.publish(paneRef.current.slotId, payload?.kLineData?.timestamp ?? null)
    })

    // On-chart TradingView Study Legend Action Buttons (👁 Hide / ⚙ Settings / ✕ Delete) (fixes BUG-010!)
    chart.subscribeAction('onIndicatorTooltipFeatureClick', (data) => {
      const payload = data as {
        paneId?: string
        indicator?: { id?: string; name?: string }
        feature?: { id?: string }
      } | null
      const featureId = payload?.feature?.id
      const klineIndId = payload?.indicator?.id
      if (!featureId || !klineIndId) return

      const currentPane = paneRef.current
      const spec =
        currentPane.indicators.find((i) => i.id === klineIndId) ??
        currentPane.indicators.find((i) => i.name === payload?.indicator?.name)
      if (!spec) return

      if (featureId === 'toggle_visible') {
        updateIndicatorInActivePane(spec.id, { visible: !spec.visible }, currentPane.slotId)
      } else if (featureId === 'settings') {
        // KLineChart v10 dispatches feature clicks on `mouseDownEvent`; defer opening the
        // native `<dialog closedby="any">` until after `pointerup`/`click` finishes so the
        // same mouse-up outside the centered dialog never triggers light-dismiss.
        let opened = false
        const openDeferred = () => {
          if (opened) return
          opened = true
          window.setTimeout(() => {
            openIndicatorSettings(currentPane.slotId, spec.id)
          }, 0)
        }
        window.addEventListener('pointerup', openDeferred, { once: true })
        window.setTimeout(openDeferred, 180)
      } else if (featureId === 'remove') {
        removeIndicatorFromActivePane(spec.id, currentPane.slotId)
      }
    })

    // Causal Bar Replay fallback click handler (fixed KLineChart v10 `data.current.timestamp`, BUG-014!)
    chart.subscribeAction('onCandleBarClick', (data) => {
      if (!isActiveRef.current || !replaySelectingCutRef.current) return
      const payload = data as {
        data?: { current?: KLineData; timestamp?: number }
      } | null
      const ts = payload?.data?.current?.timestamp ?? payload?.data?.timestamp
      if (typeof ts === 'number') {
        if (replayActiveRef.current) {
          void jumpReplay(ts)
        } else {
          void startReplay(ts)
        }
      }
    })

    // Post-mount safe measurement so 2nd/3rd/4th panes never stay blank at 0x0
    scheduleSafeResize()

    const ro = new ResizeObserver((entries) => {
      const entry = entries[0]
      if (!entry) return
      const { width, height } = entry.contentRect
      if (width > 10 && height > 10) {
        chartRef.current?.resize()
      }
    })
    ro.observe(el)

    return () => {
      ro.disconnect()
      crosshairBus.unregister(initialPane.slotId)
      void api.unsubscribeBars(streamId)
      subscribeCallbackRef.current = null
      createdIndicatorIdsRef.current.clear()
      dispose(el)
      chartRef.current = null
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  // 2. React to symbol or interval changes (skipping initial mount; calling only changed setter)
  useEffect(() => {
    const chart = chartRef.current
    if (!chart) return
    const prev = appliedPaneRef.current
    const tickerChanged = prev.ticker !== pane.ticker
    const intervalChanged = prev.interval !== pane.interval
    if (!tickerChanged && !intervalChanged) return

    appliedPaneRef.current = { ticker: pane.ticker, interval: pane.interval }
    historyLimitRef.current = 600

    if (tickerChanged) {
      precisionSyncedTickerRef.current = null
      setSelectedDrawing(null)
      undoStackRef.current = []
      const prec = guessPrecision(pane.ticker)
      const isVn =
        pane.ticker.startsWith('HOSE:') ||
        pane.ticker.startsWith('HNX:') ||
        pane.ticker.startsWith('UPCOM:')
      chart.setTimezone(isVn ? 'Asia/Ho_Chi_Minh' : 'Etc/UTC')
      chart.setSymbol({
        ticker: pane.ticker,
        pricePrecision: prec.pricePrecision,
        volumePrecision: prec.volumePrecision,
      })
    }
    if (intervalChanged) {
      chart.setPeriod(intervalToKLinePeriod(pane.interval))
    }
  }, [pane.ticker, pane.interval])

  // 3. React to theme, chart style, Heikin-Ashi, and countdown toggle
  const prevStyleRef = useRef(pane.chartStyle)
  useEffect(() => {
    const chart = chartRef.current
    if (!chart) return
    chart.setStyles(buildSuperchartsStyles(theme, pane.chartStyle, pane.showCountdown))
    const toggledHeikin =
      (prevStyleRef.current === 'heikin_ashi') !== (pane.chartStyle === 'heikin_ashi')
    prevStyleRef.current = pane.chartStyle
    if (toggledHeikin) {
      chart.resetData()
    }
  }, [theme, pane.chartStyle, pane.showCountdown])

  // 4. React to Y-axis scale mode (Normal / % / Logarithm)
  useEffect(() => {
    const chart = chartRef.current
    if (!chart) return
    chart.overrideYAxis({
      paneId: 'candle_pane',
      name: pane.priceScale,
    })
  }, [pane.priceScale])

  // 5. Reconcile active indicators on this chart pane (with unique IDs for EMA 20 + EMA 50 + EMA 200!)
  useEffect(() => {
    const chart = chartRef.current
    if (!chart) return

    const desiredMap = new Map(pane.indicators.map((ind) => [ind.id, ind]))
    const activeMap = createdIndicatorIdsRef.current

    for (const [specId, tracked] of Array.from(activeMap.entries())) {
      if (!desiredMap.has(specId)) {
        chart.removeIndicator({ id: tracked.klineId, paneId: tracked.paneId })
        activeMap.delete(specId)
      }
    }

    for (const spec of pane.indicators) {
      const mergedStyles = {
        ...(spec.styles ?? {}),
        tooltip: {
          features: buildIndicatorTooltipFeatures(theme, spec.visible),
        },
      }
      const signature = JSON.stringify({
        name: spec.name,
        calcParams: spec.calcParams,
        visible: spec.visible,
        isMain: spec.isMain,
        styles: spec.styles ?? null,
        theme,
      })
      const existing = activeMap.get(spec.id)
      if (!existing) {
        const targetPaneId = spec.isMain ? 'candle_pane' : `pane_${spec.id}`
        const klineId = chart.createIndicator(
          {
            id: spec.id,
            name: spec.name,
            paneId: targetPaneId,
            calcParams: spec.calcParams,
            visible: spec.visible,
            styles: mergedStyles as Record<string, unknown>,
          },
          spec.isMain,
        )
        if (klineId) {
          activeMap.set(spec.id, { klineId, paneId: targetPaneId, signature })
          if (!spec.isMain) {
            chart.setPaneOptions({
              id: targetPaneId,
              height: isMultiPane ? 52 : 90,
              minHeight: 34,
            })
          }
        }
      } else if (existing.signature !== signature) {
        chart.overrideIndicator({
          id: existing.klineId,
          name: spec.name,
          paneId: existing.paneId,
          calcParams: spec.calcParams,
          visible: spec.visible,
          styles: mergedStyles as Record<string, unknown>,
        })
        activeMap.set(spec.id, { ...existing, signature })
      }
    }
  }, [pane.indicators, isMultiPane, theme])

  // 5b. Re-measure & compact sub-pane heights whenever gridLayout changes (`1 / 2v / 2h / 3 / 4`, fixes BUG-006!)
  useEffect(() => {
    const chart = chartRef.current
    if (!chart) return
    for (const tracked of createdIndicatorIdsRef.current.values()) {
      if (tracked.paneId !== 'candle_pane') {
        chart.setPaneOptions({
          id: tracked.paneId,
          height: isMultiPane ? 52 : 90,
          minHeight: 34,
        })
      }
    }
    scheduleSafeResize()
  }, [isMultiPane, gridLayout, scheduleSafeResize])

  // 6. Load & restore persistent SQLite drawings whenever ticker changes
  useEffect(() => {
    const chart = chartRef.current
    if (!chart) return
    let cancelled = false
    chart.removeOverlay({ groupId: 'user_drawings' })

    void api.getDrawings(pane.ticker).then((records) => {
      if (cancelled || !chartRef.current) return
      for (const rec of records) {
        recreateOverlayFromRecord(rec)
      }
    })
    return () => {
      cancelled = true
    }
  }, [pane.ticker, recreateOverlayFromRecord])

  // 7. Trigger interactive drawing tool when selected in Left Drawing Rail (active pane only)
  useEffect(() => {
    const chart = chartRef.current
    if (!chart || !isActive || !activeDrawingTool) return
    const createdId = chart.createOverlay({
      name: activeDrawingTool,
      groupId: 'user_drawings',
      paneId: 'candle_pane',
      mode: magnetMode,
      lock: drawingsLocked,
      visible: !drawingsHidden,
      ...attachDrawingCallbacks(true),
    })
    if (typeof createdId === 'string') {
      inProgressDrawingIdRef.current = createdId
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeDrawingTool, isActive])

  // 8. Sync lock / hide / magnet state independently (never clobber individual drawing lock when toggling magnet!)
  const prevDrawFlagsRef = useRef({ drawingsLocked, drawingsHidden, magnetMode })
  useEffect(() => {
    const chart = chartRef.current
    const prev = prevDrawFlagsRef.current
    if (!chart) return
    if (prev.drawingsLocked !== drawingsLocked) {
      chart.overrideOverlay({ groupId: 'user_drawings', lock: drawingsLocked })
    }
    if (prev.drawingsHidden !== drawingsHidden) {
      chart.overrideOverlay({ groupId: 'user_drawings', visible: !drawingsHidden })
    }
    if (prev.magnetMode !== magnetMode) {
      chart.overrideOverlay({ groupId: 'user_drawings', mode: magnetMode })
    }
    prevDrawFlagsRef.current = { drawingsLocked, drawingsHidden, magnetMode }
  }, [drawingsLocked, drawingsHidden, magnetMode])

  // 9. Clear all user drawings on active pane
  useEffect(() => {
    if (clearDrawingsNonce === 0 || !isActive || !chartRef.current) return
    chartRef.current.removeOverlay({ groupId: 'user_drawings' })
    setSelectedDrawing(null)
    undoStackRef.current = []
    void api.clearDrawings(pane.ticker)
    pushToast('info', `Đã xóa toàn bộ hình vẽ trên ${pane.ticker}`)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [clearDrawingsNonce])

  // 10. Render Vietnam Ceiling / Reference / Floor band lines
  useEffect(() => {
    const chart = chartRef.current
    if (!chart) return
    chart.removeOverlay({ groupId: 'vn_bands' })
    if (!pane.showVnBands || !vnBand) return

    const lines: Array<{ id: string; tag: 'TRẦN' | 'THAM CHIẾU' | 'SÀN'; value: number; color: string }> = [
      { id: `vn_ceil_${pane.slotId}`, tag: 'TRẦN', value: vnBand.ceiling, color: '#CE93D8' },
      { id: `vn_ref_${pane.slotId}`, tag: 'THAM CHIẾU', value: vnBand.reference, color: '#FBC02D' },
      { id: `vn_floor_${pane.slotId}`, tag: 'SÀN', value: vnBand.floor, color: '#26C6DA' },
    ]
    for (const line of lines) {
      chart.createOverlay({
        id: line.id,
        name: 'vnPriceBandLine',
        groupId: 'vn_bands',
        paneId: 'candle_pane',
        lock: true,
        points: [{ value: line.value }],
        extendData: { tag: line.tag, color: line.color },
      })
    }
  }, [pane.showVnBands, pane.slotId, vnBand])

  // 11. Render Paper Broker open position & pending order lines on matching chart
  useEffect(() => {
    const chart = chartRef.current
    if (!chart) return
    chart.removeOverlay({ groupId: 'broker_orders' })
    if (!broker || broker.ticker !== pane.ticker) return

    if (broker.openPosition) {
      const pos = broker.openPosition
      const unreal = (broker.markPrice - pos.avg_entry_price) * pos.quantity * (pos.side === 'Long' ? 1 : -1)
      const sign = unreal >= 0 ? '+' : ''
      chart.createOverlay({
        id: `pos_entry_${pane.slotId}`,
        name: 'brokerOrderLine',
        groupId: 'broker_orders',
        paneId: 'candle_pane',
        lock: true,
        points: [{ value: pos.avg_entry_price }],
        extendData: {
          label: `${pos.side === 'Long' ? 'LONG' : 'SHORT'} ${pos.quantity.toLocaleString('en-US')} @ ${pos.avg_entry_price.toLocaleString('en-US')} (${sign}${Math.round(unreal).toLocaleString('en-US')})`,
          color: unreal >= 0 ? '#089981' : '#F23645',
          dashed: false,
        },
      })
      if (pos.stop_loss) {
        chart.createOverlay({
          id: `pos_sl_${pane.slotId}`,
          name: 'brokerOrderLine',
          groupId: 'broker_orders',
          paneId: 'candle_pane',
          lock: true,
          points: [{ value: pos.stop_loss }],
          extendData: { label: `SL ${pos.stop_loss.toLocaleString('vi-VN')}`, color: '#F23645', dashed: true },
        })
      }
      if (pos.take_profit) {
        chart.createOverlay({
          id: `pos_tp_${pane.slotId}`,
          name: 'brokerOrderLine',
          groupId: 'broker_orders',
          paneId: 'candle_pane',
          lock: true,
          points: [{ value: pos.take_profit }],
          extendData: { label: `TP ${pos.take_profit.toLocaleString('vi-VN')}`, color: '#089981', dashed: true },
        })
      }
    }

    for (const item of broker.orders) {
      if (item.order.status !== 'Pending') continue
      const price =
        typeof item.order.kind === 'object' && 'Limit' in item.order.kind
          ? item.order.kind.Limit.limit_price
          : typeof item.order.kind === 'object' && 'Stop' in item.order.kind
            ? item.order.kind.Stop.stop_price
            : null
      if (price === null) continue
      chart.createOverlay({
        id: `ord_${item.order.id}_${pane.slotId}`,
        name: 'brokerOrderLine',
        groupId: 'broker_orders',
        paneId: 'candle_pane',
        lock: true,
        points: [{ value: price }],
        extendData: {
          label: `${item.vnOrderType} ${item.order.side === 'Buy' ? 'MUA' : 'BÁN'} ${item.order.quantity.toLocaleString('vi-VN')} @ ${price.toLocaleString('vi-VN')}`,
          color: item.order.side === 'Buy' ? '#2962FF' : '#FF9800',
          dashed: true,
        },
      })
    }
  }, [broker, pane.ticker, pane.slotId])

  // 11b. Plot Strategy Backtest Buy (▲) & Sell (▼) Markers directly on candlesticks (fixes BUG-018!)
  useEffect(() => {
    const chart = chartRef.current
    if (!chart) return
    chart.removeOverlay({ groupId: 'backtest_signals' })
    if (
      !backtestOverlay ||
      !backtestOverlay.visible ||
      backtestOverlay.ticker !== pane.ticker ||
      backtestOverlay.interval !== pane.interval
    ) {
      return
    }

    const dataList = chart.getDataList()
    const barByTs = new Map<number, KLineData>()
    for (const bar of dataList) {
      barByTs.set(bar.timestamp, bar)
    }

    // Limit visual density smoothly to at most 80 trades (160 markers) so canvas stays 120 FPS
    const trades = backtestOverlay.trades.slice(-80)
    for (const tr of trades) {
      const isShort = tr.side === 'SHORT'
      const entryBar = barByTs.get(tr.entryTimestampMs)
      const exitBar = barByTs.get(tr.exitTimestampMs)
      const entryValue = isShort
        ? (entryBar?.high ?? tr.entryPrice)
        : (entryBar?.low ?? tr.entryPrice)
      const exitValue = isShort
        ? (exitBar?.low ?? tr.exitPrice)
        : (exitBar?.high ?? tr.exitPrice)

      chart.createOverlay({
        id: `bt_in_${tr.tradeId}_${pane.slotId}`,
        name: 'backtestTradeMarker',
        groupId: 'backtest_signals',
        paneId: 'candle_pane',
        lock: true,
        points: [{ timestamp: tr.entryTimestampMs, value: entryValue }],
        extendData: {
          kind: isShort ? 'sell' : 'buy',
          label: isShort ? `SHORT #${tr.tradeId}` : `MUA #${tr.tradeId}`,
          color: isShort ? '#F23645' : '#089981',
        },
      })

      const sign = tr.returnPct >= 0 ? '+' : ''
      chart.createOverlay({
        id: `bt_out_${tr.tradeId}_${pane.slotId}`,
        name: 'backtestTradeMarker',
        groupId: 'backtest_signals',
        paneId: 'candle_pane',
        lock: true,
        points: [{ timestamp: tr.exitTimestampMs, value: exitValue }],
        extendData: {
          kind: isShort ? 'buy' : 'sell',
          label: `${tr.exitReason} ${sign}${tr.returnPct.toFixed(1)}%`,
          color: tr.netPnl >= 0 ? '#2962FF' : '#F23645',
        },
      })
    }
  }, [backtestOverlay, pane.ticker, pane.interval, pane.slotId, dataReadyNonce])

  // 12. Bar Replay resetData() & single-bar subscribeBar push
  useEffect(() => {
    if (replayResetNonce === 0 || !isActive || !chartRef.current) return
    chartRef.current.resetData()
  }, [replayResetNonce, isActive])

  useEffect(() => {
    if (!replaySteppedBar?.bar || !isActive || !subscribeCallbackRef.current) return
    let bar: KLineData = replaySteppedBar.bar
    if (paneRef.current.chartStyle === 'heikin_ashi') {
      const [ha] = toHeikinAshi([bar], lastHaBarRef.current)
      if (ha) {
        bar = ha
        lastHaBarRef.current = { open: ha.open, close: ha.close }
      }
    }
    subscribeCallbackRef.current(bar)
  }, [replaySteppedBar, isActive])

  // 13. Scroll to specific trade timestamp (auto-widens history if trade is older than loaded window, fixes BUG-019!)
  useEffect(() => {
    if (!scrollToTimestampCmd || scrollToTimestampCmd.slotId !== pane.slotId || !chartRef.current) {
      return
    }
    const targetTs = scrollToTimestampCmd.timestampMs
    const dataList = chartRef.current.getDataList()
    if (dataList.length > 0 && targetTs < dataList[0].timestamp && historyLimitRef.current < 2500) {
      historyLimitRef.current = 2500
      pendingScrollTsRef.current = targetTs
      chartRef.current.resetData()
      return
    }
    chartRef.current.scrollToTimestamp(targetTs, 260)
  }, [scrollToTimestampCmd, pane.slotId])

  // 14. Export PNG snapshot
  useEffect(() => {
    if (exportPngNonce === 0 || !isActive || !chartRef.current) return
    const bg = theme === 'dark' ? '#131722' : '#FFFFFF'
    const url = chartRef.current.getConvertPictureUrl(true, 'png', bg)
    const link = document.createElement('a')
    link.href = url
    link.download = `${pane.ticker.replace(':', '_')}_${pane.interval}.png`
    link.click()
    pushToast('success', `Đã xuất ảnh biểu đồ ${pane.ticker} (${pane.interval})`)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [exportPngNonce])

  // 15. Active-pane TradingView Keyboard Shortcuts (Delete/Backspace, Esc, Ctrl/Cmd+Z, Alt+R) (fixes BUG-009!)
  useEffect(() => {
    if (!isActive) return
    const onKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null
      const inInput =
        target &&
        (target.tagName === 'INPUT' ||
          target.tagName === 'TEXTAREA' ||
          target.tagName === 'SELECT' ||
          target.isContentEditable)
      const st = useWorkspaceStore.getState()
      if (inInput || st.symbolSearchOpen || st.indicatorsModalOpen || st.editingIndicator) return

      if ((e.key === 'Delete' || e.key === 'Backspace') && selectedDrawingRef.current) {
        e.preventDefault()
        deleteSelectedDrawingById(selectedDrawingRef.current.id)
        return
      }

      if (e.key === 'Escape') {
        setContextMenu(null)
        if (replaySelectingCutRef.current) {
          setReplaySelectingCut(false)
        }
        if (inProgressDrawingIdRef.current && chartRef.current) {
          chartRef.current.removeOverlay({ id: inProgressDrawingIdRef.current, groupId: 'user_drawings' })
          inProgressDrawingIdRef.current = null
        }
        if (st.activeDrawingTool) {
          setActiveDrawingTool(null)
        }
        if (selectedDrawingRef.current) {
          setSelectedDrawing(null)
        }
        return
      }

      if ((e.metaKey || e.ctrlKey) && !e.shiftKey && e.key.toLowerCase() === 'z') {
        if (undoStackRef.current.length > 0) {
          e.preventDefault()
          handleUndoDrawing()
        }
        return
      }

      if (e.altKey && !e.ctrlKey && !e.metaKey && e.key.toLowerCase() === 'r') {
        e.preventDefault()
        handleResetView()
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [isActive, deleteSelectedDrawingById, handleResetView, handleUndoDrawing, setActiveDrawingTool, setReplaySelectingCut])

  // ─── Floating Drawing Toolbar Handlers ────────────────────────────────────

  const updateSelectedStyle = (patch: Partial<SelectedDrawingState>) => {
    const cur = selectedDrawingRef.current
    const chart = chartRef.current
    if (!cur || !chart) return
    const [existing] = chart.getOverlays({ id: cur.id, groupId: 'user_drawings' })
    if (existing) {
      undoStackRef.current = [
        ...undoStackRef.current.slice(-29),
        { kind: 'modified', before: overlayToRecord(existing, paneRef.current.ticker) },
      ]
    }
    const next: SelectedDrawingState = { ...cur, ...patch }
    const styles = buildDrawingStylesPatch(next.color, next.size, next.lineStyle)
    chart.overrideOverlay({
      id: cur.id,
      groupId: 'user_drawings',
      styles,
      lock: next.lock,
    })
    setSelectedDrawing(next)
    const [updated] = chart.getOverlays({ id: cur.id, groupId: 'user_drawings' })
    if (updated) {
      void api.upsertDrawing(overlayToRecord(updated, paneRef.current.ticker))
    }
  }

  const handleCloneSelectedDrawing = () => {
    const cur = selectedDrawingRef.current
    const chart = chartRef.current
    if (!cur || !chart) return
    const [existing] = chart.getOverlays({ id: cur.id, groupId: 'user_drawings' })
    if (!existing) return
    const clonedPoints = existing.points.map((pt) => ({
      ...pt,
      value: typeof pt.value === 'number' ? pt.value * 1.012 : pt.value,
    }))
    const newId = `clone_${Date.now()}_${Math.floor(Math.random() * 1000)}`
    const created = chart.createOverlay({
      id: newId,
      name: existing.name,
      groupId: 'user_drawings',
      paneId: existing.paneId || 'candle_pane',
      points: clonedPoints,
      styles: (existing.styles as Record<string, unknown> | undefined) ?? undefined,
      extendData: existing.extendData ?? undefined,
      lock: false,
      visible: true,
      mode: existing.mode,
      ...attachDrawingCallbacks(false),
    })
    if (typeof created === 'string') {
      const [ov] = chart.getOverlays({ id: created, groupId: 'user_drawings' })
      if (ov) {
        const rec = overlayToRecord(ov, paneRef.current.ticker)
        undoStackRef.current = [...undoStackRef.current.slice(-29), { kind: 'created', record: rec }]
        void api.upsertDrawing(rec)
        setSelectedDrawing(extractSelectedState(ov))
      }
    }
  }

  // ─── Interactive Scissor Curtain Handlers (Bar Replay) ────────────────────

  const resolveTimestampAtX = (x: number): number | null => {
    const chart = chartRef.current
    if (!chart) return null
    const dataList = chart.getDataList()
    if (dataList.length === 0) return null

    const converted = chart.convertFromPixel([{ x }], { paneId: 'candle_pane' })
    const pt = Array.isArray(converted) ? converted[0] : converted
    if (typeof pt?.timestamp === 'number' && Number.isFinite(pt.timestamp)) {
      return pt.timestamp
    }
    if (typeof pt?.dataIndex === 'number' && Number.isFinite(pt.dataIndex)) {
      const clampedIdx = Math.max(1, Math.min(dataList.length - 1, Math.round(pt.dataIndex)))
      return dataList[clampedIdx]?.timestamp ?? null
    }
    return dataList[Math.max(1, Math.floor(dataList.length / 2))]?.timestamp ?? null
  }

  // Initialize Scissor Curtain cleanly at 55% width whenever cut-picking starts
  useEffect(() => {
    if (!isActive || !replaySelectingCut) return
    const el = containerRef.current
    if (!el) return
    const rect = el.getBoundingClientRect()
    const x = Math.max(24, Math.round(rect.width * 0.55))
    if (curtainLineRef.current) {
      curtainLineRef.current.style.transform = `translate3d(${x}px, 0, 0)`
      curtainLineRef.current.style.opacity = '1'
    }
    if (curtainMaskRef.current) {
      curtainMaskRef.current.style.left = `${x}px`
      curtainMaskRef.current.style.opacity = '1'
    }
    const ts = resolveTimestampAtX(x)
    curtainHoverTsRef.current = ts
    if (curtainBadgeRef.current) {
      curtainBadgeRef.current.textContent =
        ts !== null ? `✂ ${formatScissorDate(ts, paneRef.current.interval)}` : '✂ Di chuyển chuột để chọn điểm cắt'
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isActive, replaySelectingCut])

  const handleCurtainPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    const el = containerRef.current
    if (!el) return
    const rect = el.getBoundingClientRect()
    const x = Math.max(8, Math.min(e.clientX - rect.left, Math.max(8, rect.width - 68)))

    if (curtainLineRef.current) {
      curtainLineRef.current.style.transform = `translate3d(${x}px, 0, 0)`
      curtainLineRef.current.style.opacity = '1'
    }
    if (curtainMaskRef.current) {
      curtainMaskRef.current.style.left = `${x}px`
      curtainMaskRef.current.style.opacity = '1'
    }

    const ts = resolveTimestampAtX(x)
    curtainHoverTsRef.current = ts
    if (curtainBadgeRef.current) {
      curtainBadgeRef.current.textContent =
        ts !== null ? `✂ ${formatScissorDate(ts, paneRef.current.interval)}` : '✂ Chọn nến cắt'
    }
  }

  const handleCurtainClick = (e: React.MouseEvent<HTMLDivElement>) => {
    e.stopPropagation()
    const el = containerRef.current
    if (!el) return
    const rect = el.getBoundingClientRect()
    const x = Math.max(8, Math.min(e.clientX - rect.left, Math.max(8, rect.width - 68)))
    const ts = resolveTimestampAtX(x) ?? curtainHoverTsRef.current
    if (ts === null) return

    if (replayActiveRef.current) {
      void jumpReplay(ts)
    } else {
      void startReplay(ts)
    }
  }

  // ─── Right-Click Context Menu & Double-Click Y-Axis Auto-fit ──────────────

  const handleContextMenu = (e: React.MouseEvent<HTMLDivElement>) => {
    e.preventDefault()
    onActivate()
    const chart = chartRef.current
    const el = containerRef.current
    if (!chart || !el) return

    const rect = el.getBoundingClientRect()
    const localX = e.clientX - rect.left
    const localY = e.clientY - rect.top
    const converted = chart.convertFromPixel([{ x: localX, y: localY }], { paneId: 'candle_pane' })
    const pt = Array.isArray(converted) ? converted[0] : converted
    const price = typeof pt?.value === 'number' && Number.isFinite(pt.value) ? pt.value : null
    const timestamp = resolveTimestampAtX(localX)
    const prec = chart.getSymbol()?.pricePrecision ?? 0
    const priceLabel =
      price !== null
        ? price.toLocaleString('en-US', {
            minimumFractionDigits: prec,
            maximumFractionDigits: prec,
          })
        : '—'

    setContextMenu({
      x: e.clientX,
      y: e.clientY,
      price,
      timestamp,
      priceLabel,
      hasSelectedDrawing: Boolean(selectedDrawingRef.current),
    })
  }

  const handleCanvasDoubleClick = (e: React.MouseEvent<HTMLDivElement>) => {
    const el = containerRef.current
    if (!el) return
    const rect = el.getBoundingClientRect()
    const localX = e.clientX - rect.left
    // Right Y-axis strip (approx. right 78px): TradingView Auto-fit Y-axis!
    if (localX >= rect.width - 78) {
      handleAutoFitYAxis()
    }
  }

  const shortSymbol = pane.ticker.includes(':') ? pane.ticker.split(':')[1] : pane.ticker
  const exchangeCode = pane.ticker.includes(':') ? pane.ticker.split(':')[0] : ''

  return (
    <div
      ref={rootRef}
      onMouseDown={onActivate}
      onContextMenu={handleContextMenu}
      className="relative flex flex-col w-full h-full min-w-0 min-h-0 overflow-hidden"
      style={{
        backgroundColor: 'var(--tv-bg-chart)',
        outline: isMultiPane && isActive ? '1.5px solid var(--tv-accent)' : '1px solid var(--tv-border)',
        outlineOffset: '-1.5px',
      }}
    >
      {/* Multi-Chart Pane Header Strip: clear Symbol + independent Timeframe switcher per pane */}
      {isMultiPane && (
        <div
          className="flex items-center justify-between gap-1.5 px-2 shrink-0 select-none"
          style={{
            height: 24,
            backgroundColor: isActive
              ? 'color-mix(in srgb, var(--tv-accent) 12%, var(--tv-bg-panel))'
              : 'var(--tv-bg-panel)',
            borderBottom: '1px solid var(--tv-border)',
          }}
        >
          <div className="flex items-center gap-1.5 min-w-0 text-[11px]">
            <span
              className="w-1.5 h-1.5 rounded-full shrink-0"
              style={{ backgroundColor: isActive ? 'var(--tv-accent)' : 'var(--tv-text-muted)' }}
            />
            <span className="font-semibold truncate">{shortSymbol}</span>
            {exchangeCode && (
              <span className="text-[10px]" style={{ color: 'var(--tv-text-secondary)' }}>
                {exchangeCode}
              </span>
            )}
            <span
              className="px-1 py-0 rounded text-[10px] font-semibold"
              style={{ backgroundColor: 'var(--tv-accent-soft)', color: 'var(--tv-accent)' }}
            >
              {pane.interval === '1h' ? '1H' : pane.interval === '4h' ? '4H' : pane.interval}
            </span>
          </div>

          <div className="flex items-center gap-0.5 shrink-0">
            {MULTI_PANE_INTERVALS.map((iv) => (
              <button
                key={iv}
                type="button"
                onClick={(e) => {
                  e.stopPropagation()
                  onActivate()
                  setActiveInterval(iv, pane.slotId)
                }}
                className="px-1 py-0 rounded text-[10px] border-0 cursor-pointer bg-transparent"
                style={{
                  color: pane.interval === iv ? 'var(--tv-accent)' : 'var(--tv-text-secondary)',
                  backgroundColor: pane.interval === iv ? 'var(--tv-accent-soft)' : 'transparent',
                  fontWeight: pane.interval === iv ? 600 : 400,
                }}
              >
                {iv === '1h' ? '1H' : iv === '4h' ? '4H' : iv}
              </button>
            ))}
          </div>
        </div>
      )}

      {/* Main Chart Body Container */}
      <div className="relative flex-1 min-w-0 min-h-0 overflow-hidden" onDoubleClick={handleCanvasDoubleClick}>
        {/* Top-Right Status & VN Price Band Pill */}
        <div className="pointer-events-none absolute top-1.5 right-16 z-10 flex items-center gap-1.5 text-[10px]">
          {!isMultiPane && vnBand && pane.showVnBands && (
            <div
              className="flex items-center gap-2 px-2 py-0.5 rounded tabular-nums"
              style={{
                backgroundColor: 'color-mix(in srgb, var(--tv-bg-panel) 84%, transparent)',
                border: '1px solid var(--tv-border)',
              }}
            >
              <span style={{ color: 'var(--tv-vn-ceil)' }}>Trần {vnBand.ceiling.toLocaleString('en-US')}</span>
              <span style={{ color: 'var(--tv-vn-ref)' }}>TC {vnBand.reference.toLocaleString('en-US')}</span>
              <span style={{ color: 'var(--tv-vn-floor)' }}>Sàn {vnBand.floor.toLocaleString('en-US')}</span>
            </div>
          )}
          {dataSourceBadge && (
            <span
              className="px-1.5 py-0.5 rounded font-medium"
              style={{
                backgroundColor:
                  dataSourceBadge === 'REPLAY'
                    ? 'var(--tv-accent)'
                    : 'color-mix(in srgb, var(--tv-bg-panel) 85%, transparent)',
                color: dataSourceBadge === 'REPLAY' ? '#FFFFFF' : 'var(--tv-text-secondary)',
                border: '1px solid var(--tv-border)',
              }}
            >
              {dataSourceBadge}
            </span>
          )}
        </div>

        {/* Floating Drawing Toolbar (appears when a user drawing is selected, fixes BUG-008!) */}
        {isActive && selectedDrawing && !replaySelectingCut && (
          <FloatingDrawingToolbar
            selected={selectedDrawing}
            boundsRef={containerRef}
            onChangeColor={(color) => updateSelectedStyle({ color })}
            onChangeSize={(size) => updateSelectedStyle({ size })}
            onChangeLineStyle={(lineStyle) => updateSelectedStyle({ lineStyle })}
            onToggleLock={() => updateSelectedStyle({ lock: !selectedDrawing.lock })}
            onClone={handleCloneSelectedDrawing}
            onDelete={() => deleteSelectedDrawingById(selectedDrawing.id)}
          />
        )}

        {/* Interactive Scissor Curtain Overlay for Bar Replay (fixes BUG-014!) */}
        {isActive && replaySelectingCut && (
          <div
            data-testid="replay-scissor-curtain"
            onPointerMove={handleCurtainPointerMove}
            onClick={handleCurtainClick}
            className="absolute inset-0 z-20 cursor-col-resize select-none"
          >
            {/* Dimmed future region to the right of the scissor line */}
            <div
              ref={curtainMaskRef}
              className="pointer-events-none absolute top-0 bottom-0 right-0 transition-opacity duration-75"
              style={{
                left: '55%',
                opacity: 0.9,
                backgroundColor:
                  theme === 'dark' ? 'rgba(19, 23, 34, 0.58)' : 'rgba(240, 243, 250, 0.66)',
              }}
            />
            {/* Full-height vertical blue scissor guide line + date/time footer badge */}
            <div
              ref={curtainLineRef}
              className="pointer-events-none absolute top-0 bottom-0 left-0 w-0"
              style={{
                transform: 'translate3d(260px, 0, 0)',
                borderLeft: '2px solid #2962FF',
              }}
            >
              <div className="absolute top-2 -translate-x-1/2 whitespace-nowrap px-2 py-0.5 rounded text-[10.5px] font-semibold bg-[#2962FF] text-white shadow-lg">
                ✂ Nhấp để cắt nến tại đây
              </div>
              <div
                ref={curtainBadgeRef}
                className="absolute bottom-6 -translate-x-1/2 whitespace-nowrap px-2 py-0.5 rounded text-[10.5px] font-semibold tabular-nums bg-[#2962FF] text-white shadow-lg"
              >
                ✂ Di chuyển chuột để chọn điểm cắt
              </div>
            </div>
          </div>
        )}

        {loading && (
          <div
            className="pointer-events-none absolute bottom-8 left-3 z-10 px-2 py-1 rounded text-[11px]"
            style={{
              backgroundColor: 'var(--tv-bg-panel)',
              color: 'var(--tv-text-secondary)',
              border: '1px solid var(--tv-border)',
            }}
          >
            Đang nạp dữ liệu {pane.ticker}...
          </div>
        )}

        {/* KLineChart v10 Canvas Container */}
        <div ref={containerRef} className="w-full h-full min-w-0 min-h-0" />
      </div>

      {/* Custom Right-Click Context Menu (fixes BUG-012!) */}
      {contextMenu && (
        <ChartContextMenu
          menu={contextMenu}
          ticker={pane.ticker}
          onClose={() => setContextMenu(null)}
          onResetView={handleResetView}
          onReplayCutHere={() => {
            if (contextMenu.timestamp === null) return
            if (replay?.active) {
              void jumpReplay(contextMenu.timestamp)
            } else {
              void startReplay(contextMenu.timestamp)
            }
          }}
          onCreateAlertAtPrice={() => {
            if (contextMenu.price === null) return
            void createNewAlert(
              pane.ticker,
              'cross_any',
              Number(contextMenu.price.toFixed(2)),
              `Cảnh báo giá ${pane.ticker} chạm ${contextMenu.priceLabel}`,
            )
          }}
          onBuyLimitAtPrice={() => {
            if (contextMenu.price === null) return
            void submitQuickLimitOrder(pane.ticker, pane.interval, 'BUY', contextMenu.price)
          }}
          onSellLimitAtPrice={() => {
            if (contextMenu.price === null) return
            void submitQuickLimitOrder(pane.ticker, pane.interval, 'SELL', contextMenu.price)
          }}
          onDeleteSelectedDrawing={() => {
            if (selectedDrawingRef.current) {
              deleteSelectedDrawingById(selectedDrawingRef.current.id)
            }
          }}
          onClearAllDrawings={() => {
            chartRef.current?.removeOverlay({ groupId: 'user_drawings' })
            setSelectedDrawing(null)
            undoStackRef.current = []
            void api.clearDrawings(pane.ticker)
            pushToast('info', `Đã xóa toàn bộ hình vẽ trên ${pane.ticker}`)
          }}
        />
      )}
    </div>
  )
}
