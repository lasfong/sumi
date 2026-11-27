import type {
  AnalogConeQueryParams,
  AnalogConeResponseDto,
  AnalogConeStepDto,
  AnalogMatchCaseDto,
  BarsPageResponse,
  BrokerStateSnapshot,
  DataStatusResponse,
  IntervalKey,
  MonteCarloResponse,
  OhlcvBar,
  PriceAlertRecord,
  ReplaySnapshotResponse,
  ResolvedSymbol,
  ResolvedSymbolResponse,
  SavedDrawingRecord,
  SavedIndicatorSpec,
  SavedLayoutRecord,
  SignalCatalogItemDto,
  StockScreenerInput,
  StockScreenerResponse,
  StrategyBacktestInput,
  StrategyBacktestResponse,
  SubmitPaperOrderInput,
  SymbolSearchResult,
  VnBandLevels,
  WalkForwardResponse,
  WatchlistGroup,
  WatchlistQuoteSnapshot,
  WorkspaceBootstrapResponse,
  QmvRealChartSeriesDto,
  TopDownSymbolContextDto,
  YumekoBridgeStatus,
} from '../types/market'
import { intervalToDurationMs } from '../chart/periods'

const CURATED_SYMBOLS: Array<{
  ticker: string
  exchange: string
  symbol: string
  description: string
  marketType: string
  currency: string
  basePrice: number
  precision: number
}> = [
  { ticker: 'HOSE:FPT', exchange: 'HOSE', symbol: 'FPT', description: 'Công ty Cổ phần FPT', marketType: 'stock', currency: 'VND', basePrice: 132500, precision: 0 },
  { ticker: 'HOSE:VCB', exchange: 'HOSE', symbol: 'VCB', description: 'Ngân hàng TMCP Ngoại thương Việt Nam', marketType: 'stock', currency: 'VND', basePrice: 92400, precision: 0 },
  { ticker: 'HOSE:HPG', exchange: 'HOSE', symbol: 'HPG', description: 'Tập đoàn Hòa Phát', marketType: 'stock', currency: 'VND', basePrice: 27450, precision: 0 },
  { ticker: 'HOSE:MWG', exchange: 'HOSE', symbol: 'MWG', description: 'Đầu tư Thế Giới Di Động', marketType: 'stock', currency: 'VND', basePrice: 64200, precision: 0 },
  { ticker: 'HOSE:TCB', exchange: 'HOSE', symbol: 'TCB', description: 'Ngân hàng TMCP Kỹ thương Việt Nam', marketType: 'stock', currency: 'VND', basePrice: 24350, precision: 0 },
  { ticker: 'HOSE:MBB', exchange: 'HOSE', symbol: 'MBB', description: 'Ngân hàng TMCP Quân đội (MBBank)', marketType: 'stock', currency: 'VND', basePrice: 25600, precision: 0 },
  { ticker: 'HOSE:SSI', exchange: 'HOSE', symbol: 'SSI', description: 'Chứng khoán SSI', marketType: 'stock', currency: 'VND', basePrice: 26850, precision: 0 },
  { ticker: 'HOSE:VHM', exchange: 'HOSE', symbol: 'VHM', description: 'Vinhomes JSC', marketType: 'stock', currency: 'VND', basePrice: 41900, precision: 0 },
  { ticker: 'HOSE:VNM', exchange: 'HOSE', symbol: 'VNM', description: 'Sữa Việt Nam (Vinamilk)', marketType: 'stock', currency: 'VND', basePrice: 66800, precision: 0 },
  { ticker: 'HOSE:VIC', exchange: 'HOSE', symbol: 'VIC', description: 'Tập đoàn Vingroup', marketType: 'stock', currency: 'VND', basePrice: 42300, precision: 0 },
  { ticker: 'HOSE:ACB', exchange: 'HOSE', symbol: 'ACB', description: 'Ngân hàng TMCP Á Châu', marketType: 'stock', currency: 'VND', basePrice: 25400, precision: 0 },
  { ticker: 'HOSE:MSN', exchange: 'HOSE', symbol: 'MSN', description: 'Tập đoàn Masan', marketType: 'stock', currency: 'VND', basePrice: 74500, precision: 0 },
  { ticker: 'HOSE:VNINDEX', exchange: 'HOSE', symbol: 'VNINDEX', description: 'Chỉ số VN-Index (HOSE)', marketType: 'index', currency: 'VND', basePrice: 1278.45, precision: 2 },
  { ticker: 'HOSE:VN30', exchange: 'HOSE', symbol: 'VN30', description: 'Chỉ số VN30 (HOSE)', marketType: 'index', currency: 'VND', basePrice: 1345.2, precision: 2 },
  { ticker: 'HNX:VN30F1!', exchange: 'HNX', symbol: 'VN30F1!', description: 'HĐTL VN30 kỳ hạn gần nhất', marketType: 'futures', currency: 'VND', basePrice: 1348.5, precision: 1 },
  { ticker: 'HNX:SHS', exchange: 'HNX', symbol: 'SHS', description: 'Chứng khoán Sài Gòn - Hà Nội', marketType: 'stock', currency: 'VND', basePrice: 15200, precision: 0 },
  { ticker: 'HNX:PVS', exchange: 'HNX', symbol: 'PVS', description: 'Dịch vụ Kỹ thuật Dầu khí Việt Nam', marketType: 'stock', currency: 'VND', basePrice: 38700, precision: 0 },
  { ticker: 'UPCOM:ACV', exchange: 'UPCOM', symbol: 'ACV', description: 'Tổng Công ty Cảng hàng không Việt Nam', marketType: 'stock', currency: 'VND', basePrice: 118500, precision: 0 },
  { ticker: 'BINANCE:BTCUSDT', exchange: 'BINANCE', symbol: 'BTCUSDT', description: 'Bitcoin / TetherUS', marketType: 'crypto', currency: 'USDT', basePrice: 91450.5, precision: 2 },
  { ticker: 'BINANCE:ETHUSDT', exchange: 'BINANCE', symbol: 'ETHUSDT', description: 'Ethereum / TetherUS', marketType: 'crypto', currency: 'USDT', basePrice: 3280.4, precision: 2 },
  { ticker: 'BINANCE:SOLUSDT', exchange: 'BINANCE', symbol: 'SOLUSDT', description: 'Solana / TetherUS', marketType: 'crypto', currency: 'USDT', basePrice: 214.8, precision: 2 },
  { ticker: 'NASDAQ:AAPL', exchange: 'NASDAQ', symbol: 'AAPL', description: 'Apple Inc.', marketType: 'stock', currency: 'USD', basePrice: 229.4, precision: 2 },
  { ticker: 'NASDAQ:NVDA', exchange: 'NASDAQ', symbol: 'NVDA', description: 'NVIDIA Corporation', marketType: 'stock', currency: 'USD', basePrice: 141.8, precision: 2 },
  { ticker: 'AMEX:SPY', exchange: 'AMEX', symbol: 'SPY', description: 'SPDR S&P 500 ETF Trust', marketType: 'fund', currency: 'USD', basePrice: 594.2, precision: 2 },
  { ticker: 'FX_IDC:USDVND', exchange: 'FX_IDC', symbol: 'USDVND', description: 'US Dollar / Vietnamese Dong', marketType: 'forex', currency: 'VND', basePrice: 25420, precision: 0 },
]

function hashStr(s: string): number {
  let h = 2166136261
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i)
    h = Math.imul(h, 16777619)
  }
  return h >>> 0
}

function generateSeries(ticker: string, interval: IntervalKey, count = 420): OhlcvBar[] {
  const meta = CURATED_SYMBOLS.find((s) => s.ticker === ticker) ?? {
    basePrice: 50000,
    precision: 0,
    exchange: 'HOSE',
  }
  const stepMs = intervalToDurationMs(interval)
  const now = Math.floor(Date.now() / stepMs) * stepMs
  let seed = hashStr(`${ticker}@${interval}`)
  const nextRand = () => {
    seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0
    return seed / 0xffffffff
  }
  const bars: OhlcvBar[] = []
  let price = meta.basePrice * 0.78
  const tick = meta.precision === 0 ? (price >= 50000 ? 100 : 50) : Math.pow(10, -meta.precision)
  const snap = (v: number) => Math.round(v / tick) * tick
  for (let i = count - 1; i >= 0; i--) {
    const ts = now - i * stepMs
    const drift = (nextRand() - 0.482) * 0.028
    const open = snap(price)
    const close = snap(Math.max(tick, open * (1 + drift)))
    const wickUp = Math.abs((nextRand() - 0.2) * 0.014) * open
    const wickDn = Math.abs((nextRand() - 0.2) * 0.014) * open
    const high = snap(Math.max(open, close) + wickUp)
    const low = snap(Math.max(tick, Math.min(open, close) - wickDn))
    const volume = Math.round((800_000 + nextRand() * 4_500_000) * (1 + Math.abs(drift) * 20))
    bars.push({ timestamp: ts, open, high, low, close, volume, turnover: close * volume })
    price = close
  }
  return bars
}

function computeBand(exchange: string, marketType: string, ref: number): VnBandLevels | null {
  if (marketType === 'index' || !['HOSE', 'HNX', 'UPCOM'].includes(exchange)) return null
  const pct = exchange === 'HOSE' ? 0.07 : exchange === 'HNX' ? 0.1 : 0.15
  const tick = exchange === 'HOSE' ? (ref >= 50000 ? 100 : ref >= 10000 ? 50 : 10) : 100
  const ceiling = Math.floor((ref * (1 + pct)) / tick) * tick
  const floor = Math.ceil((ref * (1 - pct)) / tick) * tick
  return {
    venue: exchange,
    label: `±${Math.round(pct * 100)}%`,
    reference: ref,
    ceiling,
    floor,
    tickSize: tick,
  }
}

function makeResolvedSymbol(ticker: string): ResolvedSymbol {
  const found = CURATED_SYMBOLS.find((s) => s.ticker === ticker)
  const [ex = 'HOSE', sym = ticker] = ticker.split(':')
  const precision = found?.precision ?? (['HOSE', 'HNX', 'UPCOM'].includes(ex) ? 0 : 2)
  return {
    ticker: found?.ticker ?? `${ex}:${sym}`,
    exchange: found?.exchange ?? ex,
    listed_exchange: found?.exchange ?? ex,
    symbol: found?.symbol ?? sym,
    description: found?.description ?? `${sym} (${ex})`,
    market_type: found?.marketType ?? 'stock',
    currency: found?.currency ?? (['HOSE', 'HNX', 'UPCOM'].includes(ex) ? 'VND' : 'USD'),
    timezone: ['HOSE', 'HNX', 'UPCOM'].includes(ex) ? 'Asia/Ho_Chi_Minh' : 'Etc/UTC',
    session: ['HOSE', 'HNX', 'UPCOM'].includes(ex) ? '0900-1500' : '24x7',
    price_precision: precision,
    volume_precision: found?.marketType === 'crypto' ? 4 : 0,
    pricescale: Math.pow(10, precision),
    minmov: 1,
    minmove2: 0,
    tick_size: precision === 0 ? 100 : Math.pow(10, -precision),
    variable_tick_size: ex === 'HOSE' ? '10 10000 50 50000 100' : '',
    variable_tick_table: null,
    point_value: sym.startsWith('VN30F') ? 100000 : 1,
    delay_secs: 0,
    isin: '',
  }
}

const DEFAULT_WATCHLISTS: WatchlistGroup[] = [
  {
    id: 'wl_qmv30',
    name: 'QMV30 — 30 Cổ Phiếu Trụ Cột',
    tickers: ['HOSE:FPT', 'HOSE:VCB', 'HOSE:HPG', 'HOSE:MWG', 'HOSE:TCB', 'HOSE:MBB', 'HOSE:SSI', 'HOSE:VHM', 'HOSE:VNM', 'HOSE:VIC', 'HOSE:BID', 'HOSE:CTG', 'UPCOM:BSR', 'HNX:IDC'],
    sortOrder: 0,
  },
  {
    id: 'wl_qmv60',
    name: 'QMV60 — Rổ Theo Dõi Trọng Điểm',
    tickers: ['HOSE:ACB', 'HOSE:BID', 'HOSE:CTG', 'HOSE:FPT', 'HOSE:HPG', 'HOSE:MBB', 'HOSE:MWG', 'HOSE:SSI', 'HOSE:STB', 'HOSE:TCB', 'HOSE:VCB', 'HOSE:VCI', 'HOSE:VNM', 'HOSE:VPB'],
    sortOrder: 1,
  },
  {
    id: 'wl_yumeko_core',
    name: 'Danh Mục Tham Chiếu Yumeko Core 10',
    tickers: ['HOSE:BID', 'HOSE:FPT', 'HOSE:MBB', 'HOSE:SAB', 'HOSE:SSI', 'HOSE:TCX', 'HOSE:VCB', 'HOSE:VCI', 'HOSE:VNM', 'HOSE:VTP'],
    sortOrder: 2,
  },
  {
    id: 'wl_vn_indices',
    name: 'Chỉ Số & Rào Chắn Thị Trường VN',
    tickers: ['HOSE:VNINDEX', 'HOSE:VN30', 'HNX:HNXINDEX', 'HOSE:VNXALL', 'HNX:VN30F1!'],
    sortOrder: 3,
  },
]

const lsGet = <T,>(k: string, fallback: T): T => {
  try {
    const raw = localStorage.getItem(k)
    return raw ? (JSON.parse(raw) as T) : fallback
  } catch {
    return fallback
  }
}
const lsSet = (k: string, v: unknown) => {
  try {
    localStorage.setItem(k, JSON.stringify(v))
  } catch {
    // ignore quota errors
  }
}

let webReplay: {
  ticker: string
  interval: IntervalKey
  bars: OhlcvBar[]
  cursor: number
  speed: number
} | null = null

let webBrokerOrders: BrokerStateSnapshot['orders'] = []
let webBrokerCash = 500_000_000
let webBrokerPos: BrokerStateSnapshot['openPosition'] = null
let webBrokerLots: BrokerStateSnapshot['fifoSummary']['lots'] = []
let webBrokerClosed: BrokerStateSnapshot['closedTrades'] = []
let webNextId = 1

export const webFallbackApi = {
  searchSymbols(query: string, category?: string): SymbolSearchResult[] {
    const q = query.trim().toUpperCase()
    const cat = (category ?? 'all').toLowerCase()
    return CURATED_SYMBOLS.filter((item) => {
      if (cat === 'hose' && item.exchange !== 'HOSE') return false
      if (cat === 'hnx' && (item.exchange !== 'HNX' || item.marketType === 'futures')) return false
      if (cat === 'upcom' && item.exchange !== 'UPCOM') return false
      if (cat === 'derivatives' && item.marketType !== 'futures') return false
      if (cat === 'crypto' && item.marketType !== 'crypto') return false
      if (cat === 'global' && (['HOSE', 'HNX', 'UPCOM'].includes(item.exchange) || item.marketType === 'crypto')) return false
      return !q || item.symbol.includes(q) || item.ticker.includes(q) || item.description.toUpperCase().includes(q)
    }).map((item) => ({
      ticker: item.ticker,
      exchange: item.exchange,
      prefix: item.exchange,
      symbol: item.symbol,
      description: item.description,
      market_type: item.marketType,
      currency_code: item.currency,
      country_code: ['HOSE', 'HNX', 'UPCOM'].includes(item.exchange) ? 'VN' : 'US',
      provider_id: 'web_preview',
      type_specs: [],
    }))
  },

  resolveSymbol(ticker: string): ResolvedSymbolResponse {
    const info = makeResolvedSymbol(ticker)
    const bars = generateSeries(ticker, '1D', 60)
    const last = bars[bars.length - 1]
    const prev = bars[bars.length - 2]
    const change = last && prev ? last.close - prev.close : 0
    const changePercent = prev && prev.close ? (change / prev.close) * 100 : 0
    return {
      symbolInfo: info,
      lastPrice: last?.close ?? null,
      prevClose: prev?.close ?? null,
      change,
      changePercent,
      volume: last?.volume ?? null,
      vnBand: prev ? computeBand(info.exchange, info.market_type, prev.close) : null,
    }
  },

  getBars(ticker: string, interval: IntervalKey, mode: string, timestamp?: number | null, limit = 300): BarsPageResponse {
    const info = makeResolvedSymbol(ticker)
    let all = generateSeries(ticker, interval, 480)
    const replayActive = Boolean(webReplay && webReplay.ticker === ticker && webReplay.interval === interval)
    if (replayActive && webReplay) {
      all = webReplay.bars.slice(0, webReplay.cursor + 1)
    }
    let slice: OhlcvBar[]
    if (mode === 'forward' && timestamp != null) {
      const older = all.filter((b) => b.timestamp < timestamp)
      slice = older.slice(-limit)
    } else if (mode === 'backward' && timestamp != null) {
      const newer = all.filter((b) => b.timestamp > timestamp)
      slice = newer.slice(0, limit)
    } else {
      slice = all.slice(-limit)
    }
    const prevClose = all.length >= 2 ? all[all.length - 2].close : all[all.length - 1]?.close ?? 0
    return {
      bars: slice,
      hasOlder: slice.length > 0 && slice[0].timestamp > all[0].timestamp,
      hasNewer: false,
      source: 'cache_hit',
      symbolInfo: info,
      vnBand: prevClose > 0 ? computeBand(info.exchange, info.market_type, prevClose) : null,
      replayActive,
    }
  },

  getWatchlistSnapshots(tickers: string[]): WatchlistQuoteSnapshot[] {
    return tickers.map((ticker) => {
      const res = this.resolveSymbol(ticker)
      return {
        ticker: res.symbolInfo.ticker,
        exchange: res.symbolInfo.exchange,
        symbol: res.symbolInfo.symbol,
        description: res.symbolInfo.description,
        lastPrice: res.lastPrice,
        prevClose: res.prevClose,
        change: res.change,
        changePercent: res.changePercent,
        volume: res.volume,
        pricePrecision: res.symbolInfo.price_precision,
        vnBand: res.vnBand,
        timestampMs: Date.now(),
      }
    })
  },

  getDataStatus(): DataStatusResponse {
    const today = new Date().toISOString().slice(0, 10)
    return {
      stats: {
        db_path: '~/.local/share/sumi/sumi_market.db (preview)',
        total_symbols: 473,
        total_series: 473,
        total_bars: 951420,
      },
      offlineMode: false,
      freshness: {
        expected_session_ms: Date.now(),
        expected_session_date: today,
        total_vn_d1_series: 473,
        fresh_series: 473,
        stale_series: 0,
        median_latest_ms: Date.now(),
        median_latest_date: today,
        newest_latest_ms: Date.now(),
        newest_latest_date: today,
        max_missing_sessions: 0,
      },
      dataDir: '~/.local/share/sumi',
      configDir: '~/.config/sumi',
      yumekoDbDetected: false,
    }
  },

  getCorporateEvents(_ticker: string) {
    return []
  },

  syncGetStatus() {
    return {
      deviceId: 'web-preview',
      syncFolder: null,
      syncBundleExists: false,
      autoSyncOnLaunch: false,
      lastSyncedAtMs: null,
      lastSyncedSha256: null,
      dataDir: '~/.local/share/sumi',
      configDir: '~/.config/sumi',
      preSyncBackupCount: 0,
      doraemiTransport: {
        configured: false,
        sourceFile: null,
        baseUrl: null,
        migration0006RequiredNote:
          'Doraemi OAuth 2.1 + PKCE transport đã sẵn sàng — dùng Local Cloud Folder Sync hoặc .sumi.json cho đến khi kích hoạt Migration 0006.',
        readyForCloudPush: false,
      },
    }
  },

  loadBootstrap(): WorkspaceBootstrapResponse {
    return {
      settings: lsGet<Record<string, unknown> | null>('sumi_ws_settings', null),
      layouts: lsGet<SavedLayoutRecord[]>('sumi_ws_layouts', []),
      watchlists: lsGet<WatchlistGroup[]>('sumi_ws_watchlists', DEFAULT_WATCHLISTS),
      alerts: lsGet<PriceAlertRecord[]>('sumi_ws_alerts', []),
    }
  },

  saveSettings(settings: Record<string, unknown>) {
    lsSet('sumi_ws_settings', settings)
  },

  getDrawings(ticker: string): SavedDrawingRecord[] {
    return lsGet<SavedDrawingRecord[]>(`sumi_drawings_${ticker}`, [])
  },

  upsertDrawing(d: SavedDrawingRecord) {
    const list = this.getDrawings(d.ticker).filter((x) => x.id !== d.id)
    list.push(d)
    lsSet(`sumi_drawings_${d.ticker}`, list)
  },

  deleteDrawing(ticker: string, id: string) {
    const list = this.getDrawings(ticker).filter((x) => x.id !== id)
    lsSet(`sumi_drawings_${ticker}`, list)
  },

  clearDrawings(ticker: string) {
    lsSet(`sumi_drawings_${ticker}`, [])
  },

  getPaneIndicators(slotId: string): SavedIndicatorSpec[] {
    return lsGet<SavedIndicatorSpec[]>(`sumi_inds_${slotId}`, [])
  },

  savePaneIndicators(slotId: string, inds: SavedIndicatorSpec[]) {
    lsSet(`sumi_inds_${slotId}`, inds)
  },

  saveLayout(id: string, name: string, layoutJson: Record<string, unknown>): SavedLayoutRecord {
    const rec: SavedLayoutRecord = { id, name, layoutJson, updatedAtMs: Date.now() }
    const list = lsGet<SavedLayoutRecord[]>('sumi_ws_layouts', []).filter((x) => x.id !== id)
    list.unshift(rec)
    lsSet('sumi_ws_layouts', list)
    return rec
  },

  deleteLayout(id: string) {
    lsSet('sumi_ws_layouts', lsGet<SavedLayoutRecord[]>('sumi_ws_layouts', []).filter((x) => x.id !== id))
  },

  createWatchlist(id: string, name: string): WatchlistGroup {
    const list = lsGet<WatchlistGroup[]>('sumi_ws_watchlists', DEFAULT_WATCHLISTS)
    const group: WatchlistGroup = { id, name, tickers: [], sortOrder: list.length }
    list.push(group)
    lsSet('sumi_ws_watchlists', list)
    return group
  },

  deleteWatchlist(id: string) {
    lsSet('sumi_ws_watchlists', lsGet<WatchlistGroup[]>('sumi_ws_watchlists', DEFAULT_WATCHLISTS).filter((w) => w.id !== id))
  },

  addWatchlistSymbol(watchlistId: string, ticker: string) {
    const list = lsGet<WatchlistGroup[]>('sumi_ws_watchlists', DEFAULT_WATCHLISTS).map((w) =>
      w.id === watchlistId && !w.tickers.includes(ticker) ? { ...w, tickers: [...w.tickers, ticker] } : w,
    )
    lsSet('sumi_ws_watchlists', list)
  },

  removeWatchlistSymbol(watchlistId: string, ticker: string) {
    const list = lsGet<WatchlistGroup[]>('sumi_ws_watchlists', DEFAULT_WATCHLISTS).map((w) =>
      w.id === watchlistId ? { ...w, tickers: w.tickers.filter((t) => t !== ticker) } : w,
    )
    lsSet('sumi_ws_watchlists', list)
  },

  createAlert(ticker: string, condition: string, targetPrice: number, message: string): PriceAlertRecord {
    const rec: PriceAlertRecord = {
      id: `alert_${Date.now()}`,
      ticker,
      condition,
      targetPrice,
      message,
      status: 'armed',
      createdAtMs: Date.now(),
      triggeredAtMs: null,
    }
    const list = lsGet<PriceAlertRecord[]>('sumi_ws_alerts', [])
    list.unshift(rec)
    lsSet('sumi_ws_alerts', list)
    return rec
  },

  toggleAlert(id: string, status: string) {
    const list = lsGet<PriceAlertRecord[]>('sumi_ws_alerts', []).map((a) => (a.id === id ? { ...a, status } : a))
    lsSet('sumi_ws_alerts', list)
  },

  deleteAlert(id: string) {
    lsSet('sumi_ws_alerts', lsGet<PriceAlertRecord[]>('sumi_ws_alerts', []).filter((a) => a.id !== id))
  },

  replayStart(ticker: string, interval: IntervalKey, cutTimestampMs?: number | null): ReplaySnapshotResponse {
    const bars = generateSeries(ticker, interval, 480)
    let cursor = Math.max(20, bars.length - 60)
    if (cutTimestampMs != null) {
      let cut = 1
      for (let i = 0; i < bars.length; i++) {
        if (bars[i].timestamp <= cutTimestampMs) cut = i
        else break
      }
      cursor = Math.max(1, Math.min(bars.length - 1, cut))
    }
    webReplay = { ticker, interval, bars, cursor, speed: 1 }
    return {
      active: true,
      status: 'Paused',
      ticker,
      interval,
      cursorIndex: cursor,
      totalBars: bars.length,
      cutoffTimestampMs: bars[cursor].timestamp,
      speedMultiplier: 1,
      visibleBars: bars.slice(0, cursor + 1),
      steppedBar: null,
      broker: this.brokerState(ticker, interval),
    }
  },

  replayJump(targetTimestampMs: number): ReplaySnapshotResponse {
    if (!webReplay) throw new Error('Replay chưa khởi động')
    let cut = 1
    for (let i = 0; i < webReplay.bars.length; i++) {
      if (webReplay.bars[i].timestamp <= targetTimestampMs) cut = i
      else break
    }
    webReplay.cursor = Math.max(1, Math.min(webReplay.bars.length - 1, cut))
    return {
      active: true,
      status: webReplay.cursor >= webReplay.bars.length - 1 ? 'Completed' : 'Paused',
      ticker: webReplay.ticker,
      interval: webReplay.interval,
      cursorIndex: webReplay.cursor,
      totalBars: webReplay.bars.length,
      cutoffTimestampMs: webReplay.bars[webReplay.cursor].timestamp,
      speedMultiplier: webReplay.speed,
      visibleBars: webReplay.bars.slice(0, webReplay.cursor + 1),
      steppedBar: null,
      broker: this.brokerState(webReplay.ticker, webReplay.interval),
    }
  },

  replayStep(delta: number): ReplaySnapshotResponse {
    if (!webReplay) throw new Error('Replay chưa khởi động')
    const prev = webReplay.cursor
    webReplay.cursor = Math.max(1, Math.min(webReplay.bars.length - 1, webReplay.cursor + delta))
    const stepped = delta === 1 && webReplay.cursor === prev + 1 ? webReplay.bars[webReplay.cursor] : null
    return {
      active: true,
      status: webReplay.cursor >= webReplay.bars.length - 1 ? 'Completed' : 'Paused',
      ticker: webReplay.ticker,
      interval: webReplay.interval,
      cursorIndex: webReplay.cursor,
      totalBars: webReplay.bars.length,
      cutoffTimestampMs: webReplay.bars[webReplay.cursor].timestamp,
      speedMultiplier: webReplay.speed,
      visibleBars: stepped ? [] : webReplay.bars.slice(0, webReplay.cursor + 1),
      steppedBar: stepped,
      broker: this.brokerState(webReplay.ticker, webReplay.interval),
    }
  },

  replayStop() {
    webReplay = null
  },

  brokerState(ticker: string, interval: IntervalKey): BrokerStateSnapshot {
    const res = this.resolveSymbol(ticker)
    const replayBar =
      webReplay && webReplay.ticker === ticker && webReplay.interval === interval
        ? webReplay.bars[webReplay.cursor]
        : null
    const mark = replayBar?.close ?? res.lastPrice ?? 100000
    const unreal = webBrokerPos ? (mark - webBrokerPos.avg_entry_price) * webBrokerPos.quantity : 0
    const equity = webBrokerCash + unreal
    const nowSec = replayBar ? Math.floor(replayBar.timestamp / 1000) : Math.floor(Date.now() / 1000)
    const sellable = webBrokerLots.filter((l) => nowSec >= l.sellable_at_timestamp).reduce((a, b) => a + b.remaining_qty, 0)
    const totalHeld = webBrokerLots.reduce((a, b) => a + b.remaining_qty, 0)
    return {
      ticker,
      interval,
      venue: res.symbolInfo.exchange,
      boardLot: ['HOSE', 'HNX', 'UPCOM'].includes(res.symbolInfo.exchange) ? 100 : 1,
      allowsShort: !['HOSE', 'HNX', 'UPCOM'].includes(res.symbolInfo.exchange),
      enforceVnT2LotLock: ['HOSE', 'HNX', 'UPCOM'].includes(res.symbolInfo.exchange),
      markPrice: mark,
      markTimestampMs: Date.now(),
      vnBand: res.vnBand,
      config: {
        initial_cash: 500_000_000,
        commission_bps: 15,
        tax_sell_bps: 10,
        slippage_bps: 2,
        contract_multiplier: 1,
        initial_margin_pct: 100,
        max_leverage: 1,
        allow_short: false,
        enforce_vn_t2_lot_lock: true,
        venue: res.symbolInfo.exchange,
        base_timeframe_secs: 86400,
      },
      metrics: {
        cash_balance: webBrokerCash,
        unrealized_pnl: unreal,
        equity,
        net_pnl: equity - 500_000_000,
        return_pct: ((equity - 500_000_000) / 500_000_000) * 100,
        max_drawdown_pct: 0,
        pending_order_count: webBrokerOrders.filter((o) => o.order.status === 'Pending').length,
        fill_count: webBrokerOrders.filter((o) => o.order.status === 'Filled').length,
        closed_trade_count: webBrokerClosed.length,
        win_count: webBrokerClosed.filter((t) => t.trade.net_pnl > 0).length,
        loss_count: webBrokerClosed.filter((t) => t.trade.net_pnl < 0).length,
        breakeven_count: 0,
        win_rate_pct: webBrokerClosed.length ? (webBrokerClosed.filter((t) => t.trade.net_pnl > 0).length / webBrokerClosed.length) * 100 : 0,
        avg_return_pct: 0,
        profit_factor: null,
        net_r_multiple: 0,
        avg_r_multiple: null,
        expectancy_pnl: null,
      },
      openPosition: webBrokerPos,
      orders: webBrokerOrders,
      fifoSummary: {
        venue: res.symbolInfo.exchange,
        as_of_timestamp: nowSec,
        lots: webBrokerLots,
        total_held_qty: totalHeld,
        sellable_qty: sellable,
        locked_t2_qty: totalHeld - sellable,
        earliest_unlock_timestamp: webBrokerLots.find((l) => nowSec < l.sellable_at_timestamp)?.sellable_at_timestamp ?? null,
        unsettled_sell_violation_qty: 0,
      },
      closedTrades: webBrokerClosed,
      replayLinked: Boolean(webReplay),
    }
  },

  submitPaperOrder(input: SubmitPaperOrderInput): BrokerStateSnapshot {
    const snap = this.brokerState(input.ticker, input.interval as IntervalKey)
    if (snap.boardLot === 100 && input.quantity % 100 !== 0) {
      throw new Error(`Sàn ${snap.venue}: khối lượng phải là bội số lô chẵn 100 cổ phiếu`)
    }
    const price = input.vnOrderType === 'LO' ? (input.price ?? snap.markPrice) : snap.markPrice
    if (input.vnOrderType === 'LO' && snap.vnBand) {
      if (price > snap.vnBand.ceiling || price < snap.vnBand.floor) {
        throw new Error(`Giá đặt nằm ngoài biên độ trần/sàn [${snap.vnBand.floor} — ${snap.vnBand.ceiling}]`)
      }
    }
    const id = webNextId++
    const isMarket = ['MP', 'ATO', 'ATC'].includes(input.vnOrderType)
    const nowMs = Date.now()
    const nowSec = Math.floor(nowMs / 1000)
    webBrokerOrders = [
      {
        order: {
          id,
          side: input.side === 'BUY' ? 'Buy' : 'Sell',
          kind: isMarket ? 'Market' : { Limit: { limit_price: price } },
          quantity: input.quantity,
          stop_loss: input.stopLoss ?? null,
          take_profit: input.takeProfit ?? null,
          submitted_base_index: 0,
          submitted_timestamp: nowSec,
          status: isMarket ? 'Filled' : 'Pending',
          filled_price: isMarket ? price : null,
          filled_timestamp: isMarket ? nowSec : null,
        },
        vnOrderType: input.vnOrderType,
        submittedTimestampMs: nowMs,
        filledTimestampMs: isMarket ? nowMs : null,
      },
      ...webBrokerOrders,
    ]
    if (isMarket && input.side === 'BUY') {
      // Match sumi-core accounting: cash_balance tracks realized equity ledger (deduct fill fee;
      // equity = cash_balance + unrealized_pnl). Available buying power subtracts open notional in UI.
      webBrokerCash -= price * input.quantity * 0.0015
      const prevQty = webBrokerPos?.quantity ?? 0
      const newQty = prevQty + input.quantity
      const avg = webBrokerPos ? (webBrokerPos.avg_entry_price * prevQty + price * input.quantity) / newQty : price
      webBrokerPos = {
        side: 'Long',
        quantity: newQty,
        avg_entry_price: avg,
        contract_multiplier: 1,
        opened_timestamp: nowSec,
        opened_base_index: 0,
        stop_loss: input.stopLoss ?? null,
        take_profit: input.takeProfit ?? null,
        initial_stop_loss: input.stopLoss ?? null,
        initial_risk_amount: null,
        had_initial_sl_at_open: Boolean(input.stopLoss),
      }
      webBrokerLots.push({
        lot_id: id,
        buy_fill_id: id,
        buy_base_index: 0,
        buy_timestamp: nowSec,
        buy_price: price,
        initial_qty: input.quantity,
        remaining_qty: input.quantity,
        sellable_at_timestamp: nowSec + 2 * 86400,
      })
    }
    return this.brokerState(input.ticker, input.interval as IntervalKey)
  },

  cancelPaperOrder(ticker: string, interval: IntervalKey, orderId: number): BrokerStateSnapshot {
    webBrokerOrders = webBrokerOrders.map((item) =>
      item.order.id === orderId && item.order.status === 'Pending'
        ? { ...item, order: { ...item.order, status: 'Cancelled' } }
        : item,
    )
    return this.brokerState(ticker, interval)
  },

  modifyPaperOrder(
    ticker: string,
    interval: IntervalKey,
    orderId: number,
    newPrice: number,
  ): BrokerStateSnapshot {
    webBrokerOrders = webBrokerOrders.map((item) => {
      if (item.order.id !== orderId || item.order.status !== 'Pending') return item
      const kind =
        typeof item.order.kind === 'object' && 'Stop' in item.order.kind
          ? { Stop: { stop_price: newPrice } }
          : { Limit: { limit_price: newPrice } }
      return { ...item, order: { ...item.order, kind } }
    })
    return this.brokerState(ticker, interval)
  },

  updatePositionBrackets(
    ticker: string,
    interval: IntervalKey,
    stopLoss: number | null,
    takeProfit: number | null,
  ): BrokerStateSnapshot {
    if (webBrokerPos) {
      webBrokerPos = {
        ...webBrokerPos,
        stop_loss: stopLoss,
        take_profit: takeProfit,
      }
    }
    return this.brokerState(ticker, interval)
  },

  flattenPaperPosition(ticker: string, interval: IntervalKey): BrokerStateSnapshot {
    const snap = this.brokerState(ticker, interval)
    if (webBrokerPos) {
      const nowSec = Math.floor(Date.now() / 1000)
      if (snap.enforceVnT2LotLock && snap.fifoSummary.sellable_qty <= 0) {
        throw new Error('Cổ phiếu đang khóa thanh toán VSD T+2 (về tài khoản 13:00 ICT) — chưa khả dụng để bán')
      }
      const mark = snap.markPrice
      const gross = (mark - webBrokerPos.avg_entry_price) * webBrokerPos.quantity
      const fee = mark * webBrokerPos.quantity * 0.0025
      webBrokerCash += gross - fee
      const id = webNextId++
      webBrokerClosed = [
        {
          trade: {
            trade_id: id,
            side: webBrokerPos.side,
            quantity: webBrokerPos.quantity,
            entry_price: webBrokerPos.avg_entry_price,
            exit_price: mark,
            entry_base_index: 0,
            exit_base_index: 1,
            entry_timestamp: webBrokerPos.opened_timestamp,
            exit_timestamp: nowSec,
            exit_reason: 'ManualFlatten',
            initial_stop_loss: webBrokerPos.initial_stop_loss,
            had_initial_sl_at_open: webBrokerPos.had_initial_sl_at_open,
            gross_pnl: gross,
            total_fees: fee,
            net_pnl: gross - fee,
            return_pct: ((mark - webBrokerPos.avg_entry_price) / webBrokerPos.avg_entry_price) * 100,
            r_multiple: null,
          },
          entryTimestampMs: webBrokerPos.opened_timestamp * 1000,
          exitTimestampMs: nowSec * 1000,
          maePct: -0.8,
          mfePct: 1.4,
          setupTag: '',
          mistakeTag: '',
          note: '',
        },
        ...webBrokerClosed,
      ]
      webBrokerPos = null
      webBrokerLots = []
    }
    return this.brokerState(ticker, interval)
  },

  resetPaperAccount(ticker: string, interval: IntervalKey, initialCash = 500_000_000): BrokerStateSnapshot {
    webBrokerOrders = []
    webBrokerCash = initialCash
    webBrokerPos = null
    webBrokerLots = []
    webBrokerClosed = []
    webNextId = 1
    return this.brokerState(ticker, interval)
  },

  runBacktest(input: StrategyBacktestInput): StrategyBacktestResponse {
    const presetLabels: Record<StrategyBacktestInput['preset'], string> = {
      qmv_s1: 'QMV S1 — Hỗ Trợ Cứng + Cạn Cung & Nổ Vol (KT01)',
      qmv_s2: 'QMV S2 — Bắt Đáy An Toàn & Phân Kỳ Đáy (DT1/DT2)',
      qmv_s3: 'QMV S3 — Xu Hướng Mây Ichimoku V7 (6-18-36)',
      qmv_s4: 'QMV S4 — Đồng Pha Chu Kỳ Dòng Tiền T+ & Sóng (Proxy)',
      qmv_s5: 'QMV S5 — Vượt Vùng Tắc Nghẽn & Đỉnh 50 Phiên',
      qmv_s6: 'QMV S6 — Hợp Lưu Đa Trụ Cột QMV (Multi-Gate)',
      sma_ema: 'SMA/EMA Trend Crossover',
      rsi_mean_rev: 'RSI Mean Reversion',
      macd: 'MACD Momentum',
      bb_squeeze: 'Bollinger Squeeze Breakout',
      donchian_atr: 'Donchian ATR Breakout',
      rsi_bb: 'RSI + Bollinger Confluence',
      custom: input.customRules?.name || 'Tự thiết kế QMV Signal Rules',
    }
    let bars = generateSeries(input.ticker, input.interval as IntervalKey, 480)
    if (input.fromTimestampMs != null && input.toTimestampMs != null && input.fromTimestampMs > input.toTimestampMs) {
      throw new Error("Khoảng thời gian Backtest không hợp lệ: 'Từ ngày' phải nhỏ hơn hoặc bằng 'Đến ngày'")
    }
    if (input.fromTimestampMs != null || input.toTimestampMs != null) {
      bars = bars.filter(
        (b) =>
          (input.fromTimestampMs == null || b.timestamp >= input.fromTimestampMs) &&
          (input.toTimestampMs == null || b.timestamp <= input.toTimestampMs),
      )
    }
    if (bars.length < 30) {
      throw new Error(`Không đủ lịch sử nến trong khoảng thời gian đã chọn cho ${input.ticker} (có ${bars.length} nến, cần >= 30)`)
    }
    const paramSeed =
      (input.fastSma ?? 20) +
      (input.slowEma ?? 50) +
      (input.rsiPeriod ?? 14) +
      (input.macdFast ?? 12) +
      (input.bbPeriod ?? 20) +
      (input.donchianEntry ?? 20)
    const step = Math.max(18, Math.floor(bars.length / 7))
    const hold = Math.max(8, Math.min(26, 12 + (paramSeed % 13)))
    const entryIndices: number[] = []
    for (let idx = Math.min(24, Math.floor(bars.length * 0.1)); idx + hold < bars.length; idx += step) {
      entryIndices.push(idx)
      if (entryIndices.length >= 8) break
    }

    const trades = entryIndices.map((idx, k) => {
      const entry = bars[idx]
      const exitIdx = Math.min(bars.length - 1, idx + hold)
      const exit = bars[exitIdx]
      const isShort = input.direction === 'short' || (input.direction === 'both' && k % 2 === 1)
      const ret = isShort
        ? ((entry.close - exit.close) / entry.close) * 100
        : ((exit.close - entry.close) / entry.close) * 100
      const qty = Math.max(100, Math.floor((input.initialCash * 0.85) / Math.max(1, entry.close) / 100) * 100)
      const grossPnl = (isShort ? entry.close - exit.close : exit.close - entry.close) * qty
      return {
        tradeId: k + 1,
        side: isShort ? 'SHORT' : 'LONG',
        quantity: qty,
        entryBarIndex: idx,
        exitBarIndex: exitIdx,
        entryTimestampMs: entry.timestamp,
        exitTimestampMs: exit.timestamp,
        entryPrice: entry.close,
        exitPrice: exit.close,
        barsHeld: exitIdx - idx,
        grossPnl,
        netPnl: grossPnl * 0.996,
        returnPct: ret,
        rMultiple: ret / 3.5,
        maePct: -Math.abs(ret * 0.45 + 1.1),
        mfePct: Math.abs(ret) + 2.2,
        exitReason: ret >= 0 ? 'TP' : 'SL',
      }
    })

    const totalNetPnl = trades.reduce((acc, t) => acc + t.netPnl, 0)
    const totalRetPct = input.initialCash > 0 ? (totalNetPnl / input.initialCash) * 100 : 0
    const equityCurve = bars.map((b, i) => {
      const progress = i / Math.max(1, bars.length - 1)
      const eq = input.initialCash + totalNetPnl * progress + Math.sin(i / 14) * input.initialCash * 0.025
      return {
        barIndex: i,
        timestampMs: b.timestamp,
        equity: eq,
        underwaterPct: Math.min(0, Math.sin(i / 14) * 4.5 - 1.5),
      }
    })
    const winners = trades.filter((t) => t.netPnl > 0)
    const losers = trades.filter((t) => t.netPnl <= 0)
    const sumWin = winners.reduce((a, b) => a + b.netPnl, 0)
    const sumLoss = Math.abs(losers.reduce((a, b) => a + b.netPnl, 0))

    return {
      ticker: input.ticker,
      interval: input.interval,
      presetLabel: presetLabels[input.preset] ?? 'SMA/EMA Trend Crossover',
      barCount: bars.length,
      tearsheet: {
        initial_cash: input.initialCash,
        final_cash: input.initialCash + totalNetPnl,
        final_equity: input.initialCash + totalNetPnl,
        realized_net_profit: totalNetPnl,
        total_return_abs: totalNetPnl,
        total_return_pct: totalRetPct,
        cagr_pct: totalRetPct * 0.85,
        max_drawdown_abs: input.initialCash * 0.062,
        max_drawdown_pct: 6.2,
        max_drawdown_duration_bars: 28,
        sharpe_ratio: totalRetPct >= 0 ? 1.54 : -0.45,
        sortino_ratio: totalRetPct >= 0 ? 2.12 : -0.62,
        calmar_ratio: totalRetPct / 6.2,
        num_trades: trades.length,
        num_winners: winners.length,
        num_losers: losers.length,
        num_breakeven: 0,
        win_rate_pct: trades.length ? (winners.length / trades.length) * 100 : 0,
        profit_factor: sumLoss > 0 ? sumWin / sumLoss : winners.length > 0 ? 3.5 : 0,
        expectancy_net: trades.length ? totalNetPnl / trades.length : 0,
        avg_trade_return_pct: trades.length ? trades.reduce((a, b) => a + b.returnPct, 0) / trades.length : 0,
        avg_win_pct: winners.length ? winners.reduce((a, b) => a + b.returnPct, 0) / winners.length : 0,
        avg_loss_pct: losers.length ? losers.reduce((a, b) => a + b.returnPct, 0) / losers.length : 0,
        avg_bars_held: hold,
        avg_r_multiple: 1.18,
        exposure_pct: 44.5,
        total_fees_and_tax: Math.abs(totalNetPnl) * 0.04 + 850_000,
        low_sample_warning: trades.length < 15,
      },
      inSampleTearsheet: null,
      outOfSampleTearsheet: null,
      robustness: 'Robust',
      equityCurve,
      trades,
    }
  },

  runWfo(input: StrategyBacktestInput): WalkForwardResponse {
    const bt = this.runBacktest(input)
    return {
      ticker: input.ticker,
      interval: input.interval,
      totalBars: bt.barCount,
      profitableFolds: 3,
      foldCount: 4,
      wfePct: 76.4,
      paramStabilityPct: 75.0,
      verdict: 'RobustOutOfSample',
      verdictLabel: 'WFO ROBUST',
      stitchedOosTearsheet: bt.tearsheet,
      stitchedOosCurve: bt.equityCurve,
      folds: [],
    }
  },

  runMonteCarlo(input: StrategyBacktestInput): MonteCarloResponse {
    return {
      simulations: 1000,
      seed: 202615,
      sampleCount: 30,
      horizonSteps: 30,
      validity: 'Valid',
      lowSampleWarning: false,
      statusReason: 'OK',
      metrics: {
        terminal_equity: {
          p5: input.initialCash * 0.94,
          p25: input.initialCash * 1.11,
          p50: input.initialCash * 1.24,
          p75: input.initialCash * 1.38,
          p95: input.initialCash * 1.56,
        },
        terminal_return_pct: { p5: -6, p25: 11, p50: 24, p75: 38, p95: 56 },
        max_drawdown_p50_pct: 7.4,
        max_drawdown_var_95_pct: 15.2,
        max_drawdown_cvar_95_pct: 18.6,
        max_drawdown_var_99_pct: 21.1,
        max_drawdown_cvar_99_pct: 24.0,
        prob_of_profit_pct: 88.4,
        prob_of_loss_pct: 11.6,
        prob_of_ruin_pct: 0.2,
      },
      checkpointEnvelopes: Array.from({ length: 16 }, (_, i) => {
        const f = i / 15
        return {
          p5: input.initialCash * (1 - 0.06 * f),
          p25: input.initialCash * (1 + 0.11 * f),
          p50: input.initialCash * (1 + 0.24 * f),
          p75: input.initialCash * (1 + 0.38 * f),
          p95: input.initialCash * (1 + 0.56 * f),
        }
      }),
    }
  },

  getSignalCatalog(): SignalCatalogItemDto[] {
    return [
      { id: 'health.composite_score', astAlias: 'health_score', labelEn: 'Technical Health Score', labelVi: 'Điểm Sức khỏe Kỹ thuật (4-Family)', description: 'Composite causal health score [-100..+100]', category: 'health', categoryLabel: 'Technical Health', outputType: 'FLOAT', status: 'ACTIVE', warmupBars: 50 },
      { id: 'tech.golden_cross', astAlias: 'golden_cross', labelEn: 'SMA20 / EMA50 Golden Cross', labelVi: 'Giao cắt Vàng SMA20/EMA50', description: 'Bullish moving average crossover', category: 'technical', categoryLabel: 'Technical Crossovers', outputType: 'BOOL', status: 'ACTIVE', warmupBars: 50 },
      { id: 'tech.macd_bull_cross', astAlias: 'macd_bull', labelEn: 'MACD Bullish Cross', labelVi: 'MACD Cắt Lên Đường Tín Hiệu', description: 'MACD line crosses above signal', category: 'technical', categoryLabel: 'Technical Crossovers', outputType: 'BOOL', status: 'ACTIVE', warmupBars: 35 },
      { id: 'tech.rsi_oversold_cross_up', astAlias: 'rsi_os_up', labelEn: 'RSI(14) Cross Above 30', labelVi: 'RSI(14) Vượt Lên Vùng Quá Bán 30', description: 'RSI exits oversold territory', category: 'technical', categoryLabel: 'Technical Crossovers', outputType: 'BOOL', status: 'ACTIVE', warmupBars: 15 },
      { id: 'vol.rvol_spike_2x', astAlias: 'vol_2x', labelEn: 'Volume Spike >= 2.0x', labelVi: 'Đột biến Khối lượng >= 2.0x', description: 'Relative volume surge above 2x 20-bar mean', category: 'volume', categoryLabel: 'Relative Volume', outputType: 'BOOL', status: 'ACTIVE', warmupBars: 20 },
      { id: 'ichi.kumo_breakout', astAlias: 'kumo_brk', labelEn: 'Ichimoku Kumo Breakout', labelVi: 'Phá vỡ Đám mây Ichimoku (Kumo)', description: 'Close breaks above Senkou Span A & B cloud', category: 'ichimoku', categoryLabel: 'Ichimoku Kinko Hyo', outputType: 'BOOL', status: 'ACTIVE', warmupBars: 52 },
      { id: 'pattern.bb_squeeze_breakout', astAlias: 'bb_sqz_brk', labelEn: 'Bollinger Squeeze Breakout', labelVi: 'Bứt phá Nén Bollinger Bands', description: 'Volatility expansion breakout', category: 'pattern', categoryLabel: 'Candlestick & Volatility Patterns', outputType: 'BOOL', status: 'ACTIVE', warmupBars: 20 },
    ]
  },

  runScreener(input: StockScreenerInput): StockScreenerResponse {
    const q = (input.searchQuery ?? '').trim().toUpperCase()
    const ex = input.exchangeFilter.toUpperCase()
    const regimeTarget = (input.regimeFilter ?? '').trim().toLowerCase()
    const rows = CURATED_SYMBOLS.filter((s) => s.marketType === 'stock')
      .filter((s) => {
        if (ex === 'WATCHLIST') return !input.tickers?.length || input.tickers.includes(s.ticker)
        if (ex === 'ALL') return ['HOSE', 'HNX', 'UPCOM'].includes(s.exchange)
        return s.exchange === ex
      })
      .filter((s) => !q || s.symbol.includes(q) || s.description.toUpperCase().includes(q))
      .map((s, idx) => {
        const res = this.resolveSymbol(s.ticker)
        return {
          ticker: s.ticker,
          exchange: s.exchange,
          symbol: s.symbol,
          description: s.description,
          lastClose: res.lastPrice ?? s.basePrice,
          changePct: res.changePercent ?? 0,
          lastVolume: res.volume ?? 1500000,
          rvol20: 1.1 + ((idx * 37) % 18) / 10,
          rsi14: 42 + ((idx * 19) % 34),
          macdHist: ((idx % 5) - 2) * 120,
          atrPct: 1.8 + ((idx * 7) % 20) / 10,
          regime: idx % 3 === 0 ? 'Uptrend' : idx % 3 === 1 ? 'Recovery' : 'Pullback',
          regimeShort: idx % 3 === 0 ? 'UP' : idx % 3 === 1 ? 'REC' : 'PB',
          healthScore: 35 + ((idx * 23) % 55),
          bullishSignals: 3 + (idx % 6),
          bearishSignals: idx % 3,
          matchedSignals: ['Giao cắt Vàng SMA20/EMA50', 'Phá vỡ Đám mây Ichimoku'],
          selectedSignalValue: input.signalId ? 'BULL' : null,
          lastTimestampMs: Date.now(),
        }
      })
      .filter((r) => !regimeTarget || r.regime.toLowerCase() === regimeTarget)
    return {
      totalCatalogSignals: 101,
      scannedSymbols: rows.length,
      matchedSymbols: rows.length,
      elapsedMs: 14,
      rows,
    }
  },

  computeAnalogCone(params: AnalogConeQueryParams): AnalogConeResponseDto {
    const info = makeResolvedSymbol(params.ticker)
    const horizon = Math.max(1, Math.min(20, Math.round(params.horizon ?? 10)))
    const topK = Math.max(10, Math.min(250, Math.round(params.topK ?? 100)))
    const useMkt = params.useMarketContext !== false
    const isVnAsset = ['HOSE', 'HNX', 'UPCOM'].includes(info.exchange)
    const weightProfile = useMkt && isVnAsset ? 'MarketAware' : 'EndogenousOnly'

    if (params.interval !== '1D') {
      return {
        ticker: info.ticker,
        exchange: info.exchange,
        symbol: info.symbol,
        venue: info.exchange,
        interval: String(params.interval),
        supportedInterval: false,
        replayLocked: false,
        asOfTimestampMs: null,
        asOfDate: null,
        anchorClose: null,
        horizon,
        topK,
        weightProfile,
        queryLatencyUs: 0,
        statusMessage: 'AFC hiệu lực trên khung 1D',
        forecast: null,
      }
    }

    const replayActive = Boolean(
      webReplay && webReplay.ticker.toUpperCase() === info.ticker.toUpperCase(),
    )
    let allBars = generateSeries(info.ticker, '1D', 480)
    let cutMs = params.asOfTimestampMs ?? null
    if (replayActive && webReplay) {
      const repCut = webReplay.bars[webReplay.cursor]?.timestamp ?? null
      cutMs = cutMs !== null && repCut !== null ? Math.min(cutMs, repCut) : (repCut ?? cutMs)
    }
    if (cutMs !== null) {
      allBars = allBars.filter((b) => b.timestamp <= cutMs!)
    }

    if (allBars.length <= 65) {
      return {
        ticker: info.ticker,
        exchange: info.exchange,
        symbol: info.symbol,
        venue: info.exchange,
        interval: '1D',
        supportedInterval: true,
        replayLocked: replayActive,
        asOfTimestampMs: allBars[allBars.length - 1]?.timestamp ?? null,
        asOfDate: null,
        anchorClose: allBars[allBars.length - 1]?.close ?? null,
        horizon,
        topK,
        weightProfile,
        queryLatencyUs: 120,
        statusMessage: `Cần tối thiểu 66 phiên lịch sử 1D để mã hóa trạng thái AFC (hiện có ${allBars.length})`,
        forecast: null,
      }
    }

    const snapTick = (p: number): number => {
      if (!isVnAsset || info.market_type === 'index') {
        const sc = Math.pow(10, info.price_precision)
        return Math.round(p * sc) / sc
      }
      const tick = info.exchange === 'HOSE' ? (p < 10_000 ? 10 : p < 50_000 ? 50 : 100) : 100
      return Math.max(tick, Math.round(p / tick) * tick)
    }

    const anchorBar = allBars[allBars.length - 1]
    const anchorClose = snapTick(anchorBar.close)
    const asOfMs = anchorBar.timestamp
    const asOfDate = new Date(asOfMs).toISOString().slice(0, 10)
    const qRet5 = Math.log(anchorBar.close / Math.max(1, allBars[allBars.length - 6].close))
    const qRet20 = Math.log(anchorBar.close / Math.max(1, allBars[allBars.length - 21].close))

    // Gather strictly PIT-settled historical analogs (`settleTs <= asOfMs`) across curated VN equities
    const vnStocks = CURATED_SYMBOLS.filter(
      (s) => s.marketType === 'stock' && ['HOSE', 'HNX', 'UPCOM'].includes(s.exchange),
    )
    const candidates: Array<{
      ticker: string
      symbol: string
      entryTs: number
      dist: number
      bars: OhlcvBar[]
      idx: number
    }> = []

    for (const stock of vnStocks) {
      const sBars = generateSeries(stock.ticker, '1D', 480)
      for (let t = 65; t + horizon < sBars.length; t++) {
        const settleTs = sBars[t + horizon].timestamp
        if (settleTs > asOfMs) break
        if (stock.ticker === info.ticker && t + 20 >= allBars.length) continue
        const r5 = Math.log(sBars[t].close / Math.max(1, sBars[t - 5].close))
        const r20 = Math.log(sBars[t].close / Math.max(1, sBars[t - 20].close))
        const d = Math.hypot((r5 - qRet5) * 28, (r20 - qRet20) * 18)
        candidates.push({
          ticker: stock.ticker,
          symbol: stock.symbol,
          entryTs: sBars[t].timestamp,
          dist: d,
          bars: sBars,
          idx: t,
        })
      }
    }

    candidates.sort((a, b) => a.dist - b.dist)
    const selected = candidates.slice(0, Math.min(topK, candidates.length))
    const medD = selected[Math.floor(selected.length / 2)]?.dist ?? 1
    const scaleD = medD + 1e-4
    const rawW = selected.map((c) => Math.exp((-1.5 * c.dist) / scaleD))
    const sumW = rawW.reduce((a, b) => a + b, 0) || 1
    const weights = rawW.map((w) => w / sumW)

    const bandPct =
      info.market_type === 'index' || !isVnAsset
        ? null
        : info.exchange === 'HOSE'
          ? 0.07
          : info.exchange === 'HNX'
            ? 0.1
            : 0.15

    const pathsClose: number[][] = selected.map(() => new Array<number>(horizon).fill(anchorClose))
    const pathsLow: number[][] = selected.map(() => new Array<number>(horizon).fill(anchorClose))

    for (let j = 0; j < selected.length; j++) {
      const c = selected[j]
      const baseC = c.bars[c.idx].close
      let prevRef = anchorClose
      for (let h = 0; h < horizon; h++) {
        const fut = c.bars[c.idx + 1 + h]
        let simC = anchorClose * (fut.close / baseC)
        let simL = anchorClose * (fut.low / baseC)
        if (bandPct !== null) {
          const tick = info.exchange === 'HOSE' ? (prevRef < 10_000 ? 10 : prevRef < 50_000 ? 50 : 100) : 100
          const ceil = Math.floor((prevRef * (1 + bandPct)) / tick) * tick
          const flr = Math.max(tick, Math.ceil((prevRef * (1 - bandPct)) / tick) * tick)
          simC = Math.min(ceil, Math.max(flr, snapTick(simC)))
          simL = Math.min(simC, Math.max(flr, snapTick(simL)))
        } else {
          simC = snapTick(simC)
          simL = Math.min(simC, snapTick(simL))
        }
        pathsClose[j][h] = simC
        pathsLow[j][h] = simL
        prevRef = simC
      }
    }

    const quantileAt = (vals: number[], q: number): number => {
      const pairs = vals.map((v, idx) => ({ v, w: weights[idx] })).sort((a, b) => a.v - b.v)
      let cum = 0
      for (const p of pairs) {
        cum += p.w
        if (cum >= q) return p.v
      }
      return pairs[pairs.length - 1]?.v ?? anchorClose
    }

    let cumFloorRef = anchorClose
    let cumCeilRef = anchorClose
    const steps: AnalogConeStepDto[] = []

    for (let h = 0; h < horizon; h++) {
      if (bandPct !== null) {
        const tickF = info.exchange === 'HOSE' ? (cumFloorRef < 10_000 ? 10 : cumFloorRef < 50_000 ? 50 : 100) : 100
        const tickC = info.exchange === 'HOSE' ? (cumCeilRef < 10_000 ? 10 : cumCeilRef < 50_000 ? 50 : 100) : 100
        cumFloorRef = Math.max(tickF, Math.ceil((cumFloorRef * (1 - bandPct)) / tickF) * tickF)
        cumCeilRef = Math.max(cumFloorRef, Math.floor((cumCeilRef * (1 + bandPct)) / tickC) * tickC)
      }
      const col = pathsClose.map((p) => p[h])
      const rawQs = [0.1, 0.25, 0.5, 0.75, 0.9].map((q) => snapTick(quantileAt(col, q)))
      for (let qi = 0; qi < 5; qi++) {
        if (bandPct !== null) {
          rawQs[qi] = Math.min(cumCeilRef, Math.max(cumFloorRef, rawQs[qi]))
        }
        if (qi > 0 && rawQs[qi] < rawQs[qi - 1]) rawQs[qi] = rawQs[qi - 1]
      }
      const [p10, p25, p50, p75, p90] = rawQs
      steps.push({
        step: h + 1,
        projectedTimestampMs: asOfMs + (h + 1) * 86_400_000,
        p10,
        p25,
        p50,
        p75,
        p90,
        p10RetPct: ((p10 / anchorClose) - 1) * 100,
        p25RetPct: ((p25 / anchorClose) - 1) * 100,
        p50RetPct: ((p50 / anchorClose) - 1) * 100,
        p75RetPct: ((p75 / anchorClose) - 1) * 100,
        p90RetPct: ((p90 / anchorClose) - 1) * 100,
        cumulativeFloor: bandPct !== null ? cumFloorRef : null,
        cumulativeCeiling: bandPct !== null ? cumCeilRef : null,
      })
    }

    let winW = 0
    let gain3W = 0
    let tail6W = 0
    let meanRet = 0
    for (let j = 0; j < selected.length; j++) {
      const termC = pathsClose[j][horizon - 1]
      const retPct = ((termC / anchorClose) - 1) * 100
      meanRet += weights[j] * retPct
      if (termC > anchorClose) winW += weights[j]
      if (retPct > 3) gain3W += weights[j]
      const minL = Math.min(...pathsLow[j])
      if (((minL / anchorClose) - 1) * 100 <= -6) tail6W += weights[j]
    }

    const lastStep = steps[steps.length - 1]
    const upside = Math.max(0, lastStep.p90 - anchorClose)
    const downside = Math.max(anchorClose * 0.0025, anchorClose - lastStep.p10)

    const topAnalogs: AnalogMatchCaseDto[] = selected.slice(0, 10).map((c, idx) => {
      const c0 = c.bars[c.idx].close
      const b5 = c.bars[c.idx + 5]
      const b10 = c.bars[c.idx + 10]
      return {
        ticker: c.ticker,
        symbol: c.symbol,
        entryTimestampMs: c.entryTs,
        entryDate: new Date(c.entryTs).toISOString().slice(0, 10),
        similarityPct: Math.min(100, Math.max(0, 100 * Math.exp(-c.dist / (2.5 * scaleD)))),
        distance: c.dist,
        weight: weights[idx],
        realizedT5RetPct: b5 && b5.timestamp <= asOfMs ? ((b5.close / c0) - 1) * 100 : null,
        realizedT10RetPct: b10 && b10.timestamp <= asOfMs ? ((b10.close / c0) - 1) * 100 : null,
      }
    })

    return {
      ticker: info.ticker,
      exchange: info.exchange,
      symbol: info.symbol,
      venue: info.exchange,
      interval: '1D',
      supportedInterval: true,
      replayLocked: replayActive,
      asOfTimestampMs: asOfMs,
      asOfDate,
      anchorClose,
      horizon,
      topK: selected.length,
      weightProfile,
      queryLatencyUs: 240,
      statusMessage: null,
      forecast: {
        steps,
        winRatePct: winW * 100,
        probGainGt3Pct: gain3W * 100,
        probTailDrop6Pct: tail6W * 100,
        skewRatio: Math.min(20, upside / downside),
        expectedReturnP50Pct: lastStep.p50RetPct,
        expectedReturnMeanPct: meanRet,
        queryYz20Vol: 0.019,
        pitCandidateCount: candidates.length,
        fastCandidateCount: Math.min(candidates.length, 180),
        topAnalogs,
      },
    }
  },

  yumekoGetBridgeStatus(): YumekoBridgeStatus {
    return {
      available: false,
      sqlitePath: '~/.local/share/yumeko/yumeko-app.sqlite',
      qmvSharedPath: null,
      latestAsOfDates: [],
      qmvTechnicalSymbols: 0,
      qmvActiveMoneySymbols: 0,
      qmvForeignFlowSymbols: 0,
      valuationSymbols: 0,
      dnseHistorySymbols: 0,
      macroObservationsCount: 0,
      portfolioSnapshotsCount: 0,
      watchlistSymbolsCount: 0,
      note: 'Chế độ xem trước web (Clean-Slate) — Sumi đang dùng OHLCV Proxy cục bộ.',
    }
  },

  yumekoGetTopdownContext(ticker: string, asOf: string | null): TopDownSymbolContextDto {
    const sym = ticker.includes(':') ? ticker.split(':')[1] : ticker
    return {
      symbol: sym.toUpperCase(),
      requestedAsOf: asOf,
      effectiveAsOf: asOf,
      bridgeAvailable: false,
      flowProvenance: 'local_proxy',
      vm01Market: {
        effectiveAsOf: asOf,
        marketSentiment: null,
        marketBbm05: null,
        marketBbm20: null,
        marketBbmz05: null,
        marketBbmz20: null,
        marketForeignNetValue: null,
        topSectors: [],
        macroHighlights: [],
      },
      hp01Blackbox: null,
      dn01Valuations: [],
      dm01RealHolding: null,
      dm01YumekoWatchlist: null,
      episodicTheses: [],
      dnsePitAvailable: false,
      gaps: ['Chưa kết nối Yumeko SQLite cục bộ — đang chạy chế độ độc lập (OHLCV Proxy).'],
    }
  },

  yumekoGetQmvSeries(ticker: string, asOf: string | null): QmvRealChartSeriesDto {
    const sym = ticker.includes(':') ? ticker.split(':')[1] : ticker
    return {
      symbol: sym.toUpperCase(),
      asOf,
      sourceProvenance: 'local_proxy',
      isProviderProprietary: false,
      stockBb: [],
      activeMoney: [],
      foreignFlow: [],
    }
  },
}
