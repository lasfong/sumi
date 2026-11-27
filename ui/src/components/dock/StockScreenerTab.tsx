import React, { useEffect, useMemo, useState } from 'react'
import {
  ArrowDown,
  ArrowUp,
  Bookmark,
  Download,
  FilterX,
  ListPlus,
  Plus,
  Radar,
  RefreshCw,
  SlidersHorizontal,
  Sparkles,
  Trash2,
  X,
} from 'lucide-react'
import * as api from '../../api/tauri'
import { useWorkspaceStore } from '../../store/workspaceStore'
import type {
  ScreenerExchangeFilter,
  ScreenerRowDto,
  SignalCatalogItemDto,
  StockScreenerResponse,
} from '../../types/market'

type SortKey =
  | 'symbol'
  | 'lastClose'
  | 'changePct'
  | 'lastVolume'
  | 'rvol20'
  | 'qmvFlowZ50T05'
  | 'rsi14'
  | 'pe'
  | 'roe'
  | 'qmvRealTmc'
  | 'healthScore'

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' })

const CATEGORY_TABS = [
  { key: 'all', label: 'Tất cả tín hiệu QMV' },
  { key: 'flow', label: 'Chu kỳ Dòng tiền (Proxy)' },
  { key: 'ichimoku', label: 'Ichimoku V7 (6-18-36)' },
  { key: 'vsa', label: 'VSA & Cạn cung' },
  { key: 'sr', label: 'Hỗ trợ / Tắc nghẽn / Đỉnh 50p' },
  { key: 'structure', label: 'Cấu trúc Đỉnh / Đáy' },
  { key: 'pattern', label: 'Mô hình nến (12 mẫu)' },
  { key: 'divergence', label: 'Phân kỳ RSI / MACD / CCI' },
  { key: 'regime', label: 'Xu hướng & Chế độ' },
  { key: 'technical', label: 'Giao cắt MA / Bollinger' },
  { key: 'volume', label: 'Đột biến Khối lượng' },
  { key: 'health', label: 'Điểm Sức khỏe' },
]

const QMV_QUICK_PRESETS: Array<{ label: string; title: string; signals: string[] }> = [
  {
    label: 'Hỗ trợ cạn cung (KT01)',
    title: 'Giá tích lũy sát hỗ trợ cứng >=2 phiên kèm tiết cung Vol < 0.75x MA20',
    signals: ['qmv.vsa_support_dry_up'],
  },
  {
    label: 'Vượt Mây V7 (6-18-36)',
    title: 'Giá cắt lên trên đám mây Kumo chuẩn QMV V7 (6, 18, 36, dịch 18)',
    signals: ['qmv.ichimoku_v7_kumo_break_up'],
  },
  {
    label: 'Đảo chiều T+ (Proxy)',
    title: 'Chu kỳ T+ (Z-50 OHLCV Proxy) cắt lên từ vùng Duy trì <= 35 kèm nến hồi phục',
    signals: ['qmv.reversal_up_t_plus'],
  },
  {
    label: 'Phân kỳ Đáy CCI(14)',
    title: 'Phân kỳ dương thường trên CCI(14) tại vùng đáy (trễ nhân quả 3 nến xác nhận)',
    signals: ['divergence.cci_regular_bullish'],
  },
  {
    label: 'Vượt Đỉnh 50p + Vol',
    title: 'Thiết lập đỉnh cao mới 50 phiên kèm khối lượng >= 1.5x MA20',
    signals: ['qmv.new_high_50', 'volume.rvol_spike_1_5x'],
  },
]

export const StockScreenerTab: React.FC = () => {
  const watchlists = useWorkspaceStore((s) => s.watchlists)
  const activeWatchlistId = useWorkspaceStore((s) => s.activeWatchlistId)
  const dataStatus = useWorkspaceStore((s) => s.dataStatus)
  const syncingEod = useWorkspaceStore((s) => s.syncingEod)
  const eodSyncProgress = useWorkspaceStore((s) => s.eodSyncProgress)
  const syncUniverseEod = useWorkspaceStore((s) => s.syncUniverseEod)
  const setActiveTicker = useWorkspaceStore((s) => s.setActiveTicker)
  const pushToast = useWorkspaceStore((s) => s.pushToast)
  const savedScreens = useWorkspaceStore((s) => s.savedScreens)
  const refreshSavedScreens = useWorkspaceStore((s) => s.refreshSavedScreens)
  const saveScreenerPreset = useWorkspaceStore((s) => s.saveScreenerPreset)
  const deleteScreenerPreset = useWorkspaceStore((s) => s.deleteScreenerPreset)
  const createWatchlistWithSymbols = useWorkspaceStore((s) => s.createWatchlistWithSymbols)

  const [catalog, setCatalog] = useState<SignalCatalogItemDto[]>([])
  const [showAllCatalog, setShowAllCatalog] = useState(false)
  const [showAdvancedFilters, setShowAdvancedFilters] = useState(false)

  // QMV Universe & Taxonomy filters
  const [exchangeFilter, setExchangeFilter] = useState<ScreenerExchangeFilter>('ALL_VN')
  const [sectorGroupFilter, setSectorGroupFilter] = useState<string>('')
  const [sectorFilter, setSectorFilter] = useState<string>('')
  const [priceTierFilter, setPriceTierFilter] = useState<string>('')
  const [defaultLiq50k, setDefaultLiq50k] = useState<boolean>(true)

  // Multi-Signal `AND` Builder
  const [categoryFilter, setCategoryFilter] = useState<string>('all')
  const [selectedSignals, setSelectedSignals] = useState<string[]>([])
  const [pendingSignalId, setPendingSignalId] = useState<string>('')
  const [regimeFilter, setRegimeFilter] = useState<string>('')
  const [searchQuery, setSearchQuery] = useState<string>('')

  // Numeric filters
  const [minRsi, setMinRsi] = useState<string>('')
  const [maxRsi, setMaxRsi] = useState<string>('')
  const [minRvol, setMinRvol] = useState<string>('')
  const [minChangePct, setMinChangePct] = useState<string>('')
  const [macdFilter, setMacdFilter] = useState<'all' | 'positive' | 'negative'>('all')
  const [minHealthScore, setMinHealthScore] = useState<string>('')

  // Wave 4 MTF + Fundamentals + Yumeko Blackbox + Saved Screens + Auto-Refresh
  const [weeklyRegimeFilter, setWeeklyRegimeFilter] = useState<string>('')
  const [enableMtf1h, setEnableMtf1h] = useState<boolean>(false)
  const [maxPe, setMaxPe] = useState<string>('')
  const [maxPb, setMaxPb] = useState<string>('')
  const [minRoe, setMinRoe] = useState<string>('')
  const [onlyMoneyInToday, setOnlyMoneyInToday] = useState<boolean>(false)
  const [minQmvTmc, setMinQmvTmc] = useState<string>('')
  const [autoRefreshSec, setAutoRefreshSec] = useState<0 | 30 | 60>(0)
  const [saveScreenName, setSaveScreenName] = useState<string>('')
  const [selectedSavedId, setSelectedSavedId] = useState<string>('')

  const [scanning, setScanning] = useState(false)
  const [result, setResult] = useState<StockScreenerResponse | null>(null)
  const [sortCol, setSortCol] = useState<SortKey>('healthScore')
  const [sortDir, setSortDir] = useState<'ascending' | 'descending'>('descending')
  const [liveStatus, setLiveStatus] = useState('')

  useEffect(() => {
    void api.getSignalCatalog().then(setCatalog)
    void refreshSavedScreens()
  }, [refreshSavedScreens])

  const visibleCatalog = useMemo(() => {
    const base = showAllCatalog ? catalog : catalog.filter((c) => c.isQmvCore !== false)
    if (categoryFilter === 'all') return base
    return base.filter((c) => c.category === categoryFilter)
  }, [catalog, showAllCatalog, categoryFilter])

  const signalLabelMap = useMemo(() => {
    const m = new Map<string, SignalCatalogItemDto>()
    for (const c of catalog) m.set(c.id, c)
    return m
  }, [catalog])

  const parseOptNum = (v: string): number | null => {
    const t = v.trim()
    if (!t) return null
    const n = Number(t)
    return Number.isFinite(n) ? n : null
  }

  const hasAnyActiveFilter = Boolean(
    defaultLiq50k ||
      selectedSignals.length > 0 ||
      sectorGroupFilter ||
      sectorFilter ||
      priceTierFilter ||
      regimeFilter ||
      minRsi.trim() ||
      maxRsi.trim() ||
      minRvol.trim() ||
      minChangePct.trim() ||
      macdFilter !== 'all' ||
      minHealthScore.trim() ||
      weeklyRegimeFilter ||
      enableMtf1h ||
      maxPe.trim() ||
      maxPb.trim() ||
      minRoe.trim() ||
      onlyMoneyInToday ||
      minQmvTmc.trim(),
  )

  /** Quy tắc QMV `B5`: Gỡ toàn bộ bộ lọc mặc định (gồm cả ngưỡng KLGD 50k) để quan sát toàn thị trường. */
  const handleClearAllDefaultFilters = () => {
    setDefaultLiq50k(false)
    setSelectedSignals([])
    setPendingSignalId('')
    setSectorGroupFilter('')
    setSectorFilter('')
    setPriceTierFilter('')
    setRegimeFilter('')
    setMinRsi('')
    setMaxRsi('')
    setMinRvol('')
    setMinChangePct('')
    setMacdFilter('all')
    setMinHealthScore('')
    setWeeklyRegimeFilter('')
    setEnableMtf1h(false)
    setMaxPe('')
    setMaxPb('')
    setMinRoe('')
    setOnlyMoneyInToday(false)
    setMinQmvTmc('')
  }

  const snapshotCurrentFilterJson = (): Record<string, unknown> => ({
    exchangeFilter,
    sectorGroupFilter,
    sectorFilter,
    priceTierFilter,
    defaultLiq50k,
    categoryFilter,
    selectedSignals,
    regimeFilter,
    minRsi,
    maxRsi,
    minRvol,
    minChangePct,
    macdFilter,
    minHealthScore,
    weeklyRegimeFilter,
    enableMtf1h,
    maxPe,
    maxPb,
    minRoe,
    onlyMoneyInToday,
    minQmvTmc,
  })

  const applySavedScreen = (filterJson: Record<string, unknown>) => {
    if (typeof filterJson.exchangeFilter === 'string') {
      setExchangeFilter(filterJson.exchangeFilter as ScreenerExchangeFilter)
    }
    setSectorGroupFilter(String(filterJson.sectorGroupFilter ?? ''))
    setSectorFilter(String(filterJson.sectorFilter ?? ''))
    setPriceTierFilter(String(filterJson.priceTierFilter ?? ''))
    setDefaultLiq50k(Boolean(filterJson.defaultLiq50k ?? true))
    setCategoryFilter(String(filterJson.categoryFilter ?? 'all'))
    setSelectedSignals(Array.isArray(filterJson.selectedSignals) ? (filterJson.selectedSignals as string[]) : [])
    setRegimeFilter(String(filterJson.regimeFilter ?? ''))
    setMinRsi(String(filterJson.minRsi ?? ''))
    setMaxRsi(String(filterJson.maxRsi ?? ''))
    setMinRvol(String(filterJson.minRvol ?? ''))
    setMinChangePct(String(filterJson.minChangePct ?? ''))
    setMacdFilter((filterJson.macdFilter as 'all' | 'positive' | 'negative') ?? 'all')
    setMinHealthScore(String(filterJson.minHealthScore ?? ''))
    setWeeklyRegimeFilter(String(filterJson.weeklyRegimeFilter ?? ''))
    setEnableMtf1h(Boolean(filterJson.enableMtf1h ?? false))
    setMaxPe(String(filterJson.maxPe ?? ''))
    setMaxPb(String(filterJson.maxPb ?? ''))
    setMinRoe(String(filterJson.minRoe ?? ''))
    setOnlyMoneyInToday(Boolean(filterJson.onlyMoneyInToday ?? false))
    setMinQmvTmc(String(filterJson.minQmvTmc ?? ''))
  }

  const addSignalChip = (id: string) => {
    if (!id) return
    setSelectedSignals((prev) => (prev.includes(id) ? prev : [...prev, id]))
    setPendingSignalId('')
  }

  const removeSignalChip = (id: string) => {
    setSelectedSignals((prev) => prev.filter((s) => s !== id))
  }

  const runScan = async () => {
    setScanning(true)
    try {
      const wl = watchlists.find((w) => w.id === activeWatchlistId)
      const res = await api.runStockScreener({
        exchangeFilter,
        tickers: exchangeFilter === 'WATCHLIST' ? (wl?.tickers ?? []) : undefined,
        signalIds: selectedSignals,
        signalId: selectedSignals.length === 1 ? selectedSignals[0] : undefined,
        sectorGroupFilter: sectorGroupFilter || null,
        sectorFilter: sectorFilter || null,
        priceTierFilter: priceTierFilter || null,
        minAvgVol20: defaultLiq50k ? 50_000 : null,
        categoryFilter: categoryFilter !== 'all' && selectedSignals.length === 0 ? categoryFilter : undefined,
        regimeFilter: regimeFilter || undefined,
        searchQuery: searchQuery.trim() || undefined,
        maxSymbols: 500,
        minRsi: parseOptNum(minRsi),
        maxRsi: parseOptNum(maxRsi),
        minRvol: parseOptNum(minRvol),
        minChangePct: parseOptNum(minChangePct),
        macdFilter: macdFilter === 'all' ? null : macdFilter,
        minHealthScore: parseOptNum(minHealthScore),
        weeklyRegimeFilter: weeklyRegimeFilter || null,
        enableMtf1h,
        maxPe: parseOptNum(maxPe),
        maxPb: parseOptNum(maxPb),
        minRoe: parseOptNum(minRoe),
        onlyMoneyInToday,
        minQmvTmc: parseOptNum(minQmvTmc),
      })
      setResult(res)
      setLiveStatus(`Đã quét ${res.scannedSymbols} mã VN, khớp ${res.matchedSymbols} mã theo chuẩn QMV`)
    } catch (err) {
      pushToast('error', `Lỗi quét QMV Screener: ${String(err)}`)
    } finally {
      setScanning(false)
    }
  }

  useEffect(() => {
    void runScan()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    exchangeFilter,
    sectorGroupFilter,
    sectorFilter,
    priceTierFilter,
    defaultLiq50k,
    selectedSignals,
    categoryFilter,
    regimeFilter,
    macdFilter,
    weeklyRegimeFilter,
    enableMtf1h,
    onlyMoneyInToday,
  ])

  // Auto-Refresh interval timer (`Tắt / 30s / 60s`, Wave 4)
  useEffect(() => {
    if (autoRefreshSec === 0) return
    const timer = window.setInterval(() => {
      void runScan()
    }, autoRefreshSec * 1000)
    return () => window.clearInterval(timer)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [autoRefreshSec])

  const handleSort = (col: SortKey) => {
    const nextDir = sortCol === col && sortDir === 'ascending' ? 'descending' : 'ascending'
    setSortCol(col)
    setSortDir(nextDir)
  }

  const sortedRows = useMemo(() => {
    const rows = [...(result?.rows ?? [])]
    const factor = sortDir === 'ascending' ? 1 : -1
    rows.sort((a: ScreenerRowDto, b: ScreenerRowDto) => {
      if (sortCol === 'symbol') return factor * collator.compare(a.symbol, b.symbol)
      const va = (a[sortCol] ?? -Infinity) as number
      const vb = (b[sortCol] ?? -Infinity) as number
      return factor * (va - vb)
    })
    return rows
  }, [result, sortCol, sortDir])

  const renderSortHeader = (col: SortKey, label: string, alignRight = false, title?: string) => (
    <th scope="col" aria-sort={sortCol === col ? sortDir : undefined} title={title}>
      <button
        type="button"
        onClick={() => handleSort(col)}
        className={alignRight ? 'justify-end' : 'justify-start'}
      >
        <span>{label}</span>
        {sortCol === col && (
          <span aria-hidden="true">
            {sortDir === 'ascending' ? <ArrowUp className="w-3 h-3" /> : <ArrowDown className="w-3 h-3" />}
          </span>
        )}
      </button>
    </th>
  )

  const handleExportCsv = () => {
    if (sortedRows.length === 0) {
      pushToast('info', 'Không có dữ liệu lọc QMV để xuất CSV')
      return
    }
    const headers = [
      'Ma',
      'San',
      'NganhQMV',
      'NhomDongTien',
      'GiaDong',
      'ThayDoiPct',
      'KhoiLuong',
      'KLGD20_TB',
      'RVol20',
      'QMV_Flow_Z50_T05_Proxy',
      'QMV_Flow_Z50_T20_Proxy',
      'RSI14',
      'XuHuong',
      'DiemSucKhoe',
      'TinHieuQMV',
    ]
    const esc = (v: unknown) => {
      const s = String(v ?? '')
      return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s
    }
    const lines = [
      headers.join(','),
      ...sortedRows.map((r) =>
        [
          r.symbol,
          r.exchange,
          r.qmvSectorLabel ?? '',
          r.moneyTier ?? '',
          r.lastClose,
          r.changePct.toFixed(2),
          Math.round(r.lastVolume),
          r.avgVol20 != null ? Math.round(r.avgVol20) : '',
          r.rvol20 != null ? r.rvol20.toFixed(2) : '',
          r.qmvFlowZ50T05 != null ? r.qmvFlowZ50T05.toFixed(1) : '',
          r.qmvFlowZ50T20 != null ? r.qmvFlowZ50T20.toFixed(1) : '',
          r.rsi14 != null ? r.rsi14.toFixed(1) : '',
          r.regime,
          r.healthScore != null ? r.healthScore.toFixed(0) : '',
          r.matchedSignals.join(' | '),
        ]
          .map(esc)
          .join(','),
      ),
    ]
    const blob = new Blob(['\uFEFF' + lines.join('\n')], { type: 'text/csv;charset=utf-8;' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `qmv_screener_${exchangeFilter.toLowerCase()}_${new Date().toISOString().slice(0, 10)}.csv`
    document.body.appendChild(a)
    a.click()
    a.remove()
    URL.revokeObjectURL(url)
    pushToast('success', `Đã xuất ${sortedRows.length} mã QMV Screener ra CSV`)
  }

  const tierBadgeStyle = (tier?: string) => {
    if (tier === 'DT1') return { bg: 'rgba(41, 98, 255, 0.16)', fg: '#5B8DEF' }
    if (tier === 'DT2') return { bg: 'rgba(8, 153, 129, 0.16)', fg: '#089981' }
    if (tier === 'DT3') return { bg: 'rgba(255, 152, 0, 0.16)', fg: '#FF9800' }
    return { bg: 'var(--tv-bg-elevated)', fg: 'var(--tv-text-secondary)' }
  }

  return (
    <div className="flex flex-col h-full min-h-0 overflow-hidden">
      {/* Row 1: QMV Universe, Sector, Money Tier, Liquidity Gate & Quick Actions */}
      <div
        className="flex flex-wrap items-center gap-1.5 px-3 py-1.5 shrink-0"
        style={{ borderBottom: '1px solid var(--tv-border)', backgroundColor: 'var(--tv-bg-elevated)' }}
      >
        <select
          aria-label="Rổ cổ phiếu & Sàn VN"
          value={exchangeFilter}
          onChange={(e) => setExchangeFilter(e.target.value as ScreenerExchangeFilter)}
          className="px-2 py-1 rounded text-[11.5px] font-semibold outline-none"
          style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
        >
          <option value="ALL_VN">Toàn thị trường VN (475 mã)</option>
          <option value="QMV60">★ Rổ QMV60 (60 mã trọng điểm)</option>
          <option value="QMV30">★ Rổ QMV30 (30 cổ phiếu trụ)</option>
          <option value="DT1">Dòng tiền Loại 1 (DT1 · Tạo lập)</option>
          <option value="DT2">Dòng tiền Loại 2 (DT2 · Mid-cap)</option>
          <option value="DT3">Dòng tiền Loại 3 (DT3 · Đầu cơ)</option>
          <option value="HOSE">Sàn HOSE</option>
          <option value="HNX">Sàn HNX</option>
          <option value="UPCOM">Sàn UPCOM</option>
          <option value="WATCHLIST">Danh mục đang mở</option>
        </select>

        <select
          aria-label="Nhóm ngành chu kỳ QMV"
          value={sectorGroupFilter}
          onChange={(e) => {
            setSectorGroupFilter(e.target.value)
            setSectorFilter('')
          }}
          className="px-2 py-1 rounded text-[11px] outline-none"
          style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
        >
          <option value="">Mọi Nhóm ngành (1–3)</option>
          <option value="GROUP_1">Nhóm 1: Tài chính & Đòn bẩy</option>
          <option value="GROUP_2">Nhóm 2: Chu kỳ & Tăng trưởng</option>
          <option value="GROUP_3">Nhóm 3: Phòng thủ & Tiện ích</option>
        </select>

        <select
          aria-label="Ngành QMV chi tiết"
          value={sectorFilter}
          onChange={(e) => setSectorFilter(e.target.value)}
          className="px-2 py-1 rounded text-[11px] outline-none"
          style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
        >
          <option value="">Tất cả 11 ngành QMV</option>
          <option value="BANKING">Ngân hàng</option>
          <option value="BROKERAGE">Chứng khoán</option>
          <option value="REAL_ESTATE">BĐS Dân cư</option>
          <option value="CONSTRUCTION_STEEL">XD & VLXD / Thép</option>
          <option value="RETAIL">Bán lẻ</option>
          <option value="TECH_TELECOM">Công nghệ & Viễn thông</option>
          <option value="INDUSTRIAL_LOGISTICS">KCN & Cảng / Logistics</option>
          <option value="OIL_GAS_CHEM">Dầu khí & Hóa chất</option>
          <option value="CONSUMER_FNB">Thực phẩm & Thủy sản (F&B)</option>
          <option value="UTILITIES_DEFENSIVE">Điện / Nước / Bảo hiểm / Dược</option>
          <option value="OTHER_CYCLICAL">Sản xuất & Dịch vụ khác</option>
        </select>

        <select
          aria-label="Phân tầng thị giá QMV"
          value={priceTierFilter}
          onChange={(e) => setPriceTierFilter(e.target.value)}
          className="px-2 py-1 rounded text-[11px] outline-none"
          style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
        >
          <option value="">Mọi thị giá</option>
          <option value="OVER_50K">Thị giá cao (≥ 50k)</option>
          <option value="30K_50K">Thị giá vừa (30k – 50k)</option>
          <option value="10K_30K">Thị giá 10k – 30k</option>
          <option value="UNDER_10K">Trà đá (&lt; 10k)</option>
        </select>

        <label
          className="inline-flex items-center gap-1 px-2 py-1 rounded text-[11px] cursor-pointer select-none"
          style={{
            backgroundColor: defaultLiq50k ? 'rgba(8, 153, 129, 0.14)' : 'var(--tv-bg-input)',
            border: `1px solid ${defaultLiq50k ? 'rgba(8, 153, 129, 0.45)' : 'var(--tv-border)'}`,
            color: defaultLiq50k ? '#089981' : 'var(--tv-text-secondary)',
          }}
          title="Ngưỡng thanh khoản mặc định trên QMV Screener V7: KLGD bình quân 20 phiên >= 50,000 cp"
        >
          <input
            type="checkbox"
            checked={defaultLiq50k}
            onChange={(e) => setDefaultLiq50k(e.target.checked)}
            className="m-0"
          />
          <span className="font-medium">KL20 ≥ 50k</span>
        </label>

        {hasAnyActiveFilter && (
          <button
            type="button"
            onClick={handleClearAllDefaultFilters}
            className="inline-flex items-center gap-1 px-2 py-1 rounded text-[11px] font-medium border cursor-pointer"
            style={{
              backgroundColor: 'rgba(242, 54, 69, 0.10)',
              borderColor: 'rgba(242, 54, 69, 0.35)',
              color: '#F23645',
            }}
            title="Quy tắc QMV B5: Gỡ bộ lọc mặc định để xem toàn bộ cổ phiếu kể cả thanh khoản thấp"
          >
            <FilterX className="w-3 h-3" />
            <span>Gỡ bộ lọc mặc định</span>
          </button>
        )}

        <div className="flex items-center gap-1.5 ml-auto">
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') void runScan()
            }}
            placeholder="Tìm mã..."
            className="w-20 px-2 py-1 rounded text-[11px] outline-none"
            style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
          />
          <button
            type="button"
            disabled={scanning}
            onClick={() => void runScan()}
            className="flex items-center gap-1 px-2.5 py-1 rounded text-[11.5px] font-semibold bg-[#2962FF] hover:bg-[#1E53E5] text-white border-0 cursor-pointer disabled:opacity-50"
          >
            <Radar className="w-3.5 h-3.5" />
            <span>{scanning ? 'Đang lọc...' : 'Lọc QMV'}</span>
          </button>
          <button
            type="button"
            onClick={() => setShowAdvancedFilters((v) => !v)}
            className="flex items-center gap-1 px-2 py-1 rounded text-[11px] font-medium border cursor-pointer"
            style={{
              backgroundColor: showAdvancedFilters ? 'var(--tv-accent-soft)' : 'var(--tv-bg-panel)',
              borderColor: showAdvancedFilters ? 'var(--tv-accent)' : 'var(--tv-border)',
              color: showAdvancedFilters ? 'var(--tv-accent)' : 'var(--tv-text-primary)',
            }}
            title="Mở bộ lọc nâng cao MTF 1W/1H, Cơ bản DN01 (P/E, P/B, ROE) & QMV Blackbox Thật"
          >
            <SlidersHorizontal className="w-3 h-3" />
            <span>MTF / DN01</span>
          </button>
          <select
            aria-label="Tự động quét lại định kỳ"
            value={autoRefreshSec}
            onChange={(e) => setAutoRefreshSec(Number(e.target.value) as 0 | 30 | 60)}
            className="px-1.5 py-1 rounded text-[10.5px] outline-none"
            style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
            title="Tự động quét lại Screener theo chu kỳ"
          >
            <option value={0}>Auto: Tắt</option>
            <option value={30}>Auto: 30s</option>
            <option value={60}>Auto: 60s</option>
          </select>
          <button
            type="button"
            disabled={sortedRows.length === 0}
            onClick={() => {
              const topTickers = sortedRows.slice(0, 60).map((r) => r.ticker)
              const wlName = `Lọc ${exchangeFilter} (${new Date().toISOString().slice(5, 10)})`
              void createWatchlistWithSymbols(wlName, topTickers)
            }}
            className="flex items-center gap-1 px-2 py-1 rounded text-[11px] font-medium border cursor-pointer disabled:opacity-40"
            style={{ backgroundColor: 'var(--tv-bg-panel)', borderColor: 'var(--tv-border)', color: 'var(--tv-text-primary)' }}
            title="Lưu nhanh các mã lọc được (tối đa 60 mã) thành một Danh mục Watchlist mới"
          >
            <ListPlus className="w-3 h-3" />
            <span>→ WL</span>
          </button>
          <button
            type="button"
            onClick={handleExportCsv}
            className="flex items-center gap-1 px-2 py-1 rounded text-[11px] font-medium border cursor-pointer"
            style={{ backgroundColor: 'var(--tv-bg-panel)', borderColor: 'var(--tv-border)', color: 'var(--tv-text-primary)' }}
            title="Xuất bảng kết quả QMV Screener ra CSV"
          >
            <Download className="w-3 h-3" />
            <span>CSV</span>
          </button>
        </div>
      </div>

      {/* Row 1b (Collapsible Wave 4): MTF 1W/1H + Fundamental DN01 + Yumeko QMV Blackbox + Saved Screens */}
      {showAdvancedFilters && (
        <div
          className="flex flex-wrap items-center gap-1.5 px-3 py-1.5 shrink-0 text-[10.5px] tabular-nums"
          style={{ borderBottom: '1px solid var(--tv-border)', backgroundColor: 'var(--tv-bg-elevated)' }}
        >
          <select
            aria-label="Xu hướng khung Tuần (1W)"
            value={weeklyRegimeFilter}
            onChange={(e) => setWeeklyRegimeFilter(e.target.value)}
            className="px-1.5 py-0.5 rounded outline-none"
            style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
          >
            <option value="">Khung Tuần 1W: Mọi</option>
            <option value="Uptrend">1W: Uptrend</option>
            <option value="Recovery">1W: Recovery</option>
            <option value="Pullback">1W: Pullback</option>
            <option value="SidewaysSqueeze">1W: Squeeze</option>
            <option value="Downtrend">1W: Downtrend</option>
          </select>

          <label
            className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded cursor-pointer select-none"
            style={{
              backgroundColor: enableMtf1h ? 'var(--tv-accent-soft)' : 'var(--tv-bg-input)',
              border: `1px solid ${enableMtf1h ? 'var(--tv-accent)' : 'var(--tv-border)'}`,
              color: enableMtf1h ? 'var(--tv-accent)' : 'var(--tv-text-secondary)',
            }}
            title="Phễu 2 tầng MTF: Đánh giá thêm nhịp vào lệnh khung 1H cho Top 25 mã vượt qua bộ lọc 1W+1D"
          >
            <input type="checkbox" checked={enableMtf1h} onChange={(e) => setEnableMtf1h(e.target.checked)} className="m-0" />
            <span className="font-semibold">Phễu 1H (Top 25)</span>
          </label>

          <label className="flex items-center gap-1" style={{ color: 'var(--tv-text-secondary)' }}>
            <span>%Đổi ≥</span>
            <input
              type="number"
              step="0.5"
              placeholder="-"
              value={minChangePct}
              onChange={(e) => setMinChangePct(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && void runScan()}
              className="w-11 px-1 py-0.5 rounded text-center no-spinner outline-none"
              style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
            />
          </label>

          <select
            aria-label="Lọc MACD Histogram"
            value={macdFilter}
            onChange={(e) => setMacdFilter(e.target.value as 'all' | 'positive' | 'negative')}
            className="px-1.5 py-0.5 rounded outline-none"
            style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
          >
            <option value="all">MACD: Mọi</option>
            <option value="positive">MACD &gt; 0</option>
            <option value="negative">MACD &lt; 0</option>
          </select>

          <label className="flex items-center gap-1" style={{ color: 'var(--tv-text-secondary)' }}>
            <span>SK ≥</span>
            <input
              type="number"
              placeholder="20"
              value={minHealthScore}
              onChange={(e) => setMinHealthScore(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && void runScan()}
              className="w-11 px-1 py-0.5 rounded text-center no-spinner outline-none"
              style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
            />
          </label>

          <label className="flex items-center gap-1" style={{ color: 'var(--tv-text-secondary)' }}>
            <span>P/E ≤</span>
            <input
              type="number"
              step="1"
              placeholder="15"
              value={maxPe}
              onChange={(e) => setMaxPe(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && void runScan()}
              className="w-11 px-1 py-0.5 rounded text-center no-spinner outline-none"
              style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
            />
          </label>

          <label className="flex items-center gap-1" style={{ color: 'var(--tv-text-secondary)' }}>
            <span>P/B ≤</span>
            <input
              type="number"
              step="0.2"
              placeholder="2.5"
              value={maxPb}
              onChange={(e) => setMaxPb(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && void runScan()}
              className="w-11 px-1 py-0.5 rounded text-center no-spinner outline-none"
              style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
            />
          </label>

          <label className="flex items-center gap-1" style={{ color: 'var(--tv-text-secondary)' }}>
            <span>ROE ≥</span>
            <input
              type="number"
              step="1"
              placeholder="15"
              value={minRoe}
              onChange={(e) => setMinRoe(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && void runScan()}
              className="w-11 px-1 py-0.5 rounded text-center no-spinner outline-none"
              style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
            />
          </label>

          <label
            className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded cursor-pointer select-none"
            style={{
              backgroundColor: onlyMoneyInToday ? 'rgba(8, 153, 129, 0.16)' : 'var(--tv-bg-input)',
              border: `1px solid ${onlyMoneyInToday ? '#089981' : 'var(--tv-border)'}`,
              color: onlyMoneyInToday ? '#089981' : 'var(--tv-text-secondary)',
            }}
            title="Lọc trực tiếp cờ TIENVAO_TODAY từ Hộp đen QMV Thật trong Yumeko"
          >
            <input
              type="checkbox"
              checked={onlyMoneyInToday}
              onChange={(e) => setOnlyMoneyInToday(e.target.checked)}
              className="m-0"
            />
            <span className="font-semibold">Tiền vào QMV Thật</span>
          </label>

          <label className="flex items-center gap-1" style={{ color: 'var(--tv-text-secondary)' }}>
            <span>TMC ≥</span>
            <input
              type="number"
              step="1"
              placeholder="-"
              value={minQmvTmc}
              onChange={(e) => setMinQmvTmc(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && void runScan()}
              className="w-11 px-1 py-0.5 rounded text-center no-spinner outline-none"
              style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
            />
          </label>

          {/* Saved Screens CRUD */}
          <div className="flex items-center gap-1 ml-auto">
            <Bookmark className="w-3 h-3 text-[#2962FF]" />
            <select
              aria-label="Bộ lọc đã lưu"
              value={selectedSavedId}
              onChange={(e) => {
                const id = e.target.value
                setSelectedSavedId(id)
                const found = savedScreens.find((s) => s.id === id)
                if (found) applySavedScreen(found.filterJson)
              }}
              className="max-w-[130px] px-1.5 py-0.5 rounded outline-none"
              style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
            >
              <option value="">Bộ lọc đã lưu ({savedScreens.length})</option>
              {savedScreens.map((sc) => (
                <option key={sc.id} value={sc.id}>
                  {sc.name}
                </option>
              ))}
            </select>
            {selectedSavedId && (
              <button
                type="button"
                aria-label="Xóa bộ lọc đang chọn"
                onClick={() => {
                  void deleteScreenerPreset(selectedSavedId)
                  setSelectedSavedId('')
                }}
                className="p-0.5 border-0 bg-transparent cursor-pointer text-[#F23645]"
                title="Xóa bộ lọc đang chọn"
              >
                <Trash2 className="w-3 h-3" />
              </button>
            )}
            <input
              type="text"
              value={saveScreenName}
              onChange={(e) => setSaveScreenName(e.target.value)}
              placeholder="Tên bộ lọc..."
              className="w-24 px-1.5 py-0.5 rounded outline-none"
              style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
            />
            <button
              type="button"
              onClick={() => {
                if (!saveScreenName.trim()) return
                void saveScreenerPreset(saveScreenName.trim(), snapshotCurrentFilterJson())
                setSaveScreenName('')
              }}
              className="px-2 py-0.5 rounded font-semibold bg-[#2962FF] text-white border-0 cursor-pointer"
            >
              Lưu
            </button>
          </div>
        </div>
      )}

      {/* Row 2: Multi-Signal `AND` Builder + QMV Screener Presets + Numeric Filters */}
      <div
        className="flex flex-wrap items-center gap-1.5 px-3 py-1 shrink-0 text-[11px]"
        style={{ borderBottom: '1px solid var(--tv-border)', backgroundColor: 'var(--tv-bg-panel)' }}
      >
        <select
          aria-label="Nhóm trụ cột tín hiệu QMV"
          value={categoryFilter}
          onChange={(e) => setCategoryFilter(e.target.value)}
          className="px-1.5 py-0.5 rounded text-[11px] outline-none"
          style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
        >
          {CATEGORY_TABS.map((c) => (
            <option key={c.key} value={c.key}>
              {c.label}
            </option>
          ))}
        </select>

        <select
          aria-label="Thêm tín hiệu QMV kết hợp (AND)"
          value={pendingSignalId}
          onChange={(e) => addSignalChip(e.target.value)}
          className="max-w-[235px] px-1.5 py-0.5 rounded text-[11px] outline-none"
          style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
        >
          <option value="">+ Thêm điều kiện tín hiệu QMV (AND)...</option>
          {visibleCatalog.map((sig) => (
            <option key={sig.id} value={sig.id}>
              {sig.isOhlcvProxy ? '[Proxy] ' : ''}
              {sig.labelVi}
            </option>
          ))}
        </select>

        <select
          aria-label="Lọc trạng thái xu hướng"
          value={regimeFilter}
          onChange={(e) => setRegimeFilter(e.target.value)}
          className="px-1.5 py-0.5 rounded text-[11px] outline-none"
          style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
        >
          <option value="">Mọi xu hướng</option>
          <option value="Uptrend">Uptrend (Xu hướng lên)</option>
          <option value="Recovery">Recovery (Hồi phục)</option>
          <option value="Pullback">Pullback (Điều chỉnh tích lũy)</option>
          <option value="SidewaysSqueeze">Squeeze (Nén chặt)</option>
          <option value="Downtrend">Downtrend (Xu hướng xuống)</option>
        </select>

        <label className="flex items-center gap-1 tabular-nums" style={{ color: 'var(--tv-text-secondary)' }}>
          <span>RSI</span>
          <input
            aria-label="RSI tối thiểu"
            type="number"
            placeholder="Từ"
            value={minRsi}
            onChange={(e) => setMinRsi(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && void runScan()}
            className="w-11 px-1 py-0.5 rounded text-[10.5px] text-center no-spinner outline-none"
            style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
          />
          <span>–</span>
          <input
            aria-label="RSI tối đa"
            type="number"
            placeholder="Đến"
            value={maxRsi}
            onChange={(e) => setMaxRsi(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && void runScan()}
            className="w-11 px-1 py-0.5 rounded text-[10.5px] text-center no-spinner outline-none"
            style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
          />
        </label>

        <label className="flex items-center gap-1 tabular-nums" style={{ color: 'var(--tv-text-secondary)' }}>
          <span>RVol ≥</span>
          <input
            aria-label="RVol tối thiểu"
            type="number"
            step="0.1"
            placeholder="1.3"
            value={minRvol}
            onChange={(e) => setMinRvol(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && void runScan()}
            className="w-12 px-1 py-0.5 rounded text-[10.5px] text-center no-spinner outline-none"
            style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
          />
        </label>

        <label
          className="inline-flex items-center gap-1 text-[10.5px] cursor-pointer select-none"
          style={{ color: 'var(--tv-text-secondary)' }}
          title="Mặc định ẩn các tín hiệu nến nhiễu ngoài QMV; bật để hiện đủ 123 tín hiệu sumi-core"
        >
          <input
            type="checkbox"
            checked={showAllCatalog}
            onChange={(e) => setShowAllCatalog(e.target.checked)}
            className="m-0"
          />
          <span>Đủ 123 TH</span>
        </label>

        {result && (
          <div className="ml-auto flex items-center gap-2 text-[10.5px] tabular-nums">
            {(result.staleSymbolsCount ?? 0) > 0 && (
              <button
                type="button"
                disabled={syncingEod || Boolean(dataStatus?.offlineMode)}
                onClick={() => {
                  void syncUniverseEod(exchangeFilter, 120).then(() => runScan())
                }}
                className="inline-flex items-center gap-1 px-2 py-0.5 rounded font-semibold border cursor-pointer disabled:opacity-50"
                style={{
                  backgroundColor: 'rgba(251, 192, 45, 0.14)',
                  borderColor: 'rgba(251, 192, 45, 0.45)',
                  color: '#FBC02D',
                }}
                title={`Có ${result.staleSymbolsCount}/${result.scannedSymbols} mã có nến cũ hơn phiên kỳ vọng ${result.expectedSessionDate ?? ''} (trung vị ${result.medianCachedDate ?? '—'}). Nhấn để đồng bộ EOD ngay.`}
              >
                <RefreshCw className={`w-3 h-3 ${syncingEod ? 'animate-spin' : ''}`} />
                <span>
                  {syncingEod && eodSyncProgress
                    ? `EOD ${eodSyncProgress.completed}/${eodSyncProgress.total}`
                    : `${result.staleSymbolsCount} mã cũ (${result.medianCachedDate ?? '—'}) · Đồng bộ EOD`}
                </span>
              </button>
            )}
            <span style={{ color: 'var(--tv-text-secondary)' }}>
              <strong>{result.matchedSymbols}</strong>/{result.scannedSymbols} mã · {result.elapsedMs}ms
            </span>
          </div>
        )}
      </div>

      {/* Row 3: Quick QMV Presets & Active `AND` Signal Chips */}
      <div
        className="flex flex-wrap items-center gap-1.5 px-3 py-1 shrink-0 text-[10.5px]"
        style={{ borderBottom: '1px solid var(--tv-border)', backgroundColor: 'var(--tv-bg-panel)' }}
      >
        <span className="inline-flex items-center gap-1 font-semibold text-[#2962FF]">
          <Sparkles className="w-3 h-3" />
          <span>Bộ lọc nhanh QMV:</span>
        </span>
        {QMV_QUICK_PRESETS.map((preset) => {
          const active =
            preset.signals.length === selectedSignals.length &&
            preset.signals.every((s) => selectedSignals.includes(s))
          return (
            <button
              key={preset.label}
              type="button"
              onClick={() => setSelectedSignals(active ? [] : preset.signals)}
              title={preset.title}
              className="inline-flex items-center gap-1 px-2 py-0.5 rounded border cursor-pointer"
              style={{
                backgroundColor: active ? 'var(--tv-accent-soft)' : 'var(--tv-bg-elevated)',
                borderColor: active ? 'var(--tv-accent)' : 'var(--tv-border)',
                color: active ? 'var(--tv-accent)' : 'var(--tv-text-primary)',
              }}
            >
              <Plus className="w-2.5 h-2.5" />
              <span>{preset.label}</span>
            </button>
          )
        })}

        {selectedSignals.map((sigId) => {
          const meta = signalLabelMap.get(sigId)
          return (
            <span
              key={sigId}
              className="inline-flex items-center gap-1 px-2 py-0.5 rounded font-semibold"
              style={{
                backgroundColor: 'rgba(41, 98, 255, 0.16)',
                border: '1px solid rgba(41, 98, 255, 0.45)',
                color: '#5B8DEF',
              }}
            >
              <span>{meta?.labelVi ?? sigId}</span>
              <button
                type="button"
                onClick={() => removeSignalChip(sigId)}
                className="p-0 bg-transparent border-0 cursor-pointer text-current"
                title="Gỡ điều kiện này"
              >
                <X className="w-3 h-3" />
              </button>
            </span>
          )
        })}
      </div>

      <div role="status" className="sr-only-live">
        {liveStatus}
      </div>

      {/* Semantic Sortable QMV Screener Table */}
      <div role="region" aria-labelledby="screener-table-caption" tabIndex={0} className="flex-1 overflow-auto tv-scroll">
        <table className="tv-table tabular-nums min-w-[940px]">
          <caption id="screener-table-caption" className="sr-only-live">
            Bảng lọc kỹ thuật chuẩn QMV (475 mã Việt Nam · Phân nhóm Ngành & Dòng tiền DT1/DT2/DT3)
          </caption>
          <thead>
            <tr>
              {renderSortHeader('symbol', 'Mã')}
              <th scope="col">Ngành QMV</th>
              <th scope="col" title="Phân nhóm Dòng tiền QMV: DT1 (Tạo lập/Trụ) · DT2 (Mid-cap) · DT3 (Đầu cơ)">
                DT
              </th>
              {renderSortHeader('lastClose', 'Giá đóng', true)}
              {renderSortHeader('changePct', '% Đổi', true)}
              {renderSortHeader('lastVolume', 'KLGD', true)}
              {renderSortHeader('rvol20', 'RVol(20)', true)}
              {renderSortHeader(
                'qmvFlowZ50T05',
                'Chu kỳ Z50 (T+/Sóng)',
                true,
                'OHLCV Proxy Z-50 [0..100]: <=30 Duy trì (xanh) · >=70 Bão hòa (đỏ)',
              )}
              {renderSortHeader('rsi14', 'RSI(14)', true)}
              {renderSortHeader('pe', 'P/E · ROE', true, 'Định giá DN01 / TV Cache')}
              {renderSortHeader('qmvRealTmc', 'QMV TMC', true, 'Hộp đen QMV Thật từ Yumeko')}
              <th scope="col">1W / 1D / 1H</th>
              {renderSortHeader('healthScore', 'Điểm SK', true)}
              <th scope="col">Tín hiệu QMV kích hoạt</th>
            </tr>
          </thead>
          <tbody>
            {sortedRows.length === 0 ? (
              <tr>
                <td colSpan={14} className="text-center py-6" style={{ color: 'var(--tv-text-secondary)' }}>
                  {scanning
                    ? 'Đang quét 475 mã Việt Nam theo bộ tiêu chí QMV...'
                    : 'Không có cổ phiếu nào khớp tổ hợp điều kiện hiện tại. Nhấn "Gỡ bộ lọc mặc định" để mở rộng.'}
                </td>
              </tr>
            ) : (
              sortedRows.map((row) => {
                const tb = tierBadgeStyle(row.moneyTier)
                const z5 = row.qmvFlowZ50T05
                const z20 = row.qmvFlowZ50T20
                return (
                  <tr
                    key={row.ticker}
                    className="cursor-pointer"
                    onClick={() => setActiveTicker(row.ticker)}
                    title={`Nhấp để mở biểu đồ ${row.ticker} (${row.qmvSectorLabel ?? row.exchange})`}
                  >
                    <td>
                      <div className="flex items-center gap-1">
                        <span className="font-bold text-[#2962FF]">{row.symbol}</span>
                        <span className="text-[9.5px]" style={{ color: 'var(--tv-text-muted)' }}>
                          {row.exchange}
                        </span>
                        {row.inQmv30 ? (
                          <span
                            className="px-1 py-0 rounded text-[9px] font-bold"
                            style={{ backgroundColor: 'rgba(251, 192, 45, 0.18)', color: '#FBC02D' }}
                          >
                            QMV30
                          </span>
                        ) : row.inQmv60 ? (
                          <span
                            className="px-1 py-0 rounded text-[9px] font-semibold"
                            style={{ backgroundColor: 'rgba(41, 98, 255, 0.14)', color: '#5B8DEF' }}
                          >
                            QMV60
                          </span>
                        ) : null}
                      </div>
                    </td>
                    <td className="text-[11px] max-w-[145px] truncate" style={{ color: 'var(--tv-text-secondary)' }}>
                      {row.qmvSectorLabel ?? '—'}
                    </td>
                    <td>
                      {row.moneyTier ? (
                        <span
                          className="px-1.5 py-0.5 rounded text-[10px] font-bold"
                          style={{ backgroundColor: tb.bg, color: tb.fg }}
                        >
                          {row.moneyTier}
                        </span>
                      ) : (
                        '—'
                      )}
                    </td>
                    <td className="text-right font-medium">{row.lastClose.toLocaleString('en-US')}</td>
                    <td
                      className="text-right font-semibold"
                      style={{ color: row.changePct >= 0 ? 'var(--tv-up)' : 'var(--tv-down)' }}
                    >
                      {row.changePct >= 0 ? '+' : ''}
                      {row.changePct.toFixed(2)}%
                    </td>
                    <td className="text-right">{Math.round(row.lastVolume).toLocaleString('en-US')}</td>
                    <td
                      className="text-right font-medium"
                      style={{
                        color:
                          row.rvol20 != null && row.rvol20 >= 1.5
                            ? 'var(--tv-up)'
                            : row.rvol20 != null && row.rvol20 <= 0.65
                              ? '#FBC02D'
                              : undefined,
                      }}
                    >
                      {row.rvol20 != null ? `${row.rvol20.toFixed(2)}x` : '—'}
                    </td>
                    <td className="text-right text-[11px]">
                      {z5 != null ? (
                        <span>
                          <strong
                            style={{
                              color: z5 <= 30 ? 'var(--tv-up)' : z5 >= 70 ? 'var(--tv-down)' : 'var(--tv-text-primary)',
                            }}
                          >
                            {z5.toFixed(0)}
                          </strong>
                          <span style={{ color: 'var(--tv-text-muted)' }}> / </span>
                          <span
                            style={{
                              color:
                                z20 != null && z20 <= 30
                                  ? 'var(--tv-up)'
                                  : z20 != null && z20 >= 70
                                    ? 'var(--tv-down)'
                                    : 'var(--tv-text-secondary)',
                            }}
                          >
                            {z20 != null ? z20.toFixed(0) : '—'}
                          </span>
                        </span>
                      ) : (
                        '—'
                      )}
                    </td>
                    <td
                      className="text-right"
                      style={{
                        color:
                          row.rsi14 != null && row.rsi14 <= 30
                            ? 'var(--tv-up)'
                            : row.rsi14 != null && row.rsi14 >= 70
                              ? 'var(--tv-down)'
                              : undefined,
                      }}
                    >
                      {row.rsi14 != null ? row.rsi14.toFixed(1) : '—'}
                    </td>
                    <td className="text-right text-[10.5px]">
                      {row.pe != null || row.roe != null ? (
                        <span>
                          <span>{row.pe != null ? `${row.pe.toFixed(1)}x` : '—'}</span>
                          <span style={{ color: 'var(--tv-text-muted)' }}> · </span>
                          <span style={{ color: (row.roe ?? 0) >= 15 ? 'var(--tv-up)' : 'var(--tv-text-secondary)' }}>
                            {row.roe != null ? `${row.roe.toFixed(0)}%` : '—'}
                          </span>
                        </span>
                      ) : (
                        '—'
                      )}
                    </td>
                    <td className="text-right text-[10.5px]">
                      {row.qmvRealProvenance ? (
                        <span className="inline-flex items-center gap-1">
                          {row.qmvRealMoneyInToday && (
                            <span
                              className="w-1.5 h-1.5 rounded-full"
                              style={{ backgroundColor: '#089981' }}
                              title="TIENVAO_TODAY (Yumeko QMV Thật)"
                            />
                          )}
                          <span
                            className="font-semibold"
                            style={{
                              color:
                                (row.qmvRealTmc ?? 0) > 0
                                  ? 'var(--tv-up)'
                                  : (row.qmvRealTmc ?? 0) < 0
                                    ? 'var(--tv-down)'
                                    : 'var(--tv-text-primary)',
                            }}
                          >
                            {row.qmvRealTmc != null ? row.qmvRealTmc.toFixed(1) : '✓'}
                          </span>
                        </span>
                      ) : (
                        <span style={{ color: 'var(--tv-text-muted)' }}>—</span>
                      )}
                    </td>
                    <td>
                      <div className="flex items-center gap-1">
                        {row.weeklyRegimeShort && (
                          <span
                            className="px-1 py-0 rounded text-[9.5px] font-semibold"
                            style={{ backgroundColor: 'var(--tv-bg-elevated)', color: 'var(--tv-text-secondary)' }}
                            title={`Khung tuần 1W: ${row.weeklyRegime ?? ''}`}
                          >
                            W:{row.weeklyRegimeShort}
                          </span>
                        )}
                        <span
                          className="px-1.5 py-0.5 rounded text-[10px] font-semibold"
                          style={{
                            backgroundColor:
                              row.regime === 'Uptrend' || row.regime === 'Recovery'
                                ? 'rgba(8, 153, 129, 0.16)'
                                : row.regime === 'Downtrend'
                                  ? 'rgba(242, 54, 69, 0.16)'
                                  : 'var(--tv-bg-elevated)',
                            color:
                              row.regime === 'Uptrend' || row.regime === 'Recovery'
                                ? 'var(--tv-up)'
                                : row.regime === 'Downtrend'
                                  ? 'var(--tv-down)'
                                  : 'var(--tv-text-secondary)',
                          }}
                        >
                          {row.regimeShort}
                        </span>
                        {row.hourlyTiming && (
                          <span
                            className="px-1 py-0 rounded text-[9.5px] font-bold"
                            style={{
                              backgroundColor:
                                row.hourlyTiming === 'BULL_1H'
                                  ? 'rgba(8, 153, 129, 0.18)'
                                  : row.hourlyTiming === 'PULLBACK_1H'
                                    ? 'rgba(41, 98, 255, 0.16)'
                                    : 'rgba(242, 54, 69, 0.14)',
                              color:
                                row.hourlyTiming === 'BULL_1H'
                                  ? '#089981'
                                  : row.hourlyTiming === 'PULLBACK_1H'
                                    ? '#5B8DEF'
                                    : '#F23645',
                            }}
                          >
                            {row.hourlyTiming.replace('_1H', '')}
                          </span>
                        )}
                        {row.mtfAligned && (
                          <span
                            className="px-1 py-0 rounded text-[9px] font-bold"
                            style={{ backgroundColor: 'rgba(8, 153, 129, 0.2)', color: '#089981' }}
                            title="Đồng pha Đa Khung (MTF Aligned)"
                          >
                            MTF
                          </span>
                        )}
                      </div>
                    </td>
                    <td
                      className="text-right font-bold"
                      style={{
                        color:
                          (row.healthScore ?? 0) >= 25
                            ? 'var(--tv-up)'
                            : (row.healthScore ?? 0) <= -25
                              ? 'var(--tv-down)'
                              : 'var(--tv-text-primary)',
                      }}
                    >
                      {row.healthScore != null ? row.healthScore.toFixed(0) : '—'}
                    </td>
                    <td className="max-w-[270px] truncate text-[11px]" style={{ color: 'var(--tv-text-secondary)' }}>
                      {row.matchedSignals.length > 0 ? row.matchedSignals.join(' · ') : '—'}
                    </td>
                  </tr>
                )
              })
            )}
          </tbody>
        </table>
      </div>
    </div>
  )
}
