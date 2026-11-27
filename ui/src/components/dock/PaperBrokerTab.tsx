import React, { useEffect, useState } from 'react'
import { Calculator, RefreshCw, RotateCcw, ShieldAlert, X } from 'lucide-react'
import * as api from '../../api/tauri'
import { useWorkspaceStore } from '../../store/workspaceStore'
import type { ChecklistState, DomLadderSnapshot, JournalAnalyticsResponseDto } from '../../types/market'

type BrokerSubTab = 'positions' | 'orders' | 'fifo' | 'dom' | 'journal'

const SETUP_TAGS = [
  { value: 'Untagged', label: 'Chưa gắn Setup' },
  { value: 'TrendPullback', label: 'Pullback Xu hướng' },
  { value: 'SqueezeBreakout', label: 'Breakout Nén BB' },
  { value: 'MeanReversion', label: 'Hồi quy Trung bình' },
  { value: 'SupportBounce', label: 'Bật Hỗ trợ cứng' },
  { value: 'MomentumContinuation', label: 'Tiếp diễn Động lượng' },
  { value: 'LiquiditySweep', label: 'Quét Thanh khoản' },
] as const

const MISTAKE_TAGS = [
  { value: 'None', label: 'Đúng kỷ luật' },
  { value: 'FomoChase', label: 'FOMO Đuổi giá' },
  { value: 'MovedStopLoss', label: 'Dời/Gỡ Cắt lỗ' },
  { value: 'EarlyExit', label: 'Chốt non sớm' },
  { value: 'OversizedRisk', label: 'Quá cỡ vị thế' },
  { value: 'CounterTrendImpulse', label: 'Bắt dao ngược xu hướng' },
  { value: 'IgnoredDailyBand', label: 'Vi phạm Biên độ / T+2' },
  { value: 'SkippedChecklist', label: 'Bỏ qua Checklist' },
] as const

const CHECKLIST_KEYS: Array<{ key: keyof ChecklistState; short: string; title: string }> = [
  { key: 'trend_aligned', short: 'XH', title: 'Thuận xu hướng khung lớn' },
  { key: 'level_confirmed', short: 'CĐ', title: 'Xác nhận vùng hỗ trợ/kháng cự' },
  { key: 'catalyst_or_volume_ok', short: 'Vol', title: 'Dòng tiền / Khối lượng đạt chuẩn' },
  { key: 'stop_and_lot_sized', short: 'SL', title: 'Đặt SL & tính lô 100 trước khi vào' },
  { key: 'band_and_t2_checked', short: 'T+2', title: 'Kiểm tra biên độ & khóa T+2' },
  { key: 'calm_state', short: 'TL', title: 'Tâm lý ổn định, không FOMO' },
]

const DEFAULT_CHECKLIST: ChecklistState = {
  trend_aligned: true,
  level_confirmed: false,
  catalyst_or_volume_ok: false,
  stop_and_lot_sized: true,
  band_and_t2_checked: true,
  calm_state: false,
}

function fmtMoney(v: number, isVnd: boolean): string {
  if (!Number.isFinite(v)) return '—'
  if (isVnd) return `${Math.round(v).toLocaleString('en-US')} ₫`
  return `$${v.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`
}

function fmtTs(secs: number): string {
  if (!secs) return '—'
  return new Date(secs * 1000).toLocaleString('vi-VN', {
    day: '2-digit',
    month: '2-digit',
    year: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}

export const PaperBrokerTab: React.FC = () => {
  const panes = useWorkspaceStore((s) => s.panes)
  const activeSlotId = useWorkspaceStore((s) => s.activeSlotId)
  const initialized = useWorkspaceStore((s) => s.initialized)
  const broker = useWorkspaceStore((s) => s.broker)
  const setBrokerSnapshot = useWorkspaceStore((s) => s.setBrokerSnapshot)
  const refreshBroker = useWorkspaceStore((s) => s.refreshBroker)
  const openTradePlanner = useWorkspaceStore((s) => s.openTradePlanner)
  const pushToast = useWorkspaceStore((s) => s.pushToast)

  const activePane = panes.find((p) => p.slotId === activeSlotId) ?? panes[0]

  const [subTab, setSubTab] = useState<BrokerSubTab>('positions')
  const [side, setSide] = useState<'BUY' | 'SELL'>('BUY')
  const [vnOrderType, setVnOrderType] = useState<'LO' | 'MP' | 'ATO' | 'ATC' | 'STOP'>('LO')
  const [qty, setQty] = useState<string>('1000')
  const [price, setPrice] = useState<string>('')
  const [stopLoss, setStopLoss] = useState<string>('')
  const [takeProfit, setTakeProfit] = useState<string>('')
  const [submitting, setSubmitting] = useState(false)
  const [noteDrafts, setNoteDrafts] = useState<
    Record<number, { setup: string; mistake: string; note: string; checklist: ChecklistState }>
  >({})
  const [bracketDraft, setBracketDraft] = useState<{ sl: string; tp: string }>({ sl: '', tp: '' })
  const [orderPriceDrafts, setOrderPriceDrafts] = useState<Record<number, string>>({})
  const [domSnap, setDomSnap] = useState<DomLadderSnapshot | null>(null)
  const [domLoading, setDomLoading] = useState(false)
  const [journalStats, setJournalStats] = useState<JournalAnalyticsResponseDto | null>(null)

  const isIndexTicker = (() => {
    const sym = (activePane.ticker.split(':')[1] ?? activePane.ticker).toUpperCase()
    return ['VNINDEX', 'HNXINDEX', 'UPCOMINDEX', 'VN30', 'HNX30', 'SPX', 'NDX', 'DJI', 'DXY'].includes(sym)
  })()

  // Avoid duplicate `broker_get_state` during initial bootstrap (`BUG-035`)
  useEffect(() => {
    if (!initialized) return
    if (
      broker &&
      broker.ticker.toUpperCase() === activePane.ticker.toUpperCase() &&
      (broker.interval === activePane.interval || broker.replayLinked)
    ) {
      return
    }
    void refreshBroker()
  }, [initialized, activePane.ticker, activePane.interval, broker, refreshBroker])

  // Reset default order quantity per asset class AND sync `Giá LO` when switching ticker or
  // entering/stepping/jumping Bar Replay (`BUG-034` + `BUG-041`)!
  useEffect(() => {
    if (!broker || broker.markPrice <= 0) return
    setPrice(String(broker.markPrice))
    const upper = broker.ticker.toUpperCase()
    if (upper.includes('VN30F') || upper.includes('VN301!')) {
      setQty('1')
    } else if (broker.boardLot === 100) {
      setQty('1000')
    } else if (upper.startsWith('BINANCE:') || upper.startsWith('OKX:') || upper.startsWith('BYBIT:')) {
      setQty(broker.markPrice >= 1000 ? '0.01' : '1')
    } else {
      setQty('10')
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [broker?.ticker, broker?.boardLot])

  useEffect(() => {
    if (!broker || broker.markPrice <= 0) return
    if (broker.replayLinked) {
      setPrice(String(broker.markPrice))
    }
  }, [broker?.replayLinked, broker?.markPrice, broker?.markTimestampMs])

  useEffect(() => {
    if (subTab !== 'dom') return
    setDomLoading(true)
    void api
      .computeDomLadder(activePane.ticker, activePane.interval, 8, 100)
      .then(setDomSnap)
      .catch(() => setDomSnap(null))
      .finally(() => setDomLoading(false))
  }, [subTab, activePane.ticker, activePane.interval, broker?.markTimestampMs, broker?.orders.length])

  useEffect(() => {
    if (subTab !== 'journal') return
    void api
      .brokerGetJournalAnalytics(activePane.ticker, activePane.interval)
      .then(setJournalStats)
      .catch(() => setJournalStats(null))
  }, [subTab, activePane.ticker, activePane.interval, broker?.closedTrades.length])

  const isVnd = ['HOSE', 'HNX', 'UPCOM', 'VN30F', 'Hose', 'Hnx', 'Upcom', 'Vn30Futures'].includes(
    broker?.venue ?? 'Hose',
  )
  const lotStep = broker?.boardLot ?? 100
  const numericQty = Number(qty)
  const needsExplicitPrice = vnOrderType === 'LO' || vnOrderType === 'STOP'
  const numericPrice = needsExplicitPrice ? Number(price) : (broker?.markPrice ?? 0)
  const lotInvalid = lotStep === 100 && (!Number.isFinite(numericQty) || numericQty <= 0 || numericQty % 100 !== 0)
  const bandInvalid =
    vnOrderType === 'LO' &&
    Boolean(broker?.vnBand) &&
    Number.isFinite(numericPrice) &&
    (numericPrice > (broker?.vnBand?.ceiling ?? Infinity) || numericPrice < (broker?.vnBand?.floor ?? 0))

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (lotInvalid) {
      pushToast('error', 'Sàn Việt Nam yêu cầu khối lượng là lô chẵn 100 cổ phiếu')
      return
    }
    if (bandInvalid && broker?.vnBand) {
      pushToast(
        'error',
        `Giá LO phải nằm trong biên độ [${broker.vnBand.floor.toLocaleString('en-US')} — ${broker.vnBand.ceiling.toLocaleString('en-US')}]`,
      )
      return
    }
    setSubmitting(true)
    try {
      const updated = await api.brokerSubmitOrder({
        ticker: activePane.ticker,
        interval: activePane.interval,
        side,
        vnOrderType,
        quantity: numericQty,
        price: needsExplicitPrice ? Number(price) : null,
        stopLoss: stopLoss.trim() ? Number(stopLoss) : null,
        takeProfit: takeProfit.trim() ? Number(takeProfit) : null,
      })
      setBrokerSnapshot(updated)
      pushToast(
        'success',
        `Đã đặt lệnh ${side === 'BUY' ? 'MUA' : 'BÁN'} ${vnOrderType} ${numericQty.toLocaleString('en-US')} ${activePane.ticker}`,
      )
    } catch (err) {
      pushToast('error', String(err))
    } finally {
      setSubmitting(false)
    }
  }

  const handleCancelOrder = async (orderId: number) => {
    try {
      const updated = await api.brokerCancelOrder(activePane.ticker, activePane.interval, orderId)
      setBrokerSnapshot(updated)
      pushToast('info', `Đã hủy lệnh #${orderId}`)
    } catch (err) {
      pushToast('error', String(err))
    }
  }

  const handleFlatten = async () => {
    try {
      const updated = await api.brokerFlattenPosition(activePane.ticker, activePane.interval)
      setBrokerSnapshot(updated)
      pushToast('info', `Đã đóng toàn bộ vị thế ${activePane.ticker}`)
    } catch (err) {
      pushToast('error', String(err))
    }
  }

  const handleReset = async () => {
    try {
      const updated = await api.brokerResetAccount(activePane.ticker, activePane.interval)
      setBrokerSnapshot(updated)
      pushToast('info', 'Đã khởi tạo lại tài khoản Paper Trading')
    } catch (err) {
      pushToast('error', String(err))
    }
  }

  useEffect(() => {
    if (broker?.openPosition) {
      setBracketDraft({
        sl: broker.openPosition.stop_loss ? String(broker.openPosition.stop_loss) : '',
        tp: broker.openPosition.take_profit ? String(broker.openPosition.take_profit) : '',
      })
    } else {
      setBracketDraft({ sl: '', tp: '' })
    }
  }, [broker?.ticker, broker?.openPosition?.stop_loss, broker?.openPosition?.take_profit])

  const handleUpdateBrackets = async () => {
    try {
      const sl = bracketDraft.sl.trim() ? Number(bracketDraft.sl) : null
      const tp = bracketDraft.tp.trim() ? Number(bracketDraft.tp) : null
      const updated = await api.brokerUpdateBrackets(activePane.ticker, activePane.interval, sl, tp)
      setBrokerSnapshot(updated)
      pushToast('success', 'Đã cập nhật mốc Cắt lỗ (SL) / Chốt lời (TP) cho vị thế mở')
    } catch (err) {
      pushToast('error', String(err))
    }
  }

  const handleModifyOrderPrice = async (orderId: number, fallbackPrice: number) => {
    const raw = orderPriceDrafts[orderId]
    const nextPrice = raw !== undefined && raw.trim() !== '' ? Number(raw) : fallbackPrice
    if (!Number.isFinite(nextPrice) || nextPrice <= 0) {
      pushToast('error', 'Giá sửa lệnh không hợp lệ')
      return
    }
    try {
      const updated = await api.brokerModifyOrder(activePane.ticker, activePane.interval, orderId, nextPrice)
      setBrokerSnapshot(updated)
      pushToast('success', `Đã sửa giá lệnh #${orderId} thành ${nextPrice.toLocaleString('en-US')}`)
    } catch (err) {
      pushToast('error', String(err))
    }
  }

  const saveJournalNote = async (tradeId: number) => {
    const draft = noteDrafts[tradeId]
    if (!draft) return
    await api.brokerUpdateJournalNote(
      activePane.ticker,
      tradeId,
      draft.setup,
      draft.mistake,
      draft.note,
      draft.checklist,
    )
    await refreshBroker()
    const stats = await api.brokerGetJournalAnalytics(activePane.ticker, activePane.interval).catch(() => null)
    if (stats) setJournalStats(stats)
    pushToast('success', `Đã lưu nhật ký (Setup / Sai lầm / Checklist) cho lệnh #${tradeId}`)
  }

  const contractMult =
    broker?.contractMultiplier ?? broker?.config.contract_multiplier ?? 1
  const estNotional =
    Number.isFinite(numericQty) && Number.isFinite(numericPrice)
      ? numericQty * numericPrice * (contractMult > 0 ? contractMult : 1)
      : 0
  const openPosNotional = broker?.openPosition
    ? broker.openPosition.avg_entry_price *
      broker.openPosition.quantity *
      (broker.openPosition.contract_multiplier || 1)
    : 0
  const marginPct = broker?.config.initial_margin_pct
  const marginRatio =
    typeof marginPct === 'number' && Number.isFinite(marginPct) && marginPct > 0 && marginPct < 100
      ? marginPct / 100
      : 1
  const availableCash = Math.max(0, (broker?.metrics.cash_balance ?? 0) - openPosNotional * marginRatio)

  return (
    <div className="grid grid-cols-1 lg:grid-cols-[312px_1fr] h-full min-h-0 overflow-hidden">
      {/* Left: Compact Vietnam Standard Order Ticket (100% above-the-fold without scrolling) */}
      <form
        onSubmit={handleSubmit}
        className="flex flex-col gap-1.5 px-2.5 py-2 overflow-y-auto tv-scroll"
        style={{ borderRight: '1px solid var(--tv-border)' }}
      >
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-1.5">
            <span className="font-bold text-[12px]">{activePane.ticker}</span>
            {broker?.replayLinked && (
              <span className="px-1.5 py-0 rounded text-[9.5px] font-bold bg-[#2962FF] text-white">
                REPLAY
              </span>
            )}
          </div>
          <div className="flex items-center gap-1">
            <button
              type="button"
              className="tv-btn h-5 px-1.5 text-[10.5px] text-[#2962FF]"
              onClick={() =>
                openTradePlanner({
                  ticker: activePane.ticker,
                  side: side === 'SELL' ? 'SHORT' : 'LONG',
                  entryPrice: Number(price) || broker?.markPrice,
                  stopLoss: stopLoss.trim() ? Number(stopLoss) : undefined,
                  targetPrice: takeProfit.trim() ? Number(takeProfit) : undefined,
                })
              }
              title="Mở máy tính Kế hoạch R:R & Khối lượng chuẩn (Trade Planner)"
            >
              <Calculator className="w-3 h-3" />
              <span>Kế hoạch R:R</span>
            </button>
            <button
              type="button"
              className="tv-btn h-5 px-1.5 text-[10.5px]"
              style={{ color: 'var(--tv-text-secondary)' }}
              onClick={() => void handleReset()}
              title="Reset tài khoản giả lập"
            >
              <RotateCcw className="w-3 h-3" />
              <span>Reset</span>
            </button>
          </div>
        </div>

        {/* Buy / Sell Segmented Buttons */}
        <div className="grid grid-cols-2 gap-1">
          <button
            type="button"
            className="py-1 rounded text-[11.5px] font-bold border-0 cursor-pointer"
            style={{
              backgroundColor: side === 'BUY' ? '#089981' : 'var(--tv-hover)',
              color: side === 'BUY' ? '#FFFFFF' : 'var(--tv-text-secondary)',
            }}
            onClick={() => setSide('BUY')}
          >
            MUA (BUY)
          </button>
          <button
            type="button"
            className="py-1 rounded text-[11.5px] font-bold border-0 cursor-pointer"
            style={{
              backgroundColor: side === 'SELL' ? '#F23645' : 'var(--tv-hover)',
              color: side === 'SELL' ? '#FFFFFF' : 'var(--tv-text-secondary)',
            }}
            onClick={() => setSide('SELL')}
          >
            BÁN (SELL)
          </button>
        </div>

        {/* VN Order Type Selector: LO / MP / ATO / ATC / STOP */}
        <div className="grid grid-cols-5 gap-1">
          {(['LO', 'MP', 'ATO', 'ATC', 'STOP'] as const).map((ot) => (
            <button
              key={ot}
              type="button"
              className="py-0.5 rounded text-[10.5px] font-semibold border cursor-pointer"
              style={{
                backgroundColor: vnOrderType === ot ? 'var(--tv-accent-soft)' : 'transparent',
                borderColor: vnOrderType === ot ? 'var(--tv-accent)' : 'var(--tv-border)',
                color: vnOrderType === ot ? 'var(--tv-accent)' : 'var(--tv-text-primary)',
              }}
              onClick={() => setVnOrderType(ot)}
            >
              {ot}
            </button>
          ))}
        </div>

        {/* Quantity + LO/STOP Price side-by-side in 2 columns so submit button stays 100% visible */}
        <div className={`grid gap-1.5 ${needsExplicitPrice ? 'grid-cols-2' : 'grid-cols-1'}`}>
          <div className="space-y-0.5">
            <div className="flex items-center justify-between text-[10px]" style={{ color: 'var(--tv-text-secondary)' }}>
              <span>KL {lotStep === 100 ? '(Lô 100)' : ''}</span>
              <div className="flex gap-0.5">
                {(lotStep === 100 ? [100, 500] : [1, 5]).map((step) => (
                  <button
                    key={step}
                    type="button"
                    className="px-1 rounded text-[9.5px] border-0 cursor-pointer"
                    style={{ backgroundColor: 'var(--tv-hover)', color: 'var(--tv-text-primary)' }}
                    onClick={() => setQty(String((Number(qty) || 0) + step))}
                  >
                    +{step}
                  </button>
                ))}
              </div>
            </div>
            <input
              type="number"
              step={lotStep}
              min={lotStep}
              value={qty}
              onChange={(e) => setQty(e.target.value)}
              className="w-full px-1.5 py-1 rounded text-[11.5px] tabular-nums outline-none no-spinner"
              style={{
                backgroundColor: 'var(--tv-bg-input)',
                color: 'var(--tv-text-primary)',
                border: `1px solid ${lotInvalid ? '#F23645' : 'var(--tv-border)'}`,
              }}
            />
          </div>

          {needsExplicitPrice && (
            <div className="space-y-0.5">
              <div className="flex items-center justify-between text-[10px]" style={{ color: 'var(--tv-text-secondary)' }}>
                <span>{vnOrderType === 'STOP' ? 'Giá STOP' : 'Giá LO'}</span>
                {broker?.vnBand && (
                  <div className="flex gap-0.5 tabular-nums">
                    <button
                      type="button"
                      className="px-1 rounded text-[9.5px] border-0 cursor-pointer"
                      style={{ backgroundColor: 'var(--tv-hover)', color: 'var(--tv-vn-floor)' }}
                      onClick={() => setPrice(String(broker.vnBand!.floor))}
                    >
                      Sàn
                    </button>
                    <button
                      type="button"
                      className="px-1 rounded text-[9.5px] border-0 cursor-pointer"
                      style={{ backgroundColor: 'var(--tv-hover)', color: 'var(--tv-vn-ref)' }}
                      onClick={() => setPrice(String(broker.vnBand!.reference))}
                    >
                      TC
                    </button>
                    <button
                      type="button"
                      className="px-1 rounded text-[9.5px] border-0 cursor-pointer"
                      style={{ backgroundColor: 'var(--tv-hover)', color: 'var(--tv-vn-ceil)' }}
                      onClick={() => setPrice(String(broker.vnBand!.ceiling))}
                    >
                      Trần
                    </button>
                  </div>
                )}
              </div>
              <input
                type="number"
                step="any"
                value={price}
                onChange={(e) => setPrice(e.target.value)}
                className="w-full px-1.5 py-1 rounded text-[11.5px] tabular-nums outline-none no-spinner"
                style={{
                  backgroundColor: 'var(--tv-bg-input)',
                  color: 'var(--tv-text-primary)',
                  border: `1px solid ${bandInvalid ? '#F23645' : 'var(--tv-border)'}`,
                }}
              />
            </div>
          )}
        </div>

        {(lotInvalid || (bandInvalid && broker?.vnBand)) && (
          <div className="text-[10px] leading-tight text-[#F23645]">
            {lotInvalid
              ? 'Khối lượng phải chia hết cho 100 cổ phiếu'
              : `Giá ngoài biên độ [${broker!.vnBand!.floor.toLocaleString('en-US')} — ${broker!.vnBand!.ceiling.toLocaleString('en-US')}]`}
          </div>
        )}

        {/* Optional SL / TP */}
        <div className="grid grid-cols-2 gap-1.5">
          <input
            type="number"
            step="any"
            value={stopLoss}
            onChange={(e) => setStopLoss(e.target.value)}
            placeholder="Cắt lỗ (SL)"
            className="px-1.5 py-0.5 rounded text-[11px] tabular-nums outline-none no-spinner"
            style={{
              backgroundColor: 'var(--tv-bg-input)',
              color: 'var(--tv-text-primary)',
              border: '1px solid var(--tv-border)',
            }}
          />
          <input
            type="number"
            step="any"
            value={takeProfit}
            onChange={(e) => setTakeProfit(e.target.value)}
            placeholder="Chốt lời (TP)"
            className="px-1.5 py-0.5 rounded text-[11px] tabular-nums outline-none no-spinner"
            style={{
              backgroundColor: 'var(--tv-bg-input)',
              color: 'var(--tv-text-primary)',
              border: '1px solid var(--tv-border)',
            }}
          />
        </div>

        <div className="flex items-center justify-between text-[10.5px] tabular-nums" style={{ color: 'var(--tv-text-secondary)' }}>
          <span>Giá trị ước tính:</span>
          <strong style={{ color: 'var(--tv-text-primary)' }}>{fmtMoney(estNotional, isVnd)}</strong>
        </div>

        {isIndexTicker && (
          <div className="text-[10px] leading-tight text-[#FBC02D]">
            Chỉ số thị trường không thể giao dịch trực tiếp — hãy chọn HNX:VN30F1! hoặc cổ phiếu cơ sở.
          </div>
        )}

        <button
          type="submit"
          disabled={isIndexTicker || submitting || lotInvalid || bandInvalid}
          className="w-full py-1.5 rounded text-[11.5px] font-bold text-white border-0 cursor-pointer disabled:opacity-50"
          style={{ backgroundColor: side === 'BUY' ? '#089981' : '#F23645' }}
        >
          {isIndexTicker
            ? 'Chỉ số (Không đặt lệnh)'
            : submitting
              ? 'Đang gửi lệnh...'
              : `${side === 'BUY' ? 'Đặt Mua' : 'Đặt Bán'} ${vnOrderType} · ${Number.isFinite(numericQty) ? numericQty.toLocaleString('en-US') : 0}`}
        </button>
      </form>

      {/* Right: Account Summary Strip + Positions / Orders / FIFO T+2 / Journal */}
      <div className="flex flex-col min-h-0 overflow-hidden">
        {/* Account KPI Bar */}
        <div
          className="flex flex-wrap items-center justify-between gap-2 px-3 py-1.5 text-[11px] tabular-nums shrink-0"
          style={{ borderBottom: '1px solid var(--tv-border)', backgroundColor: 'var(--tv-bg-elevated)' }}
        >
          <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
            <span>
              <span style={{ color: 'var(--tv-text-secondary)' }}>Tài sản ròng: </span>
              <strong>{fmtMoney(broker?.metrics.equity ?? 0, isVnd)}</strong>
            </span>
            <span>
              <span style={{ color: 'var(--tv-text-secondary)' }}>Tiền mặt: </span>
              <strong>{fmtMoney(availableCash, isVnd)}</strong>
            </span>
            <span>
              <span style={{ color: 'var(--tv-text-secondary)' }}>Lãi/Lỗ tạm tính: </span>
              <strong style={{ color: (broker?.metrics.unrealized_pnl ?? 0) >= 0 ? 'var(--tv-up)' : 'var(--tv-down)' }}>
                {fmtMoney(broker?.metrics.unrealized_pnl ?? 0, isVnd)}
              </strong>
            </span>
            <span>
              <span style={{ color: 'var(--tv-text-secondary)' }}>Khả dụng T+2: </span>
              <strong style={{ color: 'var(--tv-up)' }}>
                {(broker?.fifoSummary.sellable_qty ?? 0).toLocaleString('en-US')}
              </strong>
              {(broker?.fifoSummary.locked_t2_qty ?? 0) > 0 && (
                <span className="ml-1 text-[#FBC02D]">
                  (Khóa: {broker!.fifoSummary.locked_t2_qty.toLocaleString('en-US')})
                </span>
              )}
            </span>
          </div>

          <div role="tablist" aria-label="Chi tiết Paper Broker" className="flex items-center gap-1 h-7">
            {(
              [
                { id: 'positions', label: `Vị thế (${broker?.openPosition ? 1 : 0})` },
                { id: 'orders', label: `Lệnh (${broker?.orders.length ?? 0})` },
                { id: 'fifo', label: `Lô FIFO T+2 (${broker?.fifoSummary.lots.length ?? 0})` },
                { id: 'dom', label: 'DOM (OHLCV Proxy)' },
                { id: 'journal', label: `Nhật ký & SQN (${broker?.closedTrades.length ?? 0})` },
              ] as const
            ).map((t) => (
              <button
                key={t.id}
                type="button"
                role="tab"
                aria-selected={subTab === t.id}
                className="tv-tab text-[11px] px-2"
                onClick={() => setSubTab(t.id)}
              >
                {t.label}
              </button>
            ))}
          </div>
        </div>

        {/* Sub-Tab Table Content */}
        <div className="flex-1 overflow-auto tv-scroll">
          {subTab === 'positions' && (
            <table className="tv-table tabular-nums">
              <caption className="sr-only-live">Vị thế đang mở</caption>
              <thead>
                <tr>
                  <th scope="col">Mã</th>
                  <th scope="col">Chiều</th>
                  <th scope="col">Khối lượng</th>
                  <th scope="col">Giá vốn TB</th>
                  <th scope="col">Giá thị trường</th>
                  <th scope="col">Lãi / Lỗ</th>
                  <th scope="col">SL / TP</th>
                  <th scope="col">Thao tác</th>
                </tr>
              </thead>
              <tbody>
                {!broker?.openPosition ? (
                  <tr>
                    <td colSpan={8} className="text-center py-6" style={{ color: 'var(--tv-text-secondary)' }}>
                      Chưa có vị thế mở trên {activePane.ticker}. Đặt lệnh Mua/Bán ở phiếu bên trái.
                    </td>
                  </tr>
                ) : (
                  (() => {
                    const p = broker.openPosition
                    const posMult = p.contract_multiplier || contractMult || 1
                    const unreal =
                      (broker.markPrice - p.avg_entry_price) * p.quantity * posMult * (p.side === 'Long' ? 1 : -1)
                    const pct = p.avg_entry_price > 0 ? ((broker.markPrice - p.avg_entry_price) / p.avg_entry_price) * 100 * (p.side === 'Long' ? 1 : -1) : 0
                    return (
                      <tr>
                        <td className="font-semibold">{broker.ticker}</td>
                        <td>
                          <span
                            className="px-1.5 py-0.5 rounded text-[10px] font-bold"
                            style={{
                              backgroundColor: p.side === 'Long' ? 'var(--tv-up-soft)' : 'var(--tv-down-soft)',
                              color: p.side === 'Long' ? 'var(--tv-up)' : 'var(--tv-down)',
                            }}
                          >
                            {p.side === 'Long' ? 'LONG' : 'SHORT'}
                          </span>
                        </td>
                        <td>{p.quantity.toLocaleString('en-US')}</td>
                        <td>{p.avg_entry_price.toLocaleString('en-US')}</td>
                        <td>{broker.markPrice.toLocaleString('en-US')}</td>
                        <td style={{ color: unreal >= 0 ? 'var(--tv-up)' : 'var(--tv-down)', fontWeight: 600 }}>
                          {fmtMoney(unreal, isVnd)} ({pct >= 0 ? '+' : ''}{pct.toFixed(2)}%)
                        </td>
                        <td>
                          <div className="flex items-center gap-1">
                            <input
                              type="number"
                              step="any"
                              aria-label="Giá cắt lỗ SL vị thế mở"
                              placeholder="SL"
                              value={bracketDraft.sl}
                              onChange={(e) => setBracketDraft((b) => ({ ...b, sl: e.target.value }))}
                              className="w-20 px-1 py-0.5 rounded text-[11px] tabular-nums outline-none no-spinner"
                              style={{
                                backgroundColor: 'var(--tv-bg-input)',
                                color: 'var(--tv-text-primary)',
                                border: '1px solid var(--tv-border)',
                              }}
                            />
                            <input
                              type="number"
                              step="any"
                              aria-label="Giá chốt lời TP vị thế mở"
                              placeholder="TP"
                              value={bracketDraft.tp}
                              onChange={(e) => setBracketDraft((b) => ({ ...b, tp: e.target.value }))}
                              className="w-20 px-1 py-0.5 rounded text-[11px] tabular-nums outline-none no-spinner"
                              style={{
                                backgroundColor: 'var(--tv-bg-input)',
                                color: 'var(--tv-text-primary)',
                                border: '1px solid var(--tv-border)',
                              }}
                            />
                            <button
                              type="button"
                              className="px-1.5 py-0.5 rounded text-[10px] font-semibold bg-[#2962FF] text-white border-0 cursor-pointer"
                              onClick={() => void handleUpdateBrackets()}
                            >
                              Lưu
                            </button>
                          </div>
                        </td>
                        <td>
                          <button
                            type="button"
                            className="px-2 py-0.5 rounded text-[11px] font-semibold bg-[#F23645] text-white border-0 cursor-pointer"
                            onClick={() => void handleFlatten()}
                          >
                            Đóng vị thế
                          </button>
                        </td>
                      </tr>
                    )
                  })()
                )}
              </tbody>
            </table>
          )}

          {subTab === 'orders' && (
            <table className="tv-table tabular-nums">
              <caption className="sr-only-live">Danh sách lệnh chờ và lệnh khớp</caption>
              <thead>
                <tr>
                  <th scope="col">#</th>
                  <th scope="col">Chiều</th>
                  <th scope="col">Loại</th>
                  <th scope="col">Khối lượng</th>
                  <th scope="col">Giá đặt / Khớp</th>
                  <th scope="col">Trạng thái</th>
                  <th scope="col">Thời gian</th>
                  <th scope="col">Hủy</th>
                </tr>
              </thead>
              <tbody>
                {(broker?.orders ?? []).length === 0 ? (
                  <tr>
                    <td colSpan={8} className="text-center py-6" style={{ color: 'var(--tv-text-secondary)' }}>
                      Chưa có lệnh nào trong phiên.
                    </td>
                  </tr>
                ) : (
                  broker!.orders.map((item) => {
                    const o = item.order
                    const triggerPrice =
                      o.filled_price ??
                      (typeof o.kind === 'object' && 'Limit' in o.kind
                        ? o.kind.Limit.limit_price
                        : typeof o.kind === 'object' && 'Stop' in o.kind
                          ? o.kind.Stop.stop_price
                          : (broker?.markPrice ?? 0))
                    const statusText =
                      typeof o.status === 'string' ? o.status : `Rejected: ${o.status.Rejected.reason}`
                    return (
                      <tr key={o.id}>
                        <td>#{o.id}</td>
                        <td style={{ color: o.side === 'Buy' ? 'var(--tv-up)' : 'var(--tv-down)', fontWeight: 600 }}>
                          {o.side === 'Buy' ? 'MUA' : 'BÁN'}
                        </td>
                        <td className="font-semibold">{item.vnOrderType}</td>
                        <td>{o.quantity.toLocaleString('en-US')}</td>
                        <td>
                          {o.status === 'Pending' && typeof o.kind === 'object' ? (
                            <div className="flex items-center gap-1">
                              <input
                                type="number"
                                step="any"
                                aria-label={`Giá mới cho lệnh chờ #${o.id}`}
                                value={orderPriceDrafts[o.id] ?? String(triggerPrice)}
                                onChange={(e) =>
                                  setOrderPriceDrafts((m) => ({ ...m, [o.id]: e.target.value }))
                                }
                                className="w-20 px-1 py-0.5 rounded text-[11px] tabular-nums outline-none no-spinner"
                                style={{
                                  backgroundColor: 'var(--tv-bg-input)',
                                  color: 'var(--tv-text-primary)',
                                  border: '1px solid var(--tv-border)',
                                }}
                              />
                              <button
                                type="button"
                                className="px-1.5 py-0.5 rounded text-[10px] font-semibold bg-[#2962FF] text-white border-0 cursor-pointer"
                                onClick={() => void handleModifyOrderPrice(o.id, triggerPrice)}
                              >
                                Sửa
                              </button>
                            </div>
                          ) : (
                            triggerPrice.toLocaleString('en-US')
                          )}
                        </td>
                        <td>
                          <span
                            className="px-1.5 py-0.5 rounded text-[10px] font-semibold"
                            style={{
                              backgroundColor:
                                o.status === 'Filled'
                                  ? 'var(--tv-up-soft)'
                                  : o.status === 'Pending'
                                    ? 'var(--tv-accent-soft)'
                                    : 'var(--tv-hover)',
                              color:
                                o.status === 'Filled'
                                  ? 'var(--tv-up)'
                                  : o.status === 'Pending'
                                    ? 'var(--tv-accent)'
                                    : 'var(--tv-text-secondary)',
                            }}
                          >
                            {statusText}
                          </span>
                        </td>
                        <td>{fmtTs(o.filled_timestamp ?? o.submitted_timestamp)}</td>
                        <td>
                          {o.status === 'Pending' && (
                            <button
                              type="button"
                              className="tv-icon-btn w-6 h-6 hover:text-[#F23645]"
                              onClick={() => void handleCancelOrder(o.id)}
                              title="Hủy lệnh chờ"
                            >
                              <X className="w-3.5 h-3.5" />
                            </button>
                          )}
                        </td>
                      </tr>
                    )
                  })
                )}
              </tbody>
            </table>
          )}

          {subTab === 'fifo' && (
            <div className="p-2 space-y-2">
              <div
                className="flex items-center gap-2 px-2.5 py-1.5 rounded text-[11px]"
                style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}
              >
                <ShieldAlert className="w-4 h-4 text-[#FBC02D] shrink-0" />
                <span>
                  Quy tắc thanh toán <strong>VSD T+2 (về tài khoản lúc 13:00 ICT)</strong> theo thứ tự FIFO và lịch nghỉ lễ sàn Việt Nam.
                  Khả dụng bán:{' '}
                  <strong style={{ color: 'var(--tv-up)' }}>
                    {(broker?.fifoSummary.sellable_qty ?? 0).toLocaleString('en-US')} CP
                  </strong>{' '}
                  · Đang chờ về:{' '}
                  <strong style={{ color: 'var(--tv-vn-ref)' }}>
                    {(broker?.fifoSummary.locked_t2_qty ?? 0).toLocaleString('en-US')} CP
                  </strong>
                </span>
              </div>
              <table className="tv-table tabular-nums">
                <caption className="sr-only-live">Bảng lô hàng FIFO VSD T+2</caption>
                <thead>
                  <tr>
                    <th scope="col">Lô #</th>
                    <th scope="col">Ngày khớp mua</th>
                    <th scope="col">Giá vốn</th>
                    <th scope="col">KL mua ban đầu</th>
                    <th scope="col">KL còn giữ</th>
                    <th scope="col">Về tài khoản (13:00 ICT)</th>
                    <th scope="col">Trạng thái VSD</th>
                  </tr>
                </thead>
                <tbody>
                  {(broker?.fifoSummary.lots ?? []).length === 0 ? (
                    <tr>
                      <td colSpan={7} className="text-center py-5" style={{ color: 'var(--tv-text-secondary)' }}>
                        Chưa có lô cổ phiếu nào đang nắm giữ.
                      </td>
                    </tr>
                  ) : (
                    broker!.fifoSummary.lots.map((lot) => {
                      const unlocked = (broker?.fifoSummary.as_of_timestamp ?? 0) >= lot.sellable_at_timestamp
                      return (
                        <tr key={lot.lot_id}>
                          <td>#{lot.lot_id}</td>
                          <td>{fmtTs(lot.buy_timestamp)}</td>
                          <td>{lot.buy_price.toLocaleString('en-US')}</td>
                          <td>{lot.initial_qty.toLocaleString('en-US')}</td>
                          <td className="font-semibold">{lot.remaining_qty.toLocaleString('en-US')}</td>
                          <td>{fmtTs(lot.sellable_at_timestamp)}</td>
                          <td>
                            <span
                              className="px-1.5 py-0.5 rounded text-[10px] font-bold"
                              style={{
                                backgroundColor: unlocked ? 'var(--tv-up-soft)' : 'rgba(251, 192, 45, 0.16)',
                                color: unlocked ? 'var(--tv-up)' : 'var(--tv-vn-ref)',
                              }}
                            >
                              {unlocked ? 'KHẢ DỤNG BÁN' : 'KHÓA T+2'}
                            </span>
                          </td>
                        </tr>
                      )
                    })
                  )}
                </tbody>
              </table>
            </div>
          )}

          {subTab === 'dom' && (
            <div className="p-2 space-y-2">
              <div
                className="flex flex-wrap items-center justify-between gap-2 px-2.5 py-1.5 rounded text-[11px]"
                style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}
              >
                <div className="flex items-center gap-2 flex-wrap">
                  <span
                    className="px-1.5 py-0.5 rounded text-[9.5px] font-bold"
                    style={{ backgroundColor: 'rgba(251, 192, 45, 0.18)', color: '#FBC02D' }}
                    title="Dựng tổng hợp từ chuỗi nến OHLCV nhân quả (không phải dữ liệu sổ lệnh Level-2 sở giao dịch)"
                  >
                    ⚠ {domSnap?.data_fidelity ?? 'OHLCV_SYNTH_DOM_PROXY'}
                  </span>
                  <span style={{ color: 'var(--tv-text-secondary)' }}>
                    Cân bằng Mua/Bán:{' '}
                    <strong style={{ color: (domSnap?.bid_ask_imbalance_pct ?? 50) >= 50 ? 'var(--tv-up)' : 'var(--tv-down)' }}>
                      {(domSnap?.bid_ask_imbalance_pct ?? 50).toFixed(1)}%
                    </strong>
                    {domSnap?.poc_price != null && (
                      <>
                        {' '}· POC: <strong style={{ color: '#FBC02D' }}>{domSnap.poc_price.toLocaleString('en-US')}</strong>
                      </>
                    )}
                    {' '}· Nhấp mức giá để điền nhanh vào phiếu LO
                  </span>
                </div>
                <button
                  type="button"
                  className="tv-icon-btn w-6 h-6"
                  onClick={() => {
                    setDomLoading(true)
                    void api
                      .computeDomLadder(activePane.ticker, activePane.interval, 8, 100)
                      .then(setDomSnap)
                      .finally(() => setDomLoading(false))
                  }}
                  title="Tải lại DOM Ladder"
                >
                  <RefreshCw className={`w-3.5 h-3.5 ${domLoading ? 'animate-spin' : ''}`} />
                </button>
              </div>
              {domSnap ? (
                <table className="tv-table tabular-nums">
                  <thead>
                    <tr>
                      <th scope="col" className="text-right">KL Mua (Bid Proxy)</th>
                      <th scope="col" className="text-center">Mức Giá (Click đặt LO)</th>
                      <th scope="col">KL Bán (Ask Proxy)</th>
                      <th scope="col" className="text-right">Net Delta</th>
                    </tr>
                  </thead>
                  <tbody>
                    {domSnap.rows.map((row) => {
                      const bidW = domSnap.max_row_depth > 0 ? Math.min(100, (row.synth_bid_depth / domSnap.max_row_depth) * 100) : 0
                      const askW = domSnap.max_row_depth > 0 ? Math.min(100, (row.synth_ask_depth / domSnap.max_row_depth) * 100) : 0
                      const isPoc = domSnap.poc_price != null && Math.abs(row.price - domSnap.poc_price) < 1e-6
                      return (
                        <tr
                          key={row.price}
                          style={{
                            backgroundColor: row.is_at_mark
                              ? 'var(--tv-accent-soft)'
                              : row.is_at_position_entry
                                ? 'rgba(8, 153, 129, 0.12)'
                                : undefined,
                          }}
                        >
                          <td className="text-right relative">
                            {row.synth_bid_depth > 0 && (
                              <div
                                className="absolute right-0 top-0.5 bottom-0.5 rounded-l"
                                style={{ width: `${bidW}%`, backgroundColor: 'rgba(8, 153, 129, 0.22)' }}
                              />
                            )}
                            <span className="relative z-10 text-[#089981]">
                              {row.synth_bid_depth > 0 ? Math.round(row.synth_bid_depth).toLocaleString('en-US') : ''}
                            </span>
                          </td>
                          <td className="text-center">
                            <button
                              type="button"
                              className="px-2 py-0.5 rounded font-bold border-0 bg-transparent cursor-pointer hover:bg-[var(--tv-hover)] text-[var(--tv-text-primary)]"
                              onClick={() => {
                                setVnOrderType('LO')
                                setPrice(String(row.price))
                              }}
                              title="Điền mức giá này vào phiếu lệnh LO"
                            >
                              {row.price.toLocaleString('en-US')}
                              {row.band_state === 'Ceiling' && <span className="ml-1 text-[9px] text-[var(--tv-vn-ceil)]">CEIL</span>}
                              {row.band_state === 'Floor' && <span className="ml-1 text-[9px] text-[var(--tv-vn-floor)]">FLR</span>}
                              {row.band_state === 'Reference' && <span className="ml-1 text-[9px] text-[var(--tv-vn-ref)]">REF</span>}
                              {isPoc && <span className="ml-1 text-[9px] text-[#FBC02D]">POC</span>}
                            </button>
                          </td>
                          <td className="relative">
                            {row.synth_ask_depth > 0 && (
                              <div
                                className="absolute left-0 top-0.5 bottom-0.5 rounded-r"
                                style={{ width: `${askW}%`, backgroundColor: 'rgba(242, 54, 69, 0.22)' }}
                              />
                            )}
                            <span className="relative z-10 text-[#F23645]">
                              {row.synth_ask_depth > 0 ? Math.round(row.synth_ask_depth).toLocaleString('en-US') : ''}
                            </span>
                          </td>
                          <td
                            className="text-right font-medium"
                            style={{ color: row.net_delta >= 0 ? 'var(--tv-up)' : 'var(--tv-down)' }}
                          >
                            {row.net_delta >= 0 ? '+' : ''}
                            {Math.round(row.net_delta).toLocaleString('en-US')}
                          </td>
                        </tr>
                      )
                    })}
                  </tbody>
                </table>
              ) : (
                <div className="text-center py-6 text-[11.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
                  {domLoading ? 'Đang dựng sổ lệnh tổng hợp (OHLCV Proxy)...' : 'Không đủ dữ liệu để dựng DOM Ladder.'}
                </div>
              )}
            </div>
          )}

          {subTab === 'journal' && (() => {
            const partialFills = (broker?.fills ?? [])
              .filter((f) => f.realized_net_pnl !== null && f.realized_net_pnl !== undefined)
              .slice()
              .reverse()
            const closedList = broker?.closedTrades ?? []
            const ja = journalStats?.analytics
            const eqPts = journalStats?.equityPoints ?? []
            return (
              <div className="space-y-2 p-2">
                {/* Journal Behavioral Analytics Banner (`SQN`, `Cost of Mistakes`, `Discipline %`, PnL Sparkline) */}
                {ja && (
                  <div
                    className="flex flex-wrap items-center justify-between gap-2 px-2.5 py-1.5 rounded text-[11px] tabular-nums"
                    style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}
                  >
                    <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
                      <span>
                        <span style={{ color: 'var(--tv-text-secondary)' }}>SQN: </span>
                        <strong>{ja.sqn != null ? ja.sqn.toFixed(2) : '—'}</strong>
                      </span>
                      <span>
                        <span style={{ color: 'var(--tv-text-secondary)' }}>Kỷ luật Checklist: </span>
                        <strong style={{ color: ja.discipline_score_pct >= 75 ? 'var(--tv-up)' : '#FBC02D' }}>
                          {ja.discipline_score_pct.toFixed(0)}%
                        </strong>
                      </span>
                      <span>
                        <span style={{ color: 'var(--tv-text-secondary)' }}>Chi phí Sai lầm: </span>
                        <strong style={{ color: ja.cost_of_mistakes > 0 ? 'var(--tv-down)' : 'var(--tv-up)' }}>
                          {fmtMoney(ja.cost_of_mistakes, isVnd)} ({ja.mistake_trade_count} lệnh)
                        </strong>
                      </span>
                      <span>
                        <span style={{ color: 'var(--tv-text-secondary)' }}>Expectancy R: </span>
                        <strong>{ja.expectancy_r != null ? `${ja.expectancy_r >= 0 ? '+' : ''}${ja.expectancy_r.toFixed(2)}R` : '—'}</strong>
                      </span>
                    </div>
                    {eqPts.length >= 2 && (() => {
                      const vals = eqPts.map((p) => p.cumulativePnl)
                      const minV = Math.min(...vals, 0)
                      const maxV = Math.max(...vals, 1)
                      const spanV = Math.max(1e-6, maxV - minV)
                      const pts = eqPts
                        .map((p, i) => `${((i / (eqPts.length - 1)) * 110 + 2).toFixed(1)},${(24 - ((p.cumulativePnl - minV) / spanV) * 20).toFixed(1)}`)
                        .join(' ')
                      const lastPos = eqPts[eqPts.length - 1].cumulativePnl >= 0
                      return (
                        <svg viewBox="0 0 114 26" className="w-[114px] h-[26px] shrink-0">
                          <polyline fill="none" stroke={lastPos ? '#089981' : '#F23645'} strokeWidth="1.6" points={pts} />
                        </svg>
                      )
                    })()}
                  </div>
                )}

                <table className="tv-table tabular-nums">
                  <caption className="sr-only-live">Nhật ký giao dịch MAE, MFE, Setup, Sai lầm và Checklist</caption>
                  <thead>
                    <tr>
                      <th scope="col">#</th>
                      <th scope="col">Vào / Ra</th>
                      <th scope="col">Net PnL (%)</th>
                      <th scope="col">MAE / MFE</th>
                      <th scope="col">Setup & Sai lầm</th>
                      <th scope="col">Checklist (6 điểm)</th>
                      <th scope="col">Ghi chú</th>
                    </tr>
                  </thead>
                  <tbody>
                    {closedList.length === 0 && partialFills.length === 0 ? (
                      <tr>
                        <td colSpan={7} className="text-center py-6" style={{ color: 'var(--tv-text-secondary)' }}>
                          Chưa có giao dịch đã đóng hoặc chốt một phần nào trong nhật ký.
                        </td>
                      </tr>
                    ) : (
                      <>
                        {closedList.map((item) => {
                          const t = item.trade
                          const draft = noteDrafts[t.trade_id] ?? {
                            setup: item.setupTag || 'Untagged',
                            mistake: item.mistakeTag || 'None',
                            note: item.note,
                            checklist: item.checklistJson ?? DEFAULT_CHECKLIST,
                          }
                          return (
                            <tr key={`closed_${t.trade_id}`}>
                              <td>
                                #{t.trade_id} <span className="text-[10px] opacity-70">{t.side}</span>
                              </td>
                              <td>
                                {t.entry_price.toLocaleString('en-US')} → {t.exit_price.toLocaleString('en-US')}
                              </td>
                              <td style={{ color: t.net_pnl >= 0 ? 'var(--tv-up)' : 'var(--tv-down)', fontWeight: 600 }}>
                                {fmtMoney(t.net_pnl, isVnd)} ({t.return_pct >= 0 ? '+' : ''}{t.return_pct.toFixed(1)}%)
                              </td>
                              <td>
                                <span style={{ color: 'var(--tv-down)' }}>{item.maePct.toFixed(1)}%</span> /{' '}
                                <span style={{ color: 'var(--tv-up)' }}>+{item.mfePct.toFixed(1)}%</span>
                              </td>
                              <td>
                                <div className="flex flex-col gap-1">
                                  <select
                                    aria-label={`Setup lệnh #${t.trade_id}`}
                                    value={draft.setup}
                                    onChange={(e) =>
                                      setNoteDrafts((m) => ({
                                        ...m,
                                        [t.trade_id]: { ...draft, setup: e.target.value },
                                      }))
                                    }
                                    className="px-1 py-0.5 rounded text-[10.5px] outline-none"
                                    style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                                  >
                                    {SETUP_TAGS.map((s) => (
                                      <option key={s.value} value={s.value}>{s.label}</option>
                                    ))}
                                  </select>
                                  <select
                                    aria-label={`Sai lầm lệnh #${t.trade_id}`}
                                    value={draft.mistake}
                                    onChange={(e) =>
                                      setNoteDrafts((m) => ({
                                        ...m,
                                        [t.trade_id]: { ...draft, mistake: e.target.value },
                                      }))
                                    }
                                    className="px-1 py-0.5 rounded text-[10.5px] outline-none"
                                    style={{
                                      backgroundColor: 'var(--tv-bg-input)',
                                      color: draft.mistake !== 'None' ? '#F23645' : 'var(--tv-text-primary)',
                                      border: '1px solid var(--tv-border)',
                                    }}
                                  >
                                    {MISTAKE_TAGS.map((m) => (
                                      <option key={m.value} value={m.value}>{m.label}</option>
                                    ))}
                                  </select>
                                </div>
                              </td>
                              <td>
                                <div className="flex flex-wrap gap-0.5 max-w-[140px]">
                                  {CHECKLIST_KEYS.map((ck) => {
                                    const active = Boolean(draft.checklist[ck.key])
                                    return (
                                      <button
                                        key={ck.key}
                                        type="button"
                                        title={ck.title}
                                        onClick={() =>
                                          setNoteDrafts((m) => ({
                                            ...m,
                                            [t.trade_id]: {
                                              ...draft,
                                              checklist: { ...draft.checklist, [ck.key]: !active },
                                            },
                                          }))
                                        }
                                        className="px-1 py-0 rounded text-[9.5px] font-semibold border cursor-pointer"
                                        style={{
                                          backgroundColor: active ? 'rgba(8, 153, 129, 0.22)' : 'transparent',
                                          borderColor: active ? '#089981' : 'var(--tv-border)',
                                          color: active ? '#089981' : 'var(--tv-text-secondary)',
                                        }}
                                      >
                                        {ck.short}
                                      </button>
                                    )
                                  })}
                                </div>
                              </td>
                              <td>
                                <div className="flex items-center gap-1">
                                  <input
                                    type="text"
                                    value={draft.note}
                                    placeholder="Ghi chú bài học..."
                                    onChange={(e) =>
                                      setNoteDrafts((m) => ({
                                        ...m,
                                        [t.trade_id]: { ...draft, note: e.target.value },
                                      }))
                                    }
                                    className="px-1.5 py-0.5 rounded text-[11px] w-28 outline-none"
                                    style={{
                                      backgroundColor: 'var(--tv-bg-input)',
                                      color: 'var(--tv-text-primary)',
                                      border: '1px solid var(--tv-border)',
                                    }}
                                  />
                                  <button
                                    type="button"
                                    className="px-1.5 py-0.5 rounded text-[10px] bg-[#2962FF] text-white border-0 cursor-pointer"
                                    onClick={() => void saveJournalNote(t.trade_id)}
                                  >
                                    Lưu
                                  </button>
                                </div>
                              </td>
                            </tr>
                          )
                        })}
                        {/* Show realized partial-close FIFO fills immediately even while position remains open (`BUG-026`)! */}
                        {broker?.openPosition &&
                          partialFills.map((f) => {
                            const pnl = f.realized_net_pnl ?? 0
                            const pseudoId = 100_000 + f.fill_id
                            const draft = noteDrafts[pseudoId] ?? {
                              setup: 'Untagged',
                              mistake: 'None',
                              note: '',
                              checklist: DEFAULT_CHECKLIST,
                            }
                            return (
                              <tr key={`partial_${f.fill_id}`}>
                                <td>
                                  <span className="px-1 py-0.5 rounded text-[9.5px] font-semibold bg-[#2962FF]/20 text-[#2962FF]">
                                    Chốt phần #{f.fill_id}
                                  </span>
                                </td>
                                <td>
                                  @ {f.price.toLocaleString('en-US')} ({fmtTs(f.timestamp)})
                                </td>
                                <td style={{ color: pnl >= 0 ? 'var(--tv-up)' : 'var(--tv-down)', fontWeight: 600 }}>
                                  {fmtMoney(pnl, isVnd)}
                                </td>
                                <td>—</td>
                                <td>—</td>
                                <td>—</td>
                                <td>
                                  <div className="flex items-center gap-1">
                                    <input
                                      type="text"
                                      value={draft.note}
                                      placeholder="Ghi chú chốt phần..."
                                      onChange={(e) =>
                                        setNoteDrafts((m) => ({
                                          ...m,
                                          [pseudoId]: { ...draft, note: e.target.value },
                                        }))
                                      }
                                      className="px-1.5 py-0.5 rounded text-[11px] w-28 outline-none"
                                      style={{
                                        backgroundColor: 'var(--tv-bg-input)',
                                        color: 'var(--tv-text-primary)',
                                        border: '1px solid var(--tv-border)',
                                      }}
                                    />
                                    <button
                                      type="button"
                                      className="px-1.5 py-0.5 rounded text-[10px] bg-[#2962FF] text-white border-0 cursor-pointer"
                                      onClick={() => void saveJournalNote(pseudoId)}
                                    >
                                      Lưu
                                    </button>
                                  </div>
                                </td>
                              </tr>
                            )
                          })}
                      </>
                    )}
                  </tbody>
                </table>
              </div>
            )
          })()}
        </div>
      </div>
    </div>
  )
}
