import React, { useEffect, useRef, useState } from 'react'
import { AlertTriangle, Calculator, Send, X } from 'lucide-react'
import * as api from '../../api/tauri'
import { useWorkspaceStore } from '../../store/workspaceStore'
import type { TradePlanResponseDto } from '../../types/market'

function fmtMoney(v: number, isVnd: boolean): string {
  if (!Number.isFinite(v)) return '—'
  return isVnd
    ? `${Math.round(v).toLocaleString('en-US')} ₫`
    : `$${v.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`
}

export const TradePlannerModal: React.FC = () => {
  const open = useWorkspaceStore((s) => s.tradePlannerOpen)
  const draft = useWorkspaceStore((s) => s.tradePlannerDraft)
  const closeTradePlanner = useWorkspaceStore((s) => s.closeTradePlanner)
  const panes = useWorkspaceStore((s) => s.panes)
  const activeSlotId = useWorkspaceStore((s) => s.activeSlotId)
  const broker = useWorkspaceStore((s) => s.broker)
  const replay = useWorkspaceStore((s) => s.replay)
  const yumekoAsOf = useWorkspaceStore((s) => s.yumekoAsOf)
  const setBrokerSnapshot = useWorkspaceStore((s) => s.setBrokerSnapshot)
  const pushToast = useWorkspaceStore((s) => s.pushToast)

  const activePane = panes.find((p) => p.slotId === activeSlotId) ?? panes[0]
  const dialogRef = useRef<HTMLDialogElement | null>(null)

  const ticker = draft?.ticker || activePane.ticker
  const [side, setSide] = useState<'LONG' | 'SHORT'>('LONG')
  const [equity, setEquity] = useState('500000000')
  const [availableCash, setAvailableCash] = useState('500000000')
  const [riskPct, setRiskPct] = useState('1.5')
  const [maxPosPct, setMaxPosPct] = useState('35')
  const [entryPrice, setEntryPrice] = useState('')
  const [stopLoss, setStopLoss] = useState('')
  const [targetPrice, setTargetPrice] = useState('')
  const [plan, setPlan] = useState<TradePlanResponseDto | null>(null)
  const [errorMsg, setErrorMsg] = useState<string | null>(null)
  const [submitting, setSubmitting] = useState(false)

  useEffect(() => {
    const dlg = dialogRef.current
    if (!dlg) return
    if (open) {
      if (!dlg.open) dlg.showModal()
      const baseEq = broker?.metrics.equity && broker.metrics.equity > 0 ? broker.metrics.equity : 500_000_000
      const baseCash = broker?.metrics.cash_balance && broker.metrics.cash_balance > 0 ? broker.metrics.cash_balance : baseEq
      const mark = broker?.markPrice && broker.markPrice > 0 ? broker.markPrice : 100_000
      setSide(draft?.side ?? 'LONG')
      setEquity(String(Math.round(baseEq)))
      setAvailableCash(String(Math.round(baseCash)))
      const ent = draft?.entryPrice ?? mark
      const fmtDefaultLevel = (v: number) => (ent < 1_000 ? Number(v.toFixed(2)) : Math.round(v))
      setEntryPrice(String(ent))
      setStopLoss(String(draft?.stopLoss ?? fmtDefaultLevel(ent * (draft?.side === 'SHORT' ? 1.05 : 0.94))))
      setTargetPrice(String(draft?.targetPrice ?? fmtDefaultLevel(ent * (draft?.side === 'SHORT' ? 0.90 : 1.15))))
    } else if (dlg.open) {
      dlg.close()
    }
  }, [open, draft, broker?.metrics.equity, broker?.metrics.cash_balance, broker?.markPrice])

  useEffect(() => {
    const dlg = dialogRef.current
    if (!dlg) return
    const onClose = () => closeTradePlanner()
    dlg.addEventListener('close', onClose)
    return () => dlg.removeEventListener('close', onClose)
  }, [closeTradePlanner])

  // Recompute plan via `sumi-core::journal::compute_trade_plan` whenever inputs change
  useEffect(() => {
    if (!open) return
    const eq = Number(equity)
    const cash = Number(availableCash)
    const rPct = Number(riskPct)
    const mpPct = Number(maxPosPct)
    const ent = Number(entryPrice)
    const sl = Number(stopLoss)
    const tp = Number(targetPrice)
    if (![eq, cash, rPct, ent, sl, tp].every((v) => Number.isFinite(v) && v > 0)) {
      setPlan(null)
      return
    }
    const effectiveEntryTimestampMs =
      replay?.active && replay.cutoffTimestampMs
        ? replay.blindDrill?.enabled && !replay.blindDrill.revealed
          ? null
          : replay.cutoffTimestampMs
        : yumekoAsOf
          ? Date.parse(`${yumekoAsOf}T07:00:00Z`)
          : null

    let cancelled = false
    const timer = window.setTimeout(() => {
      void api
        .computeTradePlan({
          ticker,
          side,
          accountEquity: eq,
          availableCash: cash,
          riskPct: rPct,
          maxPositionPct: Number.isFinite(mpPct) && mpPct > 0 ? mpPct : 100,
          entryPrice: ent,
          stopLoss: sl,
          targetPrice: tp,
          referencePrice: broker?.vnBand?.reference ?? null,
          entryTimestampMs:
            effectiveEntryTimestampMs !== null && Number.isFinite(effectiveEntryTimestampMs)
              ? effectiveEntryTimestampMs
              : null,
        })
        .then((res) => {
          if (!cancelled) {
            setPlan(res)
            setErrorMsg(null)
          }
        })
        .catch((err) => {
          if (!cancelled) {
            setPlan(null)
            setErrorMsg(String(err))
          }
        })
    }, 120)
    return () => {
      cancelled = true
      window.clearTimeout(timer)
    }
  }, [
    open,
    ticker,
    side,
    equity,
    availableCash,
    riskPct,
    maxPosPct,
    entryPrice,
    stopLoss,
    targetPrice,
    broker?.vnBand?.reference,
    replay?.active,
    replay?.cutoffTimestampMs,
    replay?.blindDrill?.enabled,
    replay?.blindDrill?.revealed,
    yumekoAsOf,
  ])

  const isVnd = ['HOSE', 'HNX', 'UPCOM', 'VN30F'].includes((plan?.venue ?? '').toUpperCase()) ||
    ticker.startsWith('HOSE:') || ticker.startsWith('HNX:') || ticker.startsWith('UPCOM:')

  const handleSendToPaperBroker = async () => {
    if (!plan || plan.plannedQuantity <= 0) return
    setSubmitting(true)
    try {
      const updated = await api.brokerSubmitOrder({
        ticker,
        interval: activePane.interval,
        side: plan.side === 'SHORT' ? 'SELL' : 'BUY',
        vnOrderType: 'LO',
        quantity: plan.plannedQuantity,
        price: plan.snappedEntry,
        stopLoss: plan.snappedStop,
        takeProfit: plan.snappedTarget,
      })
      setBrokerSnapshot(updated)
      pushToast(
        'success',
        `Đã gửi lệnh ${plan.side === 'SHORT' ? 'BÁN' : 'MUA'} LO ${plan.plannedQuantity.toLocaleString('en-US')} ${ticker} @ ${plan.snappedEntry.toLocaleString('en-US')} (SL ${plan.snappedStop.toLocaleString('en-US')} / TP ${plan.snappedTarget.toLocaleString('en-US')})`,
      )
      closeTradePlanner()
    } catch (err) {
      pushToast('error', String(err))
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <dialog
      ref={dialogRef}
      className="tv-dialog"
      style={{ maxWidth: 'min(94vw, 640px)' }}
      aria-labelledby="trade-planner-title"
      {...({ closedby: 'any' } as Record<string, string>)}
    >
      <div
        className="flex items-center justify-between px-4 py-3 shrink-0"
        style={{ borderBottom: '1px solid var(--tv-border)' }}
      >
        <div className="flex items-center gap-2">
          <Calculator className="w-4 h-4 text-[#2962FF]" />
          <div>
            <h2 id="trade-planner-title" className="m-0 text-[14.5px] font-semibold">
              Kế Hoạch Đi Vốn & Quản Trị Rủi Ro — {ticker}
            </h2>
            <div className="text-[11px]" style={{ color: 'var(--tv-text-secondary)' }}>
              Tự động làm tròn bước giá, lô chẵn 100, trừ phí/thuế/trượt giá và tính Net R:R chuẩn sàn Việt Nam
            </div>
          </div>
        </div>
        <button type="button" className="tv-icon-btn w-7 h-7" onClick={closeTradePlanner} aria-label="Đóng Trade Planner">
          <X className="w-4 h-4" />
        </button>
      </div>

      <div className="p-4 space-y-3 overflow-y-auto tv-scroll max-h-[75vh]">
        {/* Direction & Capital Inputs */}
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 text-[11.5px]">
          <label className="space-y-1">
            <span style={{ color: 'var(--tv-text-secondary)' }}>Chiều vị thế</span>
            <select
              value={side}
              onChange={(e) => setSide(e.target.value as 'LONG' | 'SHORT')}
              className="w-full px-2 py-1 rounded outline-none"
              style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
            >
              <option value="LONG">LONG (Mua)</option>
              <option value="SHORT">SHORT (Bán khống / PS)</option>
            </select>
          </label>
          <label className="space-y-1">
            <span style={{ color: 'var(--tv-text-secondary)' }}>Vốn NAV ({isVnd ? 'VND' : 'USD'})</span>
            <input
              type="number"
              value={equity}
              onChange={(e) => setEquity(e.target.value)}
              className="w-full px-2 py-1 rounded tabular-nums outline-none no-spinner"
              style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
            />
          </label>
          <label className="space-y-1">
            <span style={{ color: 'var(--tv-text-secondary)' }}>Rủi ro tối đa (%)</span>
            <input
              type="number"
              step="0.25"
              value={riskPct}
              onChange={(e) => setRiskPct(e.target.value)}
              className="w-full px-2 py-1 rounded tabular-nums outline-none no-spinner"
              style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
            />
          </label>
          <label className="space-y-1">
            <span style={{ color: 'var(--tv-text-secondary)' }}>Trần tỷ trọng (%)</span>
            <input
              type="number"
              step="5"
              value={maxPosPct}
              onChange={(e) => setMaxPosPct(e.target.value)}
              className="w-full px-2 py-1 rounded tabular-nums outline-none no-spinner"
              style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
            />
          </label>
        </div>

        {/* Entry / Stop Loss / Take Profit */}
        <div className="grid grid-cols-3 gap-2 text-[11.5px]">
          <label className="space-y-1">
            <span className="font-semibold text-[#2962FF]">Giá vào (Entry)</span>
            <input
              type="number"
              step="any"
              value={entryPrice}
              onChange={(e) => setEntryPrice(e.target.value)}
              className="w-full px-2 py-1.5 rounded font-semibold tabular-nums outline-none no-spinner"
              style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid #2962FF' }}
            />
          </label>
          <label className="space-y-1">
            <span className="font-semibold text-[#F23645]">Điểm cắt lỗ (SL)</span>
            <input
              type="number"
              step="any"
              value={stopLoss}
              onChange={(e) => setStopLoss(e.target.value)}
              className="w-full px-2 py-1.5 rounded font-semibold tabular-nums outline-none no-spinner"
              style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid #F23645' }}
            />
          </label>
          <label className="space-y-1">
            <span className="font-semibold text-[#089981]">Mục tiêu chốt lời (TP)</span>
            <input
              type="number"
              step="any"
              value={targetPrice}
              onChange={(e) => setTargetPrice(e.target.value)}
              className="w-full px-2 py-1.5 rounded font-semibold tabular-nums outline-none no-spinner"
              style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid #089981' }}
            />
          </label>
        </div>

        {errorMsg && (
          <div className="p-2.5 rounded text-[11.5px] text-[#F23645]" style={{ backgroundColor: 'rgba(242, 54, 69, 0.12)', border: '1px solid rgba(242, 54, 69, 0.4)' }}>
            {errorMsg}
          </div>
        )}

        {plan && (
          <>
            {/* Key Sizing Metrics */}
            <div
              className="grid grid-cols-2 sm:grid-cols-4 gap-2 p-3 rounded tabular-nums"
              style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}
            >
              <div>
                <div className="text-[10.5px]" style={{ color: 'var(--tv-text-secondary)' }}>Khối lượng chuẩn ({plan.venue})</div>
                <div className="text-[16px] font-bold text-[#2962FF]">
                  {plan.plannedQuantity.toLocaleString('en-US')}
                </div>
              </div>
              <div>
                <div className="text-[10.5px]" style={{ color: 'var(--tv-text-secondary)' }}>Net R:R (sau phí/thuế)</div>
                <div
                  className="text-[16px] font-bold"
                  style={{ color: plan.netRMultiple >= 2 ? 'var(--tv-up)' : plan.netRMultiple >= 1.2 ? '#FBC02D' : 'var(--tv-down)' }}
                >
                  {plan.netRMultiple.toFixed(2)}R
                </div>
                <div className="text-[9.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
                  Gross {plan.grossRMultiple.toFixed(2)}R · Min Win {plan.requiredWinRatePct.toFixed(0)}%
                </div>
              </div>
              <div>
                <div className="text-[10.5px]" style={{ color: 'var(--tv-text-secondary)' }}>Rủi ro thực tế</div>
                <div className="text-[13.5px] font-bold text-[#F23645]">
                  -{fmtMoney(plan.plannedRiskAmount, isVnd)}
                </div>
                <div className="text-[9.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
                  {plan.plannedRiskPctActual.toFixed(2)}% NAV
                </div>
              </div>
              <div>
                <div className="text-[10.5px]" style={{ color: 'var(--tv-text-secondary)' }}>Lợi nhuận ròng dự kiến</div>
                <div className="text-[13.5px] font-bold text-[#089981]">
                  +{fmtMoney(plan.estimatedNetProfit, isVnd)}
                </div>
                <div className="text-[9.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
                  Phí/Thuế: {fmtMoney(plan.estimatedRoundTripFriction, isVnd)}
                </div>
              </div>
            </div>

            {/* Tick-snapped prices & Notional detail */}
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 text-[11px] tabular-nums px-1" style={{ color: 'var(--tv-text-secondary)' }}>
              <div>Entry làm tròn: <strong style={{ color: 'var(--tv-text-primary)' }}>{plan.snappedEntry.toLocaleString('en-US')}</strong></div>
              <div>SL làm tròn: <strong style={{ color: 'var(--tv-down)' }}>{plan.snappedStop.toLocaleString('en-US')}</strong></div>
              <div>Hòa vốn (BE): <strong style={{ color: 'var(--tv-text-primary)' }}>{plan.breakevenPrice.toLocaleString('en-US')}</strong></div>
              <div>Giá trị giải ngân: <strong style={{ color: 'var(--tv-text-primary)' }}>{fmtMoney(plan.positionNotional, isVnd)}</strong></div>
            </div>

            {plan.warnings.length > 0 && (
              <div className="p-2.5 rounded space-y-1 text-[11px]" style={{ backgroundColor: 'rgba(251, 192, 45, 0.12)', border: '1px solid rgba(251, 192, 45, 0.45)', color: '#FBC02D' }}>
                {plan.warnings.map((w, i) => (
                  <div key={i} className="flex items-start gap-1.5">
                    <AlertTriangle className="w-3.5 h-3.5 shrink-0 mt-0.5" />
                    <span>{w}</span>
                  </div>
                ))}
              </div>
            )}
          </>
        )}
      </div>

      <div
        className="flex items-center justify-end gap-2 px-4 py-2.5 shrink-0"
        style={{ borderTop: '1px solid var(--tv-border)' }}
      >
        <button type="button" className="tv-btn px-3 py-1.5 text-[12px]" onClick={closeTradePlanner}>
          Đóng
        </button>
        <button
          type="button"
          disabled={!plan || plan.plannedQuantity <= 0 || submitting}
          onClick={() => void handleSendToPaperBroker()}
          className="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded text-[12px] font-semibold bg-[#2962FF] hover:bg-[#1E53E5] text-white border-0 cursor-pointer disabled:opacity-50"
        >
          <Send className="w-3.5 h-3.5" />
          <span>{submitting ? 'Đang gửi...' : 'Gửi lệnh LO + SL/TP vào Paper Broker'}</span>
        </button>
      </div>
    </dialog>
  )
}
