import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import {
  Bell,
  BellOff,
  BellPlus,
  Eye,
  EyeOff,
  Layers,
  List,
  Lock,
  Plus,
  RefreshCw,
  Settings,
  Sparkles,
  Trash2,
  Unlock,
  Volume2,
} from 'lucide-react'
import * as api from '../api/tauri'
import { drawingSyncBus } from '../chart/drawingSyncBus'
import { useWorkspaceStore } from '../store/workspaceStore'
import type { SavedDrawingRecord, WatchlistQuoteSnapshot } from '../types/market'
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
  fibonacciLine: 'Fibonacci Thoái lui',
  fibonacciExtension: 'Fibonacci Mở rộng',
  rect: 'Vùng chữ nhật',
  circle: 'Hình tròn',
  triangle: 'Tam giác',
  longPosition: 'Kế hoạch Long R:R',
  shortPosition: 'Kế hoạch Short R:R',
  priceDateRange: 'Đo Giá & Thời gian',
  simpleAnnotation: 'Ghi chú văn bản',
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
  isSelected: boolean
  isBlindMasked: boolean
  onSelect: () => void
  onRemove: () => void
}

/**
 * Zero-DOM-Churn Watchlist Row (`BUG-030` fixed + `modern-web-guidance` `motion` compliant):
 * Keeps a stable `key={ticker}` so React NEVER unmounts/remounts rows on price ticks,
 * animating background highlights via Web Animations API only when `prefers-reduced-motion: no-preference`.
 */
const WatchlistRow: React.FC<WatchlistRowProps> = React.memo(
  ({ ticker, quote, flash, isSelected, isBlindMasked, onSelect, onRemove }) => {
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
        className="group grid grid-cols-[1fr_76px_64px_16px] gap-x-2 items-center px-2 py-1.5 cursor-pointer border-b tabular-nums"
        style={{
          borderColor: 'color-mix(in srgb, var(--tv-border) 45%, transparent)',
          backgroundColor: isSelected ? 'var(--tv-accent-soft)' : undefined,
        }}
        onClick={onSelect}
      >
        <div className="min-w-0 pr-1">
          <div className="flex items-center gap-1">
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
  const createNewAlert = useWorkspaceStore((s) => s.createNewAlert)
  const toggleAlert = useWorkspaceStore((s) => s.toggleAlert)
  const removeAlert = useWorkspaceStore((s) => s.removeAlert)
  const updateIndicatorInActivePane = useWorkspaceStore((s) => s.updateIndicatorInActivePane)
  const removeIndicatorFromActivePane = useWorkspaceStore((s) => s.removeIndicatorFromActivePane)
  const openIndicatorSettings = useWorkspaceStore((s) => s.openIndicatorSettings)
  const drawingsVersion = useWorkspaceStore((s) => s.drawingsVersion)

  const activePane = panes.find((p) => p.slotId === activeSlotId) ?? panes[0]
  const activeWatchlist = watchlists.find((w) => w.id === activeWatchlistId) ?? watchlists[0]

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

  // Alert creation form state (Price + Indicator RSI/MACD/EMA, v2.2)
  const [alertMode, setAlertMode] = useState<'price' | 'indicator'>('price')
  const [alertCondition, setAlertCondition] = useState<'cross_above' | 'cross_below' | 'cross_any'>('cross_above')
  const [alertPrice, setAlertPrice] = useState('')
  const [alertIndType, setAlertIndType] = useState<'RSI' | 'MACD' | 'EMA'>('RSI')
  const [alertIndPeriod, setAlertIndPeriod] = useState('14')
  const [alertIndThreshold, setAlertIndThreshold] = useState('70')
  const [alertSoundEnabled, setAlertSoundEnabled] = useState(true)
  const [alertMessage, setAlertMessage] = useState('')

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
      { alertKind: 'price', interval: activePane.interval, soundEnabled: alertSoundEnabled },
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
          {/* Watchlist Category Selector + Add Symbol */}
          <div className="p-2 space-y-1.5 shrink-0" style={{ borderBottom: '1px solid var(--tv-border)' }}>
            <div className="flex items-center gap-1">
              <select
                aria-label="Chọn danh mục theo dõi"
                value={activeWatchlist?.id ?? ''}
                onChange={(e) => setActiveWatchlistId(e.target.value)}
                className="flex-1 px-2 py-1 rounded text-[11.5px] font-medium outline-none cursor-pointer"
                style={{
                  backgroundColor: 'var(--tv-bg-input)',
                  color: 'var(--tv-text-primary)',
                  border: '1px solid var(--tv-border)',
                }}
              >
                {watchlists.map((w) => (
                  <option key={w.id} value={w.id}>
                    {w.name} ({w.tickers.length})
                  </option>
                ))}
              </select>
              <button
                type="button"
                className="tv-icon-btn w-7 h-7"
                onClick={() => setShowNewListForm((v) => !v)}
                title="Tạo danh mục mới"
              >
                <Plus className="w-3.5 h-3.5" />
              </button>
            </div>

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
                placeholder="Thêm mã (VD: FPT, BINANCE:BTCUSDT)"
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
              >
                +
              </button>
            </form>
          </div>

          {/* Watchlist Table Header */}
          <div
            className="grid grid-cols-[1fr_76px_64px_16px] gap-x-2 items-center px-2 py-1 text-[10.5px] font-medium shrink-0"
            style={{ color: 'var(--tv-text-secondary)', borderBottom: '1px solid var(--tv-border)' }}
          >
            <span>Mã</span>
            <span className="text-right whitespace-nowrap">Giá khớp</span>
            <span className="text-right whitespace-nowrap">% Đổi</span>
            <span />
          </div>

          {/* Watchlist Rows with Zero-Remount WAAPI Flash (`BUG-030`) & Blind Mode (`BUG-041`) */}
          <div className="flex-1 overflow-y-auto tv-scroll">
            {(activeWatchlist?.tickers ?? []).map((ticker) => {
              const isBlindRow = Boolean(replayMaskTicker && ticker.toUpperCase() === replayMaskTicker)
              return (
                <WatchlistRow
                  key={ticker}
                  ticker={ticker}
                  quote={buildEffectiveQuote(ticker)}
                  flash={quoteFlashMap[ticker]}
                  isSelected={activePane.ticker === ticker}
                  isBlindMasked={isBlindRow}
                  onSelect={() => setActiveTicker(ticker)}
                  onRemove={() => void removeSymbolFromWatchlist(ticker)}
                />
              )
            })}
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
          </div>
        </div>
      ) : rightTab === 'alerts' ? (
        /* Multi-Kind Alerts Tab Panel (Price / Drawing-Line / Indicator + Sound Chime, v2.2) */
        <div role="tabpanel" aria-labelledby="rw-tab-alerts" className="flex flex-col flex-1 min-h-0">
          <form
            onSubmit={handleCreateAlert}
            className="p-2.5 space-y-2 shrink-0"
            style={{ borderBottom: '1px solid var(--tv-border)' }}
          >
            <div className="flex items-center justify-between gap-1">
              <span className="text-[11px] font-semibold truncate" style={{ color: 'var(--tv-text-secondary)' }}>
                CẢNH BÁO · {activePane.ticker}
              </span>
              <div className="flex items-center gap-0.5 shrink-0">
                <button
                  type="button"
                  className="px-1.5 py-0.5 rounded text-[10px] font-semibold border-0 cursor-pointer"
                  style={{
                    backgroundColor: alertMode === 'price' ? 'var(--tv-accent)' : 'var(--tv-hover)',
                    color: alertMode === 'price' ? '#FFFFFF' : 'var(--tv-text-secondary)',
                  }}
                  onClick={() => setAlertMode('price')}
                >
                  Giá
                </button>
                <button
                  type="button"
                  className="px-1.5 py-0.5 rounded text-[10px] font-semibold border-0 cursor-pointer"
                  style={{
                    backgroundColor: alertMode === 'indicator' ? 'var(--tv-accent)' : 'var(--tv-hover)',
                    color: alertMode === 'indicator' ? '#FFFFFF' : 'var(--tv-text-secondary)',
                  }}
                  onClick={() => setAlertMode('indicator')}
                >
                  Chỉ báo
                </button>
              </div>
            </div>

            {alertMode === 'price' ? (
              <div className="grid grid-cols-2 gap-1.5">
                <select
                  aria-label="Điều kiện cảnh báo"
                  value={alertCondition}
                  onChange={(e) => setAlertCondition(e.target.value as 'cross_above' | 'cross_below' | 'cross_any')}
                  className="px-2 py-1 rounded text-[11.5px] outline-none"
                  style={{
                    backgroundColor: 'var(--tv-bg-input)',
                    color: 'var(--tv-text-primary)',
                    border: '1px solid var(--tv-border)',
                  }}
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
                  className="px-2 py-1 rounded text-[11.5px] tabular-nums outline-none"
                  style={{
                    backgroundColor: 'var(--tv-bg-input)',
                    color: 'var(--tv-text-primary)',
                    border: '1px solid var(--tv-border)',
                  }}
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
                    className="px-2 py-1 rounded text-[11.5px] outline-none"
                    style={{
                      backgroundColor: 'var(--tv-bg-input)',
                      color: 'var(--tv-text-primary)',
                      border: '1px solid var(--tv-border)',
                    }}
                  >
                    <option value="RSI">RSI (Wilder)</option>
                    <option value="MACD">MACD Hist (0)</option>
                    <option value="EMA">Giá cắt EMA</option>
                  </select>
                  <select
                    aria-label="Hướng cắt chỉ báo"
                    value={alertCondition}
                    onChange={(e) => setAlertCondition(e.target.value as 'cross_above' | 'cross_below' | 'cross_any')}
                    className="px-2 py-1 rounded text-[11.5px] outline-none"
                    style={{
                      backgroundColor: 'var(--tv-bg-input)',
                      color: 'var(--tv-text-primary)',
                      border: '1px solid var(--tv-border)',
                    }}
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
                      className="px-2 py-1 rounded text-[11.5px] tabular-nums outline-none"
                      style={{
                        backgroundColor: 'var(--tv-bg-input)',
                        color: 'var(--tv-text-primary)',
                        border: '1px solid var(--tv-border)',
                      }}
                    />
                    {alertIndType === 'RSI' && (
                      <input
                        type="number"
                        aria-label="Ngưỡng RSI"
                        value={alertIndThreshold}
                        onChange={(e) => setAlertIndThreshold(e.target.value)}
                        placeholder="Ngưỡng (70 / 30)"
                        className="px-2 py-1 rounded text-[11.5px] tabular-nums outline-none"
                        style={{
                          backgroundColor: 'var(--tv-bg-input)',
                          color: 'var(--tv-text-primary)',
                          border: '1px solid var(--tv-border)',
                        }}
                      />
                    )}
                  </div>
                )}
              </div>
            )}

            <input
              type="text"
              value={alertMessage}
              onChange={(e) => setAlertMessage(e.target.value)}
              placeholder="Ghi chú cảnh báo (tùy chọn)..."
              className="w-full px-2 py-1 rounded text-[11.5px] outline-none"
              style={{
                backgroundColor: 'var(--tv-bg-input)',
                color: 'var(--tv-text-primary)',
                border: '1px solid var(--tv-border)',
              }}
            />

            <div className="flex items-center justify-between text-[11px]">
              <label className="flex items-center gap-1.5 cursor-pointer" style={{ color: 'var(--tv-text-secondary)' }}>
                <input
                  type="checkbox"
                  checked={alertSoundEnabled}
                  onChange={(e) => setAlertSoundEnabled(e.target.checked)}
                />
                <span>Phát âm báo (Chime)</span>
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
              className="w-full py-1.5 rounded text-[12px] font-semibold bg-[#2962FF] hover:bg-[#1E53E5] text-white border-0 cursor-pointer"
            >
              {alertMode === 'indicator' ? `+ Thêm Cảnh Báo ${alertIndType}` : '+ Thêm Cảnh Báo Giá'}
            </button>
          </form>

          <div className="flex-1 overflow-y-auto tv-scroll p-2 space-y-1.5">
            {alerts.length === 0 ? (
              <div className="text-center py-8 text-[11.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
                Chưa có cảnh báo nào. Tạo cảnh báo Giá, Chỉ báo ở trên hoặc bấm 🔔 trên đường xu hướng.
              </div>
            ) : (
              alerts.map((a) => {
                const kindTag =
                  a.alertKind === 'drawing' ? 'ĐƯỜNG VẼ' : a.alertKind === 'indicator' ? 'CHỈ BÁO' : 'GIÁ'
                return (
                  <div
                    key={a.id}
                    className="p-2 rounded space-y-1"
                    style={{
                      backgroundColor: 'var(--tv-bg-elevated)',
                      border: '1px solid var(--tv-border)',
                    }}
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
                      </div>
                      <div className="flex items-center gap-1 shrink-0">
                        <span
                          className="px-1.5 py-0.5 rounded text-[9.5px] font-bold uppercase"
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
                          {a.status === 'armed' ? 'ĐANG CHỜ' : a.status === 'triggered' ? 'ĐÃ KHỚP' : 'TẠM DỪNG'}
                        </span>
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
                  </div>
                )
              })
            )}
          </div>
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

          {/* Saved Drawings + Timeframe Visibility */}
          <div className="space-y-1">
            <div className="flex items-center justify-between px-1">
              <span className="text-[10.5px] font-semibold uppercase tracking-wide" style={{ color: 'var(--tv-text-secondary)' }}>
                Hình vẽ PTKT ({tickerDrawings.length})
              </span>
              <span className="text-[9.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
                Khung: {activePane.interval}
              </span>
            </div>
            {tickerDrawings.length === 0 ? (
              <div className="text-[11px] px-2 py-4 text-center" style={{ color: 'var(--tv-text-secondary)' }}>
                Chưa có hình vẽ nào trên {activePane.ticker}.
              </div>
            ) : (
              tickerDrawings.map((rec) => {
                const p0Val = rec.points[0]?.value
                const tfPreset = resolveDrawingTfPreset(rec.extendData)
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
                      <div className="min-w-0">
                        <div className="text-[11.5px] font-semibold truncate">
                          {DRAWING_LABELS[rec.name] ?? rec.name}
                        </div>
                        <div className="text-[10px] tabular-nums" style={{ color: 'var(--tv-text-secondary)' }}>
                          {typeof p0Val === 'number' ? `@ ${p0Val.toLocaleString('en-US')}` : rec.id}
                        </div>
                      </div>
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

                    {/* Visibility by Timeframe selector (`Tất cả` / `Intraday <= 4H` / `Swing >= 1D` / `Chỉ khung này`) */}
                    <div className="flex items-center justify-between gap-1.5 text-[10px]">
                      <span style={{ color: 'var(--tv-text-secondary)' }}>Hiển thị khung:</span>
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
                        className="px-1.5 py-0.5 rounded text-[10px] outline-none cursor-pointer"
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
