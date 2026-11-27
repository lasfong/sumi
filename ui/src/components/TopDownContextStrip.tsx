import React from 'react'
import { Briefcase, CalendarClock, CheckCircle2, FileCheck2, Sparkles, Star } from 'lucide-react'
import { useWorkspaceStore } from '../store/workspaceStore'

export const TopDownContextStrip: React.FC = () => {
  const yumekoStatus = useWorkspaceStore((s) => s.yumekoStatus)
  const yumekoAsOf = useWorkspaceStore((s) => s.yumekoAsOf)
  const yumekoContext = useWorkspaceStore((s) => s.yumekoContext)
  const yumekoLoading = useWorkspaceStore((s) => s.yumekoLoading)
  const exportingKt01 = useWorkspaceStore((s) => s.exportingKt01)
  const setYumekoAsOf = useWorkspaceStore((s) => s.setYumekoAsOf)
  const setRightTab = useWorkspaceStore((s) => s.setRightTab)
  const exportActiveKt01Verdict = useWorkspaceStore((s) => s.exportActiveKt01Verdict)

  const bb = yumekoContext?.hp01Blackbox ?? null
  const isReal = yumekoContext?.flowProvenance === 'qmv_real' && Boolean(bb?.isProviderProprietary)
  const latestVal = yumekoContext?.dn01Valuations?.[0] ?? null
  const holding = yumekoContext?.dm01RealHolding ?? null
  const wl = yumekoContext?.dm01YumekoWatchlist ?? null
  const vm01 = yumekoContext?.vm01Market ?? null
  const asOfDates = yumekoStatus?.latestAsOfDates ?? []

  return (
    <div
      aria-label="Ngữ cảnh QMV Top-Down từ Yumeko"
      className="flex items-center justify-between gap-2 px-2 shrink-0 overflow-x-auto select-none text-[10.5px] tabular-nums"
      style={{
        height: 26,
        backgroundColor: 'var(--tv-bg-panel)',
        borderBottom: '1px solid var(--tv-border)',
        color: 'var(--tv-text-secondary)',
      }}
    >
      <div className="flex items-center gap-2 min-w-0 whitespace-nowrap">
        <button
          type="button"
          onClick={() => setRightTab('yumeko')}
          className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded font-semibold border-0 cursor-pointer"
          style={{
            backgroundColor: yumekoStatus?.available ? 'rgba(8, 153, 129, 0.16)' : 'rgba(251, 192, 45, 0.14)',
            color: yumekoStatus?.available ? '#089981' : '#FBC02D',
          }}
          title={
            yumekoStatus?.available
              ? `Cầu nối Chỉ Đọc Yumeko SQLite đang hoạt động (${yumekoStatus.qmvTechnicalSymbols} mã QMV)`
              : 'Chưa phát hiện Yumeko SQLite — Sumi chạy chế độ độc lập (OHLCV Proxy)'
          }
        >
          <Sparkles className="w-3 h-3 shrink-0" />
          <span>{yumekoStatus?.available ? 'Yumeko RO' : 'Standalone'}</span>
        </button>

        {/* VM01 Market Sentiment & BBM */}
        {vm01 && (vm01.marketSentiment !== null || vm01.marketBbm05 !== null) && (
          <button
            type="button"
            onClick={() => setRightTab('yumeko')}
            className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded border-0 bg-transparent cursor-pointer hover:bg-[var(--tv-hover)]"
            style={{ color: 'var(--tv-text-primary)' }}
            title="VM01: Tâm lý & Chu kỳ Dòng tiền Thị trường (Yumeko Blackbox)"
          >
            <span className="font-semibold text-[10px]" style={{ color: 'var(--tv-text-secondary)' }}>
              VM01
            </span>
            {vm01.marketSentiment !== null && <span>Sent {vm01.marketSentiment.toFixed(1)}</span>}
            {vm01.marketBbm05 !== null && (
              <span style={{ color: vm01.marketBbm05 >= 0 ? 'var(--tv-up)' : 'var(--tv-down)' }}>
                BBM5 {vm01.marketBbm05.toFixed(1)}
              </span>
            )}
          </button>
        )}

        {/* HP01 QMV Blackbox vs Proxy Provenance Pill */}
        <button
          type="button"
          onClick={() => setRightTab('yumeko')}
          className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded border-0 cursor-pointer"
          style={{
            backgroundColor: isReal ? 'rgba(8, 153, 129, 0.13)' : 'rgba(251, 192, 45, 0.12)',
            color: isReal ? '#089981' : '#FBC02D',
          }}
          title={
            isReal
              ? 'HP01: Dữ liệu QMV Blackbox thật đọc Chỉ Đọc từ Yumeko SQLite'
              : 'HP01: Không có dòng QMV Blackbox cho mã này — đang dùng OHLCV Proxy của Sumi'
          }
        >
          <span className="font-semibold">{isReal ? '✓ QMV Thật' : '⚠ OHLCV Proxy'}</span>
          {bb?.tmc != null && <span>TMC {bb.tmc.toFixed(1)}</span>}
          {bb?.dspi != null && <span>· DSPI {bb.dspi.toFixed(1)}</span>}
          {bb?.moneyInToday && (
            <span className="px-1 rounded text-[9.5px] font-bold bg-[#089981] text-white">TIỀN VÀO</span>
          )}
        </button>

        {/* DN01 Latest Valuation */}
        {latestVal && (latestVal.pe !== null || latestVal.pb !== null) && (
          <button
            type="button"
            onClick={() => setRightTab('yumeko')}
            className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded border-0 bg-transparent cursor-pointer hover:bg-[var(--tv-hover)]"
            style={{ color: 'var(--tv-text-primary)' }}
            title={`DN01 CSTC kỳ ${latestVal.period}`}
          >
            <span className="font-semibold text-[10px]" style={{ color: 'var(--tv-text-secondary)' }}>
              DN01
            </span>
            {latestVal.pe !== null && <span>P/E {latestVal.pe.toFixed(1)}</span>}
            {latestVal.pb !== null && <span>· P/B {latestVal.pb.toFixed(2)}</span>}
          </button>
        )}

        {/* DM01 Hard Guard F01: Real Holding strictly separated from Watchlist ★ */}
        {holding && (
          <button
            type="button"
            onClick={() => setRightTab('yumeko')}
            className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded font-semibold border-0 cursor-pointer"
            style={{
              backgroundColor: 'rgba(8, 153, 129, 0.18)',
              color: '#089981',
            }}
            title={`DM01 Nắm giữ thật (SHA-256 ${holding.sha256Verified ? 'OK' : 'WARN'}) · ${holding.quantity.toLocaleString('en-US')} cp`}
          >
            <Briefcase className="w-3 h-3" />
            <span>
              Đang nắm {holding.weightPct.toFixed(1)}% ({holding.unrealizedPnlPct >= 0 ? '+' : ''}
              {holding.unrealizedPnlPct.toFixed(1)}%)
            </span>
          </button>
        )}
        {wl && (
          <button
            type="button"
            onClick={() => setRightTab('yumeko')}
            className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded font-semibold border-0 cursor-pointer"
            style={{
              backgroundColor: 'var(--tv-accent-soft)',
              color: 'var(--tv-accent)',
            }}
            title={`DM01 Watchlist Yumeko (${wl.strategyTag}): ${wl.note}`}
          >
            <Star className="w-3 h-3" />
            <span>Theo dõi ★</span>
          </button>
        )}
        {yumekoLoading && <span className="text-[10px] opacity-70">Đang nạp...</span>}
      </div>

      {/* Right controls: PIT `as_of` Date Lock + 1-click KT01 Verdict Export */}
      <div className="flex items-center gap-1.5 shrink-0">
        <label className="inline-flex items-center gap-1" title="Khóa mốc Point-in-Time as_of đồng bộ với Yumeko">
          <CalendarClock className="w-3 h-3 shrink-0" />
          <span className="text-[10px]">as_of:</span>
          <select
            aria-label="Khóa ngày chốt as_of Point-in-Time"
            value={yumekoAsOf ?? ''}
            onChange={(e) => setYumekoAsOf(e.target.value || null)}
            className="px-1.5 py-0 rounded text-[10.5px] outline-none cursor-pointer"
            style={{
              height: 19,
              backgroundColor: yumekoAsOf ? 'var(--tv-accent-soft)' : 'var(--tv-bg-input)',
              color: yumekoAsOf ? 'var(--tv-accent)' : 'var(--tv-text-primary)',
              border: `1px solid ${yumekoAsOf ? 'var(--tv-accent)' : 'var(--tv-border)'}`,
            }}
          >
            <option value="">Mới nhất (Live)</option>
            {yumekoAsOf && !asOfDates.includes(yumekoAsOf) && (
              <option value={yumekoAsOf}>{yumekoAsOf} (Khóa PIT)</option>
            )}
            {asOfDates.slice(0, 20).map((d) => (
              <option key={d} value={d}>
                {d}
              </option>
            ))}
          </select>
        </label>

        <button
          type="button"
          disabled={exportingKt01}
          onClick={() => void exportActiveKt01Verdict(true)}
          className="inline-flex items-center gap-1 px-2 py-0 rounded font-semibold border-0 cursor-pointer text-white bg-[#2962FF] hover:bg-[#1E53E5] disabled:opacity-60"
          style={{ height: 19 }}
          title="Đánh giá & xuất gói bằng chứng kỹ thuật KT01 (JSON + Markdown có SHA-256) trả về Yumeko / agy acp"
        >
          {exportingKt01 ? <CheckCircle2 className="w-3 h-3 animate-pulse" /> : <FileCheck2 className="w-3 h-3" />}
          <span>{exportingKt01 ? 'Đang xuất...' : 'Lưu KT01'}</span>
        </button>
      </div>
    </div>
  )
}
