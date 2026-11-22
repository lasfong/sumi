import React, { useEffect, useMemo, useState } from 'react'
import { ArrowDown, ArrowUp, Radar, Search } from 'lucide-react'
import * as api from '../../api/tauri'
import { useWorkspaceStore } from '../../store/workspaceStore'
import type { ScreenerRowDto, SignalCatalogItemDto, StockScreenerResponse } from '../../types/market'

type SortKey = 'symbol' | 'lastClose' | 'changePct' | 'lastVolume' | 'rvol20' | 'rsi14' | 'healthScore'

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' })

const CATEGORY_TABS = [
  { key: 'all', label: 'Tất cả (101)' },
  { key: 'health', label: 'Sức khỏe' },
  { key: 'technical', label: 'Kỹ thuật' },
  { key: 'ichimoku', label: 'Ichimoku' },
  { key: 'divergence', label: 'Phân kỳ' },
  { key: 'volume', label: 'Khối lượng' },
  { key: 'pattern', label: 'Mô hình nến' },
  { key: 'vsa', label: 'VSA' },
  { key: 'flow', label: 'Dòng tiền' },
]

export const StockScreenerTab: React.FC = () => {
  const watchlists = useWorkspaceStore((s) => s.watchlists)
  const activeWatchlistId = useWorkspaceStore((s) => s.activeWatchlistId)
  const setActiveTicker = useWorkspaceStore((s) => s.setActiveTicker)
  const pushToast = useWorkspaceStore((s) => s.pushToast)

  const [catalog, setCatalog] = useState<SignalCatalogItemDto[]>([])
  const [exchangeFilter, setExchangeFilter] = useState<'ALL' | 'HOSE' | 'HNX' | 'UPCOM' | 'WATCHLIST'>('HOSE')
  const [categoryFilter, setCategoryFilter] = useState<string>('all')
  const [signalId, setSignalId] = useState<string>('')
  const [regimeFilter, setRegimeFilter] = useState<string>('')
  const [searchQuery, setSearchQuery] = useState<string>('')

  const [scanning, setScanning] = useState(false)
  const [result, setResult] = useState<StockScreenerResponse | null>(null)
  const [sortCol, setSortCol] = useState<SortKey>('healthScore')
  const [sortDir, setSortDir] = useState<'ascending' | 'descending'>('descending')
  const [liveStatus, setLiveStatus] = useState('')

  useEffect(() => {
    void api.getSignalCatalog().then(setCatalog)
  }, [])

  const filteredCatalogOptions = useMemo(() => {
    if (categoryFilter === 'all') return catalog
    return catalog.filter((c) => c.category === categoryFilter)
  }, [catalog, categoryFilter])

  const runScan = async () => {
    setScanning(true)
    try {
      const wl = watchlists.find((w) => w.id === activeWatchlistId)
      const res = await api.runStockScreener({
        exchangeFilter,
        tickers: exchangeFilter === 'WATCHLIST' ? (wl?.tickers ?? []) : undefined,
        signalId: signalId || undefined,
        categoryFilter: categoryFilter !== 'all' ? categoryFilter : undefined,
        regimeFilter: regimeFilter || undefined,
        searchQuery: searchQuery.trim() || undefined,
        maxSymbols: 180,
      })
      setResult(res)
      setLiveStatus(`Đã quét ${res.scannedSymbols} mã, tìm thấy ${res.matchedSymbols} mã phù hợp`)
    } catch (err) {
      pushToast('error', `Lỗi quét cổ phiếu: ${String(err)}`)
    } finally {
      setScanning(false)
    }
  }

  useEffect(() => {
    void runScan()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [exchangeFilter, categoryFilter, signalId, regimeFilter])

  const handleSort = (col: SortKey) => {
    const nextDir = sortCol === col && sortDir === 'ascending' ? 'descending' : 'ascending'
    setSortCol(col)
    setSortDir(nextDir)
    setLiveStatus(`Đã sắp xếp theo ${col}, ${nextDir === 'ascending' ? 'tăng dần' : 'giảm dần'}`)
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

  const renderSortHeader = (col: SortKey, label: string, alignRight = false) => (
    <th scope="col" aria-sort={sortCol === col ? sortDir : undefined}>
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

  return (
    <div className="flex flex-col h-full min-h-0 overflow-hidden">
      {/* Screener Filter Bar */}
      <div
        className="flex flex-wrap items-center gap-2 px-3 py-1.5 shrink-0"
        style={{ borderBottom: '1px solid var(--tv-border)', backgroundColor: 'var(--tv-bg-elevated)' }}
      >
        <select
          aria-label="Lọc sàn"
          value={exchangeFilter}
          onChange={(e) => setExchangeFilter(e.target.value as typeof exchangeFilter)}
          className="px-2 py-1 rounded text-[11.5px] font-medium outline-none"
          style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
        >
          <option value="ALL">Tất cả sàn VN</option>
          <option value="HOSE">HOSE</option>
          <option value="HNX">HNX</option>
          <option value="UPCOM">UPCOM</option>
          <option value="WATCHLIST">Danh mục đang xem</option>
        </select>

        <select
          aria-label="Nhóm tín hiệu"
          value={categoryFilter}
          onChange={(e) => {
            setCategoryFilter(e.target.value)
            setSignalId('')
          }}
          className="px-2 py-1 rounded text-[11.5px] outline-none"
          style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
        >
          {CATEGORY_TABS.map((c) => (
            <option key={c.key} value={c.key}>
              {c.label}
            </option>
          ))}
        </select>

        <select
          aria-label="Chọn 1 trong 101 tín hiệu định lượng"
          value={signalId}
          onChange={(e) => setSignalId(e.target.value)}
          className="max-w-[230px] px-2 py-1 rounded text-[11.5px] outline-none"
          style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
        >
          <option value="">Tất cả {catalog.length || 101} tín hiệu kỹ thuật</option>
          {filteredCatalogOptions.map((sig) => (
            <option key={sig.id} value={sig.id}>
              {sig.labelVi} ({sig.outputType})
            </option>
          ))}
        </select>

        <select
          aria-label="Lọc trạng thái xu hướng"
          value={regimeFilter}
          onChange={(e) => setRegimeFilter(e.target.value)}
          className="px-2 py-1 rounded text-[11.5px] outline-none"
          style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
        >
          <option value="">Mọi xu hướng</option>
          <option value="Uptrend">Uptrend</option>
          <option value="Recovery">Recovery</option>
          <option value="Pullback">Pullback</option>
          <option value="SidewaysSqueeze">Squeeze</option>
          <option value="Downtrend">Downtrend</option>
        </select>

        <div
          className="flex items-center gap-1 px-2 py-0.5 rounded"
          style={{ backgroundColor: 'var(--tv-bg-input)', border: '1px solid var(--tv-border)' }}
        >
          <Search className="w-3 h-3" style={{ color: 'var(--tv-text-secondary)' }} />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Lọc mã..."
            className="w-20 bg-transparent border-0 outline-none text-[11.5px]"
            style={{ color: 'var(--tv-text-primary)' }}
          />
        </div>

        <button
          type="button"
          disabled={scanning}
          onClick={() => void runScan()}
          className="flex items-center gap-1.5 px-3 py-1 rounded text-[11.5px] font-semibold bg-[#2962FF] hover:bg-[#1E53E5] text-white border-0 cursor-pointer disabled:opacity-50"
        >
          <Radar className="w-3.5 h-3.5" />
          <span>{scanning ? 'Đang quét...' : 'Quét 101 tín hiệu'}</span>
        </button>

        {result && (
          <span className="ml-auto text-[11px] tabular-nums" style={{ color: 'var(--tv-text-secondary)' }}>
            {result.matchedSymbols}/{result.scannedSymbols} mã · {result.totalCatalogSignals} tín hiệu · {result.elapsedMs}ms
          </span>
        )}
      </div>

      {/* Live region for screen reader sort/scan status */}
      <div role="status" className="sr-only-live">
        {liveStatus}
      </div>

      {/* Semantic Sortable Screener Table */}
      <div role="region" aria-labelledby="screener-table-caption" tabIndex={0} className="flex-1 overflow-auto tv-scroll">
        <table className="tv-table tabular-nums min-w-[780px]">
          <caption id="screener-table-caption" className="sr-only-live">
            Bảng lọc cổ phiếu theo 101 tín hiệu kỹ thuật sumi-core
          </caption>
          <thead>
            <tr>
              {renderSortHeader('symbol', 'Mã')}
              <th scope="col">Sàn</th>
              {renderSortHeader('lastClose', 'Giá đóng', true)}
              {renderSortHeader('changePct', '% Thay đổi', true)}
              {renderSortHeader('lastVolume', 'Khối lượng', true)}
              {renderSortHeader('rvol20', 'RVol(20)', true)}
              {renderSortHeader('rsi14', 'RSI(14)', true)}
              <th scope="col">Xu hướng</th>
              {renderSortHeader('healthScore', 'Sức khỏe', true)}
              <th scope="col">Tín hiệu kích hoạt</th>
            </tr>
          </thead>
          <tbody>
            {sortedRows.length === 0 ? (
              <tr>
                <td colSpan={10} className="text-center py-6" style={{ color: 'var(--tv-text-secondary)' }}>
                  {scanning ? 'Đang chạy bộ quét 101 tín hiệu sumi-core...' : 'Không có mã nào khớp bộ lọc hiện tại.'}
                </td>
              </tr>
            ) : (
              sortedRows.map((row) => (
                <tr
                  key={row.ticker}
                  className="cursor-pointer"
                  onClick={() => setActiveTicker(row.ticker)}
                  title={`Nhấp để nạp ${row.ticker} lên biểu đồ chính`}
                >
                  <td className="font-bold text-[#2962FF]">{row.symbol}</td>
                  <td style={{ color: 'var(--tv-text-secondary)' }}>{row.exchange}</td>
                  <td className="text-right font-medium">{row.lastClose.toLocaleString('en-US')}</td>
                  <td
                    className="text-right font-semibold"
                    style={{ color: row.changePct >= 0 ? 'var(--tv-up)' : 'var(--tv-down)' }}
                  >
                    {row.changePct >= 0 ? '+' : ''}
                    {row.changePct.toFixed(2)}%
                  </td>
                  <td className="text-right">{Math.round(row.lastVolume).toLocaleString('en-US')}</td>
                  <td className="text-right">
                    {row.rvol20 != null ? `${row.rvol20.toFixed(2)}x` : '—'}
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
                  <td>
                    <span
                      className="px-1.5 py-0.5 rounded text-[10px] font-semibold"
                      style={{
                        backgroundColor:
                          row.regime === 'Uptrend' || row.regime === 'Recovery'
                            ? 'var(--tv-up-soft)'
                            : row.regime === 'Downtrend'
                              ? 'var(--tv-down-soft)'
                              : 'var(--tv-hover)',
                        color:
                          row.regime === 'Uptrend' || row.regime === 'Recovery'
                            ? 'var(--tv-up)'
                            : row.regime === 'Downtrend'
                              ? 'var(--tv-down)'
                              : 'var(--tv-text-secondary)',
                      }}
                    >
                      {row.regime}
                    </span>
                  </td>
                  <td className="text-right font-bold">
                    {row.healthScore != null ? (
                      <span style={{ color: row.healthScore >= 0 ? 'var(--tv-up)' : 'var(--tv-down)' }}>
                        {row.healthScore >= 0 ? '+' : ''}
                        {row.healthScore.toFixed(0)}
                      </span>
                    ) : (
                      '—'
                    )}
                  </td>
                  <td>
                    <div className="flex items-center gap-1 max-w-[300px] overflow-hidden">
                      {row.selectedSignalValue && (
                        <span className="px-1.5 py-0.5 rounded text-[10px] font-bold bg-[#2962FF] text-white">
                          {row.selectedSignalValue}
                        </span>
                      )}
                      {row.matchedSignals.slice(0, 2).map((sig, idx) => (
                        <span
                          key={idx}
                          className="px-1.5 py-0.5 rounded text-[10px] truncate"
                          style={{ backgroundColor: 'var(--tv-hover)', color: 'var(--tv-text-primary)' }}
                        >
                          {sig}
                        </span>
                      ))}
                    </div>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  )
}
