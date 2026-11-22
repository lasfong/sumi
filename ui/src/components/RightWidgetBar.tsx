import React, { useMemo, useState } from 'react'
import { Bell, BellOff, List, Plus, RefreshCw, Trash2 } from 'lucide-react'
import { useWorkspaceStore } from '../store/workspaceStore'

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

export const RightWidgetBar: React.FC = () => {
  const rightPanelWidth = useWorkspaceStore((s) => s.rightPanelWidth)
  const watchlists = useWorkspaceStore((s) => s.watchlists)
  const activeWatchlistId = useWorkspaceStore((s) => s.activeWatchlistId)
  const quotesMap = useWorkspaceStore((s) => s.quotesMap)
  const quoteFlashMap = useWorkspaceStore((s) => s.quoteFlashMap)
  const alerts = useWorkspaceStore((s) => s.alerts)
  const panes = useWorkspaceStore((s) => s.panes)
  const activeSlotId = useWorkspaceStore((s) => s.activeSlotId)

  const setActiveWatchlistId = useWorkspaceStore((s) => s.setActiveWatchlistId)
  const setActiveTicker = useWorkspaceStore((s) => s.setActiveTicker)
  const refreshWatchlistQuotes = useWorkspaceStore((s) => s.refreshWatchlistQuotes)
  const addSymbolToWatchlist = useWorkspaceStore((s) => s.addSymbolToWatchlist)
  const removeSymbolFromWatchlist = useWorkspaceStore((s) => s.removeSymbolFromWatchlist)
  const createNewWatchlist = useWorkspaceStore((s) => s.createNewWatchlist)
  const createNewAlert = useWorkspaceStore((s) => s.createNewAlert)
  const toggleAlert = useWorkspaceStore((s) => s.toggleAlert)
  const removeAlert = useWorkspaceStore((s) => s.removeAlert)

  const activePane = panes.find((p) => p.slotId === activeSlotId) ?? panes[0]
  const activeWatchlist = watchlists.find((w) => w.id === activeWatchlistId) ?? watchlists[0]
  const activeQuote = quotesMap[activePane.ticker]

  const [rightTab, setRightTab] = useState<'watchlist' | 'alerts'>('watchlist')
  const [addTickerInput, setAddTickerInput] = useState('')
  const [showNewListForm, setShowNewListForm] = useState(false)
  const [newListName, setNewListName] = useState('')

  // Alert creation form state
  const [alertCondition, setAlertCondition] = useState<'cross_above' | 'cross_below' | 'cross_any'>('cross_above')
  const [alertPrice, setAlertPrice] = useState('')
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
    const price = Number(alertPrice)
    if (!Number.isFinite(price) || price <= 0) return
    void createNewAlert(
      activePane.ticker,
      alertCondition,
      price,
      alertMessage.trim() || `${activePane.ticker} chạm mốc ${price.toLocaleString('en-US')}`,
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
        role="tablist"
        aria-label="Chuyển đổi bảng phải"
        className="flex items-center justify-between px-1 shrink-0"
        style={{ height: 34, borderBottom: '1px solid var(--tv-border)' }}
      >
        <div className="flex items-center h-full">
          <button
            type="button"
            role="tab"
            id="rw-tab-watchlist"
            aria-selected={rightTab === 'watchlist'}
            className="tv-tab"
            onClick={() => setRightTab('watchlist')}
          >
            <List className="w-3.5 h-3.5" />
            <span>Danh mục</span>
          </button>
          <button
            type="button"
            role="tab"
            id="rw-tab-alerts"
            aria-selected={rightTab === 'alerts'}
            className="tv-tab"
            onClick={() => setRightTab('alerts')}
          >
            <Bell className="w-3.5 h-3.5" />
            <span>Cảnh báo</span>
            {armedAlertsCount > 0 && (
              <span
                className="px-1.5 rounded-full text-[10px] font-semibold"
                style={{ backgroundColor: 'var(--tv-accent-soft)', color: 'var(--tv-accent)' }}
              >
                {armedAlertsCount}
              </span>
            )}
          </button>
        </div>

        <button
          type="button"
          className="tv-icon-btn w-7 h-7"
          onClick={() => void refreshWatchlistQuotes()}
          title="Làm mới bảng giá"
        >
          <RefreshCw className="w-3.5 h-3.5" />
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

          {/* Watchlist Rows with Realtime Green/Red Flash */}
          <div className="flex-1 overflow-y-auto tv-scroll">
            {(activeWatchlist?.tickers ?? []).map((ticker) => {
              const q = quotesMap[ticker]
              const flash = quoteFlashMap[ticker]
              const isSelected = activePane.ticker === ticker
              const shortSym = ticker.includes(':') ? ticker.split(':')[1] : ticker
              const ex = ticker.includes(':') ? ticker.split(':')[0] : ''
              const chgPct = q?.changePercent ?? null

              // Exact Vietnam market price coloring (Trần tím / Sàn xanh lơ / Tham chiếu vàng / Tăng xanh / Giảm đỏ)
              let priceColor = 'var(--tv-text-primary)'
              if (q?.lastPrice != null && q.vnBand) {
                if (q.lastPrice >= q.vnBand.ceiling - 1e-6) priceColor = 'var(--tv-vn-ceil)'
                else if (q.lastPrice <= q.vnBand.floor + 1e-6) priceColor = 'var(--tv-vn-floor)'
                else if (Math.abs(q.lastPrice - q.vnBand.reference) < 1e-6) priceColor = 'var(--tv-vn-ref)'
                else if (q.lastPrice > q.vnBand.reference) priceColor = 'var(--tv-up)'
                else priceColor = 'var(--tv-down)'
              } else if (chgPct !== null) {
                priceColor = chgPct > 0 ? 'var(--tv-up)' : chgPct < 0 ? 'var(--tv-down)' : 'var(--tv-text-secondary)'
              }

              return (
                <div
                  key={`${ticker}-${flash?.nonce ?? 0}`}
                  className={`group grid grid-cols-[1fr_76px_64px_16px] gap-x-2 items-center px-2 py-1.5 cursor-pointer border-b tabular-nums ${
                    flash?.dir === 'up' ? 'flash-up' : flash?.dir === 'down' ? 'flash-down' : ''
                  }`}
                  style={{
                    borderColor: 'color-mix(in srgb, var(--tv-border) 45%, transparent)',
                    backgroundColor: isSelected ? 'var(--tv-accent-soft)' : undefined,
                  }}
                  onClick={() => setActiveTicker(ticker)}
                >
                  <div className="min-w-0 pr-1">
                    <div className="flex items-center gap-1">
                      <span className="font-semibold text-[12px] truncate">{shortSym}</span>
                      <span className="text-[9.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
                        {ex}
                      </span>
                    </div>
                  </div>
                  <div className="text-right font-medium text-[11.5px]" style={{ color: priceColor }}>
                    {formatPrice(q?.lastPrice, q?.pricePrecision ?? 0)}
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
                      void removeSymbolFromWatchlist(ticker)
                    }}
                  >
                    ×
                  </button>
                </div>
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
                  {activeQuote?.vnBand?.tickSize
                    ? `${activeQuote.vnBand.tickSize} VND`
                    : activeQuote?.pricePrecision === 0
                      ? '100'
                      : '0.01'}
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
      ) : (
        /* Price Alerts Tab Panel */
        <div role="tabpanel" aria-labelledby="rw-tab-alerts" className="flex flex-col flex-1 min-h-0">
          <form
            onSubmit={handleCreateAlert}
            className="p-2.5 space-y-2 shrink-0"
            style={{ borderBottom: '1px solid var(--tv-border)' }}
          >
            <div className="text-[11px] font-semibold" style={{ color: 'var(--tv-text-secondary)' }}>
              TẠO CẢNH BÁO GIÁ CHO {activePane.ticker}
            </div>
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
            <button
              type="submit"
              className="w-full py-1.5 rounded text-[12px] font-semibold bg-[#2962FF] hover:bg-[#1E53E5] text-white border-0 cursor-pointer"
            >
              + Thêm Cảnh Báo Giá
            </button>
          </form>

          <div className="flex-1 overflow-y-auto tv-scroll p-2 space-y-1.5">
            {alerts.length === 0 ? (
              <div className="text-center py-8 text-[11.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
                Chưa có cảnh báo giá nào. Tạo cảnh báo ở trên để theo dõi realtime.
              </div>
            ) : (
              alerts.map((a) => (
                <div
                  key={a.id}
                  className="p-2 rounded space-y-1"
                  style={{
                    backgroundColor: 'var(--tv-bg-elevated)',
                    border: '1px solid var(--tv-border)',
                  }}
                >
                  <div className="flex items-center justify-between gap-1">
                    <button
                      type="button"
                      className="font-semibold text-[12px] bg-transparent border-0 p-0 cursor-pointer text-[var(--tv-text-primary)] hover:text-[#2962FF]"
                      onClick={() => setActiveTicker(a.ticker)}
                    >
                      {a.ticker}
                    </button>
                    <div className="flex items-center gap-1">
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
                    </span>
                    <span className="font-semibold">{a.targetPrice.toLocaleString('en-US')}</span>
                  </div>
                  {a.message && (
                    <div className="text-[10.5px] truncate" style={{ color: 'var(--tv-text-secondary)' }}>
                      {a.message}
                    </div>
                  )}
                </div>
              ))
            )}
          </div>
        </div>
      )}
    </aside>
  )
}
