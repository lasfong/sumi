import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import {
  Bell,
  BellOff,
  BellPlus,
  Edit3,
  Eye,
  EyeOff,
  Layers,
  List,
  Lock,
  Plus,
  RefreshCw,
  Send,
  Settings,
  Sparkles,
  Trash2,
  Unlock,
  Volume2,
} from 'lucide-react'
import * as api from '../api/tauri'
import { drawingSyncBus } from '../chart/drawingSyncBus'
import { setOsNotificationEnabled } from '../utils/alertSound'
import { useWorkspaceStore } from '../store/workspaceStore'
import type {
  AlertDeliveryConfig,
  AlertTriggerLogRecord,
  SavedDrawingRecord,
  SignalCatalogItemDto,
  WatchlistQuoteSnapshot,
} from '../types/market'
import { playAlertChime } from '../utils/alertSound'
import { YumekoQmvPanel } from './YumekoQmvPanel'

const DRAWING_LABELS: Record<string, string> = {
  segment: 'Đường xu hướng',
  rayLine: 'Tia xu hướng',
  straightLine: 'Đường thẳng mở rộng',
  horizontalStraightLine: 'Đường ngang',
  horizontalRayLine: 'Tia ngang',
  horizontalSegment: 'Đoạn ngang',
  verticalStraightLine: 'Đường dọc',
  priceChannelLine: 'Kênh giá',
  parallelStraightLine: 'Kênh song song',
  andrewsPitchfork: 'Chĩa ba Andrews Pitchfork',
  fibonacciLine: 'Fibonacci Thoái lui',
  fibonacciExtension: 'Fibonacci Mở rộng',
  fibTimeZones: 'Vùng thời gian Fibonacci',
  rect: 'Vùng chữ nhật',
  circle: 'Hình tròn',
  triangle: 'Tam giác',
  path: 'Đường dẫn đa đoạn (Path)',
  highlighter: 'Bút dạ quang (Highlighter)',
  anchoredVwap: 'Anchored VWAP Neo Nến',
  fixedRangeVolumeProfile: 'Hồ sơ Khối lượng (FRVP)',
  longPosition: 'Kế hoạch Long R:R',
  shortPosition: 'Kế hoạch Short R:R',
  priceDateRange: 'Đo Giá & Thời gian',
  simpleAnnotation: 'Ghi chú văn bản',
}

const FLAG_COLORS: Array<{ key: string; hex: string; label: string }> = [
  { key: 'red', hex: '#F23645', label: 'Đỏ (Cảnh giác / Cắt lỗ)' },
  { key: 'orange', hex: '#FF9800', label: 'Cam (Theo dõi sát)' },
  { key: 'green', hex: '#089981', label: 'Xanh lá (Đạt điểm mua KT01)' },
  { key: 'blue', hex: '#2962FF', label: 'Xanh dương (Dẫn dắt ngành)' },
  { key: 'purple', hex: '#AB47BC', label: 'Tím (BCTC / Đột biến)' },
]

const DRAWING_FOLDERS = [
  'Mặc định',
  'Hỗ trợ & Kháng cự',
  'Xu hướng & Sóng',
  'Kế hoạch Lệnh',
  'Ghi chú',
] as const

function resolveDrawingFolder(extendData: unknown): string {
  if (!extendData || typeof extendData !== 'object') return 'Mặc định'
  const f = (extendData as Record<string, unknown>).folder
  return typeof f === 'string' && f.trim() ? f : 'Mặc định'
}

function resolveDrawingTfPreset(extendData: unknown): string {
  if (!extendData || typeof extendData !== 'object') return 'all'
  const raw = (extendData as Record<string, unknown>).visibleTimeframes
  if (!raw || raw === 'all') return 'all'
  if (raw === 'intraday') return 'intraday'
  if (raw === 'swing') return 'swing'
  if (Array.isArray(raw) && raw.length === 1 && typeof raw[0] === 'string') {
    return `only:${raw[0]}`
  }
  return 'all'
}

function formatPrice(val: number | null | undefined, precision = 0): string {
  if (val === null || val === undefined || !Number.isFinite(val)) return '—'
  return val.toLocaleString('en-US', {
    minimumFractionDigits: precision,
    maximumFractionDigits: precision,
  })
}

function formatVol(val: number | null | undefined): string {
  if (val === null || val === undefined || !Number.isFinite(val)) return '—'
  if (val >= 1_000_000) return `${(val / 1_000_000).toFixed(2)}M`
  if (val >= 1_000) return `${(val / 1_000).toFixed(1)}K`
  return Math.round(val).toLocaleString('en-US')
}

interface WatchlistRowProps {
  ticker: string
  quote: WatchlistQuoteSnapshot | undefined
  flash: { dir: 'up' | 'down'; nonce: number } | undefined
  flagColor?: string
  isSelected: boolean
  isBlindMasked: boolean
  onSelect: () => void
  onCycleFlag: () => void
  onRemove: () => void
  onDragStart: () => void
  onDragOver: (e: React.DragEvent<HTMLDivElement>) => void
  onDrop: () => void
}

/**
 * Zero-DOM-Churn Watchlist Row (`BUG-030` fixed + `modern-web-guidance` `motion` compliant + Wave 5 Flag & Drag):
 * Keeps a stable `key={ticker}` so React NEVER unmounts/remounts rows on price ticks,
 * animating background highlights via Web Animations API only when `prefers-reduced-motion: no-preference`.
 */
const WatchlistRow: React.FC<WatchlistRowProps> = React.memo(
  ({
    ticker,
    quote,
    flash,
    flagColor,
    isSelected,
    isBlindMasked,
    onSelect,
    onCycleFlag,
    onRemove,
    onDragStart,
    onDragOver,
    onDrop,
  }) => {
    const rowRef = useRef<HTMLDivElement | null>(null)
    const lastFlashNonceRef = useRef<number>(flash?.nonce ?? 0)

    useEffect(() => {
      const nonce = flash?.nonce ?? 0
      if (!flash || nonce === lastFlashNonceRef.current || isBlindMasked) {
        lastFlashNonceRef.current = nonce
        return
      }
      lastFlashNonceRef.current = nonce
      const el = rowRef.current
      if (!el || typeof el.animate !== 'function') return
      if (window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
        return
      }
      const tint = flash.dir === 'up' ? 'rgba(8, 153, 129, 0.24)' : 'rgba(242, 54, 69, 0.24)'
      el.animate([{ backgroundColor: tint }, { backgroundColor: 'transparent' }], {
        duration: 420,
        easing: 'ease-out',
      })
    }, [flash, isBlindMasked])

    const shortSym = ticker.includes(':') ? ticker.split(':')[1] : ticker
    const ex = ticker.includes(':') ? ticker.split(':')[0] : ''
    const chgPct = quote?.changePercent ?? null
    const flagHex = FLAG_COLORS.find((f) => f.key === flagColor)?.hex

    let priceColor = 'var(--tv-text-primary)'
    if (quote?.lastPrice != null && quote.vnBand) {
      if (quote.lastPrice >= quote.vnBand.ceiling - 1e-6) priceColor = 'var(--tv-vn-ceil)'
      else if (quote.lastPrice <= quote.vnBand.floor + 1e-6) priceColor = 'var(--tv-vn-floor)'
      else if (Math.abs(quote.lastPrice - quote.vnBand.reference) < 1e-6) priceColor = 'var(--tv-vn-ref)'
      else if (quote.lastPrice > quote.vnBand.reference) priceColor = 'var(--tv-up)'
      else priceColor = 'var(--tv-down)'
    } else if (chgPct !== null) {
      priceColor = chgPct > 0 ? 'var(--tv-up)' : chgPct < 0 ? 'var(--tv-down)' : 'var(--tv-text-secondary)'
    }

    return (
      <div
        ref={rowRef}
        draggable
        onDragStart={onDragStart}
        onDragOver={onDragOver}
        onDrop={onDrop}
        className="group grid grid-cols-[1fr_76px_64px_16px] gap-x-2 items-center px-2 py-1.5 cursor-pointer border-b tabular-nums"
        style={{
          borderColor: 'color-mix(in srgb, var(--tv-border) 45%, transparent)',
          backgroundColor: isSelected ? 'var(--tv-accent-soft)' : undefined,
        }}
        onClick={onSelect}
      >
        <div className="min-w-0 pr-1">
          <div className="flex items-center gap-1">
            <button
              type="button"
              aria-label={`Đánh dấu màu cờ cho ${shortSym}`}
              title="Nhấp để gắn / đổi màu cờ đánh dấu (đồng bộ đa máy)"
              onClick={(e) => {
                e.stopPropagation()
                onCycleFlag()
              }}
              className="w-2.5 h-2.5 rounded-full shrink-0 border p-0 cursor-pointer"
              style={{
                backgroundColor: flagHex ?? 'transparent',
                borderColor: flagHex ?? 'var(--tv-border)',
              }}
            />
            <span className="font-semibold text-[12px] truncate">{shortSym}</span>
            <span className="text-[9.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
              {ex}
            </span>
            {isBlindMasked && (
              <span className="px-1 rounded text-[8.5px] font-bold bg-[#2962FF] text-white">R</span>
            )}
          </div>
        </div>
        <div className="text-right font-medium text-[11.5px]" style={{ color: priceColor }}>
          {formatPrice(quote?.lastPrice, quote?.pricePrecision ?? 0)}
        </div>
        <div className="text-right font-medium text-[11px]" style={{ color: priceColor }}>
          {chgPct !== null ? `${chgPct >= 0 ? '+' : ''}${chgPct.toFixed(2)}%` : '—'}
        </div>
        <button
          type="button"
          aria-label={`Xóa ${shortSym} khỏi danh mục`}
          className="opacity-0 group-hover:opacity-100 border-0 bg-transparent p-0 cursor-pointer text-[var(--tv-text-secondary)] hover:text-[#F23645]"
          onClick={(e) => {
            e.stopPropagation()
            onRemove()
          }}
        >
          ×
        </button>
      </div>
    )
  },
)

export const RightWidgetBar: React.FC = () => {
  const rightPanelWidth = useWorkspaceStore((s) => s.rightPanelWidth)
  const watchlists = useWorkspaceStore((s) => s.watchlists)
  const activeWatchlistId = useWorkspaceStore((s) => s.activeWatchlistId)
  const quotesMap = useWorkspaceStore((s) => s.quotesMap)
  const quoteFlashMap = useWorkspaceStore((s) => s.quoteFlashMap)
  const alerts = useWorkspaceStore((s) => s.alerts)
  const panes = useWorkspaceStore((s) => s.panes)
  const activeSlotId = useWorkspaceStore((s) => s.activeSlotId)
  const replay = useWorkspaceStore((s) => s.replay)
  const replayBlindMode = useWorkspaceStore((s) => s.replayBlindMode)

  const setActiveWatchlistId = useWorkspaceStore((s) => s.setActiveWatchlistId)
  const setActiveTicker = useWorkspaceStore((s) => s.setActiveTicker)
  const refreshWatchlistQuotes = useWorkspaceStore((s) => s.refreshWatchlistQuotes)
  const addSymbolToWatchlist = useWorkspaceStore((s) => s.addSymbolToWatchlist)
  const removeSymbolFromWatchlist = useWorkspaceStore((s) => s.removeSymbolFromWatchlist)
  const createNewWatchlist = useWorkspaceStore((s) => s.createNewWatchlist)
  const renameWatchlist = useWorkspaceStore((s) => s.renameWatchlist)
  const deleteWatchlistById = useWorkspaceStore((s) => s.deleteWatchlistById)
  const reorderWatchlistSymbols = useWorkspaceStore((s) => s.reorderWatchlistSymbols)
  const symbolFlags = useWorkspaceStore((s) => s.symbolFlags)
  const setSymbolFlag = useWorkspaceStore((s) => s.setSymbolFlag)
  const scrollActiveChartToTimestamp = useWorkspaceStore((s) => s.scrollActiveChartToTimestamp)
  const createNewAlert = useWorkspaceStore((s) => s.createNewAlert)
  const updateExistingAlert = useWorkspaceStore((s) => s.updateExistingAlert)
  const toggleAlert = useWorkspaceStore((s) => s.toggleAlert)
  const removeAlert = useWorkspaceStore((s) => s.removeAlert)
  const scanAllWatchlistAlerts = useWorkspaceStore((s) => s.scanAllWatchlistAlerts)
  const pushToast = useWorkspaceStore((s) => s.pushToast)
  const updateIndicatorInActivePane = useWorkspaceStore((s) => s.updateIndicatorInActivePane)
  const removeIndicatorFromActivePane = useWorkspaceStore((s) => s.removeIndicatorFromActivePane)
  const openIndicatorSettings = useWorkspaceStore((s) => s.openIndicatorSettings)
  const drawingsVersion = useWorkspaceStore((s) => s.drawingsVersion)
  const fundamentalsByTicker = useWorkspaceStore((s) => s.fundamentalsByTicker)
  const loadFundamentalsForTicker = useWorkspaceStore((s) => s.loadFundamentalsForTicker)

  const activePane = panes.find((p) => p.slotId === activeSlotId) ?? panes[0]
  const activeWatchlist = watchlists.find((w) => w.id === activeWatchlistId) ?? watchlists[0]
  const activeFunda = fundamentalsByTicker[activePane.ticker.trim().toUpperCase()]

  useEffect(() => {
    if (!activePane.ticker || /[/*+()]/.test(activePane.ticker)) return
    void loadFundamentalsForTicker(activePane.ticker)
  }, [activePane.ticker, loadFundamentalsForTicker])

  // Blind Practice Mode (`BUG-041`): when Bar Replay is active on a ticker, substitute historical
  // `cutoffBar` + `replay.vnBand` in place of live market quotes so future prices NEVER spoil practice!
  const replayMaskTicker = replay?.active && replayBlindMode ? replay.ticker.toUpperCase() : null
  const buildEffectiveQuote = (ticker: string): WatchlistQuoteSnapshot | undefined => {
    const live = quotesMap[ticker]
    if (!replayMaskTicker || ticker.toUpperCase() !== replayMaskTicker || !replay?.cutoffBar) {
      return live
    }
    const bar = replay.cutoffBar
    const prev = replay.prevCutoffClose ?? bar.open
    const chg = bar.close - prev
    const chgPct = prev > 0 ? (chg / prev) * 100 : 0
    return {
      ticker,
      exchange: live?.exchange ?? ticker.split(':')[0] ?? '',
      symbol: live?.symbol ?? ticker.split(':')[1] ?? ticker,
      description: live?.description ?? ticker,
      lastPrice: bar.close,
      prevClose: prev,
      change: chg,
      changePercent: chgPct,
      volume: bar.volume,
      pricePrecision: live?.pricePrecision ?? (ticker.includes('VN30F') ? 1 : 0),
      vnBand: replay.vnBand ?? null,
      timestampMs: bar.timestamp,
    }
  }

  const activeQuote = buildEffectiveQuote(activePane.ticker)
  const isActiveBlindReplay = Boolean(
    replayMaskTicker && activePane.ticker.toUpperCase() === replayMaskTicker,
  )

  const rightTab = useWorkspaceStore((s) => s.rightTab)
  const setRightTab = useWorkspaceStore((s) => s.setRightTab)
  const [addTickerInput, setAddTickerInput] = useState('')
  const [showNewListForm, setShowNewListForm] = useState(false)
  const [newListName, setNewListName] = useState('')
  const [renamingList, setRenamingList] = useState(false)
  const [renameDraft, setRenameDraft] = useState('')
  const [flagFilter, setFlagFilter] = useState<string>('all')
  const [wlSort, setWlSort] = useState<{ key: 'none' | 'symbol' | 'price' | 'pct'; dir: 'asc' | 'desc' }>({
    key: 'none',
    dir: 'desc',
  })
  const [dragTicker, setDragTicker] = useState<string | null>(null)
  const [drawingFolderFilter, setDrawingFolderFilter] = useState<string>('all')

  // Object Tree live drawings list for active ticker (synced via SQLite + drawingSyncBus, v2.2)
  const [tickerDrawings, setTickerDrawings] = useState<SavedDrawingRecord[]>([])
  const reloadTickerDrawings = useCallback(() => {
    void api.getDrawings(activePane.ticker).then((recs) => setTickerDrawings(recs))
  }, [activePane.ticker])

  useEffect(() => {
    reloadTickerDrawings()
  }, [reloadTickerDrawings, drawingsVersion])

  useEffect(() => {
    return drawingSyncBus.subscribe('right_object_tree', (ev) => {
      if (ev.ticker.toUpperCase() === activePane.ticker.toUpperCase()) {
        reloadTickerDrawings()
      }
    })
  }, [activePane.ticker, reloadTickerDrawings])

  const mutateDrawingObject = (rec: SavedDrawingRecord, patch: Partial<SavedDrawingRecord>) => {
    const next: SavedDrawingRecord = { ...rec, ...patch, updatedAtMs: Date.now() }
    setTickerDrawings((prev) => prev.map((d) => (d.id === next.id ? next : d)))
    void api.upsertDrawing(next)
    drawingSyncBus.emit({
      kind: 'upsert',
      sourceSlotId: 'right_object_tree',
      ticker: next.ticker,
      record: next,
    })
  }

  const deleteDrawingObject = (rec: SavedDrawingRecord) => {
    setTickerDrawings((prev) => prev.filter((d) => d.id !== rec.id))
    void api.deleteDrawing(rec.ticker, rec.id)
    drawingSyncBus.emit({
      kind: 'delete',
      sourceSlotId: 'right_object_tree',
      ticker: rec.ticker,
      id: rec.id,
    })
  }

  // Alert 2.0 state (Price / Indicator / 123 QMV Signals / Confluence + Cadence + Logs + Telegram/Webhook, Wave 3)
  const [alertSubTab, setAlertSubTab] = useState<'list' | 'logs' | 'delivery'>('list')
  const [alertMode, setAlertMode] = useState<'price' | 'indicator' | 'signal' | 'confluence'>('price')
  const [alertCondition, setAlertCondition] = useState<'cross_above' | 'cross_below' | 'cross_any'>('cross_above')
  const [alertPrice, setAlertPrice] = useState('')
  const [alertIndType, setAlertIndType] = useState<'RSI' | 'MACD' | 'EMA'>('RSI')
  const [alertIndPeriod, setAlertIndPeriod] = useState('14')
  const [alertIndThreshold, setAlertIndThreshold] = useState('70')
  const [alertSignalId, setAlertSignalId] = useState('qmv.ichimoku_v7_bullish')
  const [alertConfluenceProfile, setAlertConfluenceProfile] = useState<'balanced' | 'trend'>('balanced')
  const [alertConfluenceThreshold, setAlertConfluenceThreshold] = useState('35')
  const [alertFrequency, setAlertFrequency] = useState<'once' | 'once_per_bar_close'>('once')
  const [alertCooldownBars, setAlertCooldownBars] = useState('0')
  const [alertSoundEnabled, setAlertSoundEnabled] = useState(true)
  const [alertMessage, setAlertMessage] = useState('')
  const [signalCatalog, setSignalCatalog] = useState<SignalCatalogItemDto[]>([])
  const [alertLogs, setAlertLogs] = useState<AlertTriggerLogRecord[]>([])
  const [deliveryCfg, setDeliveryCfg] = useState<AlertDeliveryConfig>({
    osNotificationEnabled: true,
    telegramBotToken: '',
    telegramChatId: '',
    webhookUrl: '',
    watchlistBackgroundScanEnabled: true,
  })
  const [editingAlertId, setEditingAlertId] = useState<string | null>(null)
  const [editAlertPrice, setEditAlertPrice] = useState('')
  const [editAlertMessage, setEditAlertMessage] = useState('')
  const [editAlertFreq, setEditAlertFreq] = useState<'once' | 'once_per_bar_close'>('once')
  const [editAlertCooldown, setEditAlertCooldown] = useState('0')

  useEffect(() => {
    if (rightTab !== 'alerts') return
    if (signalCatalog.length === 0) {
      void api.getSignalCatalog().then(setSignalCatalog).catch(() => {})
    }
    if (alertSubTab === 'logs') {
      void api.listAlertLogs(100).then(setAlertLogs).catch(() => {})
    }
    if (alertSubTab === 'delivery') {
      void api
        .getAlertDeliveryConfig()
        .then((cfg) => {
          setDeliveryCfg(cfg)
          setOsNotificationEnabled(cfg.osNotificationEnabled)
        })
        .catch(() => {})
    }
  }, [rightTab, alertSubTab, signalCatalog.length])

  const armedAlertsCount = useMemo(
    () => alerts.filter((a) => a.status === 'armed').length,
    [alerts],
  )

  const handleAddSymbol = (e: React.FormEvent) => {
    e.preventDefault()
    const raw = addTickerInput.trim().toUpperCase()
    if (!raw) return
    const normalized = raw.includes(':') ? raw : `HOSE:${raw}`
    void addSymbolToWatchlist(normalized)
    setAddTickerInput('')
  }

  const handleCreateAlert = (e: React.FormEvent) => {
    e.preventDefault()
    const cooldownBars = Math.max(0, Math.min(500, Number(alertCooldownBars) || 0))
    if (alertMode === 'signal') {
      const sigMeta = signalCatalog.find((s) => s.id === alertSignalId)
      const defaultMsg = `Tín hiệu QMV: ${sigMeta?.labelVi || alertSignalId} (${activePane.interval})`
      void createNewAlert(
        activePane.ticker,
        alertCondition,
        0,
        alertMessage.trim() || defaultMsg,
        {
          alertKind: 'indicator',
          interval: activePane.interval,
          paramsJson: {
            indicator: 'SIGNAL',
            signalId: alertSignalId,
            threshold: Number(alertIndThreshold) || 0,
          },
          soundEnabled: alertSoundEnabled,
          frequency: alertFrequency,
          cooldownBars,
        },
      )
      setAlertMessage('')
      return
    }
    if (alertMode === 'confluence') {
      const thr = Number(alertConfluenceThreshold)
      const safeThr = Number.isFinite(thr) ? thr : 35
      const defaultMsg = `Điểm Hợp lưu (${alertConfluenceProfile === 'trend' ? 'Trend' : 'Balanced'}) ${alertCondition === 'cross_above' ? 'cắt lên' : alertCondition === 'cross_below' ? 'cắt xuống' : 'chạm'} ${safeThr}`
      void createNewAlert(
        activePane.ticker,
        alertCondition,
        safeThr,
        alertMessage.trim() || defaultMsg,
        {
          alertKind: 'indicator',
          interval: activePane.interval,
          paramsJson: {
            indicator: 'CONFLUENCE',
            profile: alertConfluenceProfile,
            threshold: safeThr,
          },
          soundEnabled: alertSoundEnabled,
          frequency: alertFrequency,
          cooldownBars,
        },
      )
      setAlertMessage('')
      return
    }
    if (alertMode === 'indicator') {
      const period = Math.max(2, Number(alertIndPeriod) || 14)
      const threshold = Number(alertIndThreshold)
      const safeThr = Number.isFinite(threshold) ? threshold : alertIndType === 'RSI' ? 70 : 0
      const defaultMsg =
        alertIndType === 'RSI'
          ? `RSI(${period}) ${alertCondition === 'cross_above' ? 'cắt lên' : alertCondition === 'cross_below' ? 'cắt xuống' : 'chạm'} ${safeThr} (${activePane.interval})`
          : alertIndType === 'MACD'
            ? `MACD Hist ${alertCondition === 'cross_above' ? 'cắt lên 0' : alertCondition === 'cross_below' ? 'cắt xuống 0' : 'cắt mốc 0'} (${activePane.interval})`
            : `Giá ${alertCondition === 'cross_above' ? 'cắt lên' : alertCondition === 'cross_below' ? 'cắt xuống' : 'chạm'} EMA(${period}) (${activePane.interval})`
      void createNewAlert(
        activePane.ticker,
        alertCondition,
        safeThr,
        alertMessage.trim() || defaultMsg,
        {
          alertKind: 'indicator',
          interval: activePane.interval,
          paramsJson: {
            indicator: alertIndType,
            period,
            threshold: safeThr,
            fast: 12,
            slow: 26,
            signal: 9,
          },
          soundEnabled: alertSoundEnabled,
          frequency: alertFrequency,
          cooldownBars,
        },
      )
      setAlertMessage('')
      return
    }
    const price = Number(alertPrice)
    if (!Number.isFinite(price) || price <= 0) return
    void createNewAlert(
      activePane.ticker,
      alertCondition,
      price,
      alertMessage.trim() || `${activePane.ticker} chạm mốc ${price.toLocaleString('en-US')}`,
      {
        alertKind: 'price',
        interval: activePane.interval,
        soundEnabled: alertSoundEnabled,
        frequency: alertFrequency,
        cooldownBars,
      },
    )
    setAlertPrice('')
    setAlertMessage('')
  }

  return (
    <aside
      aria-label="Bảng Danh mục theo dõi và Cảnh báo"
      className="flex flex-col shrink-0 h-full overflow-hidden"
      style={{
        width: rightPanelWidth,
        backgroundColor: 'var(--tv-bg-panel)',
        borderLeft: '1px solid var(--tv-border)',
      }}
    >
      {/* Right Rail Mode Switcher Tabs */}
      <div
        className="flex items-center justify-between px-1 shrink-0"
        style={{ height: 34, borderBottom: '1px solid var(--tv-border)' }}
      >
        <div role="tablist" aria-label="Chuyển đổi bảng phải" className="flex items-center h-full min-w-0">
          <button
            type="button"
            role="tab"
            id="rw-tab-watchlist"
            aria-selected={rightTab === 'watchlist'}
            className="tv-tab px-1.5 text-[11px] gap-1 whitespace-nowrap"
            onClick={() => setRightTab('watchlist')}
          >
            <List className="w-3 h-3 shrink-0" aria-hidden="true" />
            <span>Danh mục</span>
          </button>
          <button
            type="button"
            role="tab"
            id="rw-tab-alerts"
            aria-selected={rightTab === 'alerts'}
            className="tv-tab px-1.5 text-[11px] gap-1 whitespace-nowrap"
            onClick={() => setRightTab('alerts')}
          >
            <Bell className="w-3 h-3 shrink-0" aria-hidden="true" />
            <span>Cảnh báo</span>
            {armedAlertsCount > 0 && (
              <span
                className="px-1 rounded-full text-[9px] font-semibold"
                style={{ backgroundColor: 'var(--tv-accent-soft)', color: 'var(--tv-accent)' }}
              >
                {armedAlertsCount}
              </span>
            )}
          </button>
          <button
            type="button"
            role="tab"
            id="rw-tab-objects"
            aria-selected={rightTab === 'objects'}
            className="tv-tab px-1.5 text-[11px] gap-1 whitespace-nowrap"
            onClick={() => {
              setRightTab('objects')
              reloadTickerDrawings()
            }}
          >
            <Layers className="w-3 h-3 shrink-0" aria-hidden="true" />
            <span>Đối tượng</span>
          </button>
          <button
            type="button"
            role="tab"
            id="rw-tab-yumeko"
            aria-selected={rightTab === 'yumeko'}
            className="tv-tab px-1.5 text-[11px] gap-1 whitespace-nowrap"
            onClick={() => setRightTab('yumeko')}
          >
            <Sparkles className="w-3 h-3 shrink-0" aria-hidden="true" />
            <span>Yumeko</span>
          </button>
        </div>

        <button
          type="button"
          className="tv-icon-btn w-6 h-6 shrink-0"
          onClick={() => void refreshWatchlistQuotes()}
          title="Làm mới bảng giá"
        >
          <RefreshCw className="w-3 h-3" />
        </button>
      </div>

      {rightTab === 'watchlist' ? (
        <div role="tabpanel" aria-labelledby="rw-tab-watchlist" className="flex flex-col flex-1 min-h-0">
          {/* Watchlist Category Selector + Rename/Delete + Flag Filter + Add Symbol/Section (Wave 5 Pro) */}
          <div className="p-2 space-y-1.5 shrink-0" style={{ borderBottom: '1px solid var(--tv-border)' }}>
            <div className="flex items-center gap-1">
              <select
                aria-label="Chọn danh mục theo dõi"
                value={activeWatchlist?.id ?? ''}
                onChange={(e) => {
                  setActiveWatchlistId(e.target.value)
                  setRenamingList(false)
                }}
                className="flex-1 min-w-0 px-2 py-1 rounded text-[11.5px] font-medium outline-none cursor-pointer"
                style={{
                  backgroundColor: 'var(--tv-bg-input)',
                  color: 'var(--tv-text-primary)',
                  border: '1px solid var(--tv-border)',
                }}
              >
                {watchlists.map((w) => (
                  <option key={w.id} value={w.id}>
                    {w.name} ({w.tickers.filter((t) => !t.startsWith('###')).length})
                  </option>
                ))}
              </select>
              <button
                type="button"
                className="tv-icon-btn w-6 h-6 shrink-0"
                onClick={() => {
                  if (activeWatchlist) {
                    setRenameDraft(activeWatchlist.name)
                    setRenamingList((v) => !v)
                    setShowNewListForm(false)
                  }
                }}
                title="Đổi tên danh mục hiện tại"
              >
                <Edit3 className="w-3 h-3" />
              </button>
              <button
                type="button"
                className="tv-icon-btn w-6 h-6 shrink-0"
                onClick={() => {
                  setShowNewListForm((v) => !v)
                  setRenamingList(false)
                }}
                title="Tạo danh mục mới"
              >
                <Plus className="w-3.5 h-3.5" />
              </button>
              {watchlists.length > 1 && activeWatchlist && (
                <button
                  type="button"
                  className="tv-icon-btn w-6 h-6 shrink-0 hover:text-[#F23645]"
                  onClick={() => void deleteWatchlistById(activeWatchlist.id)}
                  title="Xóa danh mục hiện tại"
                >
                  <Trash2 className="w-3 h-3" />
                </button>
              )}
            </div>

            {renamingList && activeWatchlist && (
              <div className="flex gap-1">
                <input
                  type="text"
                  value={renameDraft}
                  onChange={(e) => setRenameDraft(e.target.value)}
                  placeholder="Tên mới cho danh mục..."
                  className="flex-1 px-2 py-1 rounded text-[11px] outline-none"
                  style={{
                    backgroundColor: 'var(--tv-bg-input)',
                    border: '1px solid var(--tv-border)',
                    color: 'var(--tv-text-primary)',
                  }}
                />
                <button
                  type="button"
                  className="px-2 py-1 rounded text-[11px] font-semibold bg-[#089981] text-white border-0 cursor-pointer"
                  onClick={() => {
                    if (!renameDraft.trim()) return
                    void renameWatchlist(activeWatchlist.id, renameDraft)
                    setRenamingList(false)
                  }}
                >
                  Lưu
                </button>
              </div>
            )}

            {showNewListForm && (
              <div className="flex gap-1">
                <input
                  type="text"
                  value={newListName}
                  onChange={(e) => setNewListName(e.target.value)}
                  placeholder="Tên danh mục mới..."
                  className="flex-1 px-2 py-1 rounded text-[11px] outline-none"
                  style={{
                    backgroundColor: 'var(--tv-bg-input)',
                    border: '1px solid var(--tv-border)',
                    color: 'var(--tv-text-primary)',
                  }}
                />
                <button
                  type="button"
                  className="px-2 py-1 rounded text-[11px] font-semibold bg-[#2962FF] text-white border-0 cursor-pointer"
                  onClick={() => {
                    if (!newListName.trim()) return
                    void createNewWatchlist(newListName)
                    setNewListName('')
                    setShowNewListForm(false)
                  }}
                >
                  Tạo
                </button>
              </div>
            )}

            <form onSubmit={handleAddSymbol} className="flex gap-1">
              <input
                type="text"
                value={addTickerInput}
                onChange={(e) => setAddTickerInput(e.target.value)}
                placeholder="Thêm mã (FPT) hoặc ###PHÂN MỤC..."
                className="flex-1 px-2 py-1 rounded text-[11px] outline-none"
                style={{
                  backgroundColor: 'var(--tv-bg-input)',
                  border: '1px solid var(--tv-border)',
                  color: 'var(--tv-text-primary)',
                }}
              />
              <button
                type="submit"
                className="px-2 py-1 rounded text-[11px] font-medium border cursor-pointer"
                style={{
                  backgroundColor: 'var(--tv-hover)',
                  borderColor: 'var(--tv-border)',
                  color: 'var(--tv-text-primary)',
                }}
                title="Thêm mã hoặc tiêu đề phân mục (###SECTION:Tên)"
              >
                +
              </button>
            </form>

            {/* Color Flag Filter Strip */}
            <div className="flex items-center justify-between gap-1 text-[10px]">
              <div className="flex items-center gap-1">
                <button
                  type="button"
                  onClick={() => setFlagFilter('all')}
                  className="px-1.5 py-0.5 rounded border-0 cursor-pointer text-[9.5px] font-semibold"
                  style={{
                    backgroundColor: flagFilter === 'all' ? 'var(--tv-accent-soft)' : 'transparent',
                    color: flagFilter === 'all' ? 'var(--tv-accent)' : 'var(--tv-text-secondary)',
                  }}
                >
                  Tất cả
                </button>
                {FLAG_COLORS.map((fl) => (
                  <button
                    key={fl.key}
                    type="button"
                    aria-label={`Lọc theo cờ ${fl.label}`}
                    title={fl.label}
                    onClick={() => setFlagFilter((prev) => (prev === fl.key ? 'all' : fl.key))}
                    className="w-3.5 h-3.5 rounded-full p-0 cursor-pointer"
                    style={{
                      backgroundColor: fl.hex,
                      border: flagFilter === fl.key ? '2px solid #FFFFFF' : '1px solid rgba(255,255,255,0.2)',
                      opacity: flagFilter === 'all' || flagFilter === fl.key ? 1 : 0.35,
                    }}
                  />
                ))}
              </div>
              <button
                type="button"
                onClick={() => {
                  const label = `###SECTION:NHÓM ${(activeWatchlist?.tickers.filter((t) => t.startsWith('###')).length ?? 0) + 1}`
                  void addSymbolToWatchlist(label)
                }}
                className="border-0 bg-transparent p-0 text-[9.5px] cursor-pointer text-[var(--tv-text-secondary)] hover:text-[var(--tv-accent)]"
                title="Thêm thanh phân mục (Section Header) để gom nhóm mã"
              >
                + Phân mục
              </button>
            </div>
          </div>

          {/* Sortable Watchlist Table Header */}
          <div
            className="grid grid-cols-[1fr_76px_64px_16px] gap-x-2 items-center px-2 py-1 text-[10.5px] font-medium shrink-0 select-none"
            style={{ color: 'var(--tv-text-secondary)', borderBottom: '1px solid var(--tv-border)' }}
          >
            {(
              [
                { key: 'symbol', label: 'Mã', align: 'text-left' },
                { key: 'price', label: 'Giá khớp', align: 'text-right' },
                { key: 'pct', label: '% Đổi', align: 'text-right' },
              ] as const
            ).map((col) => {
              const activeSort = wlSort.key === col.key
              return (
                <button
                  key={col.key}
                  type="button"
                  onClick={() =>
                    setWlSort((prev) =>
                      prev.key !== col.key
                        ? { key: col.key, dir: col.key === 'symbol' ? 'asc' : 'desc' }
                        : prev.dir === 'desc'
                          ? { key: col.key, dir: 'asc' }
                          : { key: 'none', dir: 'desc' },
                    )
                  }
                  className={`${col.align} whitespace-nowrap border-0 bg-transparent p-0 cursor-pointer font-medium text-[10.5px]`}
                  style={{ color: activeSort ? 'var(--tv-accent)' : 'var(--tv-text-secondary)' }}
                >
                  {col.label}
                  {activeSort ? (wlSort.dir === 'asc' ? ' ▲' : ' ▼') : ''}
                </button>
              )
            })}
            <span />
          </div>

          {/* Watchlist Rows with Section Headers, Color Flags, Drag Reorder & Blind Mode */}
          <div className="flex-1 overflow-y-auto tv-scroll">
            {(() => {
              const rawTickers = activeWatchlist?.tickers ?? []
              let items = rawTickers.filter((tk) => {
                if (tk.startsWith('###')) return flagFilter === 'all' && wlSort.key === 'none'
                if (flagFilter === 'all') return true
                return symbolFlags[tk.toUpperCase()] === flagFilter
              })
              if (wlSort.key !== 'none') {
                items = [...items].sort((a, b) => {
                  const qa = buildEffectiveQuote(a)
                  const qb = buildEffectiveQuote(b)
                  let diff = 0
                  if (wlSort.key === 'symbol') {
                    diff = a.localeCompare(b)
                  } else if (wlSort.key === 'price') {
                    diff = (qa?.lastPrice ?? -Infinity) - (qb?.lastPrice ?? -Infinity)
                  } else {
                    diff = (qa?.changePercent ?? -Infinity) - (qb?.changePercent ?? -Infinity)
                  }
                  return wlSort.dir === 'asc' ? diff : -diff
                })
              }
              const cycleFlagFor = (tk: string) => {
                const cur = symbolFlags[tk.toUpperCase()]
                const idx = FLAG_COLORS.findIndex((f) => f.key === cur)
                const next = idx === -1 ? FLAG_COLORS[0].key : idx + 1 < FLAG_COLORS.length ? FLAG_COLORS[idx + 1].key : null
                void setSymbolFlag(tk, next)
              }
              const handleDropOn = (targetTicker: string) => {
                if (!dragTicker || !activeWatchlist || dragTicker === targetTicker) return
                const list = [...activeWatchlist.tickers]
                const fromIdx = list.indexOf(dragTicker)
                const toIdx = list.indexOf(targetTicker)
                if (fromIdx === -1 || toIdx === -1) return
                const [moved] = list.splice(fromIdx, 1)
                list.splice(toIdx, 0, moved)
                setDragTicker(null)
                setWlSort({ key: 'none', dir: 'desc' })
                void reorderWatchlistSymbols(activeWatchlist.id, list)
              }
              return items.map((ticker) => {
                if (ticker.startsWith('###')) {
                  const title = ticker.replace(/^###(?:SECTION:)?/i, '').trim() || 'PHÂN MỤC'
                  return (
                    <div
                      key={ticker}
                      draggable
                      onDragStart={() => setDragTicker(ticker)}
                      onDragOver={(e) => e.preventDefault()}
                      onDrop={() => handleDropOn(ticker)}
                      className="group flex items-center justify-between px-2 py-1 text-[10px] font-bold uppercase tracking-wider select-none"
                      style={{
                        backgroundColor: 'var(--tv-bg-elevated)',
                        color: 'var(--tv-text-secondary)',
                        borderBottom: '1px solid var(--tv-border)',
                      }}
                    >
                      <span className="truncate">{title}</span>
                      <button
                        type="button"
                        aria-label={`Xóa phân mục ${title}`}
                        className="opacity-0 group-hover:opacity-100 border-0 bg-transparent p-0 cursor-pointer text-[var(--tv-text-secondary)] hover:text-[#F23645]"
                        onClick={() => void removeSymbolFromWatchlist(ticker)}
                      >
                        ×
                      </button>
                    </div>
                  )
                }
                const isBlindRow = Boolean(replayMaskTicker && ticker.toUpperCase() === replayMaskTicker)
                return (
                  <WatchlistRow
                    key={ticker}
                    ticker={ticker}
                    quote={buildEffectiveQuote(ticker)}
                    flash={quoteFlashMap[ticker]}
                    flagColor={symbolFlags[ticker.toUpperCase()]}
                    isSelected={activePane.ticker === ticker}
                    isBlindMasked={isBlindRow}
                    onSelect={() => setActiveTicker(ticker)}
                    onCycleFlag={() => cycleFlagFor(ticker)}
                    onRemove={() => void removeSymbolFromWatchlist(ticker)}
                    onDragStart={() => setDragTicker(ticker)}
                    onDragOver={(e) => e.preventDefault()}
                    onDrop={() => handleDropOn(ticker)}
                  />
                )
              })
            })()}
          </div>

          {/* Active Symbol Detail Card (Trần / Tham chiếu / Sàn / Bước giá / Khối lượng) */}
          <div
            className="p-2.5 shrink-0 space-y-2"
            style={{
              borderTop: '1px solid var(--tv-border)',
              backgroundColor: 'var(--tv-bg-elevated)',
            }}
          >
            {isActiveBlindReplay && (
              <div
                className="flex items-center gap-1.5 px-2 py-0.5 rounded text-[10px] font-semibold"
                style={{ backgroundColor: 'var(--tv-accent-soft)', color: 'var(--tv-accent)' }}
              >
                <EyeOff className="w-3 h-3" aria-hidden="true" />
                <span>Chế độ Luyện mù — Đang hiển thị giá tại mốc Replay</span>
              </div>
            )}
            <div className="flex items-start justify-between gap-2">
              <div className="min-w-0">
                <div className="flex items-center gap-1.5">
                  <span className="text-[14px] font-bold tracking-tight">
                    {activeQuote?.symbol ?? activePane.ticker.split(':')[1] ?? activePane.ticker}
                  </span>
                  <span
                    className="text-[10px] px-1.5 py-0.5 rounded font-semibold"
                    style={{ backgroundColor: 'var(--tv-hover)', color: 'var(--tv-text-secondary)' }}
                  >
                    {activeQuote?.exchange ?? activePane.ticker.split(':')[0]}
                  </span>
                  {activeQuote?.vnBand && (
                    <span
                      className="text-[10px] px-1 py-0.5 rounded font-medium"
                      style={{ backgroundColor: 'var(--tv-accent-soft)', color: 'var(--tv-accent)' }}
                    >
                      {activeQuote.vnBand.label}
                    </span>
                  )}
                </div>
                <div className="text-[11px] truncate" style={{ color: 'var(--tv-text-secondary)' }}>
                  {activeQuote?.description ?? activePane.ticker}
                </div>
              </div>
              <div className="text-right tabular-nums shrink-0">
                <div className="text-[15px] font-bold">
                  {formatPrice(activeQuote?.lastPrice, activeQuote?.pricePrecision ?? 0)}
                </div>
                <div
                  className="text-[11px] font-semibold"
                  style={{
                    color:
                      (activeQuote?.changePercent ?? 0) >= 0 ? 'var(--tv-up)' : 'var(--tv-down)',
                  }}
                >
                  {activeQuote?.change != null
                    ? `${activeQuote.change >= 0 ? '+' : ''}${formatPrice(activeQuote.change, activeQuote.pricePrecision)} (${(activeQuote.changePercent ?? 0).toFixed(2)}%)`
                    : '—'}
                </div>
              </div>
            </div>

            {activeQuote?.vnBand ? (
              <div className="grid grid-cols-3 gap-1.5 text-center tabular-nums">
                <div className="rounded py-1 px-1" style={{ backgroundColor: 'var(--tv-bg-panel)', border: '1px solid var(--tv-border)' }}>
                  <div className="text-[9.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
                    Trần
                  </div>
                  <div className="font-bold text-[11.5px]" style={{ color: 'var(--tv-vn-ceil)' }}>
                    {formatPrice(activeQuote.vnBand.ceiling, activeQuote.pricePrecision)}
                  </div>
                </div>
                <div className="rounded py-1 px-1" style={{ backgroundColor: 'var(--tv-bg-panel)', border: '1px solid var(--tv-border)' }}>
                  <div className="text-[9.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
                    Tham chiếu
                  </div>
                  <div className="font-bold text-[11.5px]" style={{ color: 'var(--tv-vn-ref)' }}>
                    {formatPrice(activeQuote.vnBand.reference, activeQuote.pricePrecision)}
                  </div>
                </div>
                <div className="rounded py-1 px-1" style={{ backgroundColor: 'var(--tv-bg-panel)', border: '1px solid var(--tv-border)' }}>
                  <div className="text-[9.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
                    Sàn
                  </div>
                  <div className="font-bold text-[11.5px]" style={{ color: 'var(--tv-vn-floor)' }}>
                    {formatPrice(activeQuote.vnBand.floor, activeQuote.pricePrecision)}
                  </div>
                </div>
              </div>
            ) : null}

            <div
              className="flex items-center justify-between text-[11px] tabular-nums pt-1 border-t"
              style={{ borderColor: 'var(--tv-border)', color: 'var(--tv-text-secondary)' }}
            >
              <span>
                Bước giá:{' '}
                <strong style={{ color: 'var(--tv-text-primary)' }}>
                  {(() => {
                    const isVn30f =
                      activePane.ticker.toUpperCase().includes('VN30F') ||
                      activePane.ticker.toUpperCase().includes('VN301!')
                    if (isVn30f) return `${activeQuote?.vnBand?.tickSize ?? 0.1} điểm`
                    if (activeQuote?.vnBand?.tickSize) return `${activeQuote.vnBand.tickSize} VND`
                    const prec = activeQuote?.pricePrecision ?? 2
                    if (prec === 0) return '100'
                    return (10 ** -prec).toFixed(prec)
                  })()}
                </strong>
              </span>
              <span>
                KL:{' '}
                <strong style={{ color: 'var(--tv-text-primary)' }}>
                  {formatVol(activeQuote?.volume)}
                </strong>
              </span>
            </div>

            {/* Compact Fundamental & Valuation Card (DN01 / TradingView Cache, Wave 4) */}
            {activeFunda && activeFunda.source !== 'none' && !isActiveBlindReplay && (
              <div
                className="pt-1.5 border-t space-y-1 text-[10.5px] tabular-nums"
                style={{ borderColor: 'var(--tv-border)' }}
              >
                <div className="flex items-center justify-between gap-1">
                  <span className="font-semibold uppercase tracking-wide text-[9.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
                    Cơ bản &amp; Định giá
                  </span>
                  <span
                    className="px-1.5 py-0 rounded text-[9px] font-semibold"
                    style={{
                      backgroundColor:
                        activeFunda.source === 'yumeko_dn01'
                          ? 'rgba(8, 153, 129, 0.16)'
                          : 'rgba(41, 98, 255, 0.14)',
                      color: activeFunda.source === 'yumeko_dn01' ? '#089981' : '#5B8DEF',
                    }}
                  >
                    {activeFunda.source === 'yumeko_dn01' ? 'Yumeko DN01' : 'TV Cache'}
                  </span>
                </div>
                <div className="grid grid-cols-3 gap-1">
                  <div className="rounded px-1.5 py-0.5" style={{ backgroundColor: 'var(--tv-bg-panel)' }}>
                    <div className="text-[9px]" style={{ color: 'var(--tv-text-secondary)' }}>P/E</div>
                    <div className="font-semibold">{activeFunda.pe != null ? `${activeFunda.pe.toFixed(1)}x` : '—'}</div>
                  </div>
                  <div className="rounded px-1.5 py-0.5" style={{ backgroundColor: 'var(--tv-bg-panel)' }}>
                    <div className="text-[9px]" style={{ color: 'var(--tv-text-secondary)' }}>P/B</div>
                    <div className="font-semibold">{activeFunda.pb != null ? `${activeFunda.pb.toFixed(2)}x` : '—'}</div>
                  </div>
                  <div className="rounded px-1.5 py-0.5" style={{ backgroundColor: 'var(--tv-bg-panel)' }}>
                    <div className="text-[9px]" style={{ color: 'var(--tv-text-secondary)' }}>ROE</div>
                    <div
                      className="font-semibold"
                      style={{ color: (activeFunda.roe ?? 0) >= 15 ? 'var(--tv-up)' : undefined }}
                    >
                      {activeFunda.roe != null ? `${activeFunda.roe.toFixed(1)}%` : '—'}
                    </div>
                  </div>
                </div>
                <div className="flex items-center justify-between text-[10px]" style={{ color: 'var(--tv-text-secondary)' }}>
                  <span className="truncate max-w-[140px]" title={activeFunda.sector ?? ''}>
                    {activeFunda.sector ?? '—'}
                  </span>
                  <span>
                    {activeFunda.marketCapBasic != null && activeFunda.marketCapBasic > 0
                      ? `VH: ${(activeFunda.marketCapBasic / 1e12).toFixed(1)} nghìn tỷ`
                      : activeFunda.eps != null
                        ? `EPS: ${Math.round(activeFunda.eps).toLocaleString('en-US')}`
                        : ''}
                  </span>
                </div>
              </div>
            )}
          </div>
        </div>
      ) : rightTab === 'alerts' ? (
        /* Alerts 2.0 Panel: Price / Indicator / 123 QMV Signals / Confluence + Logs + Telegram/Webhook (Wave 3) */
        <div role="tabpanel" aria-labelledby="rw-tab-alerts" className="flex flex-col flex-1 min-h-0">
          {/* Alerts 2.0 Sub-Navigation Strip */}
          <div
            className="flex items-center justify-between gap-1 px-2 py-1 shrink-0"
            style={{ borderBottom: '1px solid var(--tv-border)', backgroundColor: 'var(--tv-bg-elevated)' }}
          >
            <div className="flex items-center gap-0.5">
              {(
                [
                  { id: 'list', label: `Cảnh báo (${alerts.length})` },
                  { id: 'logs', label: 'Lịch sử' },
                  { id: 'delivery', label: 'Kênh báo' },
                ] as const
              ).map((st) => (
                <button
                  key={st.id}
                  type="button"
                  onClick={() => setAlertSubTab(st.id)}
                  className="px-1.5 py-0.5 rounded text-[10px] font-semibold border-0 cursor-pointer"
                  style={{
                    backgroundColor: alertSubTab === st.id ? 'var(--tv-accent-soft)' : 'transparent',
                    color: alertSubTab === st.id ? 'var(--tv-accent)' : 'var(--tv-text-secondary)',
                  }}
                >
                  {st.label}
                </button>
              ))}
            </div>
            <button
              type="button"
              className="px-1.5 py-0.5 rounded text-[10px] font-semibold border cursor-pointer"
              style={{ borderColor: 'var(--tv-border)', backgroundColor: 'transparent', color: 'var(--tv-text-primary)' }}
              onClick={() => void scanAllWatchlistAlerts(false)}
              title="Quét ngay tất cả cảnh báo đang chờ trên mọi mã Watchlist"
            >
              Quét WL
            </button>
          </div>

          {alertSubTab === 'delivery' ? (
            <div className="flex-1 overflow-y-auto tv-scroll p-2.5 space-y-2.5 text-[11px]">
              <div className="text-[10.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
                Cấu hình lưu riêng tại <code>~/.config/sumi/alerts_secrets.json</code> (quyền 0600, tuyệt đối không xuất vào Sync Bundle).
              </div>
              <label className="flex items-center gap-1.5 cursor-pointer">
                <input
                  type="checkbox"
                  checked={deliveryCfg.osNotificationEnabled}
                  onChange={(e) => setDeliveryCfg((c) => ({ ...c, osNotificationEnabled: e.target.checked }))}
                />
                <span>Thông báo Hệ điều hành (OS Notification)</span>
              </label>
              <label className="flex items-center gap-1.5 cursor-pointer">
                <input
                  type="checkbox"
                  checked={deliveryCfg.watchlistBackgroundScanEnabled}
                  onChange={(e) => setDeliveryCfg((c) => ({ ...c, watchlistBackgroundScanEnabled: e.target.checked }))}
                />
                <span>Tự quét Watchlist Alerts mỗi 60 giây</span>
              </label>
              <div className="space-y-1">
                <div style={{ color: 'var(--tv-text-secondary)' }}>Telegram Bot Token</div>
                <input
                  type="password"
                  value={deliveryCfg.telegramBotToken}
                  onChange={(e) => setDeliveryCfg((c) => ({ ...c, telegramBotToken: e.target.value }))}
                  placeholder="123456:ABC-DEF..."
                  className="w-full px-2 py-1 rounded text-[11px] outline-none"
                  style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                />
              </div>
              <div className="space-y-1">
                <div style={{ color: 'var(--tv-text-secondary)' }}>Telegram Chat ID</div>
                <input
                  type="text"
                  value={deliveryCfg.telegramChatId}
                  onChange={(e) => setDeliveryCfg((c) => ({ ...c, telegramChatId: e.target.value }))}
                  placeholder="-100123456789 hoặc user chat id"
                  className="w-full px-2 py-1 rounded text-[11px] outline-none"
                  style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                />
              </div>
              <div className="space-y-1">
                <div style={{ color: 'var(--tv-text-secondary)' }}>Webhook POST URL (Discord / Slack / n8n)</div>
                <input
                  type="url"
                  value={deliveryCfg.webhookUrl}
                  onChange={(e) => setDeliveryCfg((c) => ({ ...c, webhookUrl: e.target.value }))}
                  placeholder="https://..."
                  className="w-full px-2 py-1 rounded text-[11px] outline-none"
                  style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                />
              </div>
              <div className="flex items-center gap-1.5 pt-1">
                <button
                  type="button"
                  className="flex-1 py-1.5 rounded text-[11.5px] font-semibold bg-[#2962FF] text-white border-0 cursor-pointer"
                  onClick={() => {
                    void api
                      .saveAlertDeliveryConfig(deliveryCfg)
                      .then((saved) => {
                        setDeliveryCfg(saved)
                        setOsNotificationEnabled(saved.osNotificationEnabled)
                        pushToast('success', 'Đã lưu cấu hình kênh báo riêng tư (0600)')
                      })
                      .catch((err) => pushToast('error', String(err)))
                  }}
                >
                  Lưu cấu hình
                </button>
                <button
                  type="button"
                  className="inline-flex items-center gap-1 px-2.5 py-1.5 rounded text-[11px] font-semibold border cursor-pointer"
                  style={{ borderColor: 'var(--tv-border)', backgroundColor: 'var(--tv-bg-elevated)', color: 'var(--tv-text-primary)' }}
                  onClick={() => {
                    void api
                      .testAlertDelivery()
                      .then((ch) => pushToast('info', `Đã gửi thử qua kênh: ${ch.join(', ')}`))
                      .catch((err) => pushToast('error', String(err)))
                  }}
                >
                  <Send className="w-3 h-3" />
                  <span>Gửi thử</span>
                </button>
              </div>
            </div>
          ) : alertSubTab === 'logs' ? (
            <div className="flex-1 flex flex-col min-h-0">
              <div className="flex items-center justify-between px-2.5 py-1 text-[10.5px]" style={{ borderBottom: '1px solid var(--tv-border)', color: 'var(--tv-text-secondary)' }}>
                <span>Nhật ký kích hoạt ({alertLogs.length})</span>
                {alertLogs.length > 0 && (
                  <button
                    type="button"
                    className="border-0 bg-transparent p-0 text-[#F23645] cursor-pointer text-[10.5px]"
                    onClick={() => void api.clearAlertLogs().then(() => setAlertLogs([]))}
                  >
                    Xóa log
                  </button>
                )}
              </div>
              <div className="flex-1 overflow-y-auto tv-scroll p-2 space-y-1.5">
                {alertLogs.length === 0 ? (
                  <div className="text-center py-8 text-[11.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
                    Chưa có cảnh báo nào kích hoạt trong lịch sử.
                  </div>
                ) : (
                  alertLogs.map((lg) => (
                    <div
                      key={lg.id}
                      className="p-2 rounded text-[10.5px] space-y-0.5"
                      style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}
                    >
                      <div className="flex items-center justify-between">
                        <strong className="text-[#2962FF]">{lg.ticker}</strong>
                        <span className="tabular-nums" style={{ color: 'var(--tv-text-secondary)' }}>
                          {new Date(lg.triggeredAtMs).toLocaleString('vi-VN', { day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit' })}
                        </span>
                      </div>
                      <div>{lg.message}</div>
                      <div className="flex items-center justify-between text-[9.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
                        <span>Giá khớp: {lg.triggerPrice.toLocaleString('en-US')}</span>
                        <span>Kênh: {lg.deliveredChannels}</span>
                      </div>
                    </div>
                  ))
                )}
              </div>
            </div>
          ) : (
            <>
              <form
                onSubmit={handleCreateAlert}
                className="p-2.5 space-y-2 shrink-0"
                style={{ borderBottom: '1px solid var(--tv-border)' }}
              >
                <div className="flex items-center justify-between gap-1">
                  <span className="text-[10.5px] font-semibold truncate" style={{ color: 'var(--tv-text-secondary)' }}>
                    {activePane.ticker}
                  </span>
                  <div className="flex items-center gap-0.5 shrink-0">
                    {(
                      [
                        { id: 'price', label: 'Giá' },
                        { id: 'indicator', label: 'Chỉ báo' },
                        { id: 'signal', label: '123 QMV' },
                        { id: 'confluence', label: 'Hợp lưu' },
                      ] as const
                    ).map((m) => (
                      <button
                        key={m.id}
                        type="button"
                        className="px-1.5 py-0.5 rounded text-[9.5px] font-semibold border-0 cursor-pointer"
                        style={{
                          backgroundColor: alertMode === m.id ? 'var(--tv-accent)' : 'var(--tv-hover)',
                          color: alertMode === m.id ? '#FFFFFF' : 'var(--tv-text-secondary)',
                        }}
                        onClick={() => setAlertMode(m.id)}
                      >
                        {m.label}
                      </button>
                    ))}
                  </div>
                </div>

                {alertMode === 'price' ? (
                  <div className="grid grid-cols-2 gap-1.5">
                    <select
                      aria-label="Điều kiện cảnh báo"
                      value={alertCondition}
                      onChange={(e) => setAlertCondition(e.target.value as 'cross_above' | 'cross_below' | 'cross_any')}
                      className="px-2 py-1 rounded text-[11px] outline-none"
                      style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                    >
                      <option value="cross_above">Cắt lên trên ▲</option>
                      <option value="cross_below">Cắt xuống dưới ▼</option>
                      <option value="cross_any">Chạm mức giá ◇</option>
                    </select>
                    <input
                      type="number"
                      step="any"
                      value={alertPrice}
                      onChange={(e) => setAlertPrice(e.target.value)}
                      placeholder={activeQuote?.lastPrice ? String(activeQuote.lastPrice) : 'Mức giá...'}
                      className="px-2 py-1 rounded text-[11px] tabular-nums outline-none"
                      style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                    />
                  </div>
                ) : alertMode === 'signal' ? (
                  <select
                    aria-label="Chọn tín hiệu từ 123 QMV Signal Catalog"
                    value={alertSignalId}
                    onChange={(e) => setAlertSignalId(e.target.value)}
                    className="w-full px-2 py-1 rounded text-[11px] outline-none"
                    style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                  >
                    {(signalCatalog.length > 0 ? signalCatalog : [{ id: 'qmv.ichimoku_v7_bullish', labelVi: 'Ichimoku V7 Bullish', categoryLabel: 'QMV' } as SignalCatalogItemDto]).map((sig) => (
                      <option key={sig.id} value={sig.id}>
                        [{sig.categoryLabel}] {sig.labelVi}
                      </option>
                    ))}
                  </select>
                ) : alertMode === 'confluence' ? (
                  <div className="grid grid-cols-3 gap-1">
                    <select
                      value={alertConfluenceProfile}
                      onChange={(e) => setAlertConfluenceProfile(e.target.value as 'balanced' | 'trend')}
                      className="px-1.5 py-1 rounded text-[10.5px] outline-none"
                      style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                    >
                      <option value="balanced">Balanced</option>
                      <option value="trend">Trend</option>
                    </select>
                    <select
                      value={alertCondition}
                      onChange={(e) => setAlertCondition(e.target.value as 'cross_above' | 'cross_below' | 'cross_any')}
                      className="px-1.5 py-1 rounded text-[10.5px] outline-none"
                      style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                    >
                      <option value="cross_above">Cắt lên ▲</option>
                      <option value="cross_below">Cắt xuống ▼</option>
                    </select>
                    <input
                      type="number"
                      value={alertConfluenceThreshold}
                      onChange={(e) => setAlertConfluenceThreshold(e.target.value)}
                      placeholder="35"
                      className="px-1.5 py-1 rounded text-[11px] tabular-nums outline-none"
                      style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                    />
                  </div>
                ) : (
                  <div className="space-y-1.5">
                    <div className="grid grid-cols-2 gap-1.5">
                      <select
                        aria-label="Chọn chỉ báo cảnh báo"
                        value={alertIndType}
                        onChange={(e) => {
                          const next = e.target.value as 'RSI' | 'MACD' | 'EMA'
                          setAlertIndType(next)
                          setAlertIndPeriod(next === 'EMA' ? '20' : '14')
                          setAlertIndThreshold(next === 'RSI' ? '70' : '0')
                        }}
                        className="px-2 py-1 rounded text-[11px] outline-none"
                        style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                      >
                        <option value="RSI">RSI (Wilder)</option>
                        <option value="MACD">MACD Hist (0)</option>
                        <option value="EMA">Giá cắt EMA</option>
                      </select>
                      <select
                        aria-label="Hướng cắt chỉ báo"
                        value={alertCondition}
                        onChange={(e) => setAlertCondition(e.target.value as 'cross_above' | 'cross_below' | 'cross_any')}
                        className="px-2 py-1 rounded text-[11px] outline-none"
                        style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                      >
                        <option value="cross_above">Cắt lên ▲</option>
                        <option value="cross_below">Cắt xuống ▼</option>
                        <option value="cross_any">Cắt hai chiều ◇</option>
                      </select>
                    </div>
                    {alertIndType !== 'MACD' && (
                      <div className="grid grid-cols-2 gap-1.5">
                        <input
                          type="number"
                          aria-label="Chu kỳ chỉ báo"
                          value={alertIndPeriod}
                          onChange={(e) => setAlertIndPeriod(e.target.value)}
                          placeholder="Chu kỳ (14)"
                          className="px-2 py-1 rounded text-[11px] tabular-nums outline-none"
                          style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                        />
                        {alertIndType === 'RSI' && (
                          <input
                            type="number"
                            aria-label="Ngưỡng RSI"
                            value={alertIndThreshold}
                            onChange={(e) => setAlertIndThreshold(e.target.value)}
                            placeholder="Ngưỡng (70 / 30)"
                            className="px-2 py-1 rounded text-[11px] tabular-nums outline-none"
                            style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                          />
                        )}
                      </div>
                    )}
                  </div>
                )}

                {/* Cadence + Cooldown */}
                <div className="grid grid-cols-[1fr_76px] gap-1.5">
                  <select
                    aria-label="Tần suất kích hoạt"
                    value={alertFrequency}
                    onChange={(e) => setAlertFrequency(e.target.value as 'once' | 'once_per_bar_close')}
                    className="px-2 py-1 rounded text-[10.5px] outline-none"
                    style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                  >
                    <option value="once">Chỉ báo 1 lần (Once)</option>
                    <option value="once_per_bar_close">Mỗi lần đóng nến (Lặp lại)</option>
                  </select>
                  <input
                    type="number"
                    min={0}
                    max={500}
                    value={alertCooldownBars}
                    onChange={(e) => setAlertCooldownBars(e.target.value)}
                    placeholder="Nghỉ nến"
                    title="Số phiên nghỉ (cooldown bars) giữa 2 lần kích hoạt"
                    className="px-1.5 py-1 rounded text-[10.5px] tabular-nums outline-none"
                    style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                  />
                </div>

                <input
                  type="text"
                  value={alertMessage}
                  onChange={(e) => setAlertMessage(e.target.value)}
                  placeholder="Ghi chú cảnh báo (tùy chọn)..."
                  className="w-full px-2 py-1 rounded text-[11px] outline-none"
                  style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                />

                <div className="flex items-center justify-between text-[10.5px]">
                  <label className="flex items-center gap-1.5 cursor-pointer" style={{ color: 'var(--tv-text-secondary)' }}>
                    <input
                      type="checkbox"
                      checked={alertSoundEnabled}
                      onChange={(e) => setAlertSoundEnabled(e.target.checked)}
                    />
                    <span>Âm báo (Chime)</span>
                  </label>
                  <button
                    type="button"
                    className="inline-flex items-center gap-1 border-0 bg-transparent p-0 cursor-pointer text-[10.5px] text-[var(--tv-accent)] hover:underline"
                    onClick={() => playAlertChime()}
                    title="Nghe thử âm báo chuông"
                  >
                    <Volume2 className="w-3 h-3" aria-hidden="true" />
                    <span>Thử âm</span>
                  </button>
                </div>

                <button
                  type="submit"
                  className="w-full py-1.5 rounded text-[11.5px] font-semibold bg-[#2962FF] hover:bg-[#1E53E5] text-white border-0 cursor-pointer"
                >
                  {alertMode === 'signal'
                    ? '+ Thêm Cảnh Báo Tín Hiệu QMV'
                    : alertMode === 'confluence'
                      ? '+ Thêm Cảnh Báo Hợp Lưu'
                      : alertMode === 'indicator'
                        ? `+ Thêm Cảnh Báo ${alertIndType}`
                        : '+ Thêm Cảnh Báo Giá'}
                </button>
              </form>

              <div className="flex-1 overflow-y-auto tv-scroll p-2 space-y-1.5">
                {alerts.length === 0 ? (
                  <div className="text-center py-8 text-[11.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
                    Chưa có cảnh báo nào. Tạo cảnh báo Giá, Chỉ báo, 123 QMV hoặc Hợp lưu ở trên.
                  </div>
                ) : (
                  alerts.map((a) => {
                    const kindTag =
                      a.alertKind === 'drawing' ? 'ĐƯỜNG VẼ' : a.alertKind === 'indicator' ? 'CHỈ BÁO' : 'GIÁ'
                    const isEditing = editingAlertId === a.id
                    return (
                      <div
                        key={a.id}
                        className="p-2 rounded space-y-1"
                        style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}
                      >
                        <div className="flex items-center justify-between gap-1">
                          <div className="flex items-center gap-1 min-w-0">
                            <button
                              type="button"
                              className="font-semibold text-[12px] bg-transparent border-0 p-0 cursor-pointer text-[var(--tv-text-primary)] hover:text-[#2962FF] truncate"
                              onClick={() => setActiveTicker(a.ticker)}
                            >
                              {a.ticker}
                            </button>
                            <span
                              className="px-1 py-0 rounded text-[9px] font-bold shrink-0"
                              style={{ backgroundColor: 'var(--tv-hover)', color: 'var(--tv-text-secondary)' }}
                            >
                              {kindTag}
                            </span>
                            {a.frequency === 'once_per_bar_close' && (
                              <span className="px-1 py-0 rounded text-[8.5px] font-bold bg-[#2962FF]/20 text-[#2962FF]">
                                LẶP
                              </span>
                            )}
                          </div>
                          <div className="flex items-center gap-1 shrink-0">
                            <span
                              className="px-1.5 py-0.5 rounded text-[9px] font-bold uppercase"
                              style={{
                                backgroundColor:
                                  a.status === 'armed'
                                    ? 'var(--tv-up-soft)'
                                    : a.status === 'triggered'
                                      ? 'var(--tv-accent-soft)'
                                      : 'var(--tv-hover)',
                                color:
                                  a.status === 'armed'
                                    ? 'var(--tv-up)'
                                    : a.status === 'triggered'
                                      ? 'var(--tv-accent)'
                                      : 'var(--tv-text-secondary)',
                              }}
                            >
                              {a.status === 'armed' ? 'CHỜ' : a.status === 'triggered' ? 'KHỚP' : 'DỪNG'}
                            </span>
                            <button
                              type="button"
                              className="border-0 bg-transparent p-0.5 cursor-pointer text-[var(--tv-text-secondary)] hover:text-[#2962FF]"
                              onClick={() => {
                                if (isEditing) {
                                  setEditingAlertId(null)
                                } else {
                                  setEditingAlertId(a.id)
                                  setEditAlertPrice(String(a.targetPrice))
                                  setEditAlertMessage(a.message)
                                  setEditAlertFreq(a.frequency === 'once_per_bar_close' ? 'once_per_bar_close' : 'once')
                                  setEditAlertCooldown(String(a.cooldownBars ?? 0))
                                }
                              }}
                              title="Sửa cảnh báo"
                            >
                              <Edit3 className="w-3 h-3" />
                            </button>
                            <button
                              type="button"
                              className="border-0 bg-transparent p-0.5 cursor-pointer text-[var(--tv-text-secondary)] hover:text-[var(--tv-text-primary)]"
                              onClick={() => void toggleAlert(a.id, a.status === 'armed' ? 'paused' : 'armed')}
                              title={a.status === 'armed' ? 'Tạm dừng cảnh báo' : 'Kích hoạt lại cảnh báo'}
                            >
                              {a.status === 'armed' ? <BellOff className="w-3.5 h-3.5" /> : <Bell className="w-3.5 h-3.5" />}
                            </button>
                            <button
                              type="button"
                              className="border-0 bg-transparent p-0.5 cursor-pointer text-[var(--tv-text-secondary)] hover:text-[#F23645]"
                              onClick={() => void removeAlert(a.id)}
                              title="Xóa cảnh báo"
                            >
                              <Trash2 className="w-3.5 h-3.5" />
                            </button>
                          </div>
                        </div>

                        {isEditing ? (
                          <div className="space-y-1.5 pt-1 border-t" style={{ borderColor: 'var(--tv-border)' }}>
                            <div className="grid grid-cols-2 gap-1">
                              <input
                                type="number"
                                step="any"
                                value={editAlertPrice}
                                onChange={(e) => setEditAlertPrice(e.target.value)}
                                className="px-1.5 py-0.5 rounded text-[10.5px] tabular-nums outline-none"
                                style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                              />
                              <select
                                value={editAlertFreq}
                                onChange={(e) => setEditAlertFreq(e.target.value as 'once' | 'once_per_bar_close')}
                                className="px-1 py-0.5 rounded text-[10px] outline-none"
                                style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                              >
                                <option value="once">1 lần</option>
                                <option value="once_per_bar_close">Lặp nến</option>
                              </select>
                            </div>
                            <div className="flex gap-1">
                              <input
                                type="text"
                                value={editAlertMessage}
                                onChange={(e) => setEditAlertMessage(e.target.value)}
                                className="flex-1 px-1.5 py-0.5 rounded text-[10.5px] outline-none"
                                style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                              />
                              <button
                                type="button"
                                className="px-2 py-0.5 rounded text-[10px] font-semibold bg-[#089981] text-white border-0 cursor-pointer"
                                onClick={() => {
                                  void updateExistingAlert({
                                    id: a.id,
                                    targetPrice: Number(editAlertPrice) || a.targetPrice,
                                    message: editAlertMessage,
                                    frequency: editAlertFreq,
                                    cooldownBars: Number(editAlertCooldown) || 0,
                                    rearm: true,
                                  }).then(() => setEditingAlertId(null))
                                }}
                              >
                                Lưu
                              </button>
                            </div>
                          </div>
                        ) : (
                          <>
                            <div className="flex items-center justify-between text-[11px] tabular-nums">
                              <span style={{ color: 'var(--tv-text-secondary)' }}>
                                {a.condition === 'cross_above'
                                  ? 'Cắt lên ▲'
                                  : a.condition === 'cross_below'
                                    ? 'Cắt xuống ▼'
                                    : 'Chạm mức ◇'}
                                {a.interval ? ` · ${a.interval}` : ''}
                              </span>
                              <span className="font-semibold">{a.targetPrice.toLocaleString('en-US')}</span>
                            </div>
                            {a.message && (
                              <div className="text-[10.5px] truncate" style={{ color: 'var(--tv-text-secondary)' }}>
                                {a.message}
                              </div>
                            )}
                          </>
                        )}
                      </div>
                    )
                  })
                )}
              </div>
            </>
          )}
        </div>
      ) : rightTab === 'yumeko' ? (
        <YumekoQmvPanel />
      ) : (
        /* Object Tree Tab Panel (Indicators + Drawings + Timeframe Visibility, v2.2) */
        <div role="tabpanel" aria-labelledby="rw-tab-objects" className="flex flex-col flex-1 min-h-0 overflow-y-auto tv-scroll p-2 space-y-3">
          {/* Active Pane Indicators */}
          <div className="space-y-1">
            <div className="text-[10.5px] font-semibold uppercase tracking-wide px-1" style={{ color: 'var(--tv-text-secondary)' }}>
              Chỉ báo trên {activePane.ticker} ({activePane.indicators.length})
            </div>
            {activePane.indicators.length === 0 ? (
              <div className="text-[11px] px-2 py-2" style={{ color: 'var(--tv-text-secondary)' }}>
                Chưa bật chỉ báo nào trên ô hiện tại.
              </div>
            ) : (
              activePane.indicators.map((ind) => (
                <div
                  key={ind.id}
                  className="flex items-center justify-between gap-1 px-2 py-1.5 rounded"
                  style={{
                    backgroundColor: 'var(--tv-bg-elevated)',
                    border: '1px solid var(--tv-border)',
                    opacity: ind.visible ? 1 : 0.55,
                  }}
                >
                  <div className="min-w-0">
                    <div className="text-[11.5px] font-semibold truncate">
                      {ind.name}
                      {ind.calcParams.length > 0 ? `(${ind.calcParams.join(',')})` : ''}
                    </div>
                    <div className="text-[9.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
                      {ind.isMain ? 'Chồng trên nến' : 'Khung phụ'}
                    </div>
                  </div>
                  <div className="flex items-center gap-0.5 shrink-0">
                    <button
                      type="button"
                      aria-label={ind.visible ? `Ẩn ${ind.name}` : `Hiện ${ind.name}`}
                      className="tv-icon-btn w-6 h-6"
                      onClick={() => updateIndicatorInActivePane(ind.id, { visible: !ind.visible })}
                    >
                      {ind.visible ? <Eye className="w-3.5 h-3.5" /> : <EyeOff className="w-3.5 h-3.5" />}
                    </button>
                    <button
                      type="button"
                      aria-label={`Cài đặt ${ind.name}`}
                      className="tv-icon-btn w-6 h-6"
                      onClick={() => openIndicatorSettings(activePane.slotId, ind.id)}
                    >
                      <Settings className="w-3.5 h-3.5" />
                    </button>
                    <button
                      type="button"
                      aria-label={`Xóa ${ind.name}`}
                      className="tv-icon-btn w-6 h-6 hover:text-[#F23645]"
                      onClick={() => removeIndicatorFromActivePane(ind.id)}
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
              ))
            )}
          </div>

          {/* Saved Drawings + Folder Grouping + Click-to-Jump + Timeframe Visibility (Wave 5 Pro) */}
          <div className="space-y-1">
            <div className="flex items-center justify-between gap-1 px-1">
              <span className="text-[10.5px] font-semibold uppercase tracking-wide" style={{ color: 'var(--tv-text-secondary)' }}>
                Hình vẽ PTKT ({tickerDrawings.length})
              </span>
              <select
                aria-label="Lọc thư mục hình vẽ"
                value={drawingFolderFilter}
                onChange={(e) => setDrawingFolderFilter(e.target.value)}
                className="px-1 py-0.5 rounded text-[9.5px] outline-none cursor-pointer"
                style={{
                  backgroundColor: 'var(--tv-bg-input)',
                  color: 'var(--tv-text-primary)',
                  border: '1px solid var(--tv-border)',
                }}
              >
                <option value="all">Tất cả thư mục</option>
                {DRAWING_FOLDERS.map((f) => (
                  <option key={f} value={f}>
                    {f}
                  </option>
                ))}
              </select>
            </div>
            {tickerDrawings.length === 0 ? (
              <div className="text-[11px] px-2 py-4 text-center" style={{ color: 'var(--tv-text-secondary)' }}>
                Chưa có hình vẽ nào trên {activePane.ticker}.
              </div>
            ) : (
              tickerDrawings
                .filter((rec) => drawingFolderFilter === 'all' || resolveDrawingFolder(rec.extendData) === drawingFolderFilter)
                .map((rec) => {
                  const p0Val = rec.points[0]?.value
                  const p0Ts = rec.points[0]?.timestamp
                  const tfPreset = resolveDrawingTfPreset(rec.extendData)
                  const folder = resolveDrawingFolder(rec.extendData)
                  const isLineTool = [
                    'segment',
                    'rayLine',
                    'straightLine',
                    'horizontalStraightLine',
                    'horizontalRayLine',
                    'horizontalSegment',
                  ].includes(rec.name)
                  return (
                    <div
                      key={rec.id}
                      className="p-2 rounded space-y-1.5"
                      style={{
                        backgroundColor: 'var(--tv-bg-elevated)',
                        border: '1px solid var(--tv-border)',
                        opacity: rec.visible ? 1 : 0.55,
                      }}
                    >
                      <div className="flex items-center justify-between gap-1">
                        <button
                          type="button"
                          onClick={() => {
                            if (typeof p0Ts === 'number' && Number.isFinite(p0Ts)) {
                              scrollActiveChartToTimestamp(p0Ts)
                            }
                          }}
                          className="min-w-0 flex-1 text-left bg-transparent border-0 p-0 cursor-pointer text-[var(--tv-text-primary)] hover:text-[#2962FF]"
                          title={typeof p0Ts === 'number' ? 'Nhấp để cuộn biểu đồ tới vị trí hình vẽ này' : undefined}
                        >
                          <div className="text-[11.5px] font-semibold truncate">
                            {DRAWING_LABELS[rec.name] ?? rec.name}
                          </div>
                          <div className="text-[10px] tabular-nums truncate" style={{ color: 'var(--tv-text-secondary)' }}>
                            {typeof p0Val === 'number' ? `@ ${p0Val.toLocaleString('en-US')}` : rec.id}
                            {folder !== 'Mặc định' ? ` · 📁 ${folder}` : ''}
                          </div>
                        </button>
                        <div className="flex items-center gap-0.5 shrink-0">
                          {isLineTool && (
                            <button
                              type="button"
                              aria-label={`Đặt cảnh báo cắt ${rec.name}`}
                              title="Đặt cảnh báo cắt đường vẽ này"
                              className="tv-icon-btn w-6 h-6 text-[#FBC02D]"
                              onClick={() =>
                                void createNewAlert(
                                  rec.ticker,
                                  'cross_any',
                                  Number((p0Val ?? 0).toFixed(2)),
                                  `Cắt ${DRAWING_LABELS[rec.name] ?? rec.name} (${rec.ticker})`,
                                  {
                                    alertKind: 'drawing',
                                    interval: activePane.interval,
                                    paramsJson: {
                                      drawingId: rec.id,
                                      drawingName: rec.name,
                                      points: rec.points,
                                    },
                                    soundEnabled: true,
                                  },
                                )
                              }
                            >
                              <BellPlus className="w-3.5 h-3.5" />
                            </button>
                          )}
                          <button
                            type="button"
                            aria-label={rec.visible ? 'Ẩn hình vẽ' : 'Hiện hình vẽ'}
                            className="tv-icon-btn w-6 h-6"
                            onClick={() => mutateDrawingObject(rec, { visible: !rec.visible })}
                          >
                            {rec.visible ? <Eye className="w-3.5 h-3.5" /> : <EyeOff className="w-3.5 h-3.5" />}
                          </button>
                          <button
                            type="button"
                            aria-label={rec.lock ? 'Mở khóa hình vẽ' : 'Khóa hình vẽ'}
                            className="tv-icon-btn w-6 h-6"
                            onClick={() => mutateDrawingObject(rec, { lock: !rec.lock })}
                          >
                            {rec.lock ? <Lock className="w-3.5 h-3.5" /> : <Unlock className="w-3.5 h-3.5" />}
                          </button>
                          <button
                            type="button"
                            aria-label="Xóa hình vẽ"
                            className="tv-icon-btn w-6 h-6 hover:text-[#F23645]"
                            onClick={() => deleteDrawingObject(rec)}
                          >
                            <Trash2 className="w-3.5 h-3.5" />
                          </button>
                        </div>
                      </div>

                      {/* Folder & Visibility by Timeframe controls */}
                      <div className="grid grid-cols-2 gap-1 text-[10px]">
                        <select
                          aria-label={`Thư mục của ${DRAWING_LABELS[rec.name] ?? rec.name}`}
                          value={folder}
                          onChange={(e) => {
                            const baseExt =
                              rec.extendData && typeof rec.extendData === 'object'
                                ? (rec.extendData as Record<string, unknown>)
                                : {}
                            mutateDrawingObject(rec, {
                              extendData: { ...baseExt, folder: e.target.value },
                            })
                          }}
                          className="px-1 py-0.5 rounded text-[10px] outline-none cursor-pointer"
                          style={{
                            backgroundColor: 'var(--tv-bg-input)',
                            color: 'var(--tv-text-primary)',
                            border: '1px solid var(--tv-border)',
                          }}
                        >
                          {DRAWING_FOLDERS.map((f) => (
                            <option key={f} value={f}>
                              📁 {f}
                            </option>
                          ))}
                        </select>
                        <select
                          aria-label={`Khung giờ hiển thị của ${DRAWING_LABELS[rec.name] ?? rec.name}`}
                          value={tfPreset}
                          onChange={(e) => {
                            const v = e.target.value
                            const visValue = v.startsWith('only:') ? [v.slice(5)] : v
                            const baseExt =
                              rec.extendData && typeof rec.extendData === 'object'
                                ? (rec.extendData as Record<string, unknown>)
                                : {}
                            mutateDrawingObject(rec, {
                              extendData: { ...baseExt, visibleTimeframes: visValue },
                            })
                          }}
                          className="px-1 py-0.5 rounded text-[10px] outline-none cursor-pointer"
                          style={{
                            backgroundColor: 'var(--tv-bg-input)',
                            color: 'var(--tv-text-primary)',
                            border: '1px solid var(--tv-border)',
                          }}
                        >
                          <option value="all">Mọi khung giờ</option>
                          <option value="intraday">Intraday (≤ 4H)</option>
                          <option value="swing">Swing (≥ 1D)</option>
                          <option value={`only:${activePane.interval}`}>Chỉ khung {activePane.interval}</option>
                        </select>
                      </div>
                    </div>
                  )
                })
            )}
          </div>
        </div>
      )}
    </aside>
  )
}
