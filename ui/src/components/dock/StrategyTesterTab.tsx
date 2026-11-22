import React, { useState } from 'react'
import { BarChart3, Dice5, Play, TrendingUp } from 'lucide-react'
import * as api from '../../api/tauri'
import { useWorkspaceStore } from '../../store/workspaceStore'
import type {
  MonteCarloResponse,
  StrategyBacktestInput,
  StrategyBacktestResponse,
  StrategyEquityPointDto,
  WalkForwardResponse,
} from '../../types/market'

type TesterView = 'overview' | 'trades' | 'wfo' | 'mc'

const PRESETS: Array<{ key: StrategyBacktestInput['preset']; label: string }> = [
  { key: 'sma_ema', label: 'SMA/EMA Trend Cross' },
  { key: 'rsi_mean_rev', label: 'RSI Mean Reversion' },
  { key: 'macd', label: 'MACD Momentum' },
  { key: 'bb_squeeze', label: 'Bollinger Squeeze Breakout' },
  { key: 'donchian_atr', label: 'Donchian ATR Breakout' },
  { key: 'rsi_bb', label: 'RSI + BB Confluence' },
]

function EquityAndDrawdownChart({ points }: { points: StrategyEquityPointDto[] }) {
  if (points.length < 2) {
    return (
      <div className="flex items-center justify-center h-full text-[11.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
        Nhấn "Chạy Backtest" để tính toán đường cong vốn & drawdown từ lõi sumi-core
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
      {/* Equity Area & Line */}
      <path d={eqArea} fill="url(#sumiEqGrad)" />
      <path d={eqLine} fill="none" stroke={strokeColor} strokeWidth="1.8" />

      {/* Separator */}
      <line x1={padX} y1={eqH + 8} x2={w - padX} y2={eqH + 8} stroke="var(--tv-border)" strokeWidth="1" />

      {/* Underwater Drawdown Area */}
      <path d={ddArea} fill="rgba(242, 54, 69, 0.28)" />
      <path d={ddLine} fill="none" stroke="#F23645" strokeWidth="1.2" />
    </svg>
  )
}

export const StrategyTesterTab: React.FC = () => {
  const panes = useWorkspaceStore((s) => s.panes)
  const activeSlotId = useWorkspaceStore((s) => s.activeSlotId)
  const scrollActiveChartToTimestamp = useWorkspaceStore((s) => s.scrollActiveChartToTimestamp)
  const pushToast = useWorkspaceStore((s) => s.pushToast)

  const activePane = panes.find((p) => p.slotId === activeSlotId) ?? panes[0]

  const [preset, setPreset] = useState<StrategyBacktestInput['preset']>('sma_ema')
  const [direction, setDirection] = useState<'long' | 'short' | 'both'>('long')
  const [fastSma, setFastSma] = useState('20')
  const [slowEma, setSlowEma] = useState('50')
  const [stopLossPct, setStopLossPct] = useState('5')
  const [takeProfitPct, setTakeProfitPct] = useState('12')
  const [initialCash, setInitialCash] = useState('500000000')
  const [minHold, setMinHold] = useState('2')

  const [view, setView] = useState<TesterView>('overview')
  const [running, setRunning] = useState<'bt' | 'wfo' | 'mc' | null>(null)
  const [btResult, setBtResult] = useState<StrategyBacktestResponse | null>(null)
  const [wfoResult, setWfoResult] = useState<WalkForwardResponse | null>(null)
  const [mcResult, setMcResult] = useState<MonteCarloResponse | null>(null)

  const isVn =
    activePane.ticker.startsWith('HOSE:') ||
    activePane.ticker.startsWith('HNX:') ||
    activePane.ticker.startsWith('UPCOM:')

  const buildInput = (): StrategyBacktestInput => ({
    ticker: activePane.ticker,
    interval: activePane.interval,
    preset,
    direction,
    initialCash: Number(initialCash) || 500_000_000,
    commissionBps: isVn ? 15 : 5,
    taxSellBps: isVn ? 10 : 0,
    slippageBps: 5,
    lotStep: isVn ? 100 : 0,
    positionSizePct: 95,
    stopLossPct: Number(stopLossPct) > 0 ? Number(stopLossPct) : null,
    takeProfitPct: Number(takeProfitPct) > 0 ? Number(takeProfitPct) : null,
    minHoldingBars: Number(minHold) || 0,
    fastSma: Number(fastSma) || 20,
    slowEma: Number(slowEma) || 50,
    inSampleSplitPct: 0.7,
    maxBars: 2500,
  })

  const handleRunBacktest = async () => {
    setRunning('bt')
    try {
      const res = await api.runStrategyBacktest(buildInput())
      setBtResult(res)
      setView('overview')
    } catch (err) {
      pushToast('error', String(err))
    } finally {
      setRunning(null)
    }
  }

  const handleRunWfo = async () => {
    setRunning('wfo')
    try {
      const res = await api.runStrategyWfo(buildInput(), 4, 0.7, false)
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
      const res = await api.runStrategyMonteCarlo(buildInput(), 1000, 50)
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
      {/* Compact Strategy Controls Header */}
      <div
        className="flex flex-wrap items-center gap-2 px-3 py-1.5 shrink-0"
        style={{ borderBottom: '1px solid var(--tv-border)', backgroundColor: 'var(--tv-bg-elevated)' }}
      >
        <select
          aria-label="Chiến lược Backtest"
          value={preset}
          onChange={(e) => setPreset(e.target.value as StrategyBacktestInput['preset'])}
          className="px-2 py-1 rounded text-[11.5px] font-medium outline-none"
          style={{
            backgroundColor: 'var(--tv-bg-input)',
            color: 'var(--tv-text-primary)',
            border: '1px solid var(--tv-border)',
          }}
        >
          {PRESETS.map((p) => (
            <option key={p.key} value={p.key}>
              {p.label}
            </option>
          ))}
        </select>

        <select
          aria-label="Chiều chiến lược"
          value={direction}
          onChange={(e) => setDirection(e.target.value as 'long' | 'short' | 'both')}
          className="px-1.5 py-1 rounded text-[11.5px] outline-none"
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

        <label className="flex items-center gap-1 text-[11px]" style={{ color: 'var(--tv-text-secondary)' }}>
          <span>Fast/Slow</span>
          <input
            type="number"
            value={fastSma}
            onChange={(e) => setFastSma(e.target.value)}
            className="w-13 px-1.5 py-0.5 rounded text-[11px] tabular-nums text-center no-spinner"
            style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
          />
          <input
            type="number"
            value={slowEma}
            onChange={(e) => setSlowEma(e.target.value)}
            className="w-13 px-1.5 py-0.5 rounded text-[11px] tabular-nums text-center no-spinner"
            style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
          />
        </label>

        <label className="flex items-center gap-1 text-[11px]" style={{ color: 'var(--tv-text-secondary)' }}>
          <span>SL/TP %</span>
          <input
            type="number"
            step="0.5"
            value={stopLossPct}
            onChange={(e) => setStopLossPct(e.target.value)}
            className="w-13 px-1.5 py-0.5 rounded text-[11px] tabular-nums text-center no-spinner"
            style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
          />
          <input
            type="number"
            step="0.5"
            value={takeProfitPct}
            onChange={(e) => setTakeProfitPct(e.target.value)}
            className="w-13 px-1.5 py-0.5 rounded text-[11px] tabular-nums text-center no-spinner"
            style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
          />
        </label>

        <label className="hidden xl:flex items-center gap-1 text-[11px]" style={{ color: 'var(--tv-text-secondary)' }}>
          <span>T+</span>
          <input
            type="number"
            min="0"
            max="10"
            value={minHold}
            onChange={(e) => setMinHold(e.target.value)}
            className="w-11 px-1.5 py-0.5 rounded text-[11px] tabular-nums text-center no-spinner"
            style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-primary)', border: '1px solid var(--tv-border)' }}
          />
        </label>

        <label className="hidden xl:flex items-center gap-1 text-[11px]" style={{ color: 'var(--tv-text-secondary)' }}>
          <span>Vốn</span>
          <input
            type="number"
            value={initialCash}
            onChange={(e) => setInitialCash(e.target.value)}
            className="w-28 px-1.5 py-0.5 rounded text-[11px] tabular-nums no-spinner"
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
            onClick={() => void handleRunWfo()}
            className="flex items-center gap-1 px-2 py-1 rounded text-[11.5px] font-medium border cursor-pointer disabled:opacity-50"
            style={{ backgroundColor: 'var(--tv-bg-panel)', borderColor: 'var(--tv-border)', color: 'var(--tv-text-primary)' }}
          >
            <BarChart3 className="w-3 h-3" />
            <span>{running === 'wfo' ? 'WFO...' : 'WFO 4-Fold'}</span>
          </button>
          <button
            type="button"
            disabled={running !== null}
            onClick={() => void handleRunMc()}
            className="flex items-center gap-1 px-2 py-1 rounded text-[11.5px] font-medium border cursor-pointer disabled:opacity-50"
            style={{ backgroundColor: 'var(--tv-bg-panel)', borderColor: 'var(--tv-border)', color: 'var(--tv-text-primary)' }}
          >
            <Dice5 className="w-3 h-3" />
            <span>{running === 'mc' ? 'MC...' : 'Monte Carlo'}</span>
          </button>
        </div>
      </div>

      {/* Sub-view Switcher & Tearsheet Summary Strip */}
      <div
        className="flex flex-wrap items-center justify-between gap-x-3 gap-y-1 px-3 py-1 shrink-0"
        style={{ minHeight: 32, borderBottom: '1px solid var(--tv-border)' }}
      >
        <div role="tablist" aria-label="Chế độ xem Strategy Tester" className="flex items-center h-7 gap-1">
          {(
            [
              { id: 'overview', label: 'Tổng quan & Đường cong vốn' },
              { id: 'trades', label: `Danh sách lệnh (${btResult?.trades.length ?? 0})` },
              { id: 'wfo', label: 'Walk-Forward (WFO)' },
              { id: 'mc', label: 'Monte Carlo Bootstrap' },
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
          <div className="flex items-center gap-3 text-[11px] tabular-nums">
            <span>
              <span style={{ color: 'var(--tv-text-secondary)' }}>Lợi nhuận: </span>
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
              <span style={{ color: 'var(--tv-text-secondary)' }}>Win Rate: </span>
              <strong>{ts.win_rate_pct.toFixed(1)}%</strong>
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
          <div className="grid grid-cols-1 lg:grid-cols-[1fr_260px] gap-3 h-full min-h-[150px]">
            <div
              className="rounded p-2 flex flex-col"
              style={{ backgroundColor: 'var(--tv-bg-chart)', border: '1px solid var(--tv-border)' }}
            >
              <div className="flex items-center justify-between text-[10.5px] mb-1" style={{ color: 'var(--tv-text-secondary)' }}>
                <span className="flex items-center gap-1">
                  <TrendingUp className="w-3.5 h-3.5" />
                  Đường cong vốn (Equity) & Sụt giảm (Underwater Drawdown) — {activePane.ticker} ({activePane.interval})
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
                <div style={{ color: 'var(--tv-text-secondary)' }}>Thắng / Thua</div>
                <div className="font-bold text-[13px]">
                  <span style={{ color: 'var(--tv-up)' }}>{ts?.num_winners ?? 0}</span> /{' '}
                  <span style={{ color: 'var(--tv-down)' }}>{ts?.num_losers ?? 0}</span>
                </div>
              </div>
              <div>
                <div style={{ color: 'var(--tv-text-secondary)' }}>TB Thắng / Thua</div>
                <div className="font-bold text-[12px]">
                  <span style={{ color: 'var(--tv-up)' }}>+{ts?.avg_win_pct.toFixed(1) ?? '0.0'}%</span> /{' '}
                  <span style={{ color: 'var(--tv-down)' }}>{ts?.avg_loss_pct.toFixed(1) ?? '0.0'}%</span>
                </div>
              </div>
              <div>
                <div style={{ color: 'var(--tv-text-secondary)' }}>Số nến giữ TB</div>
                <div className="font-bold text-[13px]">{ts ? ts.avg_bars_held.toFixed(1) : '—'}</div>
              </div>
              <div>
                <div style={{ color: 'var(--tv-text-secondary)' }}>Phí & Thuế</div>
                <div className="font-bold text-[12px]">{ts ? Math.round(ts.total_fees_and_tax).toLocaleString('en-US') : '—'}</div>
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
                <th scope="col">Ngày vào (Nhấp để xem nến)</th>
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

        {view === 'wfo' && (
          <div className="space-y-2">
            {!wfoResult ? (
              <div className="text-center py-6 text-[11.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
                Nhấn "WFO 4-Fold" để chạy tối ưu hóa Walk-Forward chống Overfitting từ lõi sumi-core.
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
                    OOS Return:{' '}
                    <strong
                      style={{
                        color: wfoResult.stitchedOosTearsheet.total_return_pct >= 0 ? 'var(--tv-up)' : 'var(--tv-down)',
                      }}
                    >
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
                Nhấn "Monte Carlo" để chạy 1,000 kịch bản Bootstrap SplitMix64 đánh giá rủi ro đuôi (VaR/CVaR).
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
                  <div style={{ color: 'var(--tv-text-secondary)' }}>Xác suất cháy (&gt;50% DD)</div>
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
