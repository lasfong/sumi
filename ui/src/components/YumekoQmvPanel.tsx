import React, { useState } from 'react'
import {
  AlertTriangle,
  Briefcase,
  CheckCircle2,
  ClipboardCopy,
  FileCheck2,
  FolderCheck,
  Plus,
  RefreshCw,
  ShieldCheck,
  Star,
} from 'lucide-react'
import * as api from '../api/tauri'
import { useWorkspaceStore } from '../store/workspaceStore'
import type { Kt01GateVerdict, ResearchExportVerificationDto } from '../types/market'

function fmtVnd(v: number | null | undefined): string {
  if (v === null || v === undefined || !Number.isFinite(v)) return '—'
  return Math.round(v).toLocaleString('en-US')
}

function fmtNum(v: number | null | undefined, digits = 2): string {
  if (v === null || v === undefined || !Number.isFinite(v)) return '—'
  return v.toFixed(digits)
}

function verdictStyle(v: Kt01GateVerdict): { bg: string; color: string } {
  switch (v) {
    case 'PASS':
      return { bg: 'rgba(8, 153, 129, 0.2)', color: '#089981' }
    case 'WAIT_PULLBACK':
      return { bg: 'rgba(251, 192, 45, 0.2)', color: '#FBC02D' }
    case 'REJECT':
      return { bg: 'rgba(242, 54, 69, 0.2)', color: '#F23645' }
    default:
      return { bg: 'var(--tv-accent-soft)', color: 'var(--tv-accent)' }
  }
}

export const YumekoQmvPanel: React.FC = () => {
  const panes = useWorkspaceStore((s) => s.panes)
  const activeSlotId = useWorkspaceStore((s) => s.activeSlotId)
  const yumekoStatus = useWorkspaceStore((s) => s.yumekoStatus)
  const yumekoAsOf = useWorkspaceStore((s) => s.yumekoAsOf)
  const yumekoContext = useWorkspaceStore((s) => s.yumekoContext)
  const yumekoQmvSeries = useWorkspaceStore((s) => s.yumekoQmvSeries)
  const yumekoLoading = useWorkspaceStore((s) => s.yumekoLoading)
  const exportingKt01 = useWorkspaceStore((s) => s.exportingKt01)
  const exportedVerdicts = useWorkspaceStore((s) => s.exportedVerdicts)
  const lastExportedReceipt = useWorkspaceStore((s) => s.lastExportedReceipt)

  const refreshYumekoStatus = useWorkspaceStore((s) => s.refreshYumekoStatus)
  const loadYumekoForActiveTicker = useWorkspaceStore((s) => s.loadYumekoForActiveTicker)
  const exportActiveKt01Verdict = useWorkspaceStore((s) => s.exportActiveKt01Verdict)
  const addIndicatorToActivePane = useWorkspaceStore((s) => s.addIndicatorToActivePane)
  const pushToast = useWorkspaceStore((s) => s.pushToast)

  const [exportCheckDir, setExportCheckDir] = useState('')
  const [exportCheckRes, setExportCheckRes] = useState<ResearchExportVerificationDto | null>(null)
  const [checkingDir, setCheckingDir] = useState(false)

  const activePane = panes.find((p) => p.slotId === activeSlotId) ?? panes[0]
  const bb = yumekoContext?.hp01Blackbox ?? null
  const isReal = yumekoContext?.flowProvenance === 'qmv_real' && Boolean(bb?.isProviderProprietary)
  const vm01 = yumekoContext?.vm01Market ?? null
  const holding = yumekoContext?.dm01RealHolding ?? null
  const wl = yumekoContext?.dm01YumekoWatchlist ?? null
  const vals = yumekoContext?.dn01Valuations ?? []
  const theses = yumekoContext?.episodicTheses ?? []

  const handleCopyMarkdown = async () => {
    const md = lastExportedReceipt?.markdownSummary
    if (!md) return
    try {
      await navigator.clipboard.writeText(md)
      pushToast('success', 'Đã sao chép KT01 Markdown vào Clipboard (dán thẳng vào agy acp / Yumeko)')
    } catch {
      pushToast('warning', 'Không thể truy cập Clipboard — hãy mở tệp Markdown từ đường dẫn đã xuất')
    }
  }

  const handleQuickAddQmvIndicator = (name: 'QMV_STOCK_BB' | 'QMV_ACTIVE_MONEY' | 'QMV_FOREIGN_FLOW') => {
    const exists = activePane.indicators.some((i) => i.name === name)
    if (exists) {
      pushToast('info', `Chỉ báo ${name} đã được bật trên khung biểu đồ hiện tại`)
      return
    }
    addIndicatorToActivePane({
      name,
      paneId: `pane_${name.toLowerCase()}`,
      isMain: false,
      calcParams: [],
      visible: true,
    })
    pushToast('success', `Đã thêm chỉ báo ${name} lên khung phụ`)
  }

  const handleVerifyResearchDir = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!exportCheckDir.trim() || checkingDir) return
    setCheckingDir(true)
    try {
      const res = await api.yumekoVerifyResearchExport(exportCheckDir.trim())
      setExportCheckRes(res)
      pushToast(
        res.valid ? 'success' : 'warning',
        res.valid
          ? `Xác thực SHA-256 hợp lệ (${res.verifiedFiles} tệp · as_of=${res.asOf ?? '—'})`
          : `Phát hiện ${res.failedFiles.length} lỗi kiểm chứng SHA-256`,
      )
    } catch (err) {
      pushToast('error', String(err))
    } finally {
      setCheckingDir(false)
    }
  }

  return (
    <div
      role="tabpanel"
      aria-labelledby="rw-tab-yumeko"
      className="flex flex-col flex-1 min-h-0 overflow-y-auto tv-scroll p-2 space-y-2.5 text-[11px] tabular-nums"
    >
      {/* 1. Read-Only Bridge Status Header */}
      <div
        className="p-2 rounded space-y-1.5"
        style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}
      >
        <div className="flex items-center justify-between gap-1">
          <div className="flex items-center gap-1.5 min-w-0">
            <ShieldCheck
              className="w-3.5 h-3.5 shrink-0"
              style={{ color: yumekoStatus?.available ? '#089981' : '#FBC02D' }}
            />
            <span className="font-semibold text-[11.5px] truncate">
              Yumeko Top-Down ({yumekoContext?.symbol ?? activePane.ticker.split(':')[1] ?? activePane.ticker})
            </span>
          </div>
          <button
            type="button"
            className="tv-icon-btn w-6 h-6 shrink-0"
            title="Tải lại ngữ cảnh Yumeko Chỉ Đọc"
            onClick={() => {
              void refreshYumekoStatus()
              void loadYumekoForActiveTicker()
            }}
          >
            <RefreshCw className={`w-3 h-3 ${yumekoLoading ? 'animate-spin' : ''}`} />
          </button>
        </div>

        <div className="flex flex-wrap items-center gap-1 text-[10px]">
          <span
            className="px-1.5 py-0.5 rounded font-semibold"
            style={{
              backgroundColor: isReal ? 'rgba(8, 153, 129, 0.16)' : 'rgba(251, 192, 45, 0.16)',
              color: isReal ? '#089981' : '#FBC02D',
            }}
          >
            {isReal ? '✓ QMV Blackbox Thật' : '⚠ OHLCV Proxy Fallback'}
          </span>
          <span
            className="px-1.5 py-0.5 rounded"
            style={{ backgroundColor: 'var(--tv-hover)', color: 'var(--tv-text-secondary)' }}
          >
            as_of: {yumekoAsOf ?? yumekoContext?.effectiveAsOf ?? 'Mới nhất'}
          </span>
          {yumekoContext?.dnsePitAvailable && (
            <span
              className="px-1.5 py-0.5 rounded font-medium"
              style={{ backgroundColor: 'var(--tv-accent-soft)', color: 'var(--tv-accent)' }}
            >
              DNSE PIT ✓
            </span>
          )}
        </div>

        {!yumekoStatus?.available && (
          <div className="text-[10px]" style={{ color: 'var(--tv-text-secondary)' }}>
            {yumekoStatus?.note ||
              'Chưa tìm thấy ~/.local/share/yumeko/yumeko-app.sqlite. Sumi vận hành đầy đủ ở chế độ độc lập.'}
          </div>
        )}

        {/* Quick 1-click plot authentic QMV chart sub-panes */}
        <div className="flex flex-wrap gap-1 pt-1 border-t" style={{ borderColor: 'var(--tv-border)' }}>
          <button
            type="button"
            onClick={() => handleQuickAddQmvIndicator('QMV_STOCK_BB')}
            className="inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded text-[10px] font-medium border cursor-pointer"
            style={{
              backgroundColor: 'var(--tv-bg-panel)',
              borderColor: 'var(--tv-border)',
              color: 'var(--tv-text-primary)',
            }}
          >
            <Plus className="w-2.5 h-2.5" />
            <span>QMV BB ({yumekoQmvSeries?.stockBb.length ?? 0})</span>
          </button>
          <button
            type="button"
            onClick={() => handleQuickAddQmvIndicator('QMV_ACTIVE_MONEY')}
            className="inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded text-[10px] font-medium border cursor-pointer"
            style={{
              backgroundColor: 'var(--tv-bg-panel)',
              borderColor: 'var(--tv-border)',
              color: 'var(--tv-text-primary)',
            }}
          >
            <Plus className="w-2.5 h-2.5" />
            <span>Cáo-Sói-Thỏ ({yumekoQmvSeries?.activeMoney.length ?? 0})</span>
          </button>
          <button
            type="button"
            onClick={() => handleQuickAddQmvIndicator('QMV_FOREIGN_FLOW')}
            className="inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded text-[10px] font-medium border cursor-pointer"
            style={{
              backgroundColor: 'var(--tv-bg-panel)',
              borderColor: 'var(--tv-border)',
              color: 'var(--tv-text-primary)',
            }}
          >
            <Plus className="w-2.5 h-2.5" />
            <span>Khối ngoại ({yumekoQmvSeries?.foreignFlow.length ?? 0})</span>
          </button>
        </div>
      </div>

      {/* 2. KT01 Gate Verdict Export (Sumi -> Yumeko Step 6) */}
      <div
        className="p-2 rounded space-y-1.5"
        style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}
      >
        <div className="flex items-center justify-between">
          <span className="text-[10.5px] font-bold uppercase tracking-wide text-[#2962FF]">
            KT01 Confirmation Gate (Step 6)
          </span>
          <span className="text-[9.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
            advisory_only
          </span>
        </div>
        <button
          type="button"
          disabled={exportingKt01}
          onClick={() => void exportActiveKt01Verdict(true)}
          className="w-full flex items-center justify-center gap-1.5 py-1.5 rounded text-[11.5px] font-semibold bg-[#2962FF] hover:bg-[#1E53E5] text-white border-0 cursor-pointer disabled:opacity-60"
        >
          <FileCheck2 className="w-3.5 h-3.5" />
          <span>{exportingKt01 ? 'Đang tính MTF + AFC + SHA-256...' : 'Đánh giá & Xuất KT01 Verdict'}</span>
        </button>

        {lastExportedReceipt && (
          <div
            className="p-2 rounded space-y-1.5"
            style={{ backgroundColor: 'var(--tv-bg-panel)', border: '1px solid var(--tv-border)' }}
          >
            <div className="flex items-center justify-between gap-1">
              <span
                className="px-1.5 py-0.5 rounded text-[10px] font-bold"
                style={verdictStyle(lastExportedReceipt.verdict.verdict)}
              >
                {lastExportedReceipt.verdict.verdict} · {lastExportedReceipt.verdict.confidenceScore.toFixed(0)}/100
              </span>
              <button
                type="button"
                onClick={() => void handleCopyMarkdown()}
                className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium border cursor-pointer"
                style={{
                  backgroundColor: 'var(--tv-hover)',
                  borderColor: 'var(--tv-border)',
                  color: 'var(--tv-text-primary)',
                }}
                title="Sao chép tóm tắt Markdown để dán vào agy acp"
              >
                <ClipboardCopy className="w-3 h-3" />
                <span>Copy MD</span>
              </button>
            </div>
            <div className="text-[10.5px] font-medium">{lastExportedReceipt.verdict.verdictLabelVi}</div>
            <div className="grid grid-cols-3 gap-1 text-[10px] text-center">
              <div className="p-1 rounded" style={{ backgroundColor: 'var(--tv-bg-elevated)' }}>
                <div style={{ color: 'var(--tv-text-secondary)' }}>Cắt lỗ SL</div>
                <div className="font-semibold text-[#F23645]">
                  {fmtVnd(lastExportedReceipt.verdict.executionLevels.stopLossVnd)}
                </div>
              </div>
              <div className="p-1 rounded" style={{ backgroundColor: 'var(--tv-bg-elevated)' }}>
                <div style={{ color: 'var(--tv-text-secondary)' }}>Mục tiêu 1</div>
                <div className="font-semibold text-[#089981]">
                  {fmtVnd(lastExportedReceipt.verdict.executionLevels.takeProfit1Vnd)}
                </div>
              </div>
              <div className="p-1 rounded" style={{ backgroundColor: 'var(--tv-bg-elevated)' }}>
                <div style={{ color: 'var(--tv-text-secondary)' }}>R:R (TP1)</div>
                <div className="font-semibold">
                  {lastExportedReceipt.verdict.executionLevels.rrRatioTp1.toFixed(2)}R
                </div>
              </div>
            </div>
            <div className="text-[9.5px] truncate" style={{ color: 'var(--tv-text-secondary)' }}>
              SHA-256: {lastExportedReceipt.verdict.verdictSha256.slice(0, 16)}…
            </div>
          </div>
        )}
      </div>

      {/* 3. DM01 Hard Guard F01: Real Holding vs Watchlist ★ (Strictly Separated) */}
      {(holding || wl) && (
        <div className="space-y-1.5">
          {holding && (
            <div
              className="p-2 rounded space-y-1"
              style={{
                backgroundColor: 'rgba(8, 153, 129, 0.08)',
                border: '1px solid rgba(8, 153, 129, 0.4)',
              }}
            >
              <div className="flex items-center justify-between font-semibold text-[#089981]">
                <span className="inline-flex items-center gap-1">
                  <Briefcase className="w-3 h-3" />
                  <span>DM01 · Nắm Giữ Thật</span>
                </span>
                <span className="text-[9.5px]">{holding.sha256Verified ? 'SHA-256 ✓' : 'Chưa xác thực'}</span>
              </div>
              <div className="grid grid-cols-2 gap-x-2 gap-y-0.5 text-[10.5px]">
                <span style={{ color: 'var(--tv-text-secondary)' }}>Khối lượng:</span>
                <span className="text-right font-semibold">{holding.quantity.toLocaleString('en-US')} cp</span>
                <span style={{ color: 'var(--tv-text-secondary)' }}>Giá vốn TB:</span>
                <span className="text-right">{fmtVnd(holding.avgCostVnd)} VND</span>
                <span style={{ color: 'var(--tv-text-secondary)' }}>Tỷ trọng NAV:</span>
                <span className="text-right font-semibold">{holding.weightPct.toFixed(1)}%</span>
                <span style={{ color: 'var(--tv-text-secondary)' }}>Lãi / Lỗ tạm:</span>
                <span
                  className="text-right font-semibold"
                  style={{ color: holding.unrealizedPnlVnd >= 0 ? 'var(--tv-up)' : 'var(--tv-down)' }}
                >
                  {holding.unrealizedPnlPct >= 0 ? '+' : ''}
                  {holding.unrealizedPnlPct.toFixed(2)}% ({fmtVnd(holding.unrealizedPnlVnd)})
                </span>
              </div>
            </div>
          )}

          {wl && (
            <div
              className="p-2 rounded space-y-1"
              style={{
                backgroundColor: 'var(--tv-accent-soft)',
                border: '1px solid var(--tv-accent)',
              }}
            >
              <div className="flex items-center justify-between font-semibold" style={{ color: 'var(--tv-accent)' }}>
                <span className="inline-flex items-center gap-1">
                  <Star className="w-3 h-3" />
                  <span>DM01 · Watchlist Yumeko ★</span>
                </span>
                <span className="text-[9.5px]">{wl.strategyTag || 'Theo dõi'}</span>
              </div>
              {wl.note && <div className="text-[10.5px]">{wl.note}</div>}
              {(wl.targetBuyVnd || wl.invalidationLevelVnd) && (
                <div className="flex justify-between text-[10px]" style={{ color: 'var(--tv-text-secondary)' }}>
                  <span>Vùng mua: {fmtVnd(wl.targetBuyVnd)}</span>
                  <span>Hủy kịch bản: {fmtVnd(wl.invalidationLevelVnd)}</span>
                </div>
              )}
            </div>
          )}
        </div>
      )}

      {/* 4. HP01 QMV Blackbox Stock Details (Normalized VND + kVND Scale Guard) */}
      {bb && (
        <div
          className="p-2 rounded space-y-1.5"
          style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}
        >
          <div className="flex items-center justify-between">
            <span className="text-[10.5px] font-bold uppercase tracking-wide text-[#089981]">
              HP01 · QMV Blackbox ({bb.contentDate})
            </span>
            {bb.inQmv60 && (
              <span className="px-1 rounded text-[9px] font-bold bg-[#089981] text-white">QMV60</span>
            )}
          </div>
          <div className="grid grid-cols-2 gap-x-2 gap-y-0.5 text-[10.5px]">
            <span style={{ color: 'var(--tv-text-secondary)' }}>TMC / TMA20 / 50:</span>
            <span className="text-right font-medium">
              {fmtNum(bb.tmc, 1)} / {fmtNum(bb.tma20, 1)} / {fmtNum(bb.tma50, 1)}
            </span>
            <span style={{ color: 'var(--tv-text-secondary)' }}>DSPI (MA5 / 20):</span>
            <span className="text-right font-medium">
              {fmtNum(bb.dspi, 1)} ({fmtNum(bb.dspiMa5, 1)} / {fmtNum(bb.dspiMa20, 1)})
            </span>
            <span style={{ color: 'var(--tv-text-secondary)' }}>MPICP All / Z:</span>
            <span className="text-right">
              {fmtNum(bb.mpicpAll, 1)} / {fmtNum(bb.mpicpZ, 2)}
            </span>
            <span style={{ color: 'var(--tv-text-secondary)' }}>Cầu (DM) / Cung (DS):</span>
            <span className="text-right">
              {fmtNum(bb.dm, 1)} / {fmtNum(bb.ds, 1)}
            </span>
            {bb.stopLossVnd != null && (
              <>
                <span style={{ color: 'var(--tv-text-secondary)' }}>SL QMV (VND):</span>
                <span className="text-right font-semibold text-[#F23645]">{fmtVnd(bb.stopLossVnd)}</span>
              </>
            )}
            {(bb.targetPrice1Vnd != null || bb.targetPrice2Vnd != null) && (
              <>
                <span style={{ color: 'var(--tv-text-secondary)' }}>TP1 / TP2 QMV:</span>
                <span className="text-right font-semibold text-[#089981]">
                  {fmtVnd(bb.targetPrice1Vnd)} / {fmtVnd(bb.targetPrice2Vnd)}
                </span>
              </>
            )}
            {bb.divisionAllocationGroup && (
              <>
                <span style={{ color: 'var(--tv-text-secondary)' }}>Nhóm phân bổ:</span>
                <span className="text-right truncate">{bb.divisionAllocationGroup}</span>
              </>
            )}
          </div>
          {bb.activeFlags.length > 0 && (
            <div className="flex flex-wrap gap-1 pt-1 border-t" style={{ borderColor: 'var(--tv-border)' }}>
              {bb.activeFlags.map((f) => (
                <span
                  key={f}
                  className="px-1.5 py-0 rounded text-[9.5px] font-semibold"
                  style={{ backgroundColor: 'rgba(8, 153, 129, 0.16)', color: '#089981' }}
                >
                  {f}
                </span>
              ))}
            </div>
          )}
        </div>
      )}

      {/* 5. DN01 8-Quarter CSTC Valuation Table */}
      {vals.length > 0 && (
        <div
          className="p-2 rounded space-y-1"
          style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}
        >
          <div className="text-[10.5px] font-bold uppercase tracking-wide" style={{ color: 'var(--tv-text-secondary)' }}>
            DN01 · Định giá CSTC ({vals.length} kỳ)
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-[10px] border-collapse">
              <thead>
                <tr style={{ color: 'var(--tv-text-secondary)', borderBottom: '1px solid var(--tv-border)' }}>
                  <th className="text-left py-0.5 font-medium">Kỳ</th>
                  <th className="text-right py-0.5 font-medium">P/E</th>
                  <th className="text-right py-0.5 font-medium">P/B</th>
                  <th className="text-right py-0.5 font-medium">ROE%</th>
                </tr>
              </thead>
              <tbody>
                {vals.slice(0, 6).map((q) => (
                  <tr
                    key={`${q.period}_${q.sourceFamily}`}
                    style={{ borderBottom: '1px solid color-mix(in srgb, var(--tv-border) 40%, transparent)' }}
                  >
                    <td className="py-0.5 font-medium">{q.period}</td>
                    <td className="text-right py-0.5">{fmtNum(q.pe, 1)}</td>
                    <td className="text-right py-0.5">{fmtNum(q.pb, 2)}</td>
                    <td className="text-right py-0.5">{q.roe != null ? `${q.roe.toFixed(1)}%` : '—'}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* 6. VM01 Top Sectors & Macro */}
      {vm01 && (vm01.topSectors.length > 0 || vm01.macroHighlights.length > 0) && (
        <div
          className="p-2 rounded space-y-1.5"
          style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}
        >
          <div className="text-[10.5px] font-bold uppercase tracking-wide" style={{ color: 'var(--tv-text-secondary)' }}>
            VM01 · Vĩ mô & Ngành dẫn dắt
          </div>
          {vm01.topSectors.slice(0, 5).map((s) => (
            <div key={s.icb2Code} className="flex items-center justify-between text-[10.5px]">
              <span className="truncate pr-2">{s.icb2Name}</span>
              <span
                className="font-semibold shrink-0"
                style={{ color: (s.changePct ?? 0) >= 0 ? 'var(--tv-up)' : 'var(--tv-down)' }}
              >
                {s.changePct != null ? `${s.changePct >= 0 ? '+' : ''}${s.changePct.toFixed(2)}%` : '—'}
              </span>
            </div>
          ))}
          {vm01.macroHighlights.length > 0 && (
            <div className="pt-1 border-t space-y-0.5" style={{ borderColor: 'var(--tv-border)' }}>
              {vm01.macroHighlights.slice(0, 4).map((m) => (
                <div key={`${m.indicatorCode}_${m.periodDate}`} className="flex items-center justify-between text-[10px]">
                  <span className="truncate pr-2" style={{ color: 'var(--tv-text-secondary)' }}>
                    {m.indicatorCode} ({m.periodDate})
                  </span>
                  <span className="font-medium shrink-0">
                    {m.value != null ? `${m.value.toFixed(2)} ${m.unit}` : m.valueText ?? '—'}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* 7. Episodic Theses from Yumeko Memory */}
      {theses.length > 0 && (
        <div
          className="p-2 rounded space-y-1"
          style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}
        >
          <div className="text-[10.5px] font-bold uppercase tracking-wide" style={{ color: 'var(--tv-text-secondary)' }}>
            Ký ức Luận điểm Yumeko ({theses.length})
          </div>
          {theses.slice(0, 4).map((t) => (
            <div
              key={t.decisionId}
              className="p-1.5 rounded space-y-0.5"
              style={{ backgroundColor: 'var(--tv-bg-panel)', border: '1px solid var(--tv-border)' }}
            >
              <div className="flex items-center justify-between text-[10px]">
                <span className="font-semibold">{t.stance}</span>
                <span style={{ color: 'var(--tv-text-secondary)' }}>{t.asOf}</span>
              </div>
              <div className="text-[10px] leading-snug" style={{ color: 'var(--tv-text-secondary)' }}>
                {t.thesisSummary}
              </div>
            </div>
          ))}
        </div>
      )}

      {/* 8. Recent Exported KT01 Verdicts */}
      {exportedVerdicts.length > 0 && (
        <div
          className="p-2 rounded space-y-1"
          style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}
        >
          <div className="text-[10.5px] font-bold uppercase tracking-wide" style={{ color: 'var(--tv-text-secondary)' }}>
            Lịch sử KT01 đã xuất ({exportedVerdicts.length})
          </div>
          {exportedVerdicts.slice(0, 6).map((v) => (
            <div
              key={v.verdictId}
              className="flex items-center justify-between gap-1 text-[10px] py-0.5 border-b last:border-b-0"
              style={{ borderColor: 'color-mix(in srgb, var(--tv-border) 45%, transparent)' }}
            >
              <div className="min-w-0">
                <span className="font-semibold">{v.symbol}</span>{' '}
                <span style={{ color: 'var(--tv-text-secondary)' }}>({v.asOf})</span>
              </div>
              <span className="px-1.5 py-0 rounded font-bold shrink-0" style={verdictStyle(v.verdict)}>
                {v.verdict} {v.confidenceScore.toFixed(0)}
              </span>
            </div>
          ))}
        </div>
      )}

      {/* 9. Research Export SHA-256 Verifier (`yumeko research export`) */}
      <form
        onSubmit={handleVerifyResearchDir}
        className="p-2 rounded space-y-1.5"
        style={{ backgroundColor: 'var(--tv-bg-elevated)', border: '1px solid var(--tv-border)' }}
      >
        <div className="flex items-center gap-1 text-[10.5px] font-semibold" style={{ color: 'var(--tv-text-secondary)' }}>
          <FolderCheck className="w-3.5 h-3.5" />
          <span>Kiểm chứng Research Export (SHA-256)</span>
        </div>
        <div className="flex gap-1">
          <input
            type="text"
            value={exportCheckDir}
            onChange={(e) => setExportCheckDir(e.target.value)}
            placeholder="Đường dẫn thư mục manifest.json..."
            className="flex-1 min-w-0 px-1.5 py-1 rounded text-[10.5px] outline-none"
            style={{
              backgroundColor: 'var(--tv-bg-input)',
              color: 'var(--tv-text-primary)',
              border: '1px solid var(--tv-border)',
            }}
          />
          <button
            type="submit"
            disabled={checkingDir}
            className="px-2 py-1 rounded text-[10.5px] font-semibold border cursor-pointer shrink-0"
            style={{
              backgroundColor: 'var(--tv-hover)',
              borderColor: 'var(--tv-border)',
              color: 'var(--tv-text-primary)',
            }}
          >
            Kiểm tra
          </button>
        </div>
        {exportCheckRes && (
          <div className="flex items-center gap-1 text-[10px]">
            {exportCheckRes.valid ? (
              <CheckCircle2 className="w-3.5 h-3.5 text-[#089981] shrink-0" />
            ) : (
              <AlertTriangle className="w-3.5 h-3.5 text-[#FBC02D] shrink-0" />
            )}
            <span>
              {exportCheckRes.valid
                ? `Hợp lệ (${exportCheckRes.verifiedFiles} tệp · as_of=${exportCheckRes.asOf ?? '—'})`
                : `Lỗi (${exportCheckRes.failedFiles.join(', ')})`}
            </span>
          </div>
        )}
      </form>
    </div>
  )
}
