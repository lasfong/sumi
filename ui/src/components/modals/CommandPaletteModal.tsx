import React, { useEffect, useMemo, useRef, useState } from 'react'
import {
  Activity,
  Calculator,
  Command,
  Dice5,
  FileCheck2,
  Filter,
  Grid2x2,
  Maximize2,
  Moon,
  RefreshCw,
  Search,
  Sparkles,
  Sun,
  TrendingUp,
} from 'lucide-react'
import * as api from '../../api/tauri'
import { useWorkspaceStore } from '../../store/workspaceStore'
import type { GridLayoutKind, IntervalKey, SymbolSearchResult } from '../../types/market'

export interface PaletteCommandItem {
  id: string
  category: string
  title: string
  subtitle: string
  shortcut?: string
  keywords: string
  run: () => void
}

export function filterPaletteCommands(items: PaletteCommandItem[], rawQuery: string): PaletteCommandItem[] {
  const q = rawQuery.replace(/^>\s*/, '').trim().toLowerCase()
  if (!q) return items
  return items.filter(
    (item) =>
      item.title.toLowerCase().includes(q) ||
      item.subtitle.toLowerCase().includes(q) ||
      item.category.toLowerCase().includes(q) ||
      item.keywords.toLowerCase().includes(q),
  )
}

export const CommandPaletteModal: React.FC = () => {
  const open = useWorkspaceStore((s) => s.commandPaletteOpen)
  const theme = useWorkspaceStore((s) => s.theme)
  const setCommandPaletteOpen = useWorkspaceStore((s) => s.setCommandPaletteOpen)
  const setActiveTicker = useWorkspaceStore((s) => s.setActiveTicker)
  const setActiveInterval = useWorkspaceStore((s) => s.setActiveInterval)
  const setGridLayout = useWorkspaceStore((s) => s.setGridLayout)
  const toggleMaximizeSlot = useWorkspaceStore((s) => s.toggleMaximizeSlot)
  const toggleTheme = useWorkspaceStore((s) => s.toggleTheme)
  const setIndicatorsModalOpen = useWorkspaceStore((s) => s.setIndicatorsModalOpen)
  const openTradePlanner = useWorkspaceStore((s) => s.openTradePlanner)
  const setBottomDockTab = useWorkspaceStore((s) => s.setBottomDockTab)
  const setRightTab = useWorkspaceStore((s) => s.setRightTab)
  const startRandomBlindDrill = useWorkspaceStore((s) => s.startRandomBlindDrill)
  const exportActiveKt01Verdict = useWorkspaceStore((s) => s.exportActiveKt01Verdict)
  const syncUniverseEod = useWorkspaceStore((s) => s.syncUniverseEod)
  const undoIndicatorChange = useWorkspaceStore((s) => s.undoIndicatorChange)

  const dialogRef = useRef<HTMLDialogElement | null>(null)
  const inputRef = useRef<HTMLInputElement | null>(null)
  const [query, setQuery] = useState('')
  const [symbolHits, setSymbolHits] = useState<SymbolSearchResult[]>([])
  const [activeIdx, setActiveIdx] = useState(0)

  const commands = useMemo<PaletteCommandItem[]>(() => {
    const close = () => setCommandPaletteOpen(false)
    const intervals: IntervalKey[] = ['15m', '1h', '4h', '1D', '1W', '1M']
    const layouts: Array<{ id: GridLayoutKind; label: string }> = [
      { id: '1', label: 'Bố cục 1 Biểu đồ đơn' },
      { id: '2v', label: 'Bố cục 2 Biểu đồ dọc (Trái / Phải)' },
      { id: '2h', label: 'Bố cục 2 Biểu đồ ngang (Trên / Dưới)' },
      { id: '3', label: 'Bố cục 3 Biểu đồ (1 Chính + 2 Phụ)' },
      { id: '4', label: 'Bố cục 4 Biểu đồ đa khung giờ' },
    ]
    return [
      {
        id: 'cmd-blind-drill',
        category: 'Replay',
        title: 'Bắt đầu Luyện Mù Ngẫu Nhiên (True Blind Drill)',
        subtitle: 'Chọn ngẫu nhiên 1 mã QMV60, chuẩn hóa giá về 100.00 và ẩn danh tính',
        keywords: 'blind replay drill luyen mu ngau nhien',
        run: () => {
          close()
          void startRandomBlindDrill()
        },
      },
      {
        id: 'cmd-kt01-export',
        category: 'Yumeko KT01',
        title: 'Xuất gói bằng chứng kỹ thuật KT01 (SHA-256)',
        subtitle: 'Chốt TechnicalEvidenceBundleV1 + đồng bộ Yumeko dropzone',
        keywords: 'yumeko kt01 verdict export sha256',
        run: () => {
          close()
          void exportActiveKt01Verdict(true)
        },
      },
      {
        id: 'cmd-indicators',
        category: 'Biểu đồ',
        title: 'Mở Thư viện Chỉ báo (QMV Real, Kỹ thuật, DN01)',
        subtitle: 'Thêm Ichimoku V7, AFC, AVWAP, QMV_STOCK_BB, FIN_PE/ROE...',
        keywords: 'indicator chi bao ichimoku rsi macd avwap qmv',
        run: () => {
          close()
          setIndicatorsModalOpen(true)
        },
      },
      {
        id: 'cmd-trade-planner',
        category: 'Thực thi',
        title: 'Mở Kế hoạch Lệnh (Trade Planner · Lô 100 / R:R)',
        subtitle: 'Tính khối lượng rủi ro chuẩn sàn VN & gửi thẳng Paper Broker',
        keywords: 'trade planner ke hoach lenh risk rr lot',
        run: () => {
          close()
          openTradePlanner()
        },
      },
      {
        id: 'cmd-screener',
        category: 'Bộ lọc',
        title: 'Mở Bộ lọc Cổ phiếu 2.0 (MTF 1W/1D/1H + DN01 + QMV)',
        subtitle: 'Lọc đa khung giờ và lưu bộ lọc tùy chỉnh',
        keywords: 'screener bo loc co phieu mtf',
        run: () => {
          close()
          setBottomDockTab('screener')
        },
      },
      {
        id: 'cmd-strategy',
        category: 'Kiểm định',
        title: 'Mở Strategy Tester 2.0 (Backtest / WFO / Monte Carlo)',
        subtitle: 'Pyramiding, TP1 Scale-out, Bar Magnifier 1H, Heatmap Tháng',
        keywords: 'backtest strategy wfo monte carlo kiem dinh',
        run: () => {
          close()
          setBottomDockTab('strategy')
        },
      },
      {
        id: 'cmd-breadth',
        category: 'Thị trường',
        title: 'Mở Độ rộng Thị trường & Sức mạnh Ngành (Breadth / RRG)',
        subtitle: 'A/D Line, McClellan, % > MA50/200 & Biểu đồ RRG 19 ngành',
        keywords: 'breadth rrg do rong thi truong nganh',
        run: () => {
          close()
          setBottomDockTab('breadth')
        },
      },
      {
        id: 'cmd-yumeko-panel',
        category: 'Yumeko KT01',
        title: 'Mở bảng Ngữ cảnh Top-Down Yumeko (VM01 → DM01)',
        subtitle: 'Xem Blackbox QMV thật, BCTC 8 quý, Vị thế DM01',
        keywords: 'yumeko topdown vm01 hp01 dn01 dm01',
        run: () => {
          close()
          setRightTab('yumeko')
        },
      },
      {
        id: 'cmd-maximize-pane',
        category: 'Bố cục',
        title: 'Phóng to / Khôi phục ô biểu đồ hiện tại',
        subtitle: 'Chế độ toàn khung cho 1 ô trong bố cục đa biểu đồ',
        shortcut: 'Alt+Enter',
        keywords: 'maximize zoom full pane phong to o bieu do',
        run: () => {
          close()
          toggleMaximizeSlot()
        },
      },
      {
        id: 'cmd-theme',
        category: 'Giao diện',
        title: theme === 'dark' ? 'Chuyển sang Giao diện Sáng (Light Mode)' : 'Chuyển sang Giao diện Tối (Dark Mode)',
        subtitle: 'Đổi bảng màu TradingView Supercharts',
        keywords: 'theme dark light sang toi giao dien',
        run: () => {
          close()
          toggleTheme()
        },
      },
      {
        id: 'cmd-sync-eod',
        category: 'Dữ liệu',
        title: 'Đồng bộ nến EOD mới nhất cho toàn thị trường VN',
        subtitle: 'Cập nhật nhanh dữ liệu cuối ngày cho các mã bị trễ phiên',
        keywords: 'sync eod cap nhat du lieu nen',
        run: () => {
          close()
          void syncUniverseEod('ALL_VN', 150)
        },
      },
      {
        id: 'cmd-undo-ind',
        category: 'Biểu đồ',
        title: 'Hoàn tác thay đổi chỉ báo gần nhất (Undo Indicator)',
        subtitle: 'Khôi phục trạng thái chỉ báo trước đó trên ô hiện tại',
        shortcut: 'Ctrl/Cmd+Z',
        keywords: 'undo indicator hoan tac chi bao',
        run: () => {
          close()
          undoIndicatorChange()
        },
      },
      ...intervals.map((iv) => ({
        id: `cmd-iv-${iv}`,
        category: 'Khung thời gian',
        title: `Chuyển khung thời gian → ${iv.toUpperCase()}`,
        subtitle: `Áp dụng khung ${iv} cho ô biểu đồ đang chọn`,
        keywords: `timeframe interval khung ${iv}`,
        run: () => {
          close()
          setActiveInterval(iv)
        },
      })),
      ...layouts.map((ly) => ({
        id: `cmd-layout-${ly.id}`,
        category: 'Bố cục',
        title: ly.label,
        subtitle: `Chuyển lưới biểu đồ sang chế độ ${ly.id}`,
        keywords: `layout grid bo cuc ${ly.id}`,
        run: () => {
          close()
          setGridLayout(ly.id)
        },
      })),
    ]
  }, [
    theme,
    setCommandPaletteOpen,
    startRandomBlindDrill,
    exportActiveKt01Verdict,
    setIndicatorsModalOpen,
    openTradePlanner,
    setBottomDockTab,
    setRightTab,
    toggleMaximizeSlot,
    toggleTheme,
    syncUniverseEod,
    undoIndicatorChange,
    setActiveInterval,
    setGridLayout,
  ])

  useEffect(() => {
    const dlg = dialogRef.current
    if (!dlg) return
    if (open) {
      setQuery('')
      setSymbolHits([])
      setActiveIdx(0)
      if (!dlg.open) dlg.showModal()
      window.setTimeout(() => inputRef.current?.focus(), 20)
    } else if (dlg.open) {
      dlg.close()
    }
  }, [open])

  const trimmed = query.trim()
  const isCommandOnlyMode = trimmed.startsWith('>')
  const isExpression = !isCommandOnlyMode && /[/*+\-()]/.test(trimmed) && /[A-Za-z]/.test(trimmed)

  useEffect(() => {
    if (!open || isCommandOnlyMode || !trimmed || isExpression) {
      setSymbolHits([])
      return
    }
    let cancelled = false
    const timer = window.setTimeout(() => {
      void api
        .searchSymbols(trimmed, undefined, 8)
        .then((res) => {
          if (!cancelled) setSymbolHits(res)
        })
        .catch(() => {
          if (!cancelled) setSymbolHits([])
        })
    }, 90)
    return () => {
      cancelled = true
      window.clearTimeout(timer)
    }
  }, [open, isCommandOnlyMode, trimmed, isExpression])

  const matchedCommands = useMemo(() => filterPaletteCommands(commands, query), [commands, query])

  type RowEntry =
    | { kind: 'expr'; key: string; expr: string }
    | { kind: 'symbol'; key: string; sym: SymbolSearchResult }
    | { kind: 'cmd'; key: string; cmd: PaletteCommandItem }

  const rows = useMemo<RowEntry[]>(() => {
    const list: RowEntry[] = []
    if (isExpression) {
      list.push({ kind: 'expr', key: `expr:${trimmed.toUpperCase()}`, expr: trimmed.toUpperCase() })
    }
    if (!isCommandOnlyMode) {
      for (const s of symbolHits) {
        list.push({ kind: 'symbol', key: `sym:${s.ticker}`, sym: s })
      }
    }
    for (const c of matchedCommands.slice(0, isCommandOnlyMode || !trimmed ? 20 : 8)) {
      list.push({ kind: 'cmd', key: c.id, cmd: c })
    }
    return list
  }, [isExpression, trimmed, isCommandOnlyMode, symbolHits, matchedCommands])

  useEffect(() => {
    setActiveIdx(0)
  }, [query, rows.length])

  const executeRow = (row: RowEntry | undefined) => {
    if (!row) return
    if (row.kind === 'expr') {
      setActiveTicker(row.expr)
      setCommandPaletteOpen(false)
    } else if (row.kind === 'symbol') {
      setActiveTicker(row.sym.ticker)
      setCommandPaletteOpen(false)
    } else {
      row.cmd.run()
    }
  }

  const renderIcon = (cat: string) => {
    switch (cat) {
      case 'Replay':
        return <Dice5 className="w-3.5 h-3.5 text-[#FBC02D]" />
      case 'Yumeko KT01':
        return <FileCheck2 className="w-3.5 h-3.5 text-[#089981]" />
      case 'Thực thi':
        return <Calculator className="w-3.5 h-3.5 text-[#2962FF]" />
      case 'Bộ lọc':
        return <Filter className="w-3.5 h-3.5 text-[#00BCD4]" />
      case 'Kiểm định':
        return <TrendingUp className="w-3.5 h-3.5 text-[#AB47BC]" />
      case 'Bố cục':
        return <Grid2x2 className="w-3.5 h-3.5 text-[#5B8DEF]" />
      case 'Giao diện':
        return theme === 'dark' ? <Sun className="w-3.5 h-3.5 text-[#FBC02D]" /> : <Moon className="w-3.5 h-3.5 text-[#2962FF]" />
      case 'Dữ liệu':
        return <RefreshCw className="w-3.5 h-3.5 text-[#089981]" />
      case 'Thị trường':
        return <Activity className="w-3.5 h-3.5 text-[#FF9800]" />
      default:
        return <Sparkles className="w-3.5 h-3.5 text-[#2962FF]" />
    }
  }

  return (
    <dialog
      ref={dialogRef}
      aria-label="Bảng lệnh nhanh Command Palette"
      onCancel={(e) => {
        e.preventDefault()
        setCommandPaletteOpen(false)
      }}
      onClick={(e) => {
        if (e.target === dialogRef.current) setCommandPaletteOpen(false)
      }}
      className="p-0 rounded-lg shadow-2xl w-[620px] max-w-[94vw] max-h-[78vh] overflow-hidden backdrop:bg-black/60"
      style={{
        backgroundColor: 'var(--tv-bg-elevated)',
        color: 'var(--tv-text-primary)',
        border: '1px solid var(--tv-border)',
      }}
    >
      <div className="flex flex-col max-h-[78vh]">
        <div
          className="flex items-center gap-2 px-3 py-2.5 shrink-0"
          style={{ borderBottom: '1px solid var(--tv-border)' }}
        >
          <Command className="w-4 h-4 text-[#2962FF] shrink-0" />
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'ArrowDown') {
                e.preventDefault()
                setActiveIdx((i) => Math.min(rows.length - 1, i + 1))
              } else if (e.key === 'ArrowUp') {
                e.preventDefault()
                setActiveIdx((i) => Math.max(0, i - 1))
              } else if (e.key === 'Enter') {
                e.preventDefault()
                executeRow(rows[activeIdx])
              }
            }}
            placeholder="Gõ lệnh (VD: > blind, > kt01, > 1h), mã CK (FPT) hoặc công thức (HOSE:SSI / HOSE:VNINDEX)..."
            className="flex-1 bg-transparent border-0 outline-none text-[13px]"
            style={{ color: 'var(--tv-text-primary)' }}
          />
          <kbd
            className="px-1.5 py-0.5 rounded text-[10px]"
            style={{ backgroundColor: 'var(--tv-bg-input)', color: 'var(--tv-text-secondary)', border: '1px solid var(--tv-border)' }}
          >
            Esc
          </kbd>
        </div>

        <div className="flex-1 overflow-y-auto tv-scroll p-1.5 space-y-0.5">
          {rows.length === 0 ? (
            <div className="py-10 text-center text-[12px]" style={{ color: 'var(--tv-text-secondary)' }}>
              Không tìm thấy lệnh hoặc mã chứng khoán khớp "{query}".
            </div>
          ) : (
            rows.map((row, idx) => {
              const selected = idx === activeIdx
              if (row.kind === 'expr') {
                return (
                  <button
                    key={row.key}
                    type="button"
                    onMouseEnter={() => setActiveIdx(idx)}
                    onClick={() => executeRow(row)}
                    className="w-full flex items-center justify-between gap-2 px-2.5 py-2 rounded text-left border-0 cursor-pointer"
                    style={{
                      backgroundColor: selected ? 'var(--tv-accent-soft)' : 'transparent',
                      color: 'var(--tv-text-primary)',
                    }}
                  >
                    <div className="flex items-center gap-2 min-w-0">
                      <Maximize2 className="w-3.5 h-3.5 text-[#E040FB]" />
                      <div className="min-w-0">
                        <div className="text-[12px] font-semibold truncate">Mở biểu đồ công thức: {row.expr}</div>
                        <div className="text-[10.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
                          Synthetic Spread / Ratio OHLCV
                        </div>
                      </div>
                    </div>
                    <span className="px-1.5 py-0.5 rounded text-[9.5px] font-bold bg-[#E040FB]/20 text-[#E040FB]">
                      EXPR
                    </span>
                  </button>
                )
              }
              if (row.kind === 'symbol') {
                return (
                  <button
                    key={row.key}
                    type="button"
                    onMouseEnter={() => setActiveIdx(idx)}
                    onClick={() => executeRow(row)}
                    className="w-full flex items-center justify-between gap-2 px-2.5 py-1.5 rounded text-left border-0 cursor-pointer"
                    style={{
                      backgroundColor: selected ? 'var(--tv-accent-soft)' : 'transparent',
                      color: 'var(--tv-text-primary)',
                    }}
                  >
                    <div className="flex items-center gap-2 min-w-0">
                      <Search className="w-3.5 h-3.5 text-[#2962FF]" />
                      <div className="min-w-0">
                        <div className="text-[12px] font-semibold truncate">
                          {row.sym.symbol}{' '}
                          <span className="text-[10.5px] font-normal" style={{ color: 'var(--tv-text-secondary)' }}>
                            {row.sym.description}
                          </span>
                        </div>
                      </div>
                    </div>
                    <span
                      className="px-1.5 py-0.5 rounded text-[9.5px] font-semibold shrink-0"
                      style={{ backgroundColor: 'var(--tv-hover)', color: 'var(--tv-text-secondary)' }}
                    >
                      {row.sym.exchange}
                    </span>
                  </button>
                )
              }
              return (
                <button
                  key={row.key}
                  type="button"
                  onMouseEnter={() => setActiveIdx(idx)}
                  onClick={() => executeRow(row)}
                  className="w-full flex items-center justify-between gap-2 px-2.5 py-1.5 rounded text-left border-0 cursor-pointer"
                  style={{
                    backgroundColor: selected ? 'var(--tv-accent-soft)' : 'transparent',
                    color: 'var(--tv-text-primary)',
                  }}
                >
                  <div className="flex items-center gap-2 min-w-0">
                    {renderIcon(row.cmd.category)}
                    <div className="min-w-0">
                      <div className="text-[12px] font-medium truncate">{row.cmd.title}</div>
                      <div className="text-[10.5px] truncate" style={{ color: 'var(--tv-text-secondary)' }}>
                        {row.cmd.subtitle}
                      </div>
                    </div>
                  </div>
                  <div className="flex items-center gap-1.5 shrink-0">
                    <span className="text-[9.5px]" style={{ color: 'var(--tv-text-secondary)' }}>
                      {row.cmd.category}
                    </span>
                    {row.cmd.shortcut && (
                      <kbd
                        className="px-1.5 py-0.5 rounded text-[9.5px]"
                        style={{
                          backgroundColor: 'var(--tv-bg-input)',
                          border: '1px solid var(--tv-border)',
                          color: 'var(--tv-text-secondary)',
                        }}
                      >
                        {row.cmd.shortcut}
                      </kbd>
                    )}
                  </div>
                </button>
              )
            })
          )}
        </div>

        <div
          className="flex items-center justify-between px-3 py-1.5 text-[10px] shrink-0"
          style={{ borderTop: '1px solid var(--tv-border)', color: 'var(--tv-text-secondary)' }}
        >
          <span>Mẹo: Gõ &gt; để lọc riêng câu lệnh · ↑↓ di chuyển · Enter thực thi</span>
          <span>Sumi v3.0 Command Palette</span>
        </div>
      </div>
    </dialog>
  )
}
