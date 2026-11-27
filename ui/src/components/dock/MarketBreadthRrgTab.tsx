import React, { useCallback, useEffect, useMemo, useState } from 'react'
import { Activity, Grid2x2, RefreshCw, Compass } from 'lucide-react'
import * as api from '../../api/tauri'
import { useWorkspaceStore } from '../../store/workspaceStore'
import type { MarketBreadthRrgResponse, RrgQuadrant, SymbolBreadthRotation } from '../../types/market'

const SCOPES = [
  { id: 'ALL_VN', label: 'Toàn VN' },
  { id: 'QMV60', label: 'QMV60' },
  { id: 'QMV30', label: 'QMV30' },
  { id: 'DT1', label: 'DT1' },
  { id: 'HOSE', label: 'HOSE' },
  { id: 'HNX', label: 'HNX' },
  { id: 'UPCOM', label: 'UPCOM' },
] as const

const QUAD_COLORS: Record<RrgQuadrant, string> = {
  Leading: '#089981',
  Weakening: '#FBC02D',
  Lagging: '#F23645',
  Improving: '#2962FF',
}

function heatTileBg(mode: 'change' | 'flow', changePct: number, buyFlowPct: number): string {
  if (mode === 'flow') {
    if (buyFlowPct >= 68) return 'rgba(8, 153, 129, 0.72)'
    if (buyFlowPct >= 56) return 'rgba(8, 153, 129, 0.42)'
    if (buyFlowPct <= 32) return 'rgba(242, 54, 69, 0.72)'
    if (buyFlowPct <= 44) return 'rgba(242, 54, 69, 0.42)'
    return 'rgba(120, 123, 134, 0.28)'
  }
  if (changePct >= 6.5) return 'rgba(186, 85, 211, 0.78)' // Trần tím VN
  if (changePct >= 2.5) return 'rgba(8, 153, 129, 0.72)'
  if (changePct > 0.05) return 'rgba(8, 153, 129, 0.40)'
  if (changePct <= -6.5) return 'rgba(0, 188, 212, 0.75)' // Sàn xanh lơ VN
  if (changePct <= -2.5) return 'rgba(242, 54, 69, 0.72)'
  if (changePct < -0.05) return 'rgba(242, 54, 69, 0.40)'
  return 'rgba(251, 192, 45, 0.30)' // Tham chiếu vàng
}

function fmtBillion(vnd: number): string {
  if (!Number.isFinite(vnd) || vnd <= 0) return '0 tỷ'
  const bn = vnd / 1_000_000_000
  if (bn >= 1000) return `${(bn / 1000).toFixed(1)} nghìn tỷ`
  return `${bn.toFixed(0)} tỷ`
}

const RrgScatterPlot: React.FC<{
  rotations: SymbolBreadthRotation[]
  selectedQuad: RrgQuadrant | 'ALL'
  onSelectSymbol: (ticker: string) => void
}> = ({ rotations, selectedQuad, onSelectSymbol }) => {
  const [hovered, setHovered] = useState<SymbolBreadthRotation | null>(null)

  const visible = useMemo(() => {
    const filtered =
      selectedQuad === 'ALL' ? rotations : rotations.filter((r) => r.quadrant === selectedQuad)
    // Show top 48 by traded value so chart stays crisp & readable
    return [...filtered]
      .sort((a, b) => b.traded_value - a.traded_value)
      .slice(0, 48)
  }, [rotations, selectedQuad])

  const W = 520
  const H = 220
  const pad = 26

  const { minX, maxX, minY, maxY } = useMemo(() => {
    let xLo = 96
    let xHi = 104
    let yLo = 96
    let yHi = 104
    for (const r of visible) {
      xLo = Math.min(xLo, r.rs_ratio)
      xHi = Math.max(xHi, r.rs_ratio)
      yLo = Math.min(yLo, r.rs_momentum)
      yHi = Math.max(yHi, r.rs_momentum)
      for (const [tx, ty] of r.trail) {
        xLo = Math.min(xLo, tx)
        xHi = Math.max(xHi, tx)
        yLo = Math.min(yLo, ty)
        yHi = Math.max(yHi, ty)
      }
    }
    const dx = Math.max(2, Math.max(100 - xLo, xHi - 100) * 1.12)
    const dy = Math.max(2, Math.max(100 - yLo, yHi - 100) * 1.12)
    return { minX: 100 - dx, maxX: 100 + dx, minY: 100 - dy, maxY: 100 + dy }
  }, [visible])

  const toX = (v: number) => pad + ((v - minX) / Math.max(1e-6, maxX - minX)) * (W - pad * 2)
  const toY = (v: number) => H - pad - ((v - minY) / Math.max(1e-6, maxY - minY)) * (H - pad * 2)
  const cx = toX(100)
  const cy = toY(100)

  return (
    <div
      className="relative rounded overflow-hidden h-full min-h-[180px]"
      style={{ backgroundColor: 'var(--tv-bg-chart)', border: '1px solid var(--tv-border)' }}
    >
      <svg viewBox={`0 0 ${W} ${H}`} className="w-full h-full">
        {/* 4 Quadrant tinted backgrounds */}
        <rect x={cx} y={pad} width={W - pad - cx} height={cy - pad} fill="rgba(8, 153, 129, 0.08)" />
        <rect x={cx} y={cy} width={W - pad - cx} height={H - pad - cy} fill="rgba(251, 192, 45, 0.07)" />
        <rect x={pad} y={cy} width={cx - pad} height={H - pad - cy} fill="rgba(242, 54, 69, 0.08)" />
        <rect x={pad} y={pad} width={cx - pad} height={cy - pad} fill="rgba(41, 98, 255, 0.08)" />

        {/* Crosshair at (100, 100) */}
        <line x1={cx} y1={pad} x2={cx} y2={H - pad} stroke="var(--tv-border)" strokeDasharray="3 3" />
        <line x1={pad} y1={cy} x2={W - pad} y2={cy} stroke="var(--tv-border)" strokeDasharray="3 3" />

        {/* Quadrant corner labels */}
        <text x={W - pad - 4} y={pad + 11} textAnchor="end" fontSize="9.5" fontWeight="700" fill="#089981">
          LEADING (Dẫn dắt)
        </text>
        <text x={W - pad - 4} y={H - pad - 4} textAnchor="end" fontSize="9.5" fontWeight="700" fill="#FBC02D">
          WEAKENING (Suy yếu)
        </text>
        <text x={pad + 4} y={H - pad - 4} textAnchor="start" fontSize="9.5" fontWeight="700" fill="#F23645">
          LAGGING (Tụt hậu)
        </text>
        <text x={pad + 4} y={pad + 11} textAnchor="start" fontSize="9.5" fontWeight="700" fill="#2962FF">
          IMPROVING (Cải thiện)
        </text>

        {/* Trails & Dots */}
        {visible.map((r) => {
          const color = QUAD_COLORS[r.quadrant] ?? '#94A3B8'
          const px = toX(r.rs_ratio)
          const py = toY(r.rs_momentum)
          const isHov = hovered?.instrument_id === r.instrument_id
          const pts =
            r.trail.length >= 2
              ? r.trail.map(([tx, ty]) => `${toX(tx).toFixed(1)},${toY(ty).toFixed(1)}`).join(' ')
              : ''
          return (
            <g
              key={r.instrument_id}
              className="cursor-pointer"
              onMouseEnter={() => setHovered(r)}
              onMouseLeave={() => setHovered((cur) => (cur?.instrument_id === r.instrument_id ? null : cur))}
              onClick={() => onSelectSymbol(r.instrument_id)}
            >
              {pts && (
                <polyline
                  points={pts}
                  fill="none"
                  stroke={color}
                  strokeOpacity={isHov ? 0.95 : 0.42}
                  strokeWidth={isHov ? 1.8 : 1.1}
                />
              )}
              <circle cx={px} cy={py} r={isHov ? 5 : 3.4} fill={color} stroke="#0D111A" strokeWidth="1" />
              <text
                x={px + 5}
                y={py + 3}
                fontSize={isHov ? '10' : '8.5'}
                fontWeight={isHov ? '700' : '600'}
                fill={isHov ? '#FFFFFF' : 'var(--tv-text-primary)'}
              >
                {r.symbol}
              </text>
            </g>
          )
        })}

        {/* Axis Captions */}
        <text x={W / 2} y={H - 6} textAnchor="middle" fontSize="9" fill="var(--tv-text-secondary)">
          JdK RS-Ratio (Sức mạnh tương đối vs VNINDEX) →
        </text>
      </svg>

      {hovered && (
        <div
          className="absolute top-2 left-2 px-2 py-1 rounded text-[10.5px] tabular-nums pointer-events-none shadow-lg"
          style={{
            backgroundColor: 'var(--tv-bg-elevated)',
            border: '1px solid var(--tv-border)',
            color: 'var(--tv-text-primary)',
          }}
        >
          <div className="font-bold">
            {hovered.symbol}{' '}
            <span style={{ color: QUAD_COLORS[hovered.quadrant] }}>({hovered.quadrant})</span>
          </div>
          <div>
            Giá: {hovered.last_close.toLocaleString('en-US')} (
            <span style={{ color: hovered.return_1bar_pct >= 0 ? 'var(--tv-up)' : 'var(--tv-down)' }}>
              {hovered.return_1bar_pct >= 0 ? '+' : ''}
              {hovered.return_1bar_pct.toFixed(2)}%
            </span>
            ) · Flow Buy: {hovered.buy_flow_ratio_pct.toFixed(0)}%
          </div>
          <div style={{ color: 'var(--tv-text-secondary)' }}>
            RS-Ratio: {hovered.rs_ratio.toFixed(2)} · RS-Mom: {hovered.rs_momentum.toFixed(2)} · Nhấp để mở biểu đồ
          </div>
        </div>
      )}
    </div>
  )
}

export const MarketBreadthRrgTab: React.FC = () => {
  const setActiveTicker = useWorkspaceStore((s) => s.setActiveTicker)
  const yumekoAsOf = useWorkspaceStore((s) => s.yumekoAsOf)
  const replay = useWorkspaceStore((s) => s.replay)
  const pushToast = useWorkspaceStore((s) => s.pushToast)

  const [scope, setScope] = useState<string>('QMV60')
  const [subMode, setSubMode] = useState<'rrg' | 'heatmap'>('rrg')
  const [heatMetric, setHeatMetric] = useState<'change' | 'flow'>('change')
  const [selectedQuad, setSelectedQuad] = useState<RrgQuadrant | 'ALL'>('ALL')
  const [loading, setLoading] = useState(false)
  const [data, setData] = useState<MarketBreadthRrgResponse | null>(null)

  const loadData = useCallback(async () => {
    setLoading(true)
    try {
      const resp = await api.getMarketBreadthAndRrg(scope, yumekoAsOf)
      setData(resp)
    } catch (err) {
      pushToast('error', `Lỗi tính Thị trường & RRG: ${String(err)}`)
    } finally {
      setLoading(false)
    }
  }, [scope, yumekoAsOf, pushToast])

  useEffect(() => {
    void loadData()
  }, [loadData, replay?.cutoffTimestampMs])

  const snap = data?.snapshot

  return (
    <div className="flex flex-col h-full min-h-0 overflow-hidden">
      {/* Top Toolbar: Scope + View Toggle + Breadth KPI Strip */}
      <div
        className="flex flex-wrap items-center justify-between gap-2 px-3 py-1.5 shrink-0"
        style={{ borderBottom: '1px solid var(--tv-border)', backgroundColor: 'var(--tv-bg-elevated)' }}
      >
        <div className="flex items-center gap-1.5 flex-wrap">
          <div className="flex items-center gap-0.5">
            {SCOPES.map((s) => (
              <button
                key={s.id}
                type="button"
                className="px-2 py-0.5 rounded text-[10.5px] font-semibold border cursor-pointer"
                style={{
                  backgroundColor: scope === s.id ? 'var(--tv-accent-soft)' : 'transparent',
                  borderColor: scope === s.id ? 'var(--tv-accent)' : 'var(--tv-border)',
                  color: scope === s.id ? 'var(--tv-accent)' : 'var(--tv-text-secondary)',
                }}
                onClick={() => setScope(s.id)}
              >
                {s.label}
              </button>
            ))}
          </div>

          <div className="flex items-center gap-0.5 ml-1">
            <button
              type="button"
              className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[10.5px] font-semibold border cursor-pointer"
              style={{
                backgroundColor: subMode === 'rrg' ? 'var(--tv-accent)' : 'transparent',
                borderColor: subMode === 'rrg' ? 'var(--tv-accent)' : 'var(--tv-border)',
                color: subMode === 'rrg' ? '#FFFFFF' : 'var(--tv-text-primary)',
              }}
              onClick={() => setSubMode('rrg')}
            >
              <Compass className="w-3 h-3" />
              <span>RRG 4 Góc Phần Tư</span>
            </button>
            <button
              type="button"
              className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[10.5px] font-semibold border cursor-pointer"
              style={{
                backgroundColor: subMode === 'heatmap' ? 'var(--tv-accent)' : 'transparent',
                borderColor: subMode === 'heatmap' ? 'var(--tv-accent)' : 'var(--tv-border)',
                color: subMode === 'heatmap' ? '#FFFFFF' : 'var(--tv-text-primary)',
              }}
              onClick={() => setSubMode('heatmap')}
            >
              <Grid2x2 className="w-3 h-3" />
              <span>Bản đồ 11 Ngành QMV</span>
            </button>
          </div>

          {subMode === 'heatmap' && (
            <select
              aria-label="Màu bản đồ nhiệt ngành"
              value={heatMetric}
              onChange={(e) => setHeatMetric(e.target.value as 'change' | 'flow')}
              className="px-1.5 py-0.5 rounded text-[10.5px] outline-none"
              style={{
                backgroundColor: 'var(--tv-bg-input)',
                color: 'var(--tv-text-primary)',
                border: '1px solid var(--tv-border)',
              }}
            >
              <option value="change">Tô màu theo % Đổi</option>
              <option value="flow">Tô màu theo Dòng tiền Mua %</option>
            </select>
          )}
        </div>

        {/* Causal Breadth KPI Strip */}
        <div className="flex items-center gap-3 text-[11px] tabular-nums flex-wrap">
          {data?.replayLocked && (
            <span className="px-1.5 py-0.5 rounded text-[9.5px] font-bold bg-[#2962FF] text-white">
              REPLAY CAUSAL
            </span>
          )}
          {snap && (
            <>
              <span>
                <span style={{ color: 'var(--tv-text-secondary)' }}>Tăng/Giảm: </span>
                <strong style={{ color: 'var(--tv-up)' }}>{snap.advancers}</strong>/
                <strong style={{ color: 'var(--tv-down)' }}>{snap.decliners}</strong>
                <span style={{ color: 'var(--tv-text-secondary)' }}> ({snap.unchanged})</span>
              </span>
              <span>
                <span style={{ color: 'var(--tv-text-secondary)' }}>&gt;SMA20: </span>
                <strong>{snap.pct_above_sma20.toFixed(0)}%</strong>
              </span>
              <span>
                <span style={{ color: 'var(--tv-text-secondary)' }}>&gt;EMA50: </span>
                <strong>{snap.pct_above_ema50.toFixed(0)}%</strong>
              </span>
              <span>
                <span style={{ color: 'var(--tv-text-secondary)' }}>Dòng tiền Mua: </span>
                <strong style={{ color: snap.money_flow_buy_pct >= 50 ? 'var(--tv-up)' : 'var(--tv-down)' }}>
                  {snap.money_flow_buy_pct.toFixed(1)}%
                </strong>
                <span style={{ color: 'var(--tv-text-secondary)' }}>
                  {' '}
                  (VN30: {snap.vn30_money_flow_buy_pct.toFixed(1)}%)
                </span>
              </span>
            </>
          )}
          <button
            type="button"
            className="tv-icon-btn w-6 h-6"
            onClick={() => void loadData()}
            disabled={loading}
            title="Tính lại Độ rộng & RRG"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin' : ''}`} />
          </button>
        </div>
      </div>

      {/* Main Content Area */}
      <div className="flex-1 min-h-0 overflow-auto tv-scroll p-2.5">
        {loading && !data ? (
          <div className="flex items-center justify-center h-full gap-2 text-[12px]" style={{ color: 'var(--tv-text-secondary)' }}>
            <Activity className="w-4 h-4 animate-pulse" />
            <span>Đang tổng hợp độ rộng thị trường & RRG nhân quả so với HOSE:VNINDEX...</span>
          </div>
        ) : !snap ? (
          <div className="text-center py-6 text-[11.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
            Chưa có dữ liệu độ rộng thị trường. Hãy đồng bộ dữ liệu EOD trong cài đặt.
          </div>
        ) : subMode === 'rrg' ? (
          <div className="grid grid-cols-1 lg:grid-cols-[1.25fr_1fr] gap-2.5 h-full min-h-[190px]">
            <RrgScatterPlot
              rotations={snap.rotations}
              selectedQuad={selectedQuad}
              onSelectSymbol={(t) => setActiveTicker(t)}
            />

            <div className="flex flex-col min-h-0 overflow-hidden rounded" style={{ border: '1px solid var(--tv-border)' }}>
              {/* Quadrant filter pills */}
              <div
                className="flex items-center gap-1 px-2 py-1 text-[10.5px] shrink-0 flex-wrap"
                style={{ backgroundColor: 'var(--tv-bg-elevated)', borderBottom: '1px solid var(--tv-border)' }}
              >
                {(
                  [
                    { id: 'ALL', label: `Tất cả (${snap.participating_count})`, color: 'var(--tv-text-primary)' },
                    { id: 'Leading', label: `Leading (${snap.quadrant_counts[0]})`, color: '#089981' },
                    { id: 'Weakening', label: `Weak (${snap.quadrant_counts[1]})`, color: '#FBC02D' },
                    { id: 'Lagging', label: `Lagging (${snap.quadrant_counts[2]})`, color: '#F23645' },
                    { id: 'Improving', label: `Impr (${snap.quadrant_counts[3]})`, color: '#2962FF' },
                  ] as const
                ).map((q) => (
                  <button
                    key={q.id}
                    type="button"
                    onClick={() => setSelectedQuad(q.id)}
                    className="px-1.5 py-0.5 rounded font-semibold border cursor-pointer"
                    style={{
                      backgroundColor: selectedQuad === q.id ? 'var(--tv-hover)' : 'transparent',
                      borderColor: selectedQuad === q.id ? q.color : 'transparent',
                      color: q.color,
                    }}
                  >
                    {q.label}
                  </button>
                ))}
              </div>
              <div className="flex-1 overflow-auto tv-scroll">
                <table className="tv-table tabular-nums">
                  <thead>
                    <tr>
                      <th scope="col">Mã</th>
                      <th scope="col">Góc RRG</th>
                      <th scope="col" className="text-right">RS-Ratio</th>
                      <th scope="col" className="text-right">RS-Mom</th>
                      <th scope="col" className="text-right">% Đổi</th>
                      <th scope="col" className="text-right">Flow Mua</th>
                    </tr>
                  </thead>
                  <tbody>
                    {snap.rotations
                      .filter((r) => selectedQuad === 'ALL' || r.quadrant === selectedQuad)
                      .slice(0, 80)
                      .map((r) => (
                        <tr
                          key={r.instrument_id}
                          className="cursor-pointer"
                          onClick={() => setActiveTicker(r.instrument_id)}
                        >
                          <td className="font-bold text-[#2962FF]">{r.symbol}</td>
                          <td>
                            <span
                              className="px-1.5 py-0 rounded text-[9.5px] font-bold"
                              style={{
                                backgroundColor: `${QUAD_COLORS[r.quadrant]}22`,
                                color: QUAD_COLORS[r.quadrant],
                              }}
                            >
                              {r.quadrant}
                            </span>
                          </td>
                          <td className="text-right">{r.rs_ratio.toFixed(2)}</td>
                          <td className="text-right">{r.rs_momentum.toFixed(2)}</td>
                          <td
                            className="text-right font-semibold"
                            style={{ color: r.return_1bar_pct >= 0 ? 'var(--tv-up)' : 'var(--tv-down)' }}
                          >
                            {r.return_1bar_pct >= 0 ? '+' : ''}
                            {r.return_1bar_pct.toFixed(2)}%
                          </td>
                          <td className="text-right">{r.buy_flow_ratio_pct.toFixed(0)}%</td>
                        </tr>
                      ))}
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        ) : (
          /* 11-Sector QMV Interactive Treemap Heatmap */
          <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-2">
            {(data?.sectors ?? []).map((sec) => (
              <div
                key={sec.sectorCode}
                className="rounded p-2 flex flex-col gap-1.5"
                style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}
              >
                <div className="flex items-center justify-between gap-1 text-[11px]">
                  <div className="min-w-0">
                    <div className="font-bold truncate">{sec.sectorLabel}</div>
                    <div className="text-[9.5px] truncate" style={{ color: 'var(--tv-text-secondary)' }}>
                      {sec.stockCount} mã · GTGD {fmtBillion(sec.totalTradedValue)} · Flow {sec.avgBuyFlowPct.toFixed(0)}%
                    </div>
                  </div>
                  <div className="text-right tabular-nums shrink-0">
                    <div
                      className="font-bold text-[12px]"
                      style={{ color: sec.weightedChangePct >= 0 ? 'var(--tv-up)' : 'var(--tv-down)' }}
                    >
                      {sec.weightedChangePct >= 0 ? '+' : ''}
                      {sec.weightedChangePct.toFixed(2)}%
                    </div>
                    <div className="text-[9.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
                      <span style={{ color: 'var(--tv-up)' }}>{sec.advancers}↑</span>{' '}
                      <span style={{ color: 'var(--tv-down)' }}>{sec.decliners}↓</span>
                    </div>
                  </div>
                </div>

                <div className="grid grid-cols-4 sm:grid-cols-5 gap-1">
                  {sec.stocks.slice(0, 10).map((st) => (
                    <button
                      key={st.ticker}
                      type="button"
                      onClick={() => setActiveTicker(st.ticker)}
                      title={`${st.ticker}: ${st.lastClose.toLocaleString('en-US')} (${st.changePct >= 0 ? '+' : ''}${st.changePct.toFixed(2)}%) · Flow Mua ${st.buyFlowRatioPct.toFixed(0)}% · RRG ${st.quadrant}`}
                      className="rounded px-1 py-1 text-center border-0 cursor-pointer transition-opacity hover:opacity-90"
                      style={{
                        backgroundColor: heatTileBg(heatMetric, st.changePct, st.buyFlowRatioPct),
                        color: '#FFFFFF',
                      }}
                    >
                      <div className="text-[10.5px] font-bold leading-tight truncate">{st.symbol}</div>
                      <div className="text-[9.5px] tabular-nums leading-tight">
                        {heatMetric === 'flow'
                          ? `${st.buyFlowRatioPct.toFixed(0)}%`
                          : `${st.changePct >= 0 ? '+' : ''}${st.changePct.toFixed(1)}%`}
                      </div>
                    </button>
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
