import React, { useEffect, useRef, useState } from 'react'
import { Award, CheckCircle2, AlertTriangle, Info, ShieldAlert, X } from 'lucide-react'
import * as api from '../../api/tauri'
import { useWorkspaceStore } from '../../store/workspaceStore'
import type { SessionDebriefReport } from '../../types/market'

export const SessionDebriefModal: React.FC = () => {
  const open = useWorkspaceStore((s) => s.debriefModalOpen)
  const setOpen = useWorkspaceStore((s) => s.setDebriefModalOpen)
  const panes = useWorkspaceStore((s) => s.panes)
  const activeSlotId = useWorkspaceStore((s) => s.activeSlotId)
  const replay = useWorkspaceStore((s) => s.replay)

  const activePane = panes.find((p) => p.slotId === activeSlotId) ?? panes[0]
  const dialogRef = useRef<HTMLDialogElement | null>(null)

  const [report, setReport] = useState<SessionDebriefReport | null>(null)
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    const dlg = dialogRef.current
    if (!dlg) return
    if (open) {
      if (!dlg.open) dlg.showModal()
      setLoading(true)
      void api
        .computeReplayDebrief(replay?.ticker || activePane.ticker, (replay?.interval as typeof activePane.interval) || activePane.interval)
        .then(setReport)
        .catch(() => setReport(null))
        .finally(() => setLoading(false))
    } else if (dlg.open) {
      dlg.close()
    }
  }, [open, replay?.ticker, replay?.interval, activePane.ticker, activePane.interval])

  useEffect(() => {
    const dlg = dialogRef.current
    if (!dlg) return
    const onClose = () => setOpen(false)
    dlg.addEventListener('close', onClose)
    return () => dlg.removeEventListener('close', onClose)
  }, [setOpen])

  const isVnd = (report?.symbol ?? activePane.ticker).startsWith('HOSE:') ||
    (report?.symbol ?? activePane.ticker).startsWith('HNX:') ||
    (report?.symbol ?? activePane.ticker).startsWith('UPCOM:')

  const fmtPnl = (v: number) =>
    isVnd
      ? `${v >= 0 ? '+' : ''}${Math.round(v).toLocaleString('en-US')} ₫`
      : `${v >= 0 ? '+' : '-'}$${Math.abs(v).toFixed(2)}`

  return (
    <dialog
      ref={dialogRef}
      className="tv-dialog"
      style={{ maxWidth: 'min(94vw, 700px)' }}
      aria-labelledby="debrief-modal-title"
      {...({ closedby: 'any' } as Record<string, string>)}
    >
      <div
        className="flex items-center justify-between px-4 py-3 shrink-0"
        style={{ borderBottom: '1px solid var(--tv-border)' }}
      >
        <div className="flex items-center gap-2">
          <Award className="w-4 h-4 text-[#FBC02D]" />
          <div>
            <h2 id="debrief-modal-title" className="m-0 text-[14.5px] font-semibold">
              Tổng Kết Phiên Luyện Tập & Kỷ Luật Thực Thi — {report?.symbol ?? activePane.ticker}
            </h2>
            <div className="text-[11px]" style={{ color: 'var(--tv-text-secondary)' }}>
              Báo cáo huấn luyện nhân quả `SessionDebriefReport` (Tuân thủ SL, Chi phí Sai lầm & Hiệu suất Setup)
            </div>
          </div>
        </div>
        <button type="button" className="tv-icon-btn w-7 h-7" onClick={() => setOpen(false)} aria-label="Đóng Tổng kết">
          <X className="w-4 h-4" />
        </button>
      </div>

      <div className="p-4 space-y-3 overflow-y-auto tv-scroll max-h-[75vh]">
        {loading ? (
          <div className="text-center py-8 text-[12px]" style={{ color: 'var(--tv-text-secondary)' }}>
            Đang tổng hợp báo cáo kỷ luật phiên luyện tập...
          </div>
        ) : !report ? (
          <div className="text-center py-8 text-[12px]" style={{ color: 'var(--tv-text-secondary)' }}>
            Không thể tải dữ liệu tổng kết phiên.
          </div>
        ) : (
          <>
            {/* KPI Grid */}
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 tabular-nums">
              <div className="p-2.5 rounded" style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}>
                <div className="text-[10.5px]" style={{ color: 'var(--tv-text-secondary)' }}>Net PnL ({report.closed_trades} lệnh)</div>
                <div className="text-[14px] font-bold" style={{ color: report.total_net_pnl >= 0 ? 'var(--tv-up)' : 'var(--tv-down)' }}>
                  {fmtPnl(report.total_net_pnl)}
                </div>
                <div className="text-[10px]" style={{ color: 'var(--tv-text-secondary)' }}>
                  {report.total_return_pct >= 0 ? '+' : ''}{report.total_return_pct.toFixed(2)}%
                </div>
              </div>
              <div className="p-2.5 rounded" style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}>
                <div className="text-[10.5px]" style={{ color: 'var(--tv-text-secondary)' }}>Tỷ lệ thắng / PF</div>
                <div className="text-[14px] font-bold">
                  {report.win_rate_pct.toFixed(0)}%{' '}
                  <span className="text-[11px] font-normal" style={{ color: 'var(--tv-text-secondary)' }}>
                    ({report.win_count}W/{report.loss_count}L)
                  </span>
                </div>
                <div className="text-[10px]" style={{ color: 'var(--tv-text-secondary)' }}>
                  PF: {report.profit_factor != null ? report.profit_factor.toFixed(2) : '—'}
                </div>
              </div>
              <div className="p-2.5 rounded" style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}>
                <div className="text-[10.5px]" style={{ color: 'var(--tv-text-secondary)' }}>Tuân thủ Cắt lỗ (SL)</div>
                <div
                  className="text-[14px] font-bold"
                  style={{
                    color:
                      report.sl_adherence_pct == null
                        ? 'var(--tv-text-secondary)'
                        : report.sl_adherence_pct >= 80
                          ? 'var(--tv-up)'
                          : '#FBC02D',
                  }}
                >
                  {report.sl_adherence_pct != null ? `${report.sl_adherence_pct.toFixed(0)}%` : '—'}
                </div>
                <div className="text-[10px]" style={{ color: 'var(--tv-text-secondary)' }}>
                  {report.sl_adherent_positions}/{report.total_opened_positions} vị thế có SL ban đầu
                </div>
              </div>
              <div className="p-2.5 rounded" style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}>
                <div className="text-[10.5px]" style={{ color: 'var(--tv-text-secondary)' }}>Tổng R / Max DD</div>
                <div className="text-[14px] font-bold">
                  {report.net_r_multiple >= 0 ? '+' : ''}{report.net_r_multiple.toFixed(2)}R
                </div>
                <div className="text-[10px] text-[#F23645]">
                  DD: -{report.max_drawdown_pct.toFixed(2)}%
                </div>
              </div>
            </div>

            {/* Coaching Callouts */}
            {report.callouts.length > 0 && (
              <div className="space-y-1.5">
                <div className="text-[11px] font-semibold uppercase tracking-wide" style={{ color: 'var(--tv-text-secondary)' }}>
                  Nhận xét Huấn luyện (Coaching Callouts)
                </div>
                {report.callouts.map((c, idx) => {
                  const tone =
                    c.severity === 'Positive'
                      ? { color: '#089981', bg: 'rgba(8, 153, 129, 0.12)', Icon: CheckCircle2 }
                      : c.severity === 'Critical'
                        ? { color: '#F23645', bg: 'rgba(242, 54, 69, 0.12)', Icon: ShieldAlert }
                        : c.severity === 'Caution'
                          ? { color: '#FBC02D', bg: 'rgba(251, 192, 45, 0.12)', Icon: AlertTriangle }
                          : { color: '#2962FF', bg: 'rgba(41, 98, 255, 0.12)', Icon: Info }
                  const Icon = tone.Icon
                  return (
                    <div
                      key={idx}
                      className="flex items-start gap-2 p-2.5 rounded text-[11.5px]"
                      style={{ backgroundColor: tone.bg, border: `1px solid ${tone.color}55` }}
                    >
                      <Icon className="w-4 h-4 shrink-0 mt-0.5" style={{ color: tone.color }} />
                      <div>
                        <div className="font-semibold" style={{ color: tone.color }}>{c.title}</div>
                        <div style={{ color: 'var(--tv-text-primary)' }}>{c.detail}</div>
                      </div>
                    </div>
                  )
                })}
              </div>
            )}

            {/* Setup Edge & Cost of Mistakes side-by-side */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              <div className="rounded overflow-hidden" style={{ border: '1px solid var(--tv-border)' }}>
                <div className="px-2.5 py-1.5 text-[11px] font-semibold" style={{ backgroundColor: 'var(--tv-bg-elevated)', borderBottom: '1px solid var(--tv-border)' }}>
                  Hiệu suất theo Setup ({report.setup_breakdown.length})
                </div>
                {report.setup_breakdown.length === 0 ? (
                  <div className="p-3 text-[11px] text-center" style={{ color: 'var(--tv-text-secondary)' }}>
                    Chưa gắn thẻ Setup cho lệnh trong Nhật ký.
                  </div>
                ) : (
                  <table className="tv-table tabular-nums">
                    <thead>
                      <tr>
                        <th scope="col">Setup</th>
                        <th scope="col" className="text-right">Lệnh</th>
                        <th scope="col" className="text-right">Win%</th>
                        <th scope="col" className="text-right">Net PnL</th>
                      </tr>
                    </thead>
                    <tbody>
                      {report.setup_breakdown.map((s) => (
                        <tr key={s.setup}>
                          <td className="font-medium">{s.setup}</td>
                          <td className="text-right">{s.trade_count}</td>
                          <td className="text-right">{s.win_rate_pct.toFixed(0)}%</td>
                          <td className="text-right font-semibold" style={{ color: s.net_pnl >= 0 ? 'var(--tv-up)' : 'var(--tv-down)' }}>
                            {fmtPnl(s.net_pnl)}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
              </div>

              <div className="rounded overflow-hidden" style={{ border: '1px solid var(--tv-border)' }}>
                <div className="px-2.5 py-1.5 text-[11px] font-semibold" style={{ backgroundColor: 'var(--tv-bg-elevated)', borderBottom: '1px solid var(--tv-border)' }}>
                  Chi phí Sai lầm Tâm lý ({report.mistake_breakdown.length})
                </div>
                {report.mistake_breakdown.length === 0 ? (
                  <div className="p-3 text-[11px] text-center" style={{ color: 'var(--tv-text-secondary)' }}>
                    Không phát hiện lỗi kỷ luật được gắn thẻ.
                  </div>
                ) : (
                  <table className="tv-table tabular-nums">
                    <thead>
                      <tr>
                        <th scope="col">Lỗi hành vi</th>
                        <th scope="col" className="text-right">Số lần</th>
                        <th scope="col" className="text-right">Thiệt hại</th>
                      </tr>
                    </thead>
                    <tbody>
                      {report.mistake_breakdown.map((m) => (
                        <tr key={m.mistake}>
                          <td className="font-medium">{m.mistake}</td>
                          <td className="text-right">{m.trade_count}</td>
                          <td className="text-right font-semibold text-[#F23645]">
                            -{fmtPnl(m.adverse_cost_abs).replace(/^[+-]/, '')}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
              </div>
            </div>
          </>
        )}
      </div>

      <div className="flex items-center justify-end px-4 py-2.5 shrink-0" style={{ borderTop: '1px solid var(--tv-border)' }}>
        <button
          type="button"
          className="px-3.5 py-1.5 rounded text-[12px] font-semibold bg-[#2962FF] text-white border-0 cursor-pointer"
          onClick={() => setOpen(false)}
        >
          Đã hiểu & Tiếp tục
        </button>
      </div>
    </dialog>
  )
}
