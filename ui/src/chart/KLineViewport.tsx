import React, { useCallback, useEffect, useRef, useState } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { dispose, init } from 'klinecharts'
import type { Chart, KLineData, Overlay, OverlayEvent } from 'klinecharts'
import * as api from '../api/tauri'
import type {
  ChartPaneConfig,
  CorporateEventMarker,
  GridLayoutKind,
  IntervalKey,
  SavedDrawingRecord,
  VnBandLevels,
} from '../types/market'
import { useWorkspaceStore, type TimezoneMode } from '../store/workspaceStore'
import { crosshairBus, timeRangeBus } from './crosshairBus'
import { drawingSyncBus } from './drawingSyncBus'
import { toHeikinAshi } from './heikinAshi'
import { formatBarCountdown, intervalToKLinePeriod } from './periods'
import { buildIndicatorTooltipFeatures, buildSuperchartsStyles } from './themes'
import {
  clearAfcForecastCache,
  hasVnindexBenchmarkBars,
  setVnindexBenchmarkBars,
} from './indicators/customIndicators'
import { COMPARE_COLORS, setCompareSeriesCache } from './indicators/fundamentalIndicators'
import { registerAllCustomIndicators } from './indicators/registerAll'
import { registerAllCustomOverlays } from './overlays/registerAll'
import {
  FloatingDrawingToolbar,
  type DrawingLineStyleKind,
  type SelectedDrawingState,
} from '../components/FloatingDrawingToolbar'
import { ChartContextMenu, type ChartContextMenuState } from '../components/ChartContextMenu'
import {
  computeVnSessionBreaks,
  isDrawingVisibleOnInterval,
  isVietnamTicker,
  snapPriceToVenueTick,
} from '../utils/vnMarket'

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
  | { kind: 'modified'; before: SavedDrawingRecord; after: SavedDrawingRecord }

function resolveEffectiveTimezone(
  mode: TimezoneMode,
  ticker: string,
  symbolTimezone?: string | null,
): string {
  if (mode === 'Asia/Ho_Chi_Minh' || mode === 'Etc/UTC') return mode
  const isVn = ticker.startsWith('HOSE:') || ticker.startsWith('HNX:') || ticker.startsWith('UPCOM:')
  if (symbolTimezone && symbolTimezone.includes('/')) {
    return symbolTimezone
  }
  return isVn ? 'Asia/Ho_Chi_Minh' : 'Etc/UTC'
}

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
  // Track previous CLOSED HA bar separately from the currently forming HA bar so intra-bar ticks
  // never drift `HA_Open` (`BUG-015`).
  const prevClosedHaBarRef = useRef<{ timestamp: number; open: number; close: number } | undefined>(undefined)
  const currentHaBarRef = useRef<{ timestamp: number; open: number; close: number } | undefined>(undefined)
  const appliedPaneRef = useRef<{ ticker: string; interval: string; adjustment: string }>({
    ticker: pane.ticker,
    interval: pane.interval,
    adjustment: pane.adjustment ?? 'splits',
  })
  const symbolTimezoneRef = useRef<string | null>(null)
  const precisionSyncedTickerRef = useRef<string | null>(null)
  const createdIndicatorIdsRef = useRef<Map<string, { klineId: string; paneId: string; signature: string }>>(
    new Map(),
  )
  const inProgressDrawingIdRef = useRef<string | null>(null)
  const dragBeforeRef = useRef<Map<string, SavedDrawingRecord>>(new Map())
  const undoStackRef = useRef<UndoEntry[]>([])
  const redoStackRef = useRef<UndoEntry[]>([])
  const historyLimitRef = useRef<number>(600)
  const pendingScrollTsRef = useRef<number | null>(null)

  const [vnBand, setVnBand] = useState<VnBandLevels | null>(null)
  const [corpEvents, setCorpEvents] = useState<CorporateEventMarker[]>([])
  const [activeCorpEvent, setActiveCorpEvent] = useState<CorporateEventMarker | null>(null)
  const [dataSourceBadge, setDataSourceBadge] = useState<string>('')
  const [loading, setLoading] = useState<boolean>(false)
  const [dataReadyNonce, setDataReadyNonce] = useState<number>(0)
  const [selectedDrawing, setSelectedDrawing] = useState<SelectedDrawingState | null>(null)
  const [contextMenu, setContextMenu] = useState<ChartContextMenuState | null>(null)

  const theme = useWorkspaceStore((s) => s.theme)
  const timezoneMode = useWorkspaceStore((s) => s.timezoneMode)
  const activeDrawingTool = useWorkspaceStore((s) => s.activeDrawingTool)
  const magnetMode = useWorkspaceStore((s) => s.magnetMode)
  const drawingsLocked = useWorkspaceStore((s) => s.drawingsLocked)
  const drawingsHidden = useWorkspaceStore((s) => s.drawingsHidden)
  const clearDrawingsNonce = useWorkspaceStore((s) => s.clearDrawingsNonce)
  const showVnSessionBreaks = useWorkspaceStore((s) => s.showVnSessionBreaks)
  const exportPngNonce = useWorkspaceStore((s) => s.exportPngNonce)
  const scrollToTimestampCmd = useWorkspaceStore((s) => s.scrollToTimestampCmd)
  const backtestOverlay = useWorkspaceStore((s) => s.backtestOverlay)
  const replay = useWorkspaceStore((s) => s.replay)
  const replaySelectingCut = useWorkspaceStore((s) => s.replaySelectingCut)
  const replayBlindMode = useWorkspaceStore((s) => s.replayBlindMode)
  const replayResetNonce = useWorkspaceStore((s) => s.replayResetNonce)
  const replayStepEvent = useWorkspaceStore((s) => s.replayStepEvent)
  const broker = useWorkspaceStore((s) => s.broker)
  const yumekoAsOf = useWorkspaceStore((s) => s.yumekoAsOf)
  const yumekoSeriesVersion = useWorkspaceStore((s) => s.yumekoSeriesVersion)
  const fundamentalsVersion = useWorkspaceStore((s) => s.fundamentalsVersion)
  const compareTickers = useWorkspaceStore((s) => s.compareTickersBySlot[pane.slotId])
  const compareVersion = useWorkspaceStore((s) => s.compareVersion)
  const removeCompareTicker = useWorkspaceStore((s) => s.removeCompareTicker)
  const loadFundamentalsForTicker = useWorkspaceStore((s) => s.loadFundamentalsForTicker)

  const setActiveInterval = useWorkspaceStore((s) => s.setActiveInterval)
  const togglePaneAdjustment = useWorkspaceStore((s) => s.togglePaneAdjustment)
  const setActiveDrawingTool = useWorkspaceStore((s) => s.setActiveDrawingTool)
  const setReplaySelectingCut = useWorkspaceStore((s) => s.setReplaySelectingCut)
  const startReplay = useWorkspaceStore((s) => s.startReplay)
  const jumpReplay = useWorkspaceStore((s) => s.jumpReplay)
  const updateIndicatorInActivePane = useWorkspaceStore((s) => s.updateIndicatorInActivePane)
  const removeIndicatorFromActivePane = useWorkspaceStore((s) => s.removeIndicatorFromActivePane)
  const openIndicatorSettings = useWorkspaceStore((s) => s.openIndicatorSettings)
  const createNewAlert = useWorkspaceStore((s) => s.createNewAlert)
  const checkLiveBarAlerts = useWorkspaceStore((s) => s.checkLiveBarAlerts)
  const submitQuickLimitOrder = useWorkspaceStore((s) => s.submitQuickLimitOrder)
  const submitQuickBracket = useWorkspaceStore((s) => s.submitQuickBracket)
  const setBrokerSnapshot = useWorkspaceStore((s) => s.setBrokerSnapshot)
  const bumpDrawingsVersion = useWorkspaceStore((s) => s.bumpDrawingsVersion)
  const pushToast = useWorkspaceStore((s) => s.pushToast)
  const lastLiveAlertTsRef = useRef<number>(0)

  const isThisPaneReplaying = Boolean(
    replay?.active && replay.ticker.toUpperCase() === pane.ticker.toUpperCase(),
  )
  const replaySelectingCutRef = useRef(replaySelectingCut)
  replaySelectingCutRef.current = replaySelectingCut
  const replayActiveRef = useRef(isThisPaneReplaying)
  replayActiveRef.current = isThisPaneReplaying
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
    drawingSyncBus.emit({
      kind: 'upsert',
      sourceSlotId: paneRef.current.slotId,
      ticker: paneRef.current.ticker,
      record,
    })
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
      onPressedMoveStart: (ev: OverlayEvent<unknown>) => {
        if (ev.overlay.groupId !== 'user_drawings') return
        dragBeforeRef.current.set(ev.overlay.id, overlayToRecord(ev.overlay, paneRef.current.ticker))
      },
      onPressedPointMoveStart: (ev: OverlayEvent<unknown>) => {
        if (ev.overlay.groupId !== 'user_drawings') return
        dragBeforeRef.current.set(ev.overlay.id, overlayToRecord(ev.overlay, paneRef.current.ticker))
      },
      onPressedMoveEnd: (ev: OverlayEvent<unknown>) => {
        if (ev.overlay.groupId === 'user_drawings') {
          const before = dragBeforeRef.current.get(ev.overlay.id)
          dragBeforeRef.current.delete(ev.overlay.id)
          const after = overlayToRecord(ev.overlay, paneRef.current.ticker)
          if (before) {
            undoStackRef.current = [...undoStackRef.current.slice(-29), { kind: 'modified', before, after }]
            redoStackRef.current = []
          }
        }
        persistOverlayToDb(ev)
        if (selectedDrawingRef.current?.id === ev.overlay.id) {
          setSelectedDrawing(extractSelectedState(ev.overlay))
        }
      },
      onPressedPointMoveEnd: (ev: OverlayEvent<unknown>) => {
        if (ev.overlay.groupId === 'user_drawings') {
          const before = dragBeforeRef.current.get(ev.overlay.id)
          dragBeforeRef.current.delete(ev.overlay.id)
          const after = overlayToRecord(ev.overlay, paneRef.current.ticker)
          if (before) {
            undoStackRef.current = [...undoStackRef.current.slice(-29), { kind: 'modified', before, after }]
            redoStackRef.current = []
          }
        }
        persistOverlayToDb(ev)
        if (selectedDrawingRef.current?.id === ev.overlay.id) {
          setSelectedDrawing(extractSelectedState(ev.overlay))
        }
      },
      onRemoved: (ev: OverlayEvent<unknown>) => {
        if (ev.overlay.groupId === 'user_drawings') {
          void api.deleteDrawing(paneRef.current.ticker, ev.overlay.id)
          drawingSyncBus.emit({
            kind: 'delete',
            sourceSlotId: paneRef.current.slotId,
            ticker: paneRef.current.ticker,
            id: ev.overlay.id,
          })
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
              redoStackRef.current = []
              void api.upsertDrawing(rec)
              drawingSyncBus.emit({
                kind: 'upsert',
                sourceSlotId: paneRef.current.slotId,
                ticker: paneRef.current.ticker,
                record: rec,
              })
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
      gap: { top: 0.24, bottom: 0.12 },
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
      gap: { top: 0.24, bottom: 0.12 },
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
      redoStackRef.current = []
    }
    chart.removeOverlay({ id, groupId: 'user_drawings' })
    void api.deleteDrawing(paneRef.current.ticker, id)
    drawingSyncBus.emit({
      kind: 'delete',
      sourceSlotId: paneRef.current.slotId,
      ticker: paneRef.current.ticker,
      id,
    })
    setSelectedDrawing(null)
  }, [])

  const handleUndoDrawing = useCallback(() => {
    const chart = chartRef.current
    const stack = undoStackRef.current
    if (!chart || stack.length === 0) return
    const entry = stack.pop()
    if (!entry) return
    redoStackRef.current = [...redoStackRef.current.slice(-29), entry]

    if (entry.kind === 'created') {
      chart.removeOverlay({ id: entry.record.id, groupId: 'user_drawings' })
      void api.deleteDrawing(paneRef.current.ticker, entry.record.id)
      drawingSyncBus.emit({
        kind: 'delete',
        sourceSlotId: paneRef.current.slotId,
        ticker: paneRef.current.ticker,
        id: entry.record.id,
      })
      setSelectedDrawing(null)
      pushToast('info', 'Đã hoàn tác hình vừa vẽ (Ctrl/Cmd+Z)')
    } else if (entry.kind === 'deleted') {
      recreateOverlayFromRecord(entry.record)
      void api.upsertDrawing(entry.record)
      drawingSyncBus.emit({
        kind: 'upsert',
        sourceSlotId: paneRef.current.slotId,
        ticker: paneRef.current.ticker,
        record: entry.record,
      })
      pushToast('info', 'Đã khôi phục hình vừa xóa (Ctrl/Cmd+Z)')
    } else if (entry.kind === 'modified') {
      chart.overrideOverlay({
        id: entry.before.id,
        styles: entry.before.styles ?? undefined,
        lock: entry.before.lock,
        points: entry.before.points,
      })
      void api.upsertDrawing(entry.before)
      drawingSyncBus.emit({
        kind: 'upsert',
        sourceSlotId: paneRef.current.slotId,
        ticker: paneRef.current.ticker,
        record: entry.before,
      })
      const [restored] = chart.getOverlays({ id: entry.before.id, groupId: 'user_drawings' })
      if (restored) setSelectedDrawing(extractSelectedState(restored))
      pushToast('info', 'Đã hoàn tác chỉnh sửa hình vẽ (Ctrl/Cmd+Z)')
    }
  }, [pushToast, recreateOverlayFromRecord])

  const handleRedoDrawing = useCallback(() => {
    const chart = chartRef.current
    const stack = redoStackRef.current
    if (!chart || stack.length === 0) return
    const entry = stack.pop()
    if (!entry) return
    undoStackRef.current = [...undoStackRef.current.slice(-29), entry]

    if (entry.kind === 'created') {
      recreateOverlayFromRecord(entry.record)
      void api.upsertDrawing(entry.record)
      drawingSyncBus.emit({
        kind: 'upsert',
        sourceSlotId: paneRef.current.slotId,
        ticker: paneRef.current.ticker,
        record: entry.record,
      })
      pushToast('info', 'Đã làm lại hình vẽ (Cmd+Shift+Z)')
    } else if (entry.kind === 'deleted') {
      chart.removeOverlay({ id: entry.record.id, groupId: 'user_drawings' })
      void api.deleteDrawing(paneRef.current.ticker, entry.record.id)
      drawingSyncBus.emit({
        kind: 'delete',
        sourceSlotId: paneRef.current.slotId,
        ticker: paneRef.current.ticker,
        id: entry.record.id,
      })
      setSelectedDrawing(null)
      pushToast('info', 'Đã làm lại thao tác xóa (Cmd+Shift+Z)')
    } else if (entry.kind === 'modified') {
      chart.overrideOverlay({
        id: entry.after.id,
        styles: entry.after.styles ?? undefined,
        lock: entry.after.lock,
        points: entry.after.points,
      })
      void api.upsertDrawing(entry.after)
      drawingSyncBus.emit({
        kind: 'upsert',
        sourceSlotId: paneRef.current.slotId,
        ticker: paneRef.current.ticker,
        record: entry.after,
      })
      const [restored] = chart.getOverlays({ id: entry.after.id, groupId: 'user_drawings' })
      if (restored) setSelectedDrawing(extractSelectedState(restored))
      pushToast('info', 'Đã làm lại chỉnh sửa hình vẽ (Cmd+Shift+Z)')
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
    const initialTzMode = useWorkspaceStore.getState().timezoneMode
    const prec = guessPrecision(initialPane.ticker)

    const chart = init(el, {
      timezone: resolveEffectiveTimezone(initialTzMode, initialPane.ticker),
      styles: buildSuperchartsStyles(
        initialTheme,
        initialPane.chartStyle,
        initialPane.showCountdown,
        initialPane.interval === '1h' ? '1H' : initialPane.interval === '4h' ? '4H' : initialPane.interval,
      ),
      layout: {
        barSpaceLimit: { min: 2, max: 46 },
        pane: { height: 112, minHeight: 54 },
        yAxis: { position: 'right', inside: false, scrollZoomEnabled: true, gap: { top: 0.24, bottom: 0.12 } },
      },
      thousandsSeparator: { sign: ',' },
      formatter: {
        formatDate: ({ dateTimeFormat, timestamp, type }) => {
          const isDailyOrCoarser = ['1D', '1W', '1M'].includes(paneRef.current.interval)
          const parts = dateTimeFormat.formatToParts(new Date(timestamp))
          const get = (k: string) => parts.find((p) => p.type === k)?.value ?? ''
          const y = get('year')
          const m = get('month')
          const d = get('day')
          const hh = get('hour')
          const mm = get('minute')
          // Never show spurious `"07:00"` on 1D/1W/1M crosshair or drawing anchor labels (`BUG-038`)!
          if (isDailyOrCoarser) {
            return type === 'xAxis' ? `${m}-${d}` : `${y}-${m}-${d}`
          }
          return `${y}-${m}-${d} ${hh}:${mm}`
        },
        formatExtendText: ({ data }) => {
          if (!paneRef.current.showCountdown) return ''
          if (replayActiveRef.current) return 'REPLAY'
          return formatBarCountdown(data.timestamp, paneRef.current.interval)
        },
      },
    })

    if (!chart) return
    let disposed = false
    chartRef.current = chart
    // Provide generous right-hand whitespace (~24 bars) for AFC Probability Cone & QMV Ichimoku Future Kumo
    chart.setOffsetRightDistance(215)
    crosshairBus.register(initialPane.slotId, chart)
    timeRangeBus.register(initialPane.slotId, chart)
    // Namespace stream ID by OS window label (`main` vs `chart_*`) so detached windows never cancel main streams (Audit Fix #2).
    let winLabel = 'main'
    if (api.isTauriRuntime()) {
      try {
        winLabel = getCurrentWindow().label || 'main'
      } catch {
        winLabel = 'main'
      }
    }
    const streamId = `stream_${winLabel}_${initialPane.slotId}`

    if (!hasVnindexBenchmarkBars()) {
      void api
        .getBars({ ticker: 'HOSE:VNINDEX', interval: '1D', mode: 'init', limit: 2500 })
        .then((benchPage) => {
          if (benchPage.bars.length > 0) {
            setVnindexBenchmarkBars(benchPage.bars)
          }
        })
        .catch(() => {})
    }

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
            adjustment: cur.adjustment ?? 'splits',
            asOfDate: useWorkspaceStore.getState().yumekoAsOf,
          })
          if (disposed || !chartRef.current) return
          // Update VN price band & badge ONLY on `init` loads so scrolling back in history
          // (`forward`) NEVER overwrites today's Ceiling/Reference/Floor band (`BUG-005`)!
          if (queryMode === 'init') {
            setVnBand(page.vnBand)
            setDataSourceBadge(
              page.replayActive
                ? 'REPLAY'
                : page.source === 'yumeko_dnse_pit'
                  ? `PIT · DNSE`
                  : page.source === 'cache_hit_dnse_fresh'
                    ? 'SQLite+DNSE'
                    : page.source === 'cache_hit'
                      ? 'SQLite'
                      : page.source === 'offline_cache'
                        ? 'Offline'
                        : 'Live',
            )
          }

          let bars: KLineData[] = page.bars
          if (cur.chartStyle === 'heikin_ashi') {
            bars = toHeikinAshi(page.bars)
            if (queryMode === 'init') {
              const last = bars[bars.length - 1]
              const prev = bars[bars.length - 2]
              currentHaBarRef.current = last
                ? { timestamp: last.timestamp, open: last.open, close: last.close }
                : undefined
              prevClosedHaBarRef.current = prev
                ? { timestamp: prev.timestamp, open: prev.open, close: prev.close }
                : undefined
            }
          } else if (queryMode === 'init') {
            currentHaBarRef.current = undefined
            prevClosedHaBarRef.current = undefined
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

            // Synchronize authoritative precision & exchange timezone once per ticker without loops (Audit Fix #3)
            if (page.symbolInfo && chartRef.current) {
              if (page.symbolInfo.timezone && page.symbolInfo.timezone.includes('/')) {
                symbolTimezoneRef.current = page.symbolInfo.timezone
                const tzMode = useWorkspaceStore.getState().timezoneMode
                if (tzMode === 'auto') {
                  chartRef.current.setTimezone(
                    resolveEffectiveTimezone('auto', cur.ticker, page.symbolInfo.timezone),
                  )
                }
              }
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
        if (disposed) return
        subscribeCallbackRef.current = callback
        const cur = paneRef.current
        if (replayActiveRef.current) return
        void api.subscribeBars(
          streamId,
          cur.ticker,
          cur.interval,
          (payload) => {
            if (payload.ticker !== paneRef.current.ticker || payload.interval !== paneRef.current.interval) {
              return
            }
            if (replayActiveRef.current) return
            let bar: KLineData = payload.bar
            if (paneRef.current.chartStyle === 'heikin_ashi') {
              // Keep `HA_Open` locked to `prevClosedHaBarRef` on intra-bar ticks (`BUG-015`)!
              const curHa = currentHaBarRef.current
              if (curHa && bar.timestamp > curHa.timestamp) {
                prevClosedHaBarRef.current = curHa
              }
              const [ha] = toHeikinAshi([bar], prevClosedHaBarRef.current)
              if (ha) {
                bar = ha
                currentHaBarRef.current = { timestamp: ha.timestamp, open: ha.open, close: ha.close }
              }
            }
            callback(bar)
            if (bar.timestamp > lastLiveAlertTsRef.current) {
              lastLiveAlertTsRef.current = bar.timestamp
              void checkLiveBarAlerts(payload.ticker, paneRef.current.interval)
            }
          },
          cur.adjustment ?? 'splits',
        )
      },
      unsubscribeBar: () => {
        subscribeCallbackRef.current = null
        void api.unsubscribeBars(streamId)
      },
    })

    // Zero-React-re-render Crosshair sync via CrosshairBus (`BUG-004` fixed for klinecharts@10.1.1!):
    // `klinecharts@10.1.1` emits raw `{ x, y, paneId }` without `kLineData`, so resolve timestamp via `convertFromPixel`.
    chart.subscribeAction('onCrosshairChange', (data) => {
      if (crosshairBus.isSuppressed(paneRef.current.slotId)) return
      const payload = data as { x?: number; y?: number; kLineData?: KLineData } | null
      if (!payload) {
        crosshairBus.publish(paneRef.current.slotId, null)
        return
      }
      let ts: number | null = payload.kLineData?.timestamp ?? null
      if (ts === null && typeof payload.x === 'number' && Number.isFinite(payload.x)) {
        const conv = chart.convertFromPixel([{ x: payload.x }], { paneId: 'candle_pane' })
        const pt = Array.isArray(conv) ? conv[0] : conv
        if (typeof pt?.timestamp === 'number' && Number.isFinite(pt.timestamp)) {
          ts = pt.timestamp
        } else if (typeof pt?.dataIndex === 'number' && Number.isFinite(pt.dataIndex)) {
          const list = chart.getDataList()
          const idx = Math.max(0, Math.min(list.length - 1, Math.round(pt.dataIndex)))
          ts = list[idx]?.timestamp ?? null
        }
      }
      crosshairBus.publish(paneRef.current.slotId, ts)
    })

    // Zero-React-re-render Time-Range & Zoom (`barSpace`) sync (`Sync Date Range`, Audit Fix #2)
    chart.subscribeAction('onVisibleRangeChange', () => {
      if (timeRangeBus.isSuppressed(paneRef.current.slotId)) return
      const range = chart.getVisibleRange()
      const list = chart.getDataList()
      if (!list.length) return
      const midIdx = Math.max(0, Math.min(list.length - 1, Math.floor((range.from + range.to) / 2)))
      const midTs = list[midIdx]?.timestamp
      if (typeof midTs === 'number' && Number.isFinite(midTs)) {
        const barSpace = chart.getBarSpace()?.bar ?? null
        timeRangeBus.publish(paneRef.current.slotId, midTs, barSpace)
      }
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
      disposed = true
      ro.disconnect()
      crosshairBus.unregister(initialPane.slotId)
      timeRangeBus.unregister(initialPane.slotId)
      void api.unsubscribeBars(streamId)
      subscribeCallbackRef.current = null
      createdIndicatorIdsRef.current.clear()
      dispose(el)
      chartRef.current = null
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  // 2. React to symbol, interval, or adjustment (`ADJ`/`RAW`) changes
  const curAdj = pane.adjustment ?? 'splits'
  useEffect(() => {
    const chart = chartRef.current
    if (!chart) return
    const prev = appliedPaneRef.current
    const tickerChanged = prev.ticker !== pane.ticker
    const intervalChanged = prev.interval !== pane.interval
    const adjChanged = prev.adjustment !== curAdj
    if (!tickerChanged && !intervalChanged && !adjChanged) return

    appliedPaneRef.current = { ticker: pane.ticker, interval: pane.interval, adjustment: curAdj }
    historyLimitRef.current = 600

    if (tickerChanged) {
      precisionSyncedTickerRef.current = null
      symbolTimezoneRef.current = null
      setActiveCorpEvent(null)
      setSelectedDrawing(null)
      undoStackRef.current = []
      redoStackRef.current = []
      const prec = guessPrecision(pane.ticker)
      chart.setTimezone(resolveEffectiveTimezone(useWorkspaceStore.getState().timezoneMode, pane.ticker))
      chart.setSymbol({
        ticker: pane.ticker,
        pricePrecision: prec.pricePrecision,
        volumePrecision: prec.volumePrecision,
      })
    }
    if (intervalChanged) {
      chart.setPeriod(intervalToKLinePeriod(pane.interval))
    }
    if (adjChanged && !tickerChanged && !intervalChanged) {
      chart.resetData()
    }
  }, [pane.ticker, pane.interval, curAdj])

  // 2b. React to manual Timezone switcher (`Auto` -> `Asia/Ho_Chi_Minh` -> `Etc/UTC`, Audit Fix #3)
  useEffect(() => {
    const chart = chartRef.current
    if (!chart) return
    chart.setTimezone(resolveEffectiveTimezone(timezoneMode, pane.ticker, symbolTimezoneRef.current))
  }, [timezoneMode, pane.ticker])

  // 2c. Live 1-second Bar Countdown repaint timer (`BUG-024`)
  // Uses `UpdateLevel.Drawer = 3` on `candle_pane` so the Y-axis last-price countdown badge redraws every second.
  useEffect(() => {
    if (!pane.showCountdown || isThisPaneReplaying) return
    const timer = window.setInterval(() => {
      const chart = chartRef.current as
        | (Chart & { updatePane?: (level?: number, paneId?: string) => void })
        | null
      if (!chart) return
      if (typeof chart.updatePane === 'function') {
        chart.updatePane(3, 'candle_pane')
      }
    }, 1000)
    return () => window.clearInterval(timer)
  }, [pane.showCountdown, isThisPaneReplaying])

  const dataStatusTotalBars = useWorkspaceStore((s) => s.dataStatus?.stats.total_bars ?? 0)
  const prevTotalBarsRef = useRef<number>(dataStatusTotalBars)
  useEffect(() => {
    if (prevTotalBarsRef.current !== dataStatusTotalBars) {
      prevTotalBarsRef.current = dataStatusTotalBars
      clearAfcForecastCache()
      const chart = chartRef.current as
        | (Chart & { updatePane?: (level?: number, paneId?: string) => void })
        | null
      chart?.updatePane?.(3, 'candle_pane')
    }
  }, [dataStatusTotalBars])

  // 2d. Reload bars whenever Yumeko Point-in-Time `as_of` lock changes (Decision #3)
  const prevYumekoAsOfRef = useRef<string | null>(yumekoAsOf)
  useEffect(() => {
    if (prevYumekoAsOfRef.current === yumekoAsOf) return
    prevYumekoAsOfRef.current = yumekoAsOf
    chartRef.current?.resetData()
  }, [yumekoAsOf])

  // 3. React to theme, chart style, Heikin-Ashi, interval label, and countdown toggle
  const prevStyleRef = useRef(pane.chartStyle)
  useEffect(() => {
    const chart = chartRef.current
    if (!chart) return
    const ivLabel = pane.interval === '1h' ? '1H' : pane.interval === '4h' ? '4H' : pane.interval
    chart.setStyles(buildSuperchartsStyles(theme, pane.chartStyle, pane.showCountdown, ivLabel))
    const toggledHeikin =
      (prevStyleRef.current === 'heikin_ashi') !== (pane.chartStyle === 'heikin_ashi')
    prevStyleRef.current = pane.chartStyle
    if (toggledHeikin) {
      chart.resetData()
    }
  }, [theme, pane.chartStyle, pane.showCountdown, pane.interval])

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
      const isYumekoRealInd =
        spec.name === 'QMV_STOCK_BB' ||
        spec.name === 'QMV_ACTIVE_MONEY' ||
        spec.name === 'QMV_FOREIGN_FLOW'
      const isFinInd =
        spec.name === 'FIN_PE' ||
        spec.name === 'FIN_PB' ||
        spec.name === 'FIN_ROE' ||
        spec.name === 'FIN_EPS_YOY'
      const mergedStyles = {
        ...(spec.styles ?? {}),
        tooltip: {
          features: buildIndicatorTooltipFeatures(theme, spec.visible),
        },
      }
      const extendData = isYumekoRealInd || isFinInd ? { ticker: pane.ticker } : undefined
      const signature = JSON.stringify({
        name: spec.name,
        calcParams: spec.calcParams,
        visible: spec.visible,
        isMain: spec.isMain,
        styles: spec.styles ?? null,
        theme,
        qmvVer: isYumekoRealInd ? `${pane.ticker}:${yumekoSeriesVersion}` : 0,
        fundaVer: isFinInd ? `${pane.ticker}:${fundamentalsVersion}` : 0,
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
            extendData,
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
          extendData,
          styles: mergedStyles as Record<string, unknown>,
        })
        activeMap.set(spec.id, { ...existing, signature })
      }
    }
  }, [pane.indicators, pane.ticker, isMultiPane, theme, yumekoSeriesVersion, fundamentalsVersion])

  // 5a2. Ensure fundamentals cache is hydrated whenever any `FIN_*` indicator is active on this pane (Wave 4)
  useEffect(() => {
    const hasFin = pane.indicators.some(
      (i) => i.name === 'FIN_PE' || i.name === 'FIN_PB' || i.name === 'FIN_ROE' || i.name === 'FIN_EPS_YOY',
    )
    if (hasFin && pane.ticker) {
      void loadFundamentalsForTicker(pane.ticker)
    }
  }, [pane.indicators, pane.ticker, loadFundamentalsForTicker])

  // 5a3. Multi-Symbol Causal Rebased Comparison Overlay (`COMPARE_OVERLAY`, Wave 4)
  useEffect(() => {
    const chart = chartRef.current
    if (!chart) return
    const cmpList = compareTickers ?? []
    const cmpIndId = `compare_overlay_${pane.slotId}`
    if (cmpList.length === 0) {
      setCompareSeriesCache(pane.slotId, null)
      chart.removeIndicator({ id: cmpIndId, paneId: 'candle_pane' })
      return
    }
    let cancelled = false
    void api
      .getCompareSeries({
        baseTicker: pane.ticker,
        compareTickers: cmpList,
        interval: pane.interval,
        limit: historyLimitRef.current,
        asOfDate: yumekoAsOf,
      })
      .then((resp) => {
        if (cancelled || !chartRef.current) return
        setCompareSeriesCache(pane.slotId, resp)
        const existing = chartRef.current.getIndicators({ id: cmpIndId, paneId: 'candle_pane' })
        if (existing.length === 0) {
          chartRef.current.createIndicator(
            {
              id: cmpIndId,
              name: 'COMPARE_OVERLAY',
              paneId: 'candle_pane',
              extendData: { slotId: pane.slotId, nonce: Date.now() },
            },
            true,
          )
        } else {
          chartRef.current.overrideIndicator({
            id: cmpIndId,
            name: 'COMPARE_OVERLAY',
            paneId: 'candle_pane',
            extendData: { slotId: pane.slotId, nonce: Date.now() },
          })
        }
      })
      .catch(() => {})
    return () => {
      cancelled = true
    }
  }, [compareTickers, compareVersion, pane.ticker, pane.interval, pane.slotId, yumekoAsOf, dataReadyNonce, replayResetNonce])

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

  // 6. Load & restore persistent SQLite drawings (filtering future-anchored drawings in Blind Replay — `BUG-042`).
  // Depends on `replayResetNonce` (start/cut/rewind/stop) instead of per-step `cutoffTimestampMs` so
  // stepping forward at 20x NEVER flickers or tears down drawings on every bar.
  useEffect(() => {
    const chart = chartRef.current
    if (!chart) return
    let cancelled = false
    const repState = useWorkspaceStore.getState().replay
    const cutoffTs =
      isThisPaneReplaying && replayBlindMode ? (repState?.cutoffTimestampMs ?? null) : null
    chart.removeOverlay({ groupId: 'user_drawings' })

    void api.getDrawings(pane.ticker).then((records) => {
      if (cancelled || !chartRef.current) return
      for (const rec of records) {
        if (!isDrawingVisibleOnInterval(rec.extendData, pane.interval)) {
          continue
        }
        if (cutoffTs !== null) {
          const hasFuturePoint = rec.points.some(
            (pt) => typeof pt.timestamp === 'number' && pt.timestamp > cutoffTs,
          )
          if (hasFuturePoint) continue
        }
        recreateOverlayFromRecord(rec)
      }
    })
    return () => {
      cancelled = true
    }
  }, [pane.ticker, pane.interval, isThisPaneReplaying, replayBlindMode, replayResetNonce, recreateOverlayFromRecord])

  // 6b. Real-time Multi-Pane & Multi-Window Drawing Sync (`BUG-027` + `BUG-043` + Object Tree v2.2)
  useEffect(() => {
    return drawingSyncBus.subscribe(pane.slotId, (ev) => {
      const chart = chartRef.current
      if (!chart || ev.ticker.toUpperCase() !== paneRef.current.ticker.toUpperCase()) return
      if (ev.kind === 'upsert') {
        const visibleOnTf = isDrawingVisibleOnInterval(ev.record.extendData, paneRef.current.interval)
        const [existing] = chart.getOverlays({ id: ev.record.id, groupId: 'user_drawings' })
        if (!visibleOnTf) {
          if (existing) {
            chart.removeOverlay({ id: ev.record.id, groupId: 'user_drawings' })
          }
          return
        }
        if (existing) {
          chart.overrideOverlay({
            id: ev.record.id,
            points: ev.record.points,
            styles: ev.record.styles ?? undefined,
            extendData: ev.record.extendData ?? undefined,
            lock: ev.record.lock || useWorkspaceStore.getState().drawingsLocked,
            visible: useWorkspaceStore.getState().drawingsHidden ? false : ev.record.visible,
          })
        } else {
          recreateOverlayFromRecord(ev.record)
        }
      } else if (ev.kind === 'delete') {
        chart.removeOverlay({ id: ev.id, groupId: 'user_drawings' })
        if (selectedDrawingRef.current?.id === ev.id) {
          setSelectedDrawing(null)
        }
      } else if (ev.kind === 'clear') {
        chart.removeOverlay({ groupId: 'user_drawings' })
        setSelectedDrawing(null)
      }
    })
  }, [pane.slotId, recreateOverlayFromRecord])

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

  // 9. Clear all user drawings on active pane (and mirror to sibling panes via drawingSyncBus)
  useEffect(() => {
    if (clearDrawingsNonce === 0 || !isActive || !chartRef.current) return
    chartRef.current.removeOverlay({ groupId: 'user_drawings' })
    setSelectedDrawing(null)
    undoStackRef.current = []
    redoStackRef.current = []
    void api.clearDrawings(pane.ticker)
    drawingSyncBus.emit({ kind: 'clear', sourceSlotId: pane.slotId, ticker: pane.ticker })
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

  // 10b. Render Vietnam Intraday Session Breaks (`09:00` Morning / `13:00` Afternoon / `14:30` ATC, includes `30m` Audit Fix #3)
  useEffect(() => {
    const chart = chartRef.current
    if (!chart) return
    chart.removeOverlay({ groupId: 'vn_session_breaks' })
    if (!showVnSessionBreaks || !isVietnamTicker(pane.ticker)) return
    if (!['1m', '5m', '15m', '30m', '1h'].includes(pane.interval)) return

    const bars = chart.getDataList()
    const markers = computeVnSessionBreaks(bars, pane.interval)
    for (let i = 0; i < markers.length; i++) {
      const m = markers[i]
      chart.createOverlay({
        id: `vn_sess_${pane.slotId}_${i}`,
        name: 'vnSessionBreakLine',
        groupId: 'vn_session_breaks',
        paneId: 'candle_pane',
        lock: true,
        points: [{ timestamp: m.timestamp }],
        extendData: { label: m.label, kind: m.kind },
      })
    }
  }, [showVnSessionBreaks, pane.ticker, pane.interval, pane.slotId, dataReadyNonce])

  // 10c. Fetch & Render Corporate Event Markers (`D` Cash Dividend / `S` Stock Split, v3.0 Wave 1)
  useEffect(() => {
    let cancelled = false
    if (!['1D', '1W', '1M'].includes(pane.interval)) {
      setCorpEvents([])
      return
    }
    void api
      .getCorporateEvents(pane.ticker)
      .then((events) => {
        if (!cancelled) setCorpEvents(events)
      })
      .catch(() => {
        if (!cancelled) setCorpEvents([])
      })
    return () => {
      cancelled = true
    }
  }, [pane.ticker, pane.interval, dataStatusTotalBars])

  useEffect(() => {
    const chart = chartRef.current
    if (!chart) return
    chart.removeOverlay({ groupId: 'corp_events' })
    if (!['1D', '1W', '1M'].includes(pane.interval) || corpEvents.length === 0) return

    const cutoffTs = isThisPaneReplaying ? (replay?.cutoffTimestampMs ?? null) : null
    const getEvTs = (e: CorporateEventMarker) => e.timestampMs ?? e.timestamp_ms ?? 0
    const visible = (cutoffTs !== null ? corpEvents.filter((e) => getEvTs(e) <= cutoffTs) : corpEvents).slice(-40)
    for (let i = 0; i < visible.length; i++) {
      const ev = visible[i]
      chart.createOverlay({
        id: `corp_ev_${pane.slotId}_${i}`,
        name: 'corporateEventMarker',
        groupId: 'corp_events',
        paneId: 'candle_pane',
        lock: true,
        points: [{ timestamp: getEvTs(ev) }],
        extendData: {
          kind: ev.kind,
          label: ev.label,
          detail: ev.detail,
          date: ev.date,
        },
      })
    }
  }, [corpEvents, pane.interval, pane.slotId, dataReadyNonce, isThisPaneReplaying, replay?.cutoffTimestampMs])

  // 11. Render Paper Broker open position & interactive Drag-to-Modify SL / TP / Pending LO lines (v2.2!)
  useEffect(() => {
    const chart = chartRef.current
    if (!chart) return
    chart.removeOverlay({ groupId: 'broker_orders' })
    if (!broker || broker.ticker !== pane.ticker) return

    const brokerIv = (broker.interval as IntervalKey) || pane.interval
    const isVnd = ['Hose', 'Hnx', 'Upcom', 'Vn30Futures'].includes(broker.venue)

    if (broker.openPosition) {
      const pos = broker.openPosition
      const mult = pos.contract_multiplier || broker.contractMultiplier || 1
      const unreal = (broker.markPrice - pos.avg_entry_price) * pos.quantity * mult * (pos.side === 'Long' ? 1 : -1)
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
        const origSl = pos.stop_loss
        const curTp = pos.take_profit ?? null
        chart.createOverlay({
          id: `pos_sl_${pane.slotId}`,
          name: 'brokerOrderLine',
          groupId: 'broker_orders',
          paneId: 'candle_pane',
          lock: false,
          points: [{ value: origSl }],
          extendData: {
            label: `⋮⋮ SL ${origSl.toLocaleString('en-US')}`,
            color: '#F23645',
            dashed: true,
            draggable: true,
            prefix: 'SL',
            entryPrice: pos.avg_entry_price,
            side: pos.side,
            qty: pos.quantity,
            mult,
            isVnd,
          },
          onPressedMoveEnd: (ev) => {
            const rawVal = ev.overlay.points[0]?.value
            if (typeof rawVal !== 'number' || !Number.isFinite(rawVal)) return false
            const snapped = snapPriceToVenueTick(broker.ticker, rawVal)
            if (Math.abs(snapped - origSl) < 1e-9) return false
            void api
              .brokerUpdateBrackets(broker.ticker, brokerIv, snapped, curTp)
              .then((updated) => {
                setBrokerSnapshot(updated)
                pushToast('success', `Đã kéo Cắt lỗ (SL) → ${snapped.toLocaleString('en-US')}`)
              })
              .catch((err) => {
                chartRef.current?.overrideOverlay({ id: `pos_sl_${pane.slotId}`, points: [{ value: origSl }] })
                pushToast('error', String(err))
              })
            return false
          },
        })
      }
      if (pos.take_profit) {
        const origTp = pos.take_profit
        const curSl = pos.stop_loss ?? null
        chart.createOverlay({
          id: `pos_tp_${pane.slotId}`,
          name: 'brokerOrderLine',
          groupId: 'broker_orders',
          paneId: 'candle_pane',
          lock: false,
          points: [{ value: origTp }],
          extendData: {
            label: `⋮⋮ TP ${origTp.toLocaleString('en-US')}`,
            color: '#089981',
            dashed: true,
            draggable: true,
            prefix: 'TP',
            entryPrice: pos.avg_entry_price,
            side: pos.side,
            qty: pos.quantity,
            mult,
            isVnd,
          },
          onPressedMoveEnd: (ev) => {
            const rawVal = ev.overlay.points[0]?.value
            if (typeof rawVal !== 'number' || !Number.isFinite(rawVal)) return false
            const snapped = snapPriceToVenueTick(broker.ticker, rawVal)
            if (Math.abs(snapped - origTp) < 1e-9) return false
            void api
              .brokerUpdateBrackets(broker.ticker, brokerIv, curSl, snapped)
              .then((updated) => {
                setBrokerSnapshot(updated)
                pushToast('success', `Đã kéo Chốt lời (TP) → ${snapped.toLocaleString('en-US')}`)
              })
              .catch((err) => {
                chartRef.current?.overrideOverlay({ id: `pos_tp_${pane.slotId}`, points: [{ value: origTp }] })
                pushToast('error', String(err))
              })
            return false
          },
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
      const origPrice = price
      const orderId = item.order.id
      const prefix = `${item.vnOrderType} ${item.order.side === 'Buy' ? 'MUA' : 'BÁN'} ${item.order.quantity.toLocaleString('en-US')}`
      chart.createOverlay({
        id: `ord_${orderId}_${pane.slotId}`,
        name: 'brokerOrderLine',
        groupId: 'broker_orders',
        paneId: 'candle_pane',
        lock: false,
        points: [{ value: origPrice }],
        extendData: {
          label: `⋮⋮ ${prefix} @ ${origPrice.toLocaleString('en-US')}`,
          color: item.order.side === 'Buy' ? '#2962FF' : '#FF9800',
          dashed: true,
          draggable: true,
          prefix,
        },
        onPressedMoveEnd: (ev) => {
          const rawVal = ev.overlay.points[0]?.value
          if (typeof rawVal !== 'number' || !Number.isFinite(rawVal)) return false
          const snapped = snapPriceToVenueTick(broker.ticker, rawVal)
          if (Math.abs(snapped - origPrice) < 1e-9) return false
          void api
            .brokerModifyOrder(broker.ticker, brokerIv, orderId, snapped)
            .then((updated) => {
              setBrokerSnapshot(updated)
              pushToast('success', `Đã dời lệnh #${orderId} → ${snapped.toLocaleString('en-US')}`)
            })
            .catch((err) => {
              chartRef.current?.overrideOverlay({
                id: `ord_${orderId}_${pane.slotId}`,
                points: [{ value: origPrice }],
              })
              pushToast('error', String(err))
            })
          return false
        },
      })
    }
  }, [broker, pane.ticker, pane.interval, pane.slotId, setBrokerSnapshot, pushToast])

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
    // AND causally hide future backtest markers during Bar Replay (`BUG-042`)!
    const cutoffTs = isThisPaneReplaying ? (replay?.cutoffTimestampMs ?? null) : null
    const rawTrades = backtestOverlay.trades.slice(-80)
    const trades =
      cutoffTs !== null ? rawTrades.filter((tr) => tr.entryTimestampMs <= cutoffTs) : rawTrades
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

      if (cutoffTs === null || tr.exitTimestampMs <= cutoffTs) {
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
    }
  }, [backtestOverlay, pane.ticker, pane.interval, pane.slotId, dataReadyNonce, isThisPaneReplaying, replay?.cutoffTimestampMs])

  // 12. Multi-Pane / Multi-Timeframe Causal Bar Replay Sync (`BUG-040` P0 FIXED!):
  // NEVER gate on `!isActive` and NEVER include `isActive` in dependency arrays — every pane
  // sharing `replay.ticker` (e.g. `1h` AND `4h` simultaneously) resets and steps in lockstep!
  const lastHandledReplayResetRef = useRef<number>(0)
  useEffect(() => {
    if (replayResetNonce === 0 || replayResetNonce === lastHandledReplayResetRef.current || !chartRef.current) {
      return
    }
    lastHandledReplayResetRef.current = replayResetNonce
    const rep = useWorkspaceStore.getState().replay
    // Reset if Replay exited (restore full history) OR if this pane displays the replayed ticker!
    if (!rep?.active || rep.ticker.toUpperCase() === paneRef.current.ticker.toUpperCase()) {
      chartRef.current.resetData()
    }
  }, [replayResetNonce])

  const lastHandledStepNonceRef = useRef<number>(0)
  useEffect(() => {
    if (!replayStepEvent || replayStepEvent.nonce === lastHandledStepNonceRef.current || !chartRef.current) {
      return
    }
    lastHandledStepNonceRef.current = replayStepEvent.nonce
    if (replayStepEvent.ticker.toUpperCase() !== paneRef.current.ticker.toUpperCase()) {
      return
    }
    const rawBar = replayStepEvent.barsByInterval[paneRef.current.interval]
    if (!rawBar) {
      // Fallback: pane interval wasn't pre-tracked yet; pull fresh causal slice from backend
      chartRef.current.resetData()
      return
    }
    if (!subscribeCallbackRef.current) {
      chartRef.current.resetData()
      return
    }
    let bar: KLineData = rawBar
    if (paneRef.current.chartStyle === 'heikin_ashi') {
      const curHa = currentHaBarRef.current
      if (curHa && bar.timestamp > curHa.timestamp) {
        prevClosedHaBarRef.current = curHa
      }
      const [ha] = toHeikinAshi([bar], prevClosedHaBarRef.current)
      if (ha) {
        bar = ha
        currentHaBarRef.current = { timestamp: ha.timestamp, open: ha.open, close: ha.close }
      }
    }
    subscribeCallbackRef.current(bar)
  }, [replayStepEvent])

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

      if ((e.metaKey || e.ctrlKey) && !e.shiftKey && (e.code === 'KeyZ' || e.key.toLowerCase() === 'z')) {
        if (undoStackRef.current.length > 0) {
          e.preventDefault()
          handleUndoDrawing()
        }
        return
      }

      if (
        (e.metaKey || e.ctrlKey) &&
        ((e.shiftKey && (e.code === 'KeyZ' || e.key.toLowerCase() === 'z')) ||
          (!e.shiftKey && (e.code === 'KeyY' || e.key.toLowerCase() === 'y')))
      ) {
        if (redoStackRef.current.length > 0) {
          e.preventDefault()
          handleRedoDrawing()
        }
        return
      }

      // Use `e.code === 'KeyR'` so macOS `Option+R` (which produces `®`) works reliably (`BUG-021`)!
      if (e.altKey && !e.ctrlKey && !e.metaKey && (e.code === 'KeyR' || e.key.toLowerCase() === 'r')) {
        e.preventDefault()
        handleResetView()
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [
    isActive,
    deleteSelectedDrawingById,
    handleResetView,
    handleUndoDrawing,
    handleRedoDrawing,
    setActiveDrawingTool,
    setReplaySelectingCut,
  ])

  // ─── Floating Drawing Toolbar Handlers ────────────────────────────────────

  const updateSelectedStyle = (patch: Partial<SelectedDrawingState>) => {
    const cur = selectedDrawingRef.current
    const chart = chartRef.current
    if (!cur || !chart) return
    const [existing] = chart.getOverlays({ id: cur.id, groupId: 'user_drawings' })
    const beforeRec = existing ? overlayToRecord(existing, paneRef.current.ticker) : null
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
      const afterRec = overlayToRecord(updated, paneRef.current.ticker)
      if (beforeRec) {
        undoStackRef.current = [...undoStackRef.current.slice(-29), { kind: 'modified', before: beforeRec, after: afterRec }]
        redoStackRef.current = []
      }
      void api.upsertDrawing(afterRec)
      drawingSyncBus.emit({
        kind: 'upsert',
        sourceSlotId: paneRef.current.slotId,
        ticker: paneRef.current.ticker,
        record: afterRec,
      })
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
        redoStackRef.current = []
        void api.upsertDrawing(rec)
        drawingSyncBus.emit({
          kind: 'upsert',
          sourceSlotId: paneRef.current.slotId,
          ticker: paneRef.current.ticker,
          record: rec,
        })
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
      data-slot-id={pane.slotId}
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
      <div
        className="relative flex-1 min-w-0 min-h-0 overflow-hidden"
        onDoubleClick={handleCanvasDoubleClick}
        onMouseLeave={() => crosshairBus.publish(pane.slotId, null)}
      >
        {/* Keyboard-accessible Indicator Legend Controls (visually hidden until focused, fixes `BUG-035`) */}
        {pane.indicators.length > 0 && (
          <div
            role="region"
            aria-label={`Chỉ báo trên ${pane.ticker}`}
            className="sr-only focus-within:not-sr-only focus-within:absolute focus-within:top-1 focus-within:left-2 focus-within:z-30 focus-within:flex focus-within:flex-wrap focus-within:gap-1 focus-within:p-1 focus-within:rounded"
            style={{ backgroundColor: 'var(--tv-bg-panel)', border: '1px solid var(--tv-border)' }}
          >
            {pane.indicators.map((ind) => (
              <span key={ind.id} className="inline-flex items-center gap-0.5 text-[10.5px]">
                <span className="font-semibold px-1">{ind.name}</span>
                <button
                  type="button"
                  className="tv-btn h-5 px-1 text-[10px]"
                  aria-label={`${ind.visible ? 'Ẩn' : 'Hiện'} chỉ báo ${ind.name}`}
                  onClick={() => updateIndicatorInActivePane(ind.id, { visible: !ind.visible }, pane.slotId)}
                >
                  {ind.visible ? 'Ẩn' : 'Hiện'}
                </button>
                <button
                  type="button"
                  className="tv-btn h-5 px-1 text-[10px]"
                  aria-label={`Cài đặt chỉ báo ${ind.name}`}
                  onClick={() => openIndicatorSettings(pane.slotId, ind.id)}
                >
                  Cài đặt
                </button>
                <button
                  type="button"
                  className="tv-btn h-5 px-1 text-[10px]"
                  aria-label={`Xóa chỉ báo ${ind.name}`}
                  onClick={() => removeIndicatorFromActivePane(ind.id, pane.slotId)}
                >
                  Xóa
                </button>
              </span>
            ))}
          </div>
        )}

        {/* Top-Left Active Multi-Symbol Comparison Pills (Wave 4) */}
        {(compareTickers?.length ?? 0) > 0 && (
          <div className="absolute top-1.5 left-2 z-10 flex flex-wrap items-center gap-1 text-[10px]">
            {(compareTickers ?? []).map((tk, idx) => (
              <span
                key={tk}
                className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded font-semibold"
                style={{
                  backgroundColor: 'color-mix(in srgb, var(--tv-bg-panel) 88%, transparent)',
                  border: `1px solid ${COMPARE_COLORS[idx % COMPARE_COLORS.length]}`,
                  color: COMPARE_COLORS[idx % COMPARE_COLORS.length],
                }}
              >
                <span>vs {tk.includes(':') ? tk.split(':')[1] : tk}</span>
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation()
                    removeCompareTicker(tk, pane.slotId)
                  }}
                  className="border-0 bg-transparent p-0 cursor-pointer text-current leading-none"
                  aria-label={`Gỡ so sánh ${tk}`}
                >
                  ×
                </button>
              </span>
            ))}
          </div>
        )}

        {/* Top-Right Status & VN Price Band Pill (formatted to ticker precision — `BUG-039`) */}
        <div className="pointer-events-none absolute top-1.5 right-16 z-10 flex items-center gap-1.5 text-[10px]">
          {!isMultiPane && vnBand && pane.showVnBands && (() => {
            const prec = guessPrecision(pane.ticker).pricePrecision
            const fmtBand = (val: number) =>
              val.toLocaleString('en-US', { minimumFractionDigits: prec, maximumFractionDigits: prec })
            return (
              <div
                className="flex items-center gap-2 px-2 py-0.5 rounded tabular-nums"
                style={{
                  backgroundColor: 'color-mix(in srgb, var(--tv-bg-panel) 84%, transparent)',
                  border: '1px solid var(--tv-border)',
                }}
              >
                <span style={{ color: 'var(--tv-vn-ceil)' }}>Trần {fmtBand(vnBand.ceiling)}</span>
                <span style={{ color: 'var(--tv-vn-ref)' }}>TC {fmtBand(vnBand.reference)}</span>
                <span style={{ color: 'var(--tv-vn-floor)' }}>Sàn {fmtBand(vnBand.floor)}</span>
              </div>
            )
          })()}
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
            onCreateAlert={() => {
              const chart = chartRef.current
              if (!chart) return
              const [ov] = chart.getOverlays({ id: selectedDrawing.id, groupId: 'user_drawings' })
              if (!ov || !ov.points.length) return
              const p0Val = ov.points[0]?.value ?? 0
              void createNewAlert(
                pane.ticker,
                'cross_any',
                Number(p0Val.toFixed(2)),
                `Cắt đường vẽ ${selectedDrawing.name} (${pane.ticker})`,
                {
                  alertKind: 'drawing',
                  interval: pane.interval,
                  paramsJson: {
                    drawingId: ov.id,
                    drawingName: ov.name,
                    points: ov.points.map((pt) => ({ timestamp: pt.timestamp, value: pt.value })),
                  },
                  soundEnabled: true,
                },
              )
            }}
            onOpenTradePlanner={() => {
              const chart = chartRef.current
              if (!chart) return
              const [ov] = chart.getOverlays({ id: selectedDrawing.id, groupId: 'user_drawings' })
              if (!ov || !ov.points.length) return
              const entry = ov.points[0]?.value
              const p1 = ov.points[1]?.value
              const p2 = ov.points[2]?.value
              const isShort = ov.name === 'shortPosition'
              let tp = p1
              let sl = p2
              if (typeof entry === 'number' && typeof p1 === 'number' && typeof p2 === 'number') {
                if (isShort) {
                  tp = Math.min(p1, p2)
                  sl = Math.max(p1, p2)
                } else {
                  tp = Math.max(p1, p2)
                  sl = Math.min(p1, p2)
                }
              }
              useWorkspaceStore.getState().openTradePlanner({
                ticker: pane.ticker,
                side: isShort ? 'SHORT' : 'LONG',
                entryPrice: entry,
                stopLoss: sl,
                targetPrice: tp,
              })
            }}
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

        {/* Bottom-Right TradingView ADJ/RAW & Corporate Events Bar (v3.0 Wave 1) */}
        <div className="absolute bottom-7 right-16 z-10 flex items-center gap-1 text-[10px]">
          {corpEvents.length > 0 && ['1D', '1W', '1M'].includes(pane.interval) && (
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation()
                const latest = corpEvents[corpEvents.length - 1]
                setActiveCorpEvent((prev) => (prev ? null : latest))
              }}
              className="px-1.5 py-0.5 rounded font-semibold border cursor-pointer tabular-nums"
              style={{
                backgroundColor: 'color-mix(in srgb, var(--tv-bg-panel) 90%, transparent)',
                borderColor: 'var(--tv-border)',
                color: '#089981',
              }}
              title="Xem sự kiện quyền / cổ tức (D), chia tách (S) và kỳ BCTC (E)"
            >
              D/S/E ({corpEvents.length})
            </button>
          )}
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation()
              togglePaneAdjustment(pane.slotId)
            }}
            className="px-1.5 py-0.5 rounded font-semibold border cursor-pointer tracking-wide"
            style={{
              backgroundColor:
                curAdj === 'splits'
                  ? 'var(--tv-accent-soft)'
                  : 'color-mix(in srgb, var(--tv-bg-panel) 90%, transparent)',
              borderColor: curAdj === 'splits' ? 'var(--tv-accent)' : 'var(--tv-border)',
              color: curAdj === 'splits' ? 'var(--tv-accent)' : '#FBC02D',
            }}
            title={
              curAdj === 'splits'
                ? 'Đang xem giá ĐÃ ĐIỀU CHỈNH cổ tức & chia tách (ADJ) — Nhấn để xem giá gốc (RAW)'
                : 'Đang xem giá GỐC chưa điều chỉnh (RAW) — Nhấn để chuyển về giá điều chỉnh (ADJ)'
            }
          >
            {curAdj === 'splits' ? 'ADJ' : 'RAW'}
          </button>
        </div>

        {activeCorpEvent && (
          <div
            className="absolute bottom-14 right-16 z-20 max-w-xs p-2.5 rounded-md shadow-2xl text-[11px] space-y-1"
            style={{
              backgroundColor: 'var(--tv-bg-elevated)',
              border: '1px solid var(--tv-border)',
              color: 'var(--tv-text-primary)',
            }}
          >
            <div className="flex items-center justify-between gap-2">
              <span className="font-semibold">
                Sự kiện quyền gần nhất ({corpEvents.length})
              </span>
              <button
                type="button"
                className="border-0 bg-transparent cursor-pointer text-[var(--tv-text-secondary)]"
                onClick={() => setActiveCorpEvent(null)}
              >
                ×
              </button>
            </div>
            <div className="max-h-36 overflow-y-auto tv-scroll space-y-1">
              {[...corpEvents]
                .reverse()
                .slice(0, 8)
                .map((ev, idx) => {
                  const ts = ev.timestampMs ?? ev.timestamp_ms ?? 0
                  const k = (ev.kind ?? 'D').toUpperCase()
                  const isS = k === 'S' || k === 'SPLIT'
                  const isE = k === 'E' || k === 'EARNINGS'
                  const tag = isS ? 'S' : isE ? 'E' : 'D'
                  const color = isS ? '#AB47BC' : isE ? '#2962FF' : '#089981'
                  return (
                    <button
                      key={`${ts}_${idx}`}
                      type="button"
                      onClick={() => chartRef.current?.scrollToTimestamp(ts, 220)}
                      className="w-full text-left px-1.5 py-1 rounded hover:bg-[var(--tv-hover)] border-0 bg-transparent cursor-pointer text-[var(--tv-text-primary)]"
                    >
                      <div className="flex items-center justify-between gap-2">
                        <span className="font-semibold" style={{ color }}>
                          [{tag}] {ev.label}
                        </span>
                        <span className="text-[10px] tabular-nums" style={{ color: 'var(--tv-text-secondary)' }}>
                          {ev.date}
                        </span>
                      </div>
                      <div className="text-[10px]" style={{ color: 'var(--tv-text-secondary)' }}>
                        {ev.detail}
                      </div>
                    </button>
                  )
                })}
            </div>
          </div>
        )}

        {/* KLineChart v10 Canvas Container */}
        <div ref={containerRef} className="w-full h-full min-w-0 min-h-0" />
      </div>

      {/* Custom Right-Click Context Menu (fixes BUG-012 + Chart Trading SL/TP v2.2!) */}
      {contextMenu && (
        <ChartContextMenu
          menu={contextMenu}
          ticker={pane.ticker}
          hasOpenPosition={Boolean(broker?.openPosition && broker.ticker === pane.ticker)}
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
          onSetStopLossAtPrice={() => {
            if (contextMenu.price === null) return
            void submitQuickBracket('sl', contextMenu.price)
          }}
          onSetTakeProfitAtPrice={() => {
            if (contextMenu.price === null) return
            void submitQuickBracket('tp', contextMenu.price)
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
            redoStackRef.current = []
            void api.clearDrawings(pane.ticker)
            drawingSyncBus.emit({ kind: 'clear', sourceSlotId: pane.slotId, ticker: pane.ticker })
            bumpDrawingsVersion()
            pushToast('info', `Đã xóa toàn bộ hình vẽ trên ${pane.ticker}`)
          }}
        />
      )}
    </div>
  )
}
