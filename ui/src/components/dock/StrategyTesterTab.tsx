import React, { useEffect, useMemo, useState } from 'react'
import {
  AlertTriangle,
  BarChart3,
  Briefcase,
  Calendar,
  Dice5,
  Eye,
  EyeOff,
  Play,
  Plus,
  Scissors,
  ShieldCheck,
  SlidersHorizontal,
  Trash2,
  TrendingUp,
} from 'lucide-react'
import * as api from '../../api/tauri'
import { useWorkspaceStore } from '../../store/workspaceStore'
import type {
  CustomClauseInput,
  MonteCarloResponse,
  PortfolioBacktestResponse,
  SignalCatalogItemDto,
  StrategyBacktestInput,
  StrategyBacktestResponse,
  StrategyEquityPointDto,
  StrategyPresetSlug,
  WalkForwardResponse,
} from '../../types/market'

type TesterView = 'overview' | 'trades' | 'portfolio' | 'wfo' | 'mc'

const QMV_PRESETS: Array<{ key: StrategyPresetSlug; label: string }> = [
  { key: 'qmv_s1', label: '★ QMV S1 — Hỗ Trợ Cứng + Cạn Cung & Nổ Vol (KT01)' },
  { key: 'qmv_s2', label: '★ QMV S2 — Bắt Đáy An Toàn & Phân Kỳ Đáy (DT1/DT2)' },
  { key: 'qmv_s3', label: '★ QMV S3 — Xu Hướng Mây Ichimoku V7 (6-18-36)' },
  { key: 'qmv_s4', label: '★ QMV S4 — Đồng Pha Chu Kỳ Dòng Tiền T+ & Sóng (Proxy)' },
  { key: 'qmv_s5', label: '★ QMV S5 — Vượt Vùng Tắc Nghẽn & Đỉnh 50 Phiên' },
  { key: 'qmv_s6', label: '★ QMV S6 — Hợp Lưu Đa Trụ Cột QMV (Multi-Gate)' },
  { key: 'custom', label: '✦ Tự thiết kế Quy tắc Tín hiệu QMV' },
]

const BASELINE_PRESETS: Array<{ key: StrategyPresetSlug; label: string }> = [
  { key: 'sma_ema', label: 'Tham chiếu: SMA/EMA Trend Cross' },
  { key: 'rsi_mean_rev', label: 'Tham chiếu: RSI Mean Reversion' },
  { key: 'macd', label: 'Tham chiếu: MACD Momentum' },
  { key: 'bb_squeeze', label: 'Tham chiếu: Bollinger Squeeze Breakout' },
  { key: 'donchian_atr', label: 'Tham chiếu: Donchian ATR Breakout' },
  { key: 'rsi_bb', label: 'Tham chiếu: RSI + BB Confluence' },
]

const QMV_PORTFOLIO_GROUPS: Array<{ value: string; label: string }> = [
  { value: 'QMV:QMV30', label: '★ Rổ QMV30 — 30 Cổ phiếu Trụ cột' },
  { value: 'QMV:QMV60', label: '★ Rổ QMV60 — 60 Cổ phiếu Trọng điểm' },
  { value: 'QMV:CORE10', label: '★ Danh mục Yumeko Core 10' },
  { value: 'QMV:DT1', label: 'Dòng tiền Loại 1 (DT1 · Tạo lập / Large-cap)' },
  { value: 'QMV:DT2', label: 'Dòng tiền Loại 2 (DT2 · Mid-cap)' },
  { value: 'QMV:DT3', label: 'Dòng tiền Loại 3 (DT3 · Small-cap / Đầu cơ)' },
  { value: 'QMV:GROUP:GROUP_1', label: 'Nhóm ngành 1: Tài chính & Đòn bẩy' },
  { value: 'QMV:GROUP:GROUP_2', label: 'Nhóm ngành 2: Chu kỳ & Tăng trưởng' },
  { value: 'QMV:GROUP:GROUP_3', label: 'Nhóm ngành 3: Phòng thủ & Tiện ích' },
  { value: 'QMV:SECTOR:BANKING', label: 'Ngành: Ngân hàng' },
  { value: 'QMV:SECTOR:BROKERAGE', label: 'Ngành: Chứng khoán' },
  { value: 'QMV:SECTOR:REAL_ESTATE', label: 'Ngành: BĐS Dân cư' },
  { value: 'QMV:SECTOR:CONSTRUCTION_STEEL', label: 'Ngành: XD & VLXD / Thép' },
  { value: 'QMV:SECTOR:RETAIL', label: 'Ngành: Bán lẻ' },
  { value: 'QMV:SECTOR:TECH_TELECOM', label: 'Ngành: Công nghệ & Viễn thông' },
  { value: 'QMV:SECTOR:INDUSTRIAL_LOGISTICS', label: 'Ngành: KCN & Cảng / Logistics' },
  { value: 'QMV:SECTOR:OIL_GAS_CHEM', label: 'Ngành: Dầu khí & Hóa chất' },
  { value: 'QMV:SECTOR:CONSUMER_FNB', label: 'Ngành: Thực phẩm & Thủy sản (F&B)' },
  { value: 'QMV:SECTOR:UTILITIES_DEFENSIVE', label: 'Ngành: Điện / Nước / Bảo hiểm / Dược' },
]

function EquityAndDrawdownChart({ points }: { points: StrategyEquityPointDto[] }) {
  if (points.length < 2) {
    return (
      <div className="flex items-center justify-center h-full text-[11.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
        Nhấn "Backtest" để kiểm định nhân quả (HOSE ±7%, lô 100, T+2/T+3, phí 0.15% + thuế bán 0.10%) trên dữ liệu thật
      </div>
    )
  }

  const w = 720
  const eqH = 92
  const ddH = 42
  const padX = 8

  const eqs = points.map((p) => p.equity)
  const minEq = Math.min(...eqs)
  const maxEq = Math.max(...eqs)
  const eqSpan = Math.max(1e-6, maxEq - minEq)
  const minDd = Math.min(...points.map((p) => p.underwaterPct), -1)

  const toX = (i: number) => padX + (i / (points.length - 1)) * (w - padX * 2)
  const toEqY = (v: number) => 6 + (1 - (v - minEq) / eqSpan) * (eqH - 12)
  const toDdY = (pct: number) => eqH + 8 + (Math.min(0, pct) / minDd) * (ddH - 6)

  const eqLine = points.map((p, i) => `${i === 0 ? 'M' : 'L'}${toX(i).toFixed(1)},${toEqY(p.equity).toFixed(1)}`).join(' ')
  const eqArea = `${eqLine} L${toX(points.length - 1).toFixed(1)},${eqH} L${toX(0).toFixed(1)},${eqH} Z`

  const ddLine = points.map((p, i) => `${i === 0 ? 'M' : 'L'}${toX(i).toFixed(1)},${toDdY(p.underwaterPct).toFixed(1)}`).join(' ')
  const ddArea = `M${toX(0).toFixed(1)},${eqH + 8} ${ddLine.slice(1)} L${toX(points.length - 1).toFixed(1)},${eqH + 8} Z`

  const profitable = points[points.length - 1].equity >= points[0].equity
  const strokeColor = profitable ? '#089981' : '#F23645'

  return (
    <svg viewBox={`0 0 ${w} ${eqH + ddH + 12}`} className="w-full h-full max-h-[146px]" preserveAspectRatio="none">
      <defs>
        <linearGradient id="sumiEqGrad" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor={strokeColor} stopOpacity="0.28" />
          <stop offset="100%" stopColor={strokeColor} stopOpacity="0.02" />
        </linearGradient>
      </defs>
      <path d={eqArea} fill="url(#sumiEqGrad)" />
      <path d={eqLine} fill="none" stroke={strokeColor} strokeWidth="1.8" />
      <line x1={padX} y1={eqH + 8} x2={w - padX} y2={eqH + 8} stroke="var(--tv-border)" strokeWidth="1" />
      <path d={ddArea} fill="rgba(242, 54, 69, 0.28)" />
      <path d={ddLine} fill="none" stroke="#F23645" strokeWidth="1.2" />
    </svg>
  )
}

export const StrategyTesterTab: React.FC = () => {
  const panes = useWorkspaceStore((s) => s.panes)
  const activeSlotId = useWorkspaceStore((s) => s.activeSlotId)
  const watchlists = useWorkspaceStore((s) => s.watchlists)
  const setActiveTicker = useWorkspaceStore((s) => s.setActiveTicker)
  const replay = useWorkspaceStore((s) => s.replay)
  const backtestOverlay = useWorkspaceStore((s) => s.backtestOverlay)
  const scrollActiveChartToTimestamp = useWorkspaceStore((s) => s.scrollActiveChartToTimestamp)
  const setBacktestOverlay = useWorkspaceStore((s) => s.setBacktestOverlay)
  const toggleBacktestOverlayVisible = useWorkspaceStore((s) => s.toggleBacktestOverlayVisible)
  const pushToast = useWorkspaceStore((s) => s.pushToast)

  const activePane = panes.find((p) => p.slotId === activeSlotId) ?? panes[0]

  const [preset, setPreset] = useState<StrategyPresetSlug>('qmv_s1')
  const [direction, setDirection] = useState<'long' | 'short' | 'both'>('long')
  const [signalCatalog, setSignalCatalog] = useState<SignalCatalogItemDto[]>([])

  // QMV Strategy Playbook Parameters (`S1..S6` + QMV Risk Rules)
  const [qmvSupportWindow, setQmvSupportWindow] = useState('20')
  const [qmvRvolConfirm, setQmvRvolConfirm] = useState('1.3')
  const [qmvTrailMode, setQmvTrailMode] = useState('0')
  const [qmvDivOsc, setQmvDivOsc] = useState('3')
  const [qmvConfirmMode, setQmvConfirmMode] = useState('0')
  const [qmvIchiVariant, setQmvIchiVariant] = useState('0')
  const [qmvFlowHorizonPair, setQmvFlowHorizonPair] = useState('0')
  const [qmvFlowLowZone, setQmvFlowLowZone] = useState('35')
  const [qmvFlowHighZone, setQmvFlowHighZone] = useState('75')
  const [qmvMinPillars, setQmvMinPillars] = useState('3')
  const [qmvUseVnindexGate, setQmvUseVnindexGate] = useState(true)
  const [useSupportStop, setUseSupportStop] = useState(true)
  const [qmvWaveTrailing23, setQmvWaveTrailing23] = useState(true)

  // Custom QMV Signal Rule Builder state
  const [customRuleName, setCustomRuleName] = useState('QMV Vượt Mây V7 + Dòng Tiền T+')
  const [entryCombinator, setEntryCombinator] = useState<'and' | 'or'>('and')
  const [entryClauses, setEntryClauses] = useState<CustomClauseInput[]>([
    { catalogId: 'qmv.ichimoku_v7_bullish', boolExpected: true },
    { catalogId: 'qmv.flow_turn_up_t_plus', boolExpected: true },
  ])
  const [exitClauseEnabled, setExitClauseEnabled] = useState(true)
  const [exitClause, setExitClause] = useState<CustomClauseInput>({
    catalogId: 'qmv.flow_turn_down_70_80',
    boolExpected: true,
  })
  const [minHealthScore, setMinHealthScore] = useState('')

  // Portfolio Multi-Symbol Backtest state (supports QMV Groups & Watchlists)
  const [portfolioTarget, setPortfolioTarget] = useState<string>('QMV:QMV30')
  const [portfolioWeighting, setPortfolioWeighting] = useState<'equal' | 'inverse_vol'>('equal')
  const [portfolioResult, setPortfolioResult] = useState<PortfolioBacktestResponse | null>(null)

  useEffect(() => {
    void api.getSignalCatalog().then(setSignalCatalog).catch(() => {})
  }, [])

  const qmvCatalogSignals = useMemo(
    () => signalCatalog.filter((s) => s.isQmvCore !== false),
    [signalCatalog],
  )

  const isFloatSignal = (catalogId: string): boolean => {
    const found = signalCatalog.find((c) => c.id === catalogId)
    return found ? found.outputType.toLowerCase() === 'float' : catalogId.includes('relative_volume')
  }

  // Baseline strategy parameters
  const [fastSma, setFastSma] = useState('20')
  const [slowEma, setSlowEma] = useState('50')
  const [rsiPeriod, setRsiPeriod] = useState('14')
  const [rsiOversold, setRsiOversold] = useState('30')
  const [rsiOverbought, setRsiOverbought] = useState('70')
  const [macdFast, setMacdFast] = useState('12')
  const [macdSlow, setMacdSlow] = useState('26')
  const [macdSignal, setMacdSignal] = useState('9')
  const [macdTrendEma, setMacdTrendEma] = useState('200')
  const [bbPeriod, setBbPeriod] = useState('20')
  const [bbStdMult, setBbStdMult] = useState('2.0')
  const [maxSqueezeBwPct, setMaxSqueezeBwPct] = useState('10')
  const [donchianEntry, setDonchianEntry] = useState('20')
  const [donchianExit, setDonchianExit] = useState('10')

  const [fromDate, setFromDate] = useState('')
  const [toDate, setToDate] = useState('')

  const [stopLossPct, setStopLossPct] = useState('8.0')
  const [takeProfitPct, setTakeProfitPct] = useState('')
  const [initialCash, setInitialCash] = useState('500000000')
  const [minHold, setMinHold] = useState('2')

  const [view, setView] = useState<TesterView>('overview')
  const [running, setRunning] = useState<'bt' | 'wfo' | 'mc' | 'pf' | null>(null)
  const [btResult, setBtResult] = useState<StrategyBacktestResponse | null>(null)
  const [wfoResult, setWfoResult] = useState<WalkForwardResponse | null>(null)
  const [mcResult, setMcResult] = useState<MonteCarloResponse | null>(null)

  const upperTicker = activePane.ticker.toUpperCase()
  const isVn30f = upperTicker.includes('VN30F') || upperTicker.includes('VN301!')
  const isVnStock =
    !isVn30f &&
    (upperTicker.startsWith('HOSE:') || upperTicker.startsWith('HNX:') || upperTicker.startsWith('UPCOM:'))
  const isVn = isVnStock || isVn30f
  const isQmvPreset = preset.startsWith('qmv_')

  useEffect(() => {
    if (isVn30f) {
      setDirection('both')
      setMinHold('0')
      setInitialCash('500000000')
    } else if (isVnStock) {
      setDirection('long')
      setMinHold('2')
      setInitialCash('500000000')
    } else {
      setDirection('both')
      setMinHold('0')
      setInitialCash('100000')
    }
  }, [isVn30f, isVnStock, activePane.ticker])

  const parseDateStartMs = (val: string): number | null => {
    if (!val) return null
    const ms = Date.parse(`${val}T00:00:00Z`)
    return Number.isFinite(ms) ? ms : null
  }
  const parseDateEndMs = (val: string): number | null => {
    if (!val) return null
    const ms = Date.parse(`${val}T23:59:59.999Z`)
    return Number.isFinite(ms) ? ms : null
  }

  const handleUseReplayRange = () => {
    if (!replay?.active || !replay.cutoffTimestampMs) return
    const endDate = new Date(replay.cutoffTimestampMs)
    const startDate = new Date(replay.cutoffTimestampMs - 365 * 86_400_000)
    setFromDate(startDate.toISOString().slice(0, 10))
    setToDate(endDate.toISOString().slice(0, 10))
    pushToast('info', `Đã lấy khoảng thời gian Backtest theo mốc Replay đến ${endDate.toLocaleDateString('vi-VN')}`)
  }

  const buildInput = (): StrategyBacktestInput => {
    const parsedMinHealth =
      minHealthScore.trim() !== '' && Number.isFinite(Number(minHealthScore))
        ? Number(minHealthScore)
        : null
    const supWin = Number(qmvSupportWindow) || 20
    return {
      ticker: activePane.ticker,
      interval: activePane.interval,
      preset,
      direction,
      initialCash: Number(initialCash) || (isVn ? 500_000_000 : 100_000),
      commissionBps: isVn30f ? 2 : isVnStock ? 15 : 5,
      taxSellBps: isVnStock ? 10 : 0,
      slippageBps: isVn30f ? 1 : 5,
      lotStep: isVn30f ? 1 : isVnStock ? 100 : 0,
      positionSizePct: 95,
      stopLossPct: Number(stopLossPct) > 0 ? Number(stopLossPct) : null,
      takeProfitPct: Number(takeProfitPct) > 0 ? Number(takeProfitPct) : null,
      minHoldingBars: Number(minHold) || 0,
      fastSma: Number(fastSma) || 20,
      slowEma: Number(slowEma) || 50,
      rsiPeriod: Number(rsiPeriod) || 14,
      rsiOversold: Number(rsiOversold) || 30,
      rsiOverbought: Number(rsiOverbought) || 70,
      rsiConfluenceBuy: Number(rsiOversold) || 35,
      rsiConfluenceSell: Number(rsiOverbought) || 65,
      macdFast: Number(macdFast) || 12,
      macdSlow: Number(macdSlow) || 26,
      macdSignal: Number(macdSignal) || 9,
      macdTrendEma: Number(macdTrendEma) || 200,
      bbPeriod: Number(bbPeriod) || 20,
      bbStdMult: Number(bbStdMult) || 2.0,
      maxSqueezeBwPct: Number(maxSqueezeBwPct) || 10,
      donchianEntry: Number(donchianEntry) || 20,
      donchianExit: Number(donchianExit) || 10,
      inSampleSplitPct: 0.7,
      maxBars: 2500,
      fromTimestampMs: parseDateStartMs(fromDate),
      toTimestampMs: parseDateEndMs(toDate),
      maxLeverage: isVn30f ? 5.88 : 1.0,
      qmvSupportWindow: supWin,
      qmvRvolConfirm: Number(qmvRvolConfirm) || 1.3,
      qmvTrailMode: Number(qmvTrailMode) || 0,
      qmvDivOsc: Number(qmvDivOsc) || 3,
      qmvConfirmMode: Number(qmvConfirmMode) || 0,
      qmvIchiVariant: Number(qmvIchiVariant) || 0,
      qmvFlowHorizonPair: Number(qmvFlowHorizonPair) || 0,
      qmvFlowLowZone: Number(qmvFlowLowZone) || 35,
      qmvFlowHighZone: Number(qmvFlowHighZone) || 75,
      qmvMinPillars: Number(qmvMinPillars) || 3,
      qmvUseVnindexGate: isQmvPreset ? qmvUseVnindexGate : false,
      supportStopLookback: isQmvPreset && useSupportStop ? supWin : null,
      qmvWaveTrailing23: isQmvPreset ? qmvWaveTrailing23 : false,
      customRules:
        preset === 'custom'
          ? {
              name: customRuleName.trim() || 'Chiến lược QMV tự thiết kế',
              entryCombinator,
              entryClauses,
              exitCombinator: 'or',
              exitClauses: exitClauseEnabled ? [exitClause] : [],
              minHealthScore: parsedMinHealth,
            }
          : null,
    }
  }

  const handleRunBacktest = async () => {
    setRunning('bt')
    try {
      const res = await api.runStrategyBacktest(buildInput())
      setBtResult(res)
      setBacktestOverlay({
        ticker: res.ticker,
        interval: res.interval,
        presetLabel: res.presetLabel,
        trades: res.trades,
        visible: true,
      })
      setView('overview')
    } catch (err) {
      pushToast('error', String(err))
    } finally {
      setRunning(null)
    }
  }

  const handleRunPortfolio = async () => {
    const isQmvGroup = portfolioTarget.startsWith('QMV:')
    const qmvGroup = isQmvGroup ? portfolioTarget.slice(4) : null
    const wl = !isQmvGroup ? watchlists.find((w) => w.id === portfolioTarget) : undefined
    const tickers = isQmvGroup ? [] : (wl?.tickers ?? [activePane.ticker])
    if (!isQmvGroup && tickers.length === 0) {
      pushToast('error', 'Danh mục chưa có mã nào để chạy Portfolio Backtest')
      return
    }
    setRunning('pf')
    try {
      const res = await api.runStrategyPortfolio({
        backtest: buildInput(),
        tickers,
        qmvGroup,
        weighting: portfolioWeighting,
      })
      setPortfolioResult(res)
      setView('portfolio')
      pushToast(
        'success',
        `Hoàn tất Backtest nhóm (${res.constituentCount} mã): ${res.tearsheet.total_return_pct >= 0 ? '+' : ''}${res.tearsheet.total_return_pct.toFixed(2)}%`,
      )
    } catch (err) {
      pushToast('error', String(err))
    } finally {
      setRunning(null)
    }
  }

  const handleRunWfo = async () => {
    setRunning('wfo')
    try {
      const res = await api.runStrategyWfo(buildInput(), 5, 0.7, false)
      setWfoResult(res)
      setView('wfo')
    } catch (err) {
      pushToast('error', String(err))
    } finally {
      setRunning(null)
    }
  }

  const handleRunMc = async () => {
    setRunning('mc')
    try {
      const res = await api.runStrategyMonteCarlo(buildInput(), 2000, 50)
      setMcResult(res)
      setView('mc')
    } catch (err) {
      pushToast('error', String(err))
    } finally {
      setRunning(null)
    }
  }

  const ts = btResult?.tearsheet

  return (
    <div className="flex flex-col h-full min-h-0 overflow-hidden">
      {/* Row 1: QMV Strategy Playbook Selector + Risk & Date Controls + Action Buttons */}
      <div
        className="flex flex-wrap items-center gap-1.5 px-3 py-1.5 shrink-0"
        style={{ borderBottom: '1px solid var(--tv-border)', backgroundColor: 'var(--tv-bg-elevated)' }}
      >
        <select
          aria-label="Playbook Chiến lược QMV"
          value={preset}
          onChange={(e) => setPreset(e.target.value as StrategyPresetSlug)}
          className="px-2 py-1 rounded text-[11.5px] font-semibold outline-none max-w-[310px]"
          style={{
            backgroundColor: 'var(--tv-bg-input)',
            color: 'var(--tv-text-primary)',
            border: '1px solid var(--tv-border)',
          }}
        >
          <optgroup label="6 Họ Chiến Lược QMV Cốt Lõi">
            {QMV_PRESETS.map((p) => (
              <option key={p.key} value={p.key}>
                {p.label}
              </option>
            ))}
          </optgroup>
          <optgroup label="Chiến Lược Định Lượng Tham Chiếu">
            {BASELINE_PRESETS.map((p) => (
              <option key={p.key} value={p.key}>
                {p.label}
              </option>
            ))}
          </optgroup>
        </select>

        {!isVnStock && (
          <select
            aria-label="Chiều chiến lược"
            value={direction}
            onChange={(e) => setDirection(e.target.value as 'long' | 'short' | 'both')}
            className="px-1.5 py-1 rounded text-[11px] outline-none"
            style={{
              backgroundColor: 'var(--tv-bg-input)',
              color: 'var(--tv-text-primary)',
              border: '1px solid var(--tv-border)',
            }}
          >
            <option value="long">Long Only</option>
            <option value="both">Long & Short</option>
            <option value="short">Short Only</option>
          </select>
        )}

        {/* Date Range Filter */}
        <div className="flex items-center gap-1 text-[11px]" style={{ color: 'var(--tv-text-secondary)' }}>
          <Calendar className="w-3 h-3 shrink-0" />
          <input
            type="date"
            aria-label="Từ ngày Backtest"
            value={fromDate}
            onChange={(e) => setFromDate(e.target.value)}
            title="Từ ngày (để trống = toàn bộ lịch sử)"
            className="px-1.5 py-0.5 rounded text-[10.5px] tabular-nums outline-none"
            style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
          />
          <span>→</span>
          <input
            type="date"
            aria-label="Đến ngày Backtest"
            value={toDate}
            onChange={(e) => setToDate(e.target.value)}
            title="Đến ngày (để trống = hiện tại)"
            className="px-1.5 py-0.5 rounded text-[10.5px] tabular-nums outline-none"
            style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
          />
          {replay?.active && (
            <button
              type="button"
              onClick={handleUseReplayRange}
              className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10.5px] border cursor-pointer"
              style={{
                backgroundColor: 'var(--tv-accent-soft)',
                borderColor: 'var(--tv-accent)',
                color: 'var(--tv-accent)',
              }}
              title="Lấy khoảng thời gian theo mốc Bar Replay hiện tại"
            >
              <Scissors className="w-2.5 h-2.5" />
              <span>Theo Replay</span>
            </button>
          )}
        </div>

        <label className="flex items-center gap-1 text-[11px]" style={{ color: 'var(--tv-text-secondary)' }}>
          <span>SL/TP %</span>
          <input
            type="number"
            step="0.5"
            value={stopLossPct}
            onChange={(e) => setStopLossPct(e.target.value)}
            className="w-12 px-1 py-0.5 rounded text-[11px] tabular-nums text-center no-spinner"
            style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
          />
          <input
            type="number"
            step="0.5"
            value={takeProfitPct}
            onChange={(e) => setTakeProfitPct(e.target.value)}
            className="w-12 px-1 py-0.5 rounded text-[11px] tabular-nums text-center no-spinner"
            style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
          />
        </label>

        <div className="flex items-center gap-1 ml-auto">
          <button
            type="button"
            disabled={running !== null}
            onClick={() => void handleRunBacktest()}
            className="flex items-center gap-1 px-2.5 py-1 rounded text-[11.5px] font-semibold bg-[#2962FF] hover:bg-[#1E53E5] text-white border-0 cursor-pointer disabled:opacity-50"
          >
            <Play className="w-3 h-3" />
            <span>{running === 'bt' ? 'Đang chạy...' : 'Backtest'}</span>
          </button>
          <button
            type="button"
            disabled={running !== null}
            onClick={() => void handleRunPortfolio()}
            className="flex items-center gap-1 px-2 py-1 rounded text-[11px] font-medium border cursor-pointer disabled:opacity-50"
            style={{ backgroundColor: 'var(--tv-bg-panel)', borderColor: 'var(--tv-border)', color: 'var(--tv-text-primary)' }}
            title="Chạy Backtest phân nhóm trên Rổ QMV30 / QMV60 / DT1-3 / Ngành"
          >
            <Briefcase className="w-3 h-3" />
            <span>{running === 'pf' ? 'Nhóm...' : 'Phân nhóm QMV'}</span>
          </button>
          <button
            type="button"
            disabled={running !== null}
            onClick={() => void handleRunWfo()}
            className="flex items-center gap-1 px-2 py-1 rounded text-[11px] font-medium border cursor-pointer disabled:opacity-50"
            style={{ backgroundColor: 'var(--tv-bg-panel)', borderColor: 'var(--tv-border)', color: 'var(--tv-text-primary)' }}
            title="Walk-Forward 5-Fold Out-of-Sample chống Overfitting"
          >
            <BarChart3 className="w-3 h-3" />
            <span>{running === 'wfo' ? 'WFO...' : 'WFO 5-Fold'}</span>
          </button>
          <button
            type="button"
            disabled={running !== null}
            onClick={() => void handleRunMc()}
            className="flex items-center gap-1 px-2 py-1 rounded text-[11px] font-medium border cursor-pointer disabled:opacity-50"
            style={{ backgroundColor: 'var(--tv-bg-panel)', borderColor: 'var(--tv-border)', color: 'var(--tv-text-primary)' }}
            title="Monte Carlo 2,000 kịch bản đánh giá VaR95 / CVaR95"
          >
            <Dice5 className="w-3 h-3" />
            <span>{running === 'mc' ? 'MC...' : 'Monte Carlo'}</span>
          </button>
        </div>
      </div>

      {/* Row 1b: Empirically Validated Stage-4 WFO Optimal Playbooks per QMV Group */}
      <div
        className="flex flex-wrap items-center gap-1.5 px-3 py-1 shrink-0 text-[10.5px]"
        style={{ borderBottom: '1px solid var(--tv-border)', backgroundColor: 'var(--tv-bg-panel)' }}
      >
        <span className="font-semibold text-[#2962FF]">Playbook WFO Tối ưu:</span>
        <button
          type="button"
          onClick={() => {
            setPreset('qmv_s3')
            setQmvIchiVariant('1')
            setQmvRvolConfirm('1.2')
            setQmvTrailMode('0')
            setQmvUseVnindexGate(true)
            setUseSupportStop(true)
            setQmvWaveTrailing23(true)
            setStopLossPct('8.5')
            setTakeProfitPct('')
            setPortfolioTarget('QMV:QMV30')
            setPortfolioWeighting('inverse_vol')
          }}
          className="px-2 py-0.5 rounded border cursor-pointer"
          style={{ backgroundColor: 'var(--tv-bg-elevated)', borderColor: 'var(--tv-border)', color: 'var(--tv-text-primary)' }}
          title="WFO OOS QMV30: +34.6% · MC ProbProfit 65.0% · VaR95 DD chỉ -7.7%"
        >
          QMV30 VaR Thấp (S3 · Ichi 3-6-18)
        </button>
        <button
          type="button"
          onClick={() => {
            setPreset('qmv_s2')
            setQmvDivOsc('3')
            setQmvConfirmMode('1')
            setQmvUseVnindexGate(true)
            setUseSupportStop(true)
            setQmvWaveTrailing23(true)
            setStopLossPct('7.0')
            setTakeProfitPct('18')
            setPortfolioTarget('QMV:CORE10')
            setPortfolioWeighting('inverse_vol')
          }}
          className="px-2 py-0.5 rounded border cursor-pointer"
          style={{ backgroundColor: 'var(--tv-bg-elevated)', borderColor: 'var(--tv-border)', color: 'var(--tv-text-primary)' }}
          title="WFO OOS Yumeko Core 10: Win Rate 56.8% · PF 1.70 · Max DD -9.6% (Cấm áp dụng cho DT3/Trà đá!)"
        >
          Core10 Bắt Đáy (S2 · CCI14)
        </button>
        <button
          type="button"
          onClick={() => {
            setPreset('qmv_s5')
            setMaxSqueezeBwPct('12')
            setQmvRvolConfirm('1.6')
            setQmvUseVnindexGate(true)
            setUseSupportStop(true)
            setQmvWaveTrailing23(true)
            setStopLossPct('8.0')
            setTakeProfitPct('')
            setPortfolioTarget('QMV:CORE10')
            setPortfolioWeighting('inverse_vol')
          }}
          className="px-2 py-0.5 rounded border cursor-pointer"
          style={{ backgroundColor: 'var(--tv-bg-elevated)', borderColor: 'var(--tv-border)', color: 'var(--tv-text-primary)' }}
          title="WFO OOS Core10/QMV30: PF_IV +49.7% · MC ProbProfit 67.2% · PF 1.59"
        >
          Vượt Tắc Nghẽn (S5 · RVol≥1.6)
        </button>
        <button
          type="button"
          onClick={() => {
            setPreset('qmv_s6')
            setQmvMinPillars('3')
            setQmvIchiVariant('0')
            setQmvTrailMode('0')
            setQmvUseVnindexGate(true)
            setUseSupportStop(true)
            setQmvWaveTrailing23(true)
            setStopLossPct('8.0')
            setTakeProfitPct('')
            setPortfolioTarget('QMV:SECTOR:TECH_TELECOM')
            setPortfolioWeighting('inverse_vol')
          }}
          className="px-2 py-0.5 rounded border cursor-pointer"
          style={{ backgroundColor: 'var(--tv-bg-elevated)', borderColor: 'var(--tv-border)', color: 'var(--tv-text-primary)' }}
          title="WFO OOS Công nghệ & Bán lẻ: PF_IV +47.8%..+60.2% · Sharpe 0.57 · PF 1.48"
        >
          Công Nghệ & Bán Lẻ (S6 · 3/4 Trụ)
        </button>
        <button
          type="button"
          onClick={() => {
            setPreset('bb_squeeze')
            setBbPeriod('30')
            setBbStdMult('1.8')
            setMaxSqueezeBwPct('10')
            setStopLossPct('7.0')
            setTakeProfitPct('')
            setPortfolioTarget('QMV:SECTOR:BANKING')
            setPortfolioWeighting('inverse_vol')
          }}
          className="px-2 py-0.5 rounded border cursor-pointer"
          style={{ backgroundColor: 'var(--tv-bg-elevated)', borderColor: 'var(--tv-border)', color: 'var(--tv-text-primary)' }}
          title="WFO OOS Ngân hàng: OOS Mean +21.1% (Med +14.9%) · PF_IV +25.7% · MC ProbProfit 60.6%"
        >
          Ngân Hàng Nén (BB 30, 1.8)
        </button>
        <button
          type="button"
          onClick={() => {
            setPreset('sma_ema')
            setFastSma('30')
            setSlowEma('50')
            setStopLossPct('8.0')
            setTakeProfitPct('')
            setPortfolioTarget('QMV:SECTOR:BROKERAGE')
            setPortfolioWeighting('inverse_vol')
          }}
          className="px-2 py-0.5 rounded border cursor-pointer"
          style={{ backgroundColor: 'var(--tv-bg-elevated)', borderColor: 'var(--tv-border)', color: 'var(--tv-text-primary)' }}
          title="WFO OOS Chứng khoán / BĐS / Thép / KCN: PF_IV +56%..+178% · Bắt buộc phân bổ 1/σ (InverseVol20)"
        >
          Sóng Dài CK/Thép/KCN (SMA30/EMA50)
        </button>
      </div>

      {/* Row 2: QMV Playbook Dynamic Parameters & Risk Guards */}
      {isQmvPreset && (
        <div
          className="flex flex-wrap items-center gap-2 px-3 py-1 shrink-0 text-[11px]"
          style={{ borderBottom: '1px solid var(--tv-border)', backgroundColor: 'var(--tv-bg-panel)' }}
        >
          <span className="inline-flex items-center gap-1 font-semibold text-[#089981]">
            <ShieldCheck className="w-3.5 h-3.5" />
            <span>Kỷ luật QMV:</span>
          </span>

          <label className="inline-flex items-center gap-1 cursor-pointer select-none" style={{ color: 'var(--tv-text-secondary)' }} title="Khóa mua mới khi HOSE:VNINDEX rơi vào trạng thái Downtrend (Close < MA50 & MA20 < MA50)">
            <input type="checkbox" checked={qmvUseVnindexGate} onChange={(e) => setQmvUseVnindexGate(e.target.checked)} className="m-0" />
            <span>Cổng VNINDEX</span>
          </label>

          <label className="inline-flex items-center gap-1 cursor-pointer select-none" style={{ color: 'var(--tv-text-secondary)' }} title="Đặt điểm cắt lỗ cứng ngay dưới đáy hỗ trợ gần nhất (kết hợp trần lỗ SL%)">
            <input type="checkbox" checked={useSupportStop} onChange={(e) => setUseSupportStop(e.target.checked)} className="m-0" />
            <span>SL Đáy hỗ trợ</span>
          </label>

          <label className="inline-flex items-center gap-1 cursor-pointer select-none" style={{ color: 'var(--tv-text-secondary)' }} title="Quy tắc giữ 2/3 biên độ sóng QMV: khóa lợi nhuận tại Đáy + 2/3*(Đỉnh - Đáy) khi sóng >= 1.8 ATR">
            <input type="checkbox" checked={qmvWaveTrailing23} onChange={(e) => setQmvWaveTrailing23(e.target.checked)} className="m-0" />
            <span>Giữ 2/3 sóng</span>
          </label>

          {(preset === 'qmv_s1' || useSupportStop) && (
            <label className="flex items-center gap-1" style={{ color: 'var(--tv-text-secondary)' }}>
              <span>Đáy (p):</span>
              <select
                value={qmvSupportWindow}
                onChange={(e) => setQmvSupportWindow(e.target.value)}
                className="px-1.5 py-0.5 rounded text-[10.5px] outline-none"
                style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
              >
                <option value="10">10 phiên</option>
                <option value="20">20 phiên</option>
                <option value="50">50 phiên</option>
              </select>
            </label>
          )}

          {(preset === 'qmv_s1' || preset === 'qmv_s3' || preset === 'qmv_s5') && (
            <label className="flex items-center gap-1" style={{ color: 'var(--tv-text-secondary)' }}>
              <span>RVol ≥</span>
              <input
                type="number"
                step="0.1"
                value={qmvRvolConfirm}
                onChange={(e) => setQmvRvolConfirm(e.target.value)}
                className="w-12 px-1 py-0.5 rounded text-[10.5px] text-center tabular-nums no-spinner"
                style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
              />
            </label>
          )}

          {preset === 'qmv_s2' && (
            <>
              <label className="flex items-center gap-1" style={{ color: 'var(--tv-text-secondary)' }}>
                <span>Phân kỳ:</span>
                <select
                  value={qmvDivOsc}
                  onChange={(e) => setQmvDivOsc(e.target.value)}
                  className="px-1.5 py-0.5 rounded text-[10.5px] outline-none"
                  style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                >
                  <option value="3">Hợp lưu RSI / MACD / CCI</option>
                  <option value="0">RSI(14)</option>
                  <option value="1">MACD(12,26,9)</option>
                  <option value="2">CCI(14)</option>
                </select>
              </label>
              <label className="flex items-center gap-1" style={{ color: 'var(--tv-text-secondary)' }}>
                <span>Xác nhận:</span>
                <select
                  value={qmvConfirmMode}
                  onChange={(e) => setQmvConfirmMode(e.target.value)}
                  className="px-1.5 py-0.5 rounded text-[10.5px] outline-none"
                  style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                >
                  <option value="0">Vượt High nến đảo chiều</option>
                  <option value="1">Lấy lại đường MA20</option>
                </select>
              </label>
            </>
          )}

          {(preset === 'qmv_s3' || preset === 'qmv_s6') && (
            <label className="flex items-center gap-1" style={{ color: 'var(--tv-text-secondary)' }}>
              <span>Ichimoku:</span>
              <select
                value={qmvIchiVariant}
                onChange={(e) => setQmvIchiVariant(e.target.value)}
                className="px-1.5 py-0.5 rounded text-[10.5px] outline-none"
                style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
              >
                <option value="0">Chuẩn QMV V7 (6-18-36)</option>
                <option value="1">Ngắn hạn QMV (3-6-18)</option>
                <option value="2">Cổ điển (9-26-52)</option>
              </select>
            </label>
          )}

          {preset === 'qmv_s4' && (
            <>
              <label className="flex items-center gap-1" style={{ color: 'var(--tv-text-secondary)' }}>
                <span>Cặp chu kỳ Proxy:</span>
                <select
                  value={qmvFlowHorizonPair}
                  onChange={(e) => setQmvFlowHorizonPair(e.target.value)}
                  className="px-1.5 py-0.5 rounded text-[10.5px] outline-none"
                  style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                >
                  <option value="0">T+2 (5p) & Sóng (20p)</option>
                  <option value="1">T+ (3p) & Ngắn (10p)</option>
                  <option value="2">Ngắn (10p) & Trung (50p)</option>
                </select>
              </label>
              <label className="flex items-center gap-1" style={{ color: 'var(--tv-text-secondary)' }}>
                <span>Duy trì / Bão hòa:</span>
                <input
                  type="number"
                  value={qmvFlowLowZone}
                  onChange={(e) => setQmvFlowLowZone(e.target.value)}
                  className="w-11 px-1 py-0.5 rounded text-[10.5px] text-center tabular-nums no-spinner"
                  style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                />
                <input
                  type="number"
                  value={qmvFlowHighZone}
                  onChange={(e) => setQmvFlowHighZone(e.target.value)}
                  className="w-11 px-1 py-0.5 rounded text-[10.5px] text-center tabular-nums no-spinner"
                  style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                />
              </label>
            </>
          )}

          {preset === 'qmv_s6' && (
            <label className="flex items-center gap-1" style={{ color: 'var(--tv-text-secondary)' }}>
              <span>Hội tụ tối thiểu:</span>
              <select
                value={qmvMinPillars}
                onChange={(e) => setQmvMinPillars(e.target.value)}
                className="px-1.5 py-0.5 rounded text-[10.5px] outline-none"
                style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
              >
                <option value="3">≥ 3 / 4 trụ cột QMV</option>
                <option value="4">Đủ 4 / 4 trụ cột QMV</option>
              </select>
            </label>
          )}

          <label className="flex items-center gap-1 ml-auto" style={{ color: 'var(--tv-text-secondary)' }}>
            <span>Dời thoát:</span>
            <select
              value={qmvTrailMode}
              onChange={(e) => setQmvTrailMode(e.target.value)}
              className="px-1.5 py-0.5 rounded text-[10.5px] outline-none"
              style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
            >
              <option value="0">Gãy MA20 + Áp lực bán</option>
              <option value="1">Gãy Kijun (18)</option>
              <option value="2">Đảo chiều Dòng tiền 70–80</option>
            </select>
          </label>
        </div>
      )}

      {/* Row 2b: Baseline Strategy Parameters (when a baseline reference preset is selected) */}
      {!isQmvPreset && preset !== 'custom' && (
        <div
          className="flex flex-wrap items-center gap-2 px-3 py-1 shrink-0 text-[11px] tabular-nums"
          style={{ borderBottom: '1px solid var(--tv-border)', backgroundColor: 'var(--tv-bg-panel)', color: 'var(--tv-text-secondary)' }}
        >
          {preset === 'sma_ema' && (
            <label className="flex items-center gap-1">
              <span>SMA / EMA:</span>
              <input type="number" value={fastSma} onChange={(e) => setFastSma(e.target.value)} className="w-12 px-1 py-0.5 rounded text-center no-spinner" style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }} />
              <input type="number" value={slowEma} onChange={(e) => setSlowEma(e.target.value)} className="w-12 px-1 py-0.5 rounded text-center no-spinner" style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }} />
            </label>
          )}
          {preset === 'rsi_mean_rev' && (
            <label className="flex items-center gap-1">
              <span>RSI / OS / OB:</span>
              <input type="number" value={rsiPeriod} onChange={(e) => setRsiPeriod(e.target.value)} className="w-11 px-1 py-0.5 rounded text-center no-spinner" style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }} />
              <input type="number" value={rsiOversold} onChange={(e) => setRsiOversold(e.target.value)} className="w-11 px-1 py-0.5 rounded text-center no-spinner" style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }} />
              <input type="number" value={rsiOverbought} onChange={(e) => setRsiOverbought(e.target.value)} className="w-11 px-1 py-0.5 rounded text-center no-spinner" style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }} />
            </label>
          )}
          {preset === 'macd' && (
            <label className="flex items-center gap-1">
              <span>MACD / EMA:</span>
              <input type="number" value={macdFast} onChange={(e) => setMacdFast(e.target.value)} className="w-11 px-1 py-0.5 rounded text-center no-spinner" style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }} />
              <input type="number" value={macdSlow} onChange={(e) => setMacdSlow(e.target.value)} className="w-11 px-1 py-0.5 rounded text-center no-spinner" style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }} />
              <input type="number" value={macdSignal} onChange={(e) => setMacdSignal(e.target.value)} className="w-10 px-1 py-0.5 rounded text-center no-spinner" style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }} />
              <input type="number" value={macdTrendEma} onChange={(e) => setMacdTrendEma(e.target.value)} className="w-12 px-1 py-0.5 rounded text-center no-spinner" style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }} />
            </label>
          )}
          {preset === 'bb_squeeze' && (
            <label className="flex items-center gap-1">
              <span>BB / Std / Sqz%:</span>
              <input type="number" value={bbPeriod} onChange={(e) => setBbPeriod(e.target.value)} className="w-11 px-1 py-0.5 rounded text-center no-spinner" style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }} />
              <input type="number" step="0.1" value={bbStdMult} onChange={(e) => setBbStdMult(e.target.value)} className="w-11 px-1 py-0.5 rounded text-center no-spinner" style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }} />
              <input type="number" step="0.5" value={maxSqueezeBwPct} onChange={(e) => setMaxSqueezeBwPct(e.target.value)} className="w-11 px-1 py-0.5 rounded text-center no-spinner" style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }} />
            </label>
          )}
          {preset === 'donchian_atr' && (
            <label className="flex items-center gap-1">
              <span>Donchian Vào / Ra:</span>
              <input type="number" value={donchianEntry} onChange={(e) => setDonchianEntry(e.target.value)} className="w-12 px-1 py-0.5 rounded text-center no-spinner" style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }} />
              <input type="number" value={donchianExit} onChange={(e) => setDonchianExit(e.target.value)} className="w-12 px-1 py-0.5 rounded text-center no-spinner" style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }} />
            </label>
          )}
          {preset === 'rsi_bb' && (
            <label className="flex items-center gap-1">
              <span>RSI / BB / Std:</span>
              <input type="number" value={rsiPeriod} onChange={(e) => setRsiPeriod(e.target.value)} className="w-11 px-1 py-0.5 rounded text-center no-spinner" style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }} />
              <input type="number" value={bbPeriod} onChange={(e) => setBbPeriod(e.target.value)} className="w-11 px-1 py-0.5 rounded text-center no-spinner" style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }} />
              <input type="number" step="0.1" value={bbStdMult} onChange={(e) => setBbStdMult(e.target.value)} className="w-11 px-1 py-0.5 rounded text-center no-spinner" style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }} />
            </label>
          )}
        </div>
      )}

      {/* Custom QMV Signal Rule Builder Panel */}
      {preset === 'custom' && (
        <div
          className="flex flex-wrap items-center gap-2 px-3 py-1.5 shrink-0 text-[11px]"
          style={{ borderBottom: '1px solid var(--tv-border)', backgroundColor: 'var(--tv-bg-panel)' }}
        >
          <span className="inline-flex items-center gap-1 font-semibold text-[#2962FF]">
            <SlidersHorizontal className="w-3 h-3" />
            <span>Quy tắc QMV:</span>
          </span>
          <input
            aria-label="Tên chiến lược QMV tự thiết kế"
            type="text"
            value={customRuleName}
            onChange={(e) => setCustomRuleName(e.target.value)}
            className="w-40 px-1.5 py-0.5 rounded text-[11px] outline-none"
            style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
          />
          <select
            aria-label="Toán tử kết hợp điều kiện vào lệnh"
            value={entryCombinator}
            onChange={(e) => setEntryCombinator(e.target.value as 'and' | 'or')}
            className="px-1.5 py-0.5 rounded text-[11px] font-semibold outline-none"
            style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
          >
            <option value="and">AND (Đồng thời)</option>
            <option value="or">OR (Bất kỳ)</option>
          </select>

          {entryClauses.map((clause, idx) => {
            const floatMode = isFloatSignal(clause.catalogId)
            return (
              <div
                key={idx}
                className="flex items-center gap-1 px-1.5 py-0.5 rounded"
                style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}
              >
                <select
                  aria-label={`Tín hiệu vào lệnh ${idx + 1}`}
                  value={clause.catalogId}
                  onChange={(e) => {
                    const nextId = e.target.value
                    const nextFloat = isFloatSignal(nextId)
                    setEntryClauses((prev) =>
                      prev.map((c, i) =>
                        i === idx
                          ? nextFloat
                            ? { catalogId: nextId, op: '>=', threshold: 1.3 }
                            : { catalogId: nextId, boolExpected: true }
                          : c,
                      ),
                    )
                  }}
                  className="max-w-[190px] bg-transparent text-[11px] outline-none"
                  style={{ color: 'var(--tv-text-primary)' }}
                >
                  {qmvCatalogSignals.length === 0 ? (
                    <option value={clause.catalogId}>{clause.catalogId}</option>
                  ) : (
                    qmvCatalogSignals.map((s) => (
                      <option key={s.id} value={s.id}>
                        {s.labelVi} ({s.outputType})
                      </option>
                    ))
                  )}
                </select>
                {floatMode ? (
                  <>
                    <select
                      value={clause.op ?? '>='}
                      onChange={(e) =>
                        setEntryClauses((prev) =>
                          prev.map((c, i) =>
                            i === idx ? { ...c, op: e.target.value as CustomClauseInput['op'] } : c,
                          ),
                        )
                      }
                      className="px-1 py-0.2 rounded text-[10.5px] outline-none"
                      style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                    >
                      <option value=">">&gt;</option>
                      <option value=">=">&ge;</option>
                      <option value="<">&lt;</option>
                      <option value="<=">&le;</option>
                      <option value="cross_above">Cắt lên</option>
                      <option value="cross_below">Cắt xuống</option>
                    </select>
                    <input
                      type="number"
                      step="0.1"
                      value={clause.threshold ?? 1.0}
                      onChange={(e) =>
                        setEntryClauses((prev) =>
                          prev.map((c, i) => (i === idx ? { ...c, threshold: Number(e.target.value) } : c)),
                        )
                      }
                      className="w-12 px-1 py-0.2 rounded text-[10.5px] text-center tabular-nums no-spinner outline-none"
                      style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                    />
                  </>
                ) : (
                  <span className="text-[10px] font-semibold text-[#089981]">Kích hoạt</span>
                )}
                {entryClauses.length > 1 && (
                  <button
                    type="button"
                    onClick={() => setEntryClauses((prev) => prev.filter((_, i) => i !== idx))}
                    className="p-0.5 rounded bg-transparent border-0 cursor-pointer"
                    style={{ color: 'var(--tv-text-secondary)' }}
                  >
                    <Trash2 className="w-3 h-3" />
                  </button>
                )}
              </div>
            )
          })}

          {entryClauses.length < 4 && (
            <button
              type="button"
              onClick={() =>
                setEntryClauses((prev) => [...prev, { catalogId: 'qmv.vsa_support_dry_up', boolExpected: true }])
              }
              className="inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded text-[10.5px] border cursor-pointer"
              style={{ backgroundColor: 'transparent', borderColor: 'var(--tv-border)', color: 'var(--tv-text-primary)' }}
            >
              <Plus className="w-3 h-3" />
              <span>Điều kiện</span>
            </button>
          )}

          <label className="flex items-center gap-1 ml-1" style={{ color: 'var(--tv-text-secondary)' }}>
            <input
              type="checkbox"
              checked={exitClauseEnabled}
              onChange={(e) => setExitClauseEnabled(e.target.checked)}
            />
            <span>Thoát:</span>
            {exitClauseEnabled && (
              <select
                value={exitClause.catalogId}
                onChange={(e) => {
                  const id = e.target.value
                  setExitClause(
                    isFloatSignal(id)
                      ? { catalogId: id, op: '>', threshold: 70 }
                      : { catalogId: id, boolExpected: true },
                  )
                }}
                className="max-w-[170px] px-1 py-0.5 rounded text-[10.5px] outline-none"
                style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
              >
                {qmvCatalogSignals.map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.labelVi}
                  </option>
                ))}
              </select>
            )}
          </label>

          <label className="flex items-center gap-1" style={{ color: 'var(--tv-text-secondary)' }}>
            <span>Điểm SK ≥</span>
            <input
              type="number"
              placeholder="Tắt"
              value={minHealthScore}
              onChange={(e) => setMinHealthScore(e.target.value)}
              className="w-12 px-1 py-0.5 rounded text-[10.5px] text-center tabular-nums no-spinner outline-none"
              style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
            />
          </label>
        </div>
      )}

      {/* Sub-view Switcher & Tearsheet Summary Strip */}
      <div
        className="flex flex-wrap items-center justify-between gap-x-3 gap-y-1 px-3 py-1 shrink-0"
        style={{ minHeight: 32, borderBottom: '1px solid var(--tv-border)' }}
      >
        <div role="tablist" aria-label="Chế độ xem Strategy Tester" className="flex items-center h-7 gap-1">
          {(
            [
              { id: 'overview', label: 'Tổng quan & IS/OOS' },
              { id: 'trades', label: `Danh sách lệnh (${btResult?.trades.length ?? 0})` },
              {
                id: 'portfolio',
                label: portfolioResult
                  ? `Phân nhóm QMV (${portfolioResult.constituentCount} mã)`
                  : 'Phân nhóm QMV (Rổ / Ngành / DT)',
              },
              { id: 'wfo', label: 'Walk-Forward (WFO)' },
              { id: 'mc', label: 'Monte Carlo (VaR/CVaR)' },
            ] as const
          ).map((t) => (
            <button
              key={t.id}
              type="button"
              role="tab"
              aria-selected={view === t.id}
              className="tv-tab text-[11px] px-2"
              onClick={() => setView(t.id)}
            >
              {t.label}
            </button>
          ))}
        </div>

        {ts && (
          <div className="flex items-center gap-2.5 text-[11px] tabular-nums">
            {ts.low_sample_warning && (
              <span
                className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-semibold"
                style={{ backgroundColor: 'rgba(251, 192, 45, 0.18)', color: '#FBC02D' }}
                title="Cỡ mẫu ít (< 30 lệnh) — rủi ro nhiễu thống kê cao, nên kiểm định trên Phân nhóm QMV (Portfolio)"
              >
                <AlertTriangle className="w-3 h-3" />
                <span>Ít lệnh ({ts.num_trades})</span>
              </span>
            )}
            {backtestOverlay && (
              <button
                type="button"
                onClick={toggleBacktestOverlayVisible}
                className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[10.5px] font-medium border cursor-pointer"
                style={{
                  backgroundColor: backtestOverlay.visible ? 'var(--tv-accent-soft)' : 'transparent',
                  borderColor: backtestOverlay.visible ? 'var(--tv-accent)' : 'var(--tv-border)',
                  color: backtestOverlay.visible ? 'var(--tv-accent)' : 'var(--tv-text-secondary)',
                }}
              >
                {backtestOverlay.visible ? <Eye className="w-3 h-3" /> : <EyeOff className="w-3 h-3" />}
                <span>{backtestOverlay.visible ? 'Tín hiệu nến' : 'Đã ẩn'}</span>
              </button>
            )}
            <span>
              <span style={{ color: 'var(--tv-text-secondary)' }}>LN Ròng: </span>
              <strong style={{ color: ts.total_return_pct >= 0 ? 'var(--tv-up)' : 'var(--tv-down)' }}>
                {ts.total_return_pct >= 0 ? '+' : ''}
                {ts.total_return_pct.toFixed(2)}%
              </strong>
            </span>
            <span>
              <span style={{ color: 'var(--tv-text-secondary)' }}>Max DD: </span>
              <strong style={{ color: 'var(--tv-down)' }}>-{ts.max_drawdown_pct.toFixed(2)}%</strong>
            </span>
            <span>
              <span style={{ color: 'var(--tv-text-secondary)' }}>Sharpe: </span>
              <strong>{ts.sharpe_ratio != null ? ts.sharpe_ratio.toFixed(2) : '—'}</strong>
            </span>
            <span>
              <span style={{ color: 'var(--tv-text-secondary)' }}>PF: </span>
              <strong>{ts.profit_factor != null ? ts.profit_factor.toFixed(2) : '—'}</strong>
            </span>
            {btResult?.robustness && (
              <span
                className="px-1.5 py-0.5 rounded text-[10px] font-bold"
                style={{ backgroundColor: 'var(--tv-accent-soft)', color: 'var(--tv-accent)' }}
              >
                {btResult.robustness}
              </span>
            )}
          </div>
        )}
      </div>

      {/* Main Content */}
      <div className="flex-1 min-h-0 overflow-auto tv-scroll p-2.5">
        {view === 'overview' && (
          <div className="grid grid-cols-1 lg:grid-cols-[1fr_285px] gap-3 h-full min-h-[150px]">
            <div
              className="rounded p-2 flex flex-col"
              style={{ backgroundColor: 'var(--tv-bg-chart)', border: '1px solid var(--tv-border)' }}
            >
              <div className="flex items-center justify-between text-[10.5px] mb-1" style={{ color: 'var(--tv-text-secondary)' }}>
                <span className="flex items-center gap-1">
                  <TrendingUp className="w-3.5 h-3.5" />
                  Đường cong vốn & Sụt giảm (Underwater Drawdown) — {activePane.ticker} ({activePane.interval})
                </span>
                {btResult && <span>{btResult.barCount} nến · {btResult.presetLabel}</span>}
              </div>
              <div className="flex-1 min-h-0">
                <EquityAndDrawdownChart points={btResult?.equityCurve ?? []} />
              </div>
            </div>

            <div
              className="rounded p-2.5 grid grid-cols-2 gap-2 content-start text-[11px] tabular-nums"
              style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}
            >
              <div>
                <div style={{ color: 'var(--tv-text-secondary)' }}>CAGR</div>
                <div className="font-bold text-[13px]">{ts?.cagr_pct != null ? `${ts.cagr_pct.toFixed(2)}%` : '—'}</div>
              </div>
              <div>
                <div style={{ color: 'var(--tv-text-secondary)' }}>Sortino / Calmar</div>
                <div className="font-bold text-[13px]">
                  {ts?.sortino_ratio != null ? ts.sortino_ratio.toFixed(2) : '—'} /{' '}
                  {ts?.calmar_ratio != null ? ts.calmar_ratio.toFixed(2) : '—'}
                </div>
              </div>
              <div>
                <div style={{ color: 'var(--tv-text-secondary)' }}>In-Sample (70%)</div>
                <div
                  className="font-bold text-[12.5px]"
                  style={{
                    color:
                      (btResult?.inSampleTearsheet?.total_return_pct ?? 0) >= 0 ? 'var(--tv-up)' : 'var(--tv-down)',
                  }}
                >
                  {btResult?.inSampleTearsheet
                    ? `${btResult.inSampleTearsheet.total_return_pct >= 0 ? '+' : ''}${btResult.inSampleTearsheet.total_return_pct.toFixed(1)}%`
                    : '—'}
                </div>
              </div>
              <div>
                <div style={{ color: 'var(--tv-text-secondary)' }}>Out-of-Sample (30%)</div>
                <div
                  className="font-bold text-[12.5px]"
                  style={{
                    color:
                      (btResult?.outOfSampleTearsheet?.total_return_pct ?? 0) >= 0 ? 'var(--tv-up)' : 'var(--tv-down)',
                  }}
                >
                  {btResult?.outOfSampleTearsheet
                    ? `${btResult.outOfSampleTearsheet.total_return_pct >= 0 ? '+' : ''}${btResult.outOfSampleTearsheet.total_return_pct.toFixed(1)}%`
                    : '—'}
                </div>
              </div>
              <div>
                <div style={{ color: 'var(--tv-text-secondary)' }}>Thắng / Thua (Win%)</div>
                <div className="font-bold text-[12px]">
                  <span style={{ color: 'var(--tv-up)' }}>{ts?.num_winners ?? 0}</span>/
                  <span style={{ color: 'var(--tv-down)' }}>{ts?.num_losers ?? 0}</span>
                  {ts ? ` (${ts.win_rate_pct.toFixed(0)}%)` : ''}
                </div>
              </div>
              <div>
                <div style={{ color: 'var(--tv-text-secondary)' }}>Nến giữ TB · Phí/Thuế</div>
                <div className="font-bold text-[11.5px]">
                  {ts ? `${ts.avg_bars_held.toFixed(1)}p · ${(ts.total_fees_and_tax / 1e6).toFixed(1)}tr` : '—'}
                </div>
              </div>
            </div>
          </div>
        )}

        {view === 'trades' && (
          <table className="tv-table tabular-nums">
            <caption className="sr-only-live">Danh sách lệnh Backtest — nhấp để cuộn biểu đồ</caption>
            <thead>
              <tr>
                <th scope="col">#</th>
                <th scope="col">Chiều</th>
                <th scope="col">Ngày vào (Nhấp xem nến)</th>
                <th scope="col">Giá vào</th>
                <th scope="col">Ngày ra</th>
                <th scope="col">Giá ra</th>
                <th scope="col">Net PnL</th>
                <th scope="col">%</th>
                <th scope="col">MAE / MFE</th>
                <th scope="col">Lý do thoát</th>
              </tr>
            </thead>
            <tbody>
              {(btResult?.trades ?? []).length === 0 ? (
                <tr>
                  <td colSpan={10} className="text-center py-6" style={{ color: 'var(--tv-text-secondary)' }}>
                    Chạy Backtest để xem danh sách lệnh chi tiết.
                  </td>
                </tr>
              ) : (
                btResult!.trades.map((tr) => (
                  <tr
                    key={tr.tradeId}
                    className="cursor-pointer"
                    onClick={() => scrollActiveChartToTimestamp(tr.entryTimestampMs)}
                    title="Nhấp để cuộn biểu đồ chính tới đúng cây nến vào lệnh"
                  >
                    <td>#{tr.tradeId}</td>
                    <td style={{ color: tr.side === 'LONG' ? 'var(--tv-up)' : 'var(--tv-down)', fontWeight: 600 }}>
                      {tr.side}
                    </td>
                    <td className="text-[#2962FF] underline">
                      {new Date(tr.entryTimestampMs).toLocaleDateString('vi-VN')}
                    </td>
                    <td>{tr.entryPrice.toLocaleString('en-US')}</td>
                    <td>{new Date(tr.exitTimestampMs).toLocaleDateString('vi-VN')}</td>
                    <td>{tr.exitPrice.toLocaleString('en-US')}</td>
                    <td style={{ color: tr.netPnl >= 0 ? 'var(--tv-up)' : 'var(--tv-down)', fontWeight: 600 }}>
                      {Math.round(tr.netPnl).toLocaleString('en-US')}
                    </td>
                    <td style={{ color: tr.returnPct >= 0 ? 'var(--tv-up)' : 'var(--tv-down)' }}>
                      {tr.returnPct >= 0 ? '+' : ''}
                      {tr.returnPct.toFixed(2)}%
                    </td>
                    <td>
                      <span style={{ color: 'var(--tv-down)' }}>{tr.maePct.toFixed(1)}%</span> /{' '}
                      <span style={{ color: 'var(--tv-up)' }}>+{tr.mfePct.toFixed(1)}%</span>
                    </td>
                    <td>{tr.exitReason}</td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        )}

        {view === 'portfolio' && (
          <div role="tabpanel" className="space-y-2.5">
            <div
              className="flex flex-wrap items-center gap-2 p-2 rounded text-[11px]"
              style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}
            >
              <label className="flex items-center gap-1.5" style={{ color: 'var(--tv-text-secondary)' }}>
                <span>Phân nhóm kiểm định:</span>
                <select
                  aria-label="Chọn Rổ / Nhóm Dòng tiền / Ngành QMV"
                  value={portfolioTarget}
                  onChange={(e) => setPortfolioTarget(e.target.value)}
                  className="px-2 py-0.5 rounded text-[11px] font-semibold outline-none"
                  style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                >
                  <optgroup label="Phân nhóm Chuẩn QMV (Rổ · Dòng tiền · Ngành)">
                    {QMV_PORTFOLIO_GROUPS.map((g) => (
                      <option key={g.value} value={g.value}>
                        {g.label}
                      </option>
                    ))}
                  </optgroup>
                  <optgroup label="Watchlist Cá Nhân">
                    {watchlists.map((w) => (
                      <option key={w.id} value={w.id}>
                        {w.name} ({w.tickers.length} mã)
                      </option>
                    ))}
                  </optgroup>
                </select>
              </label>

              <label className="flex items-center gap-1.5" style={{ color: 'var(--tv-text-secondary)' }}>
                <span>Phân bổ vốn:</span>
                <select
                  aria-label="Phương pháp phân bổ tỷ trọng danh mục"
                  value={portfolioWeighting}
                  onChange={(e) => setPortfolioWeighting(e.target.value as 'equal' | 'inverse_vol')}
                  className="px-2 py-0.5 rounded text-[11px] outline-none"
                  style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
                >
                  <option value="equal">Tỷ trọng đều (1/N)</option>
                  <option value="inverse_vol">Nghịch đảo biến động (1/σ · 20 phiên)</option>
                </select>
              </label>

              <button
                type="button"
                disabled={running !== null}
                onClick={() => void handleRunPortfolio()}
                className="flex items-center gap-1 px-2.5 py-1 rounded text-[11px] font-semibold bg-[#2962FF] hover:bg-[#1E53E5] text-white border-0 cursor-pointer disabled:opacity-50"
              >
                <Briefcase className="w-3 h-3" />
                <span>{running === 'pf' ? 'Đang chạy...' : 'Chạy Kiểm Định Phân Nhóm'}</span>
              </button>

              {portfolioResult && (
                <div className="flex flex-wrap items-center gap-3 ml-auto tabular-nums">
                  <span>
                    <span style={{ color: 'var(--tv-text-secondary)' }}>LN Nhóm: </span>
                    <strong style={{ color: portfolioResult.tearsheet.total_return_pct >= 0 ? 'var(--tv-up)' : 'var(--tv-down)' }}>
                      {portfolioResult.tearsheet.total_return_pct >= 0 ? '+' : ''}
                      {portfolioResult.tearsheet.total_return_pct.toFixed(2)}%
                    </strong>
                  </span>
                  <span>
                    <span style={{ color: 'var(--tv-text-secondary)' }}>CAGR: </span>
                    <strong>
                      {portfolioResult.tearsheet.cagr_pct != null
                        ? `${portfolioResult.tearsheet.cagr_pct.toFixed(2)}%`
                        : '—'}
                    </strong>
                  </span>
                  <span>
                    <span style={{ color: 'var(--tv-text-secondary)' }}>Max DD: </span>
                    <strong style={{ color: 'var(--tv-down)' }}>-{portfolioResult.tearsheet.max_drawdown_pct.toFixed(2)}%</strong>
                  </span>
                  <span>
                    <span style={{ color: 'var(--tv-text-secondary)' }}>Sharpe: </span>
                    <strong>
                      {portfolioResult.tearsheet.sharpe_ratio != null
                        ? portfolioResult.tearsheet.sharpe_ratio.toFixed(2)
                        : '—'}
                    </strong>
                  </span>
                </div>
              )}
            </div>

            {!portfolioResult ? (
              <div className="text-center py-6 text-[11.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
                Chọn Rổ QMV30, QMV60, Nhóm Dòng tiền (DT1/DT2/DT3) hoặc từng Ngành QMV để kiểm định chiến lược đa mã trên dữ liệu thật `sumi_market.db`.
              </div>
            ) : (
              <div className="grid grid-cols-1 lg:grid-cols-[1fr_1.15fr] gap-2.5">
                <div
                  className="rounded p-2 flex flex-col min-h-[155px]"
                  style={{ backgroundColor: 'var(--tv-bg-chart)', border: '1px solid var(--tv-border)' }}
                >
                  <div className="flex items-center justify-between text-[10.5px] mb-1" style={{ color: 'var(--tv-text-secondary)' }}>
                    <span>
                      Đường cong vốn Nhóm ({portfolioResult.constituentCount} mã · {portfolioResult.weightingLabel})
                    </span>
                    <span>{portfolioResult.presetLabel}</span>
                  </div>
                  <div className="flex-1 min-h-0">
                    <EquityAndDrawdownChart points={portfolioResult.equityCurve} />
                  </div>
                  {portfolioResult.survivorBiasCaveat && (
                    <div className="mt-1 text-[10px]" style={{ color: 'var(--tv-vn-ref)' }}>
                      Lưu ý: {portfolioResult.survivorBiasCaveat}
                    </div>
                  )}
                </div>

                <div
                  className="rounded overflow-auto max-h-[210px] tv-scroll"
                  style={{ border: '1px solid var(--tv-border)' }}
                >
                  <table className="tv-table tabular-nums">
                    <caption className="sr-only-live">Đóng góp từng mã trong Phân nhóm QMV</caption>
                    <thead>
                      <tr>
                        <th scope="col">Mã</th>
                        <th scope="col" className="text-right">Tỷ trọng</th>
                        <th scope="col" className="text-right">LN độc lập</th>
                        <th scope="col" className="text-right">Max DD</th>
                        <th scope="col" className="text-right">Đóng góp LN</th>
                        <th scope="col" className="text-right">Lệnh</th>
                      </tr>
                    </thead>
                    <tbody>
                      {portfolioResult.constituents.map((c) => (
                        <tr
                          key={c.instrument_id}
                          className="cursor-pointer"
                          onClick={() => setActiveTicker(c.instrument_id || c.symbol)}
                          title={`Nhấp để mở biểu đồ ${c.instrument_id}`}
                        >
                          <td className="font-bold text-[#2962FF]">{c.symbol}</td>
                          <td className="text-right">{c.final_weight_pct.toFixed(1)}%</td>
                          <td
                            className="text-right font-semibold"
                            style={{ color: c.standalone_return_pct >= 0 ? 'var(--tv-up)' : 'var(--tv-down)' }}
                          >
                            {c.standalone_return_pct >= 0 ? '+' : ''}
                            {c.standalone_return_pct.toFixed(2)}%
                          </td>
                          <td className="text-right" style={{ color: 'var(--tv-down)' }}>
                            -{c.standalone_max_dd_pct.toFixed(1)}%
                          </td>
                          <td
                            className="text-right font-semibold"
                            style={{ color: c.weighted_return_contribution_pct >= 0 ? 'var(--tv-up)' : 'var(--tv-down)' }}
                          >
                            {c.weighted_return_contribution_pct >= 0 ? '+' : ''}
                            {c.weighted_return_contribution_pct.toFixed(2)}%
                          </td>
                          <td className="text-right">{c.num_trades}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            )}
          </div>
        )}

        {view === 'wfo' && (
          <div className="space-y-2">
            {!wfoResult ? (
              <div className="text-center py-6 text-[11.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
                Nhấn "WFO 5-Fold" để chạy tối ưu hóa Walk-Forward Out-of-Sample chống Overfitting trên lưới tham số QMV.
              </div>
            ) : (
              <>
                <div className="flex flex-wrap items-center gap-4 text-[11.5px] tabular-nums">
                  <span className="px-2 py-0.5 rounded font-bold bg-[#2962FF] text-white">
                    {wfoResult.verdictLabel}
                  </span>
                  <span>
                    Fold có lãi: <strong>{wfoResult.profitableFolds}/{wfoResult.foldCount}</strong>
                  </span>
                  <span>
                    Pardo WFE: <strong>{wfoResult.wfePct != null ? `${wfoResult.wfePct.toFixed(1)}%` : '—'}</strong>
                  </span>
                  <span>
                    Độ ổn định tham số: <strong>{wfoResult.paramStabilityPct.toFixed(1)}%</strong>
                  </span>
                  <span>
                    OOS Stitched Return:{' '}
                    <strong
                      style={{
                        color: wfoResult.stitchedOosTearsheet.total_return_pct >= 0 ? 'var(--tv-up)' : 'var(--tv-down)',
                      }}
                    >
                      {wfoResult.stitchedOosTearsheet.total_return_pct >= 0 ? '+' : ''}
                      {wfoResult.stitchedOosTearsheet.total_return_pct.toFixed(2)}%
                    </strong>
                  </span>
                </div>
                <div className="h-[130px] rounded p-1.5" style={{ backgroundColor: 'var(--tv-bg-chart)', border: '1px solid var(--tv-border)' }}>
                  <EquityAndDrawdownChart points={wfoResult.stitchedOosCurve} />
                </div>
              </>
            )}
          </div>
        )}

        {view === 'mc' && (
          <div className="space-y-2">
            {!mcResult ? (
              <div className="text-center py-6 text-[11.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
                Nhấn "Monte Carlo" để chạy 2,000 kịch bản Bootstrap SplitMix64 đánh giá rủi ro đuôi (VaR95 / CVaR95).
              </div>
            ) : !mcResult.metrics ? (
              <div className="text-center py-6 text-[11.5px]" style={{ color: 'var(--tv-vn-ref)' }}>
                Không đủ số lệnh mẫu để chạy Monte Carlo ({mcResult.statusReason})
              </div>
            ) : (
              <div className="grid grid-cols-2 md:grid-cols-5 gap-2 tabular-nums text-[11.5px]">
                <div className="p-2 rounded" style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}>
                  <div style={{ color: 'var(--tv-text-secondary)' }}>Xác suất có lãi</div>
                  <div className="text-[15px] font-bold" style={{ color: 'var(--tv-up)' }}>
                    {mcResult.metrics.prob_of_profit_pct.toFixed(1)}%
                  </div>
                </div>
                <div className="p-2 rounded" style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}>
                  <div style={{ color: 'var(--tv-text-secondary)' }}>Xác suất sụt &gt;50%</div>
                  <div className="text-[15px] font-bold" style={{ color: 'var(--tv-down)' }}>
                    {mcResult.metrics.prob_of_ruin_pct.toFixed(2)}%
                  </div>
                </div>
                <div className="p-2 rounded" style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}>
                  <div style={{ color: 'var(--tv-text-secondary)' }}>Lợi nhuận P50 (Trung vị)</div>
                  <div className="text-[15px] font-bold">
                    {mcResult.metrics.terminal_return_pct.p50 >= 0 ? '+' : ''}
                    {mcResult.metrics.terminal_return_pct.p50.toFixed(2)}%
                  </div>
                </div>
                <div className="p-2 rounded" style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}>
                  <div style={{ color: 'var(--tv-text-secondary)' }}>Biên P5 — P95</div>
                  <div className="text-[13px] font-bold">
                    <span style={{ color: 'var(--tv-down)' }}>{mcResult.metrics.terminal_return_pct.p5.toFixed(1)}%</span>
                    {' .. '}
                    <span style={{ color: 'var(--tv-up)' }}>+{mcResult.metrics.terminal_return_pct.p95.toFixed(1)}%</span>
                  </div>
                </div>
                <div className="p-2 rounded" style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}>
                  <div style={{ color: 'var(--tv-text-secondary)' }}>Max DD VaR95 / CVaR95</div>
                  <div className="text-[13px] font-bold" style={{ color: 'var(--tv-down)' }}>
                    -{mcResult.metrics.max_drawdown_var_95_pct.toFixed(1)}% / -{mcResult.metrics.max_drawdown_cvar_95_pct.toFixed(1)}%
                  </div>
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  )
}
