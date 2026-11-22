import React, { useEffect, useRef, useState } from 'react'
import { dispose, init } from 'klinecharts'
import type { Chart, KLineData, OverlayEvent } from 'klinecharts'
import * as api from '../api/tauri'
import type { ChartPaneConfig, SavedDrawingRecord, VnBandLevels } from '../types/market'
import { useWorkspaceStore } from '../store/workspaceStore'
import { toHeikinAshi } from './heikinAshi'
import { formatBarCountdown, intervalToKLinePeriod } from './periods'
import { buildSuperchartsStyles } from './themes'
import { registerAllCustomIndicators } from './indicators/registerAll'
import { registerAllCustomOverlays } from './overlays/registerAll'

interface KLineViewportProps {
  pane: ChartPaneConfig
  isActive: boolean
  isMultiPane: boolean
  onActivate: () => void
}

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

export const KLineViewport: React.FC<KLineViewportProps> = ({
  pane,
  isActive,
  isMultiPane,
  onActivate,
}) => {
  const containerRef = useRef<HTMLDivElement | null>(null)
  const chartRef = useRef<Chart | null>(null)
  const paneRef = useRef<ChartPaneConfig>(pane)
  paneRef.current = pane

  const isActiveRef = useRef(isActive)
  isActiveRef.current = isActive

  const subscribeCallbackRef = useRef<((data: KLineData) => void) | null>(null)
  const lastHaBarRef = useRef<{ open: number; close: number } | undefined>(undefined)
  const ignoreCrosshairBroadcastRef = useRef(false)
  const appliedPaneRef = useRef<{ ticker: string; interval: string }>({
    ticker: pane.ticker,
    interval: pane.interval,
  })
  const precisionSyncedTickerRef = useRef<string | null>(null)
  const createdIndicatorIdsRef = useRef<Map<string, { klineId: string; paneId: string; signature: string }>>(
    new Map(),
  )

  const [vnBand, setVnBand] = useState<VnBandLevels | null>(null)
  const [dataSourceBadge, setDataSourceBadge] = useState<string>('')
  const [loading, setLoading] = useState<boolean>(false)

  const theme = useWorkspaceStore((s) => s.theme)
  const sync = useWorkspaceStore((s) => s.sync)
  const activeDrawingTool = useWorkspaceStore((s) => s.activeDrawingTool)
  const magnetMode = useWorkspaceStore((s) => s.magnetMode)
  const drawingsLocked = useWorkspaceStore((s) => s.drawingsLocked)
  const drawingsHidden = useWorkspaceStore((s) => s.drawingsHidden)
  const clearDrawingsNonce = useWorkspaceStore((s) => s.clearDrawingsNonce)
  const exportPngNonce = useWorkspaceStore((s) => s.exportPngNonce)
  const syncedCrosshair = useWorkspaceStore((s) => s.syncedCrosshair)
  const scrollToTimestampCmd = useWorkspaceStore((s) => s.scrollToTimestampCmd)
  const replay = useWorkspaceStore((s) => s.replay)
  const replaySelectingCut = useWorkspaceStore((s) => s.replaySelectingCut)
  const replayResetNonce = useWorkspaceStore((s) => s.replayResetNonce)
  const replaySteppedBar = useWorkspaceStore((s) => s.replaySteppedBar)
  const broker = useWorkspaceStore((s) => s.broker)

  const setActiveDrawingTool = useWorkspaceStore((s) => s.setActiveDrawingTool)
  const publishCrosshairSync = useWorkspaceStore((s) => s.publishCrosshairSync)
  const startReplay = useWorkspaceStore((s) => s.startReplay)
  const jumpReplay = useWorkspaceStore((s) => s.jumpReplay)
  const pushToast = useWorkspaceStore((s) => s.pushToast)

  const replaySelectingCutRef = useRef(replaySelectingCut)
  replaySelectingCutRef.current = replaySelectingCut
  const replayActiveRef = useRef(Boolean(replay?.active))
  replayActiveRef.current = Boolean(replay?.active)

  const persistOverlayToDb = (event: OverlayEvent<unknown>) => {
    const ov = event.overlay
    if (ov.groupId !== 'user_drawings') return
    const record: SavedDrawingRecord = {
      id: ov.id,
      ticker: paneRef.current.ticker,
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
    void api.upsertDrawing(record)
  }

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
      timezone: initialPane.ticker.startsWith('HOSE:') || initialPane.ticker.startsWith('HNX:') || initialPane.ticker.startsWith('UPCOM:')
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
            limit: 320,
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

          // Dynamically synchronize authoritative price/volume precision from TradingView/sumi-data
          // (e.g. 4–6 decimals on altcoins/forex) at most once per ticker without loops.
          if (type === 'init' && page.symbolInfo && chartRef.current) {
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

    // Crosshair sync publisher
    chart.subscribeAction('onCrosshairChange', (data) => {
      if (ignoreCrosshairBroadcastRef.current) return
      const payload = data as { kLineData?: KLineData } | null
      publishCrosshairSync(paneRef.current.slotId, payload?.kLineData?.timestamp ?? null)
    })

    // Causal Bar Replay "Jump to Bar" click handler
    chart.subscribeAction('onCandleBarClick', (data) => {
      if (!isActiveRef.current || !replaySelectingCutRef.current) return
      const payload = data as { data?: KLineData } | null
      const ts = payload?.data?.timestamp
      if (typeof ts === 'number') {
        if (replayActiveRef.current) {
          void jumpReplay(ts)
        } else {
          void startReplay(ts)
        }
      }
    })

    const ro = new ResizeObserver(() => {
      chartRef.current?.resize()
    })
    ro.observe(el)

    return () => {
      ro.disconnect()
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

    if (tickerChanged) {
      precisionSyncedTickerRef.current = null
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

  // 5. Reconcile active indicators on this chart pane
  useEffect(() => {
    const chart = chartRef.current
    if (!chart) return

    const desiredMap = new Map(pane.indicators.map((ind) => [ind.id, ind]))
    const activeMap = createdIndicatorIdsRef.current

    // Remove deleted indicators
    for (const [specId, tracked] of Array.from(activeMap.entries())) {
      if (!desiredMap.has(specId)) {
        chart.removeIndicator({ id: tracked.klineId, paneId: tracked.paneId })
        activeMap.delete(specId)
      }
    }

    // Create or update desired indicators
    for (const spec of pane.indicators) {
      const signature = JSON.stringify({
        name: spec.name,
        calcParams: spec.calcParams,
        visible: spec.visible,
        isMain: spec.isMain,
      })
      const existing = activeMap.get(spec.id)
      if (!existing) {
        const targetPaneId = spec.isMain ? 'candle_pane' : `pane_${spec.id}`
        const klineId = chart.createIndicator(
          {
            name: spec.name,
            paneId: targetPaneId,
            calcParams: spec.calcParams,
            visible: spec.visible,
          },
          spec.isMain,
        )
        if (klineId) {
          activeMap.set(spec.id, { klineId, paneId: targetPaneId, signature })
          if (!spec.isMain) {
            chart.setPaneOptions({
              id: targetPaneId,
              height: isMultiPane ? 56 : 92,
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
        })
        activeMap.set(spec.id, { ...existing, signature })
      }
    }
  }, [pane.indicators, isMultiPane])

  // 5b. Compact sub-pane heights automatically when toggling 2x2 / 2-pane multi-chart mode
  useEffect(() => {
    const chart = chartRef.current
    if (!chart) return
    for (const tracked of createdIndicatorIdsRef.current.values()) {
      if (tracked.paneId !== 'candle_pane') {
        chart.setPaneOptions({
          id: tracked.paneId,
          height: isMultiPane ? 56 : 92,
          minHeight: 34,
        })
      }
    }
    chart.resize()
  }, [isMultiPane])

  // 6. Load & restore persistent SQLite drawings whenever ticker changes
  useEffect(() => {
    const chart = chartRef.current
    if (!chart) return
    let cancelled = false
    chart.removeOverlay({ groupId: 'user_drawings' })

    void api.getDrawings(pane.ticker).then((records) => {
      if (cancelled || !chartRef.current) return
      for (const rec of records) {
        chartRef.current.createOverlay({
          id: rec.id,
          name: rec.name,
          groupId: 'user_drawings',
          paneId: rec.paneId || 'candle_pane',
          points: rec.points,
          styles: rec.styles ?? undefined,
          extendData: rec.extendData ?? undefined,
          lock: rec.lock || drawingsLocked,
          visible: drawingsHidden ? false : rec.visible,
          mode: (rec.mode as 'normal' | 'weak_magnet' | 'strong_magnet') || magnetMode,
          onPressedMoveEnd: persistOverlayToDb,
          onRemoved: (ev) => {
            if (ev.overlay.groupId === 'user_drawings') {
              void api.deleteDrawing(paneRef.current.ticker, ev.overlay.id)
            }
          },
        })
      }
    })
    return () => {
      cancelled = true
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pane.ticker])

  // 7. Trigger interactive drawing tool when selected in Left Drawing Rail (active pane only)
  useEffect(() => {
    const chart = chartRef.current
    if (!chart || !isActive || !activeDrawingTool) return
    chart.createOverlay({
      name: activeDrawingTool,
      groupId: 'user_drawings',
      paneId: 'candle_pane',
      mode: magnetMode,
      lock: drawingsLocked,
      visible: !drawingsHidden,
      onDrawEnd: (ev) => {
        persistOverlayToDb(ev)
        setActiveDrawingTool(null)
      },
      onPressedMoveEnd: persistOverlayToDb,
      onRemoved: (ev) => {
        if (ev.overlay.groupId === 'user_drawings') {
          void api.deleteDrawing(paneRef.current.ticker, ev.overlay.id)
        }
      },
    })
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeDrawingTool, isActive])

  // 8. Sync lock / hide / magnet state across existing user drawings
  useEffect(() => {
    const chart = chartRef.current
    if (!chart) return
    chart.overrideOverlay({
      groupId: 'user_drawings',
      lock: drawingsLocked,
      visible: !drawingsHidden,
      mode: magnetMode,
    })
  }, [drawingsLocked, drawingsHidden, magnetMode])

  // 9. Clear all user drawings on active pane
  useEffect(() => {
    if (clearDrawingsNonce === 0 || !isActive || !chartRef.current) return
    chartRef.current.removeOverlay({ groupId: 'user_drawings' })
    void api.clearDrawings(pane.ticker)
    pushToast('info', `Đã xóa toàn bộ hình vẽ trên ${pane.ticker}`)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [clearDrawingsNonce])

  // 10. Render Vietnam Ceiling (#CE93D8) / Reference (#FBC02D) / Floor (#26C6DA) band lines
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

  // 12. Bar Replay resetData() & single-bar subscribeBar push (100% pure KLineChart v10)
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

  // 13. Multi-chart Crosshair Sync receiver
  useEffect(() => {
    const chart = chartRef.current
    if (!chart || !sync.crosshair || !syncedCrosshair) return
    if (syncedCrosshair.sourceSlotId === pane.slotId) return
    if (syncedCrosshair.timestamp === null) {
      chart.executeAction('onCrosshairChange', null as unknown as Parameters<Chart['executeAction']>[1])
      return
    }

    const matchingBar = chart.getDataList().find((b) => b.timestamp === syncedCrosshair.timestamp)
    const pixel = chart.convertToPixel(
      matchingBar
        ? { timestamp: syncedCrosshair.timestamp, value: matchingBar.close }
        : { timestamp: syncedCrosshair.timestamp },
      { paneId: 'candle_pane' },
    )
    const coord = Array.isArray(pixel) ? pixel[0] : pixel
    if (coord && typeof coord.x === 'number' && Number.isFinite(coord.x)) {
      ignoreCrosshairBroadcastRef.current = true
      chart.executeAction('onCrosshairChange', {
        x: coord.x,
        y: typeof coord.y === 'number' && Number.isFinite(coord.y) ? coord.y : 100,
        paneId: 'candle_pane',
      })
      window.requestAnimationFrame(() => {
        ignoreCrosshairBroadcastRef.current = false
      })
    }
  }, [syncedCrosshair, sync.crosshair, pane.slotId])

  // 14. Scroll to specific trade timestamp (from Strategy Tester trade click)
  useEffect(() => {
    if (!scrollToTimestampCmd || scrollToTimestampCmd.slotId !== pane.slotId || !chartRef.current) {
      return
    }
    chartRef.current.scrollToTimestamp(scrollToTimestampCmd.timestampMs, 260)
  }, [scrollToTimestampCmd, pane.slotId])

  // 15. Export PNG snapshot
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

  return (
    <div
      onMouseDown={onActivate}
      className="relative flex flex-col w-full h-full overflow-hidden"
      style={{
        backgroundColor: 'var(--tv-bg-chart)',
        outline: isMultiPane && isActive ? '1px solid var(--tv-accent)' : '1px solid var(--tv-border)',
        outlineOffset: '-1px',
        cursor: isActive && replaySelectingCut ? 'col-resize' : undefined,
      }}
    >
      {/* Subtle Multi-Pane Status & VN Band Pill Bar (top-right corner inside chart) */}
      <div className="pointer-events-none absolute top-1.5 right-16 z-10 flex items-center gap-1.5 text-[10px]">
        {isActive && replaySelectingCut && (
          <span className="px-2 py-0.5 rounded font-semibold bg-[#2962FF] text-white shadow">
            Nhấp vào cây nến bất kỳ để chọn điểm cắt Replay
          </span>
        )}
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
              backgroundColor: dataSourceBadge === 'REPLAY' ? 'var(--tv-accent)' : 'color-mix(in srgb, var(--tv-bg-panel) 85%, transparent)',
              color: dataSourceBadge === 'REPLAY' ? '#FFFFFF' : 'var(--tv-text-secondary)',
              border: '1px solid var(--tv-border)',
            }}
          >
            {dataSourceBadge}
          </span>
        )}
      </div>

      {loading && (
        <div
          className="pointer-events-none absolute bottom-8 left-3 z-10 px-2 py-1 rounded text-[11px]"
          style={{ backgroundColor: 'var(--tv-bg-panel)', color: 'var(--tv-text-secondary)', border: '1px solid var(--tv-border)' }}
        >
          Đang nạp dữ liệu {pane.ticker}...
        </div>
      )}

      {/* KLineChart v10 Canvas Container — explicit 100% width & height */}
      <div ref={containerRef} className="w-full h-full flex-1 min-h-0" />
    </div>
  )
}
