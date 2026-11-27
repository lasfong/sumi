import React, { useEffect, useRef, useState } from 'react'
import {
  Activity,
  Camera,
  CandlestickChart,
  ChevronDown,
  Clock,
  Columns2,
  Database,
  Download,
  ExternalLink,
  FolderSync,
  Globe,
  Grid2x2,
  History,
  Layers,
  LayoutGrid,
  LayoutPanelLeft,
  Moon,
  PanelBottom,
  PanelRight,
  PlusCircle,
  RefreshCw,
  Rows2,
  Save,
  Search,
  Square,
  Sun,
  Trash2,
  Upload,
  Wifi,
  WifiOff,
  X,
} from 'lucide-react'
import * as api from '../api/tauri'
import { COMPARE_COLORS } from '../chart/indicators/fundamentalIndicators'
import { ALL_INTERVALS, QUICK_INTERVALS } from '../chart/periods'
import { useWorkspaceStore, type TimezoneMode } from '../store/workspaceStore'
import type { ChartStyleKind, GridLayoutKind, PriceScaleKind } from '../types/market'

const CHART_STYLE_OPTIONS: Array<{ key: ChartStyleKind; label: string }> = [
  { key: 'candle_solid', label: 'Nến Nhật (Candles)' },
  { key: 'candle_stroke', label: 'Nến Rỗng (Hollow Candles)' },
  { key: 'heikin_ashi', label: 'Heikin-Ashi' },
  { key: 'ohlc', label: 'Thanh OHLC (Bars)' },
  { key: 'line', label: 'Đường (Line)' },
  { key: 'area', label: 'Vùng (Area)' },
]

const SCALE_OPTIONS: Array<{ key: PriceScaleKind; short: string; label: string }> = [
  { key: 'normal', short: 'TĐ', label: 'Thang thường (Normal)' },
  { key: 'percentage', short: '%', label: 'Phần trăm (%)' },
  { key: 'logarithm', short: 'Log', label: 'Logarit (Log)' },
]

export const TopHeader: React.FC = () => {
  const theme = useWorkspaceStore((s) => s.theme)
  const timezoneMode = useWorkspaceStore((s) => s.timezoneMode)
  const gridLayout = useWorkspaceStore((s) => s.gridLayout)
  const sync = useWorkspaceStore((s) => s.sync)
  const activeSlotId = useWorkspaceStore((s) => s.activeSlotId)
  const panes = useWorkspaceStore((s) => s.panes)
  const layouts = useWorkspaceStore((s) => s.layouts)
  const indicatorTemplates = useWorkspaceStore((s) => s.indicatorTemplates)
  const showVnSessionBreaks = useWorkspaceStore((s) => s.showVnSessionBreaks)
  const dataStatus = useWorkspaceStore((s) => s.dataStatus)
  const syncStatus = useWorkspaceStore((s) => s.syncStatus)
  const importingCafeF = useWorkspaceStore((s) => s.importingCafeF)
  const syncingCatalog = useWorkspaceStore((s) => s.syncingCatalog)
  const syncingEod = useWorkspaceStore((s) => s.syncingEod)
  const eodSyncProgress = useWorkspaceStore((s) => s.eodSyncProgress)
  const rightPanelOpen = useWorkspaceStore((s) => s.rightPanelOpen)
  const bottomDockOpen = useWorkspaceStore((s) => s.bottomDockOpen)
  const replay = useWorkspaceStore((s) => s.replay)
  const replaySelectingCut = useWorkspaceStore((s) => s.replaySelectingCut)

  const openSymbolSearch = useWorkspaceStore((s) => s.openSymbolSearch)
  const setTimezoneMode = useWorkspaceStore((s) => s.setTimezoneMode)
  const pushToast = useWorkspaceStore((s) => s.pushToast)
  const setActiveInterval = useWorkspaceStore((s) => s.setActiveInterval)
  const setActiveChartStyle = useWorkspaceStore((s) => s.setActiveChartStyle)
  const setActivePriceScale = useWorkspaceStore((s) => s.setActivePriceScale)
  const toggleActiveVnBands = useWorkspaceStore((s) => s.toggleActiveVnBands)
  const toggleVnSessionBreaks = useWorkspaceStore((s) => s.toggleVnSessionBreaks)
  const toggleActiveCountdown = useWorkspaceStore((s) => s.toggleActiveCountdown)
  const setIndicatorsModalOpen = useWorkspaceStore((s) => s.setIndicatorsModalOpen)
  const saveCurrentIndicatorTemplate = useWorkspaceStore((s) => s.saveCurrentIndicatorTemplate)
  const applyIndicatorTemplate = useWorkspaceStore((s) => s.applyIndicatorTemplate)
  const deleteIndicatorTemplate = useWorkspaceStore((s) => s.deleteIndicatorTemplate)
  const setGridLayout = useWorkspaceStore((s) => s.setGridLayout)
  const setSyncOption = useWorkspaceStore((s) => s.setSyncOption)
  const saveCurrentLayout = useWorkspaceStore((s) => s.saveCurrentLayout)
  const applySavedLayout = useWorkspaceStore((s) => s.applySavedLayout)
  const deleteSavedLayout = useWorkspaceStore((s) => s.deleteSavedLayout)
  const toggleTheme = useWorkspaceStore((s) => s.toggleTheme)
  const toggleOfflineMode = useWorkspaceStore((s) => s.toggleOfflineMode)
  const importCafeFDirectory = useWorkspaceStore((s) => s.importCafeFDirectory)
  const refreshSyncStatus = useWorkspaceStore((s) => s.refreshSyncStatus)
  const syncVnUniverseCatalog = useWorkspaceStore((s) => s.syncVnUniverseCatalog)
  const syncUniverseEod = useWorkspaceStore((s) => s.syncUniverseEod)
  const bootstrap = useWorkspaceStore((s) => s.bootstrap)
  const setRightPanelOpen = useWorkspaceStore((s) => s.setRightPanelOpen)
  const setBottomDockOpen = useWorkspaceStore((s) => s.setBottomDockOpen)
  const setReplaySelectingCut = useWorkspaceStore((s) => s.setReplaySelectingCut)
  const stopReplay = useWorkspaceStore((s) => s.stopReplay)
  const triggerExportPng = useWorkspaceStore((s) => s.triggerExportPng)
  const compareTickersBySlot = useWorkspaceStore((s) => s.compareTickersBySlot)
  const addCompareTicker = useWorkspaceStore((s) => s.addCompareTicker)
  const removeCompareTicker = useWorkspaceStore((s) => s.removeCompareTicker)
  const clearCompareTickers = useWorkspaceStore((s) => s.clearCompareTickers)
  const setActiveTicker = useWorkspaceStore((s) => s.setActiveTicker)

  const activePane = panes.find((p) => p.slotId === activeSlotId) ?? panes[0]
  const activeCompares = compareTickersBySlot[activePane.slotId] ?? []
  const [openMenu, setOpenMenu] = useState<
    'interval' | 'style' | 'templates' | 'layout' | 'saved' | 'data' | 'compare' | null
  >(null)
  const [compareInput, setCompareInput] = useState('')
  const [layoutNameInput, setLayoutNameInput] = useState('')
  const [tplNameInput, setTplNameInput] = useState('')
  const [cafefDirInput, setCafefDirInput] = useState('data/raw/cafef')
  const [cafefIncludeRaw, setCafefIncludeRaw] = useState(true)
  const [dataHubTab, setDataHubTab] = useState<'market' | 'sync'>('market')
  const [syncFolderInput, setSyncFolderInput] = useState('')
  const [importBundlePath, setImportBundlePath] = useState('')
  const [syncBusy, setSyncBusy] = useState(false)
  const headerRef = useRef<HTMLElement | null>(null)

  useEffect(() => {
    if (syncStatus?.syncFolder && !syncFolderInput) {
      setSyncFolderInput(syncStatus.syncFolder)
    }
  }, [syncStatus?.syncFolder, syncFolderInput])

  useEffect(() => {
    const onPointerDown = (e: PointerEvent) => {
      if (headerRef.current && !headerRef.current.contains(e.target as Node)) {
        setOpenMenu(null)
      }
    }
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpenMenu(null)
    }
    document.addEventListener('pointerdown', onPointerDown)
    document.addEventListener('keydown', onKeyDown)
    return () => {
      document.removeEventListener('pointerdown', onPointerDown)
      document.removeEventListener('keydown', onKeyDown)
    }
  }, [])

  const shortSymbol = activePane.ticker.includes(':')
    ? activePane.ticker.split(':')[1]
    : activePane.ticker
  const exchangeLabel = activePane.ticker.includes(':')
    ? activePane.ticker.split(':')[0]
    : ''
  const isVn = ['HOSE', 'HNX', 'UPCOM'].includes(exchangeLabel)

  return (
    <header
      ref={headerRef}
      className="relative z-30 flex items-center justify-between px-2 shrink-0"
      style={{
        height: 38,
        backgroundColor: 'var(--tv-bg-panel)',
        borderBottom: '1px solid var(--tv-border)',
      }}
    >
      {/* Left Cluster: Brand + Symbol Search + Timeframes + Style + Indicators + Replay */}
      <div className="flex items-center gap-0.5 min-w-0">
        {/* Symbol Search Trigger */}
        <button
          type="button"
          className="tv-btn font-semibold px-2"
          onClick={() => openSymbolSearch('')}
          title="Tìm kiếm mã chứng khoán / Crypto / Phái sinh (gõ phím bất kỳ)"
        >
          <Search className="w-3.5 h-3.5 text-[#2962FF]" />
          <span className="text-[13px] tracking-tight">{shortSymbol}</span>
          {exchangeLabel && (
            <span
              className="text-[10px] px-1 py-0.2 rounded font-medium"
              style={{
                backgroundColor: 'var(--tv-hover)',
                color: 'var(--tv-text-secondary)',
              }}
            >
              {exchangeLabel}
            </span>
          )}
        </button>

        {/* Compare (+) & Synthetic Formula Quick Popover (Wave 4) */}
        <div className="relative">
          <button
            type="button"
            className="tv-btn px-1.5"
            data-active={activeCompares.length > 0 || openMenu === 'compare'}
            onClick={() => setOpenMenu(openMenu === 'compare' ? null : 'compare')}
            title="So sánh đa mã (% quy chiếu) & Biểu đồ Công thức Tổng hợp (Ratio/Spread)"
            aria-label="So sánh mã hoặc mở biểu đồ công thức"
          >
            <PlusCircle className="w-3.5 h-3.5" />
            {activeCompares.length > 0 && (
              <span
                className="px-1 rounded-full text-[9.5px] font-bold"
                style={{ backgroundColor: 'var(--tv-accent-soft)', color: 'var(--tv-accent)' }}
              >
                {activeCompares.length}
              </span>
            )}
          </button>
          {openMenu === 'compare' && (
            <div
              className="absolute left-0 top-[34px] w-80 rounded-md p-2.5 shadow-2xl z-50 space-y-2.5 text-[11.5px]"
              style={{
                backgroundColor: 'var(--tv-bg-elevated)',
                border: '1px solid var(--tv-border)',
              }}
            >
              <div className="flex items-center justify-between">
                <span className="text-[11px] font-semibold uppercase tracking-wide" style={{ color: 'var(--tv-text-secondary)' }}>
                  So sánh mã (% quy chiếu nhân quả)
                </span>
                {activeCompares.length > 0 && (
                  <button
                    type="button"
                    onClick={() => clearCompareTickers(activePane.slotId)}
                    className="text-[10px] border-0 bg-transparent cursor-pointer text-[#F23645]"
                  >
                    Xóa hết
                  </button>
                )}
              </div>
              <form
                className="flex gap-1"
                onSubmit={(e) => {
                  e.preventDefault()
                  if (!compareInput.trim()) return
                  addCompareTicker(compareInput, activePane.slotId)
                  setCompareInput('')
                }}
              >
                <input
                  type="text"
                  value={compareInput}
                  onChange={(e) => setCompareInput(e.target.value)}
                  placeholder="Nhập mã (VD: VNINDEX, SSI, HPG)..."
                  className="flex-1 px-2 py-1 rounded text-[11.5px] outline-none"
                  style={{
                    backgroundColor: 'var(--tv-bg-input)',
                    border: '1px solid var(--tv-border)',
                    color: 'var(--tv-text-primary)',
                  }}
                />
                <button
                  type="submit"
                  className="px-2.5 py-1 rounded text-[11px] font-semibold bg-[#2962FF] text-white border-0 cursor-pointer"
                >
                  Thêm
                </button>
              </form>

              <div className="flex flex-wrap gap-1">
                {['HOSE:VNINDEX', 'HOSE:VN30', 'HOSE:FPT', 'HOSE:SSI', 'HOSE:HPG', 'HOSE:VCB'].map((tk) => {
                  const active = activeCompares.includes(tk)
                  return (
                    <button
                      key={tk}
                      type="button"
                      onClick={() =>
                        active
                          ? removeCompareTicker(tk, activePane.slotId)
                          : addCompareTicker(tk, activePane.slotId)
                      }
                      className="px-1.5 py-0.5 rounded text-[10.5px] border cursor-pointer"
                      style={{
                        backgroundColor: active ? 'var(--tv-accent-soft)' : 'var(--tv-bg-panel)',
                        borderColor: active ? 'var(--tv-accent)' : 'var(--tv-border)',
                        color: active ? 'var(--tv-accent)' : 'var(--tv-text-primary)',
                      }}
                    >
                      {active ? '✓ ' : '+ '}
                      {tk.split(':')[1]}
                    </button>
                  )
                })}
              </div>

              {activeCompares.length > 0 && (
                <div className="space-y-1 pt-1 border-t" style={{ borderColor: 'var(--tv-border)' }}>
                  {activeCompares.map((tk, idx) => (
                    <div
                      key={tk}
                      className="flex items-center justify-between px-2 py-1 rounded"
                      style={{ backgroundColor: 'var(--tv-bg-panel)' }}
                    >
                      <span className="flex items-center gap-1.5 font-semibold text-[11px]">
                        <span
                          className="w-2.5 h-2.5 rounded-full shrink-0"
                          style={{ backgroundColor: COMPARE_COLORS[idx % COMPARE_COLORS.length] }}
                        />
                        <span>{tk}</span>
                      </span>
                      <button
                        type="button"
                        onClick={() => removeCompareTicker(tk, activePane.slotId)}
                        className="border-0 bg-transparent p-0.5 cursor-pointer text-[var(--tv-text-secondary)] hover:text-[#F23645]"
                        aria-label={`Gỡ so sánh ${tk}`}
                      >
                        <X className="w-3 h-3" />
                      </button>
                    </div>
                  ))}
                </div>
              )}

              <div className="pt-2 border-t space-y-1" style={{ borderColor: 'var(--tv-border)' }}>
                <div className="text-[10.5px] font-semibold uppercase tracking-wide" style={{ color: 'var(--tv-text-secondary)' }}>
                  Biểu đồ Công thức Tổng hợp (Ratio / Spread)
                </div>
                {[
                  { label: `${shortSymbol} / VNINDEX (Sức mạnh tương đối)`, expr: `${activePane.ticker} / HOSE:VNINDEX` },
                  { label: 'VN30F1! - VN30 (Basis Phái sinh)', expr: 'HNX:VN30F1! - HOSE:VN30' },
                  { label: 'SSI / VNINDEX (RS Chứng khoán)', expr: 'HOSE:SSI / HOSE:VNINDEX' },
                ].map((item) => (
                  <button
                    key={item.expr}
                    type="button"
                    onClick={() => {
                      setActiveTicker(item.expr, activePane.slotId)
                      setOpenMenu(null)
                    }}
                    className="w-full text-left px-2 py-1 rounded text-[11px] border-0 bg-transparent hover:bg-[var(--tv-hover)] cursor-pointer text-[var(--tv-text-primary)] truncate"
                  >
                    <span className="text-[#2962FF] font-semibold">𝑓 </span>
                    {item.label}
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>

        <div className="tv-divider-v" />

        {/* Quick Timeframe Pills */}
        <div className="flex items-center gap-0.5">
          {QUICK_INTERVALS.map((iv) => (
            <button
              key={iv}
              type="button"
              className="tv-btn px-1.5 text-[11.5px]"
              data-active={activePane.interval === iv}
              onClick={() => setActiveInterval(iv)}
            >
              {iv === '1h' ? '1H' : iv === '4h' ? '4H' : iv}
            </button>
          ))}
          <div className="relative">
            <button
              type="button"
              className="tv-btn px-1"
              onClick={() => setOpenMenu(openMenu === 'interval' ? null : 'interval')}
              title="Tất cả khung thời gian"
            >
              <ChevronDown className="w-3.5 h-3.5" style={{ color: 'var(--tv-text-secondary)' }} />
            </button>
            {openMenu === 'interval' && (
              <div
                className="absolute left-0 top-[34px] w-40 rounded-md p-1.5 shadow-2xl z-50"
                style={{
                  backgroundColor: 'var(--tv-bg-elevated)',
                  border: '1px solid var(--tv-border)',
                }}
              >
                {ALL_INTERVALS.map((iv) => (
                  <button
                    key={iv.key}
                    type="button"
                    className="w-full flex items-center justify-between px-2 py-1.5 rounded text-left text-[12px] hover:bg-[var(--tv-hover)] cursor-pointer border-0 bg-transparent text-[var(--tv-text-primary)]"
                    style={activePane.interval === iv.key ? { color: 'var(--tv-accent)', fontWeight: 600 } : undefined}
                    onClick={() => {
                      setActiveInterval(iv.key)
                      setOpenMenu(null)
                    }}
                  >
                    <span>{iv.label}</span>
                    <span className="text-[10px]" style={{ color: 'var(--tv-text-secondary)' }}>
                      {iv.group}
                    </span>
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>

        <div className="tv-divider-v" />

        {/* Chart Style Picker */}
        <div className="relative">
          <button
            type="button"
            className="tv-btn"
            onClick={() => setOpenMenu(openMenu === 'style' ? null : 'style')}
            title="Kiểu biểu đồ (Nến Nhật / Rỗng / Heikin-Ashi / Đường / Vùng)"
          >
            <CandlestickChart className="w-4 h-4" />
            <ChevronDown className="w-3 h-3" style={{ color: 'var(--tv-text-secondary)' }} />
          </button>
          {openMenu === 'style' && (
            <div
              className="absolute left-0 top-[34px] w-52 rounded-md p-1.5 shadow-2xl z-50"
              style={{
                backgroundColor: 'var(--tv-bg-elevated)',
                border: '1px solid var(--tv-border)',
              }}
            >
              {CHART_STYLE_OPTIONS.map((opt) => (
                <button
                  key={opt.key}
                  type="button"
                  className="w-full flex items-center justify-between px-2.5 py-1.5 rounded text-left text-[12px] hover:bg-[var(--tv-hover)] cursor-pointer border-0 bg-transparent text-[var(--tv-text-primary)]"
                  style={activePane.chartStyle === opt.key ? { color: 'var(--tv-accent)', fontWeight: 600 } : undefined}
                  onClick={() => {
                    setActiveChartStyle(opt.key)
                    setOpenMenu(null)
                  }}
                >
                  <span>{opt.label}</span>
                  {activePane.chartStyle === opt.key && <span>●</span>}
                </button>
              ))}
            </div>
          )}
        </div>

        <div className="tv-divider-v" />

        {/* Indicators Modal Trigger */}
        <button
          type="button"
          className="tv-btn"
          onClick={() => setIndicatorsModalOpen(true)}
          title="Chỉ báo kỹ thuật (26+ chỉ báo gốc + Ichimoku, SuperTrend, VWAP, ATR, StochRSI)"
        >
          <Activity className="w-4 h-4" />
          <span>Chỉ báo</span>
          {activePane.indicators.length > 0 && (
            <span
              className="px-1.5 py-0 rounded-full text-[10px] font-semibold"
              style={{ backgroundColor: 'var(--tv-accent-soft)', color: 'var(--tv-accent)' }}
            >
              {activePane.indicators.length}
            </span>
          )}
        </button>

        {/* Indicator Templates Dropdown (v2.2) */}
        <div className="relative">
          <button
            type="button"
            className="tv-icon-btn"
            onClick={() => setOpenMenu(openMenu === 'templates' ? null : 'templates')}
            title="Mẫu chỉ báo (Indicator Templates — áp dụng nhanh 1-click)"
            aria-label="Mẫu chỉ báo"
          >
            <Layers className="w-3.5 h-3.5" />
          </button>
          {openMenu === 'templates' && (
            <div
              className="absolute left-0 top-[34px] w-72 rounded-md p-2.5 shadow-2xl z-50 space-y-2"
              style={{
                backgroundColor: 'var(--tv-bg-elevated)',
                border: '1px solid var(--tv-border)',
              }}
            >
              <div className="text-[11px] font-semibold uppercase tracking-wide" style={{ color: 'var(--tv-text-secondary)' }}>
                Mẫu chỉ báo ({indicatorTemplates.length})
              </div>
              <div className="flex gap-1">
                <input
                  type="text"
                  value={tplNameInput}
                  onChange={(e) => setTplNameInput(e.target.value)}
                  placeholder="Lưu bộ hiện tại (VD: Swing FPT)..."
                  className="flex-1 px-2 py-1 rounded text-[11.5px] outline-none"
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
                    if (!tplNameInput.trim()) return
                    void saveCurrentIndicatorTemplate(tplNameInput.trim())
                    setTplNameInput('')
                  }}
                >
                  Lưu
                </button>
              </div>
              <div className="max-h-56 overflow-y-auto tv-scroll space-y-1">
                {indicatorTemplates.map((tpl) => (
                  <div
                    key={tpl.id}
                    className="flex items-center justify-between gap-1 px-2 py-1.5 rounded hover:bg-[var(--tv-hover)]"
                  >
                    <button
                      type="button"
                      className="flex-1 min-w-0 text-left border-0 bg-transparent p-0 cursor-pointer"
                      onClick={() => {
                        applyIndicatorTemplate(tpl)
                        setOpenMenu(null)
                      }}
                    >
                      <div className="text-[11.5px] font-semibold truncate text-[var(--tv-text-primary)]">
                        {tpl.name}
                      </div>
                      <div className="text-[10px] truncate" style={{ color: 'var(--tv-text-secondary)' }}>
                        {tpl.indicators.map((i) => i.name).join(' + ')}
                      </div>
                    </button>
                    <button
                      type="button"
                      aria-label={`Xóa mẫu ${tpl.name}`}
                      className="border-0 bg-transparent p-1 cursor-pointer text-[var(--tv-text-secondary)] hover:text-[#F23645]"
                      onClick={() => void deleteIndicatorTemplate(tpl.id)}
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Causal Bar Replay Button — opens Interactive Scissor Curtain immediately like TradingView */}
        <button
          type="button"
          className="tv-btn"
          data-active={Boolean(replay?.active) || replaySelectingCut}
          onClick={() => {
            if (replay?.active) {
              void stopReplay()
            } else {
              setReplaySelectingCut(!replaySelectingCut)
            }
          }}
          title="Phát lại nến nhân quả (Bar Replay — kéo rèm cắt chọn điểm bất kỳ trên biểu đồ)"
        >
          <History className="w-4 h-4" />
          <span>Phát lại</span>
        </button>

        <div className="tv-divider-v" />

        {/* Scale Quick Toggles (Normal / % / Log) */}
        <div className="hidden lg:flex items-center gap-0.5">
          {SCALE_OPTIONS.map((sc) => (
            <button
              key={sc.key}
              type="button"
              className="tv-btn px-1.5 text-[11px]"
              data-active={activePane.priceScale === sc.key}
              onClick={() => setActivePriceScale(sc.key)}
              title={sc.label}
            >
              {sc.short}
            </button>
          ))}
        </div>

        {/* VN Ceiling/Ref/Floor Band Toggle & VN Intraday Session Breaks Toggle */}
        {isVn && (
          <>
            <button
              type="button"
              className="hidden xl:inline-flex tv-btn px-1.5 text-[11px]"
              data-active={activePane.showVnBands}
              onClick={toggleActiveVnBands}
              title="Hiện/Ẩn biên độ Trần / Tham chiếu / Sàn Việt Nam"
            >
              <span style={{ color: 'var(--tv-vn-ceil)' }}>T</span>/
              <span style={{ color: 'var(--tv-vn-ref)' }}>TC</span>/
              <span style={{ color: 'var(--tv-vn-floor)' }}>S</span>
            </button>
            <button
              type="button"
              className="hidden xl:inline-flex tv-btn px-1.5 text-[10.5px]"
              data-active={showVnSessionBreaks}
              aria-pressed={showVnSessionBreaks}
              onClick={toggleVnSessionBreaks}
              title="Hiện/Ẩn vạch tách phiên Sáng 09:00 / Chiều 13:00 / ATC 14:30 trên khung Intraday VN"
            >
              Phiên
            </button>
          </>
        )}

        <button
          type="button"
          className="hidden xl:inline-flex tv-icon-btn"
          data-active={activePane.showCountdown}
          onClick={toggleActiveCountdown}
          title="Đếm ngược đóng nến trên trục giá"
        >
          <Clock className="w-3.5 h-3.5" />
        </button>
      </div>

      {/* Right Cluster: Multi-Chart Layout + Saved Layouts + Online/Offline + Export + Theme + Panel Toggles */}
      <div className="flex items-center gap-0.5 shrink-0">
        {/* Multi-Chart Layout Menu (1 / 2v / 2h / 4 + Sync) */}
        <div className="relative">
          <button
            type="button"
            className="tv-btn"
            onClick={() => setOpenMenu(openMenu === 'layout' ? null : 'layout')}
            title="Bố cục đa biểu đồ & Đồng bộ"
          >
            <LayoutGrid className="w-4 h-4" />
            <ChevronDown className="w-3 h-3" style={{ color: 'var(--tv-text-secondary)' }} />
          </button>
          {openMenu === 'layout' && (
            <div
              className="absolute right-0 top-[34px] w-76 rounded-md p-2.5 shadow-2xl z-50 space-y-2.5"
              style={{
                backgroundColor: 'var(--tv-bg-elevated)',
                border: '1px solid var(--tv-border)',
              }}
            >
              <div className="text-[11px] font-semibold uppercase tracking-wide" style={{ color: 'var(--tv-text-secondary)' }}>
                Khung đa biểu đồ (Đa khung thời gian)
              </div>
              <div className="grid grid-cols-5 gap-1.5">
                {(
                  [
                    { key: '1', label: '1 ô', Icon: Square },
                    { key: '2v', label: '2 dọc', Icon: Columns2 },
                    { key: '2h', label: '2 ngang', Icon: Rows2 },
                    { key: '3', label: '3 ô', Icon: LayoutPanelLeft },
                    { key: '4', label: '4 ô', Icon: Grid2x2 },
                  ] as Array<{ key: GridLayoutKind; label: string; Icon: React.ComponentType<{ className?: string }> }>
                ).map(({ key, label, Icon }) => (
                  <button
                    key={key}
                    type="button"
                    className="flex flex-col items-center gap-1 py-1.5 px-1 rounded border cursor-pointer text-[10px] whitespace-nowrap"
                    style={{
                      backgroundColor: gridLayout === key ? 'var(--tv-accent-soft)' : 'transparent',
                      borderColor: gridLayout === key ? 'var(--tv-accent)' : 'var(--tv-border)',
                      color: gridLayout === key ? 'var(--tv-accent)' : 'var(--tv-text-primary)',
                    }}
                    onClick={() => {
                      setGridLayout(key)
                      setOpenMenu(null)
                    }}
                  >
                    <Icon className="w-4 h-4 shrink-0" />
                    <span className="whitespace-nowrap">{label}</span>
                  </button>
                ))}
              </div>
              <div className="border-t pt-2 space-y-1.5" style={{ borderColor: 'var(--tv-border)' }}>
                <div className="text-[11px] font-semibold uppercase tracking-wide" style={{ color: 'var(--tv-text-secondary)' }}>
                  Đồng bộ giữa các ô
                </div>
                {(
                  [
                    { key: 'symbol', label: 'Đồng bộ Mã (Symbol)' },
                    { key: 'interval', label: 'Đồng bộ Khung giờ' },
                    { key: 'crosshair', label: 'Đồng bộ Con trỏ chữ thập' },
                    { key: 'dateRange', label: 'Đồng bộ Vùng thời gian (Time)' },
                    { key: 'drawings', label: 'Đồng bộ Hình vẽ cùng mã' },
                  ] as const
                ).map((item) => (
                  <label key={item.key} className="flex items-center justify-between text-[12px] cursor-pointer py-0.5">
                    <span>{item.label}</span>
                    <input
                      type="checkbox"
                      checked={sync[item.key]}
                      onChange={(e) => setSyncOption(item.key, e.target.checked)}
                    />
                  </label>
                ))}
              </div>
              <div className="border-t pt-2" style={{ borderColor: 'var(--tv-border)' }}>
                <button
                  type="button"
                  className="w-full tv-btn justify-center text-[11.5px] py-1"
                  onClick={() => {
                    const nextIv = activePane.interval === '1h' ? '4h' : activePane.interval === '4h' ? '1h' : '4h'
                    void api
                      .openDetachedWindow(activePane.ticker, nextIv)
                      .then((label) => {
                        if (label) {
                          pushToast('success', `Đã mở cửa sổ rời ${activePane.ticker} (${nextIv}) — đồng bộ Replay & Hình vẽ`)
                        } else {
                          pushToast('info', 'Tính năng mở cửa sổ rời khả dụng trên ứng dụng Desktop Tauri')
                        }
                      })
                      .catch((err) => pushToast('error', String(err)))
                    setOpenMenu(null)
                  }}
                >
                  <ExternalLink className="w-3.5 h-3.5" aria-hidden="true" />
                  <span>Mở cửa sổ biểu đồ rời (Đa màn hình)</span>
                </button>
              </div>
            </div>
          )}
        </div>

        {/* Saved Layouts (SQLite) */}
        <div className="relative">
          <button
            type="button"
            className="tv-btn"
            onClick={() => setOpenMenu(openMenu === 'saved' ? null : 'saved')}
            title="Lưu & Khôi phục Bố cục (SQLite)"
          >
            <Save className="w-3.5 h-3.5" />
            <span className="hidden md:inline">Lưu</span>
          </button>
          {openMenu === 'saved' && (
            <div
              className="absolute right-0 top-[34px] w-64 rounded-md p-2.5 shadow-2xl z-50 space-y-2"
              style={{
                backgroundColor: 'var(--tv-bg-elevated)',
                border: '1px solid var(--tv-border)',
              }}
            >
              <div className="text-[11px] font-semibold" style={{ color: 'var(--tv-text-secondary)' }}>
                LƯU BỐ CỤC VÀO SQLITE
              </div>
              <div className="flex gap-1.5">
                <input
                  type="text"
                  value={layoutNameInput}
                  onChange={(e) => setLayoutNameInput(e.target.value)}
                  placeholder="Tên bố cục mới..."
                  className="flex-1 px-2 py-1 rounded text-[12px] outline-none"
                  style={{
                    backgroundColor: 'var(--tv-bg-input)',
                    border: '1px solid var(--tv-border)',
                    color: 'var(--tv-text-primary)',
                  }}
                />
                <button
                  type="button"
                  className="px-2.5 py-1 rounded text-[11px] font-semibold bg-[#2962FF] text-white border-0 cursor-pointer"
                  onClick={() => {
                    if (!layoutNameInput.trim()) return
                    void saveCurrentLayout(layoutNameInput)
                    setLayoutNameInput('')
                  }}
                >
                  Lưu
                </button>
              </div>
              <div className="max-h-44 overflow-y-auto tv-scroll space-y-1">
                {layouts.length === 0 ? (
                  <div className="text-[11px] py-2 text-center" style={{ color: 'var(--tv-text-secondary)' }}>
                    Chưa có bố cục nào được lưu
                  </div>
                ) : (
                  layouts.map((l) => (
                    <div
                      key={l.id}
                      className="flex items-center justify-between px-2 py-1 rounded hover:bg-[var(--tv-hover)]"
                    >
                      <button
                        type="button"
                        className="flex-1 text-left bg-transparent border-0 text-[var(--tv-text-primary)] cursor-pointer text-[12px] truncate"
                        onClick={() => {
                          applySavedLayout(l)
                          setOpenMenu(null)
                        }}
                      >
                        {l.name}
                      </button>
                      <button
                        type="button"
                        className="p-1 bg-transparent border-0 cursor-pointer text-[var(--tv-text-secondary)] hover:text-[#F23645]"
                        onClick={() => void deleteSavedLayout(l.id)}
                        title="Xóa bố cục"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    </div>
                  ))
                )}
              </div>
            </div>
          )}
        </div>

        <div className="tv-divider-v" />

        {/* Timezone Switcher (`Auto` -> `UTC+7 VN` -> `UTC`, fixes `BUG-031`) */}
        <button
          type="button"
          className="tv-btn px-1.5 text-[10.5px] tabular-nums"
          aria-label="Chuyển đổi múi giờ biểu đồ"
          onClick={() => {
            const order: TimezoneMode[] = ['auto', 'Asia/Ho_Chi_Minh', 'Etc/UTC']
            const next = order[(order.indexOf(timezoneMode) + 1) % order.length]
            setTimezoneMode(next)
            pushToast(
              'info',
              next === 'Asia/Ho_Chi_Minh'
                ? 'Múi giờ biểu đồ: UTC+7 (Giờ Việt Nam)'
                : next === 'Etc/UTC'
                  ? 'Múi giờ biểu đồ: UTC (Quốc tế)'
                  : 'Múi giờ biểu đồ: Tự động theo sàn',
            )
          }}
          title="Đổi múi giờ biểu đồ (Tự động / UTC+7 Việt Nam / UTC)"
        >
          <Globe className="w-3.5 h-3.5" aria-hidden="true" />
          <span className="hidden xl:inline">
            {timezoneMode === 'Asia/Ho_Chi_Minh' ? 'UTC+7' : timezoneMode === 'Etc/UTC' ? 'UTC' : 'Auto TZ'}
          </span>
        </button>

        {/* Export PNG */}
        <button
          type="button"
          className="tv-icon-btn"
          onClick={triggerExportPng}
          title="Chụp ảnh biểu đồ PNG"
        >
          <Camera className="w-4 h-4" />
        </button>

        {/* Online / Offline Cache & CafeF CSV Importer Popover */}
        <div className="relative flex items-center gap-0.5">
          <button
            type="button"
            className="tv-icon-btn"
            data-active={openMenu === 'data'}
            aria-expanded={openMenu === 'data'}
            aria-label="Quản lý kho SQLite và nạp thư mục CafeF CSV"
            onClick={() => setOpenMenu(openMenu === 'data' ? null : 'data')}
            title="Quản lý dữ liệu SQLite & Nạp nhanh thư mục CafeF CSV"
          >
            <Database aria-hidden="true" className="w-4 h-4" />
          </button>
          <button
            type="button"
            className="tv-icon-btn"
            data-active={dataStatus?.offlineMode}
            aria-pressed={Boolean(dataStatus?.offlineMode)}
            aria-label={dataStatus?.offlineMode ? 'Bật lại chế độ Online Realtime' : 'Chuyển sang chế độ Offline SQLite'}
            onClick={() => void toggleOfflineMode()}
            title={
              dataStatus?.offlineMode
                ? `Đang ở chế độ Offline (${dataStatus.stats.total_symbols} mã / ${dataStatus.stats.total_bars.toLocaleString('vi-VN')} nến)`
                : `Online + Cache (${dataStatus?.stats.total_symbols ?? 0} mã trong SQLite)`
            }
          >
            {dataStatus?.offlineMode ? (
              <WifiOff aria-hidden="true" className="w-4 h-4 text-[#FBC02D]" />
            ) : (
              <Wifi aria-hidden="true" className="w-4 h-4 text-[#089981]" />
            )}
          </button>

          {openMenu === 'data' && (
            <div
              className="absolute right-0 top-[34px] w-[368px] rounded-md p-3 shadow-2xl z-50 space-y-2.5 max-h-[80vh] overflow-y-auto tv-scroll"
              style={{
                backgroundColor: 'var(--tv-bg-elevated)',
                border: '1px solid var(--tv-border)',
              }}
            >
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-1">
                  <button
                    type="button"
                    onClick={() => setDataHubTab('market')}
                    className="px-2 py-0.5 rounded text-[11px] font-semibold border cursor-pointer"
                    style={{
                      backgroundColor: dataHubTab === 'market' ? 'var(--tv-accent-soft)' : 'transparent',
                      borderColor: dataHubTab === 'market' ? 'var(--tv-accent)' : 'var(--tv-border)',
                      color: dataHubTab === 'market' ? 'var(--tv-accent)' : 'var(--tv-text-secondary)',
                    }}
                  >
                    Kho Dữ Liệu
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      setDataHubTab('sync')
                      void refreshSyncStatus()
                    }}
                    className="px-2 py-0.5 rounded text-[11px] font-semibold border cursor-pointer"
                    style={{
                      backgroundColor: dataHubTab === 'sync' ? 'var(--tv-accent-soft)' : 'transparent',
                      borderColor: dataHubTab === 'sync' ? 'var(--tv-accent)' : 'var(--tv-border)',
                      color: dataHubTab === 'sync' ? 'var(--tv-accent)' : 'var(--tv-text-secondary)',
                    }}
                  >
                    Đồng Bộ Đa Máy
                  </button>
                </div>
                <span
                  className="px-1.5 py-0.5 rounded text-[10px] font-semibold"
                  style={{
                    backgroundColor: dataStatus?.offlineMode ? 'rgba(251, 192, 45, 0.16)' : 'var(--tv-up-soft)',
                    color: dataStatus?.offlineMode ? '#FBC02D' : 'var(--tv-up)',
                  }}
                >
                  {dataStatus?.offlineMode ? 'OFFLINE' : 'ONLINE + CACHE'}
                </span>
              </div>

              {dataHubTab === 'market' ? (
                <>
                  <div
                    className="grid grid-cols-2 gap-2 p-2 rounded text-[11px] tabular-nums"
                    style={{ backgroundColor: 'var(--tv-bg-panel)', border: '1px solid var(--tv-border)' }}
                  >
                    <div>
                      <div style={{ color: 'var(--tv-text-secondary)' }}>Số mã đã lưu</div>
                      <strong className="text-[13px]">{(dataStatus?.stats.total_symbols ?? 0).toLocaleString('en-US')}</strong>
                    </div>
                    <div>
                      <div style={{ color: 'var(--tv-text-secondary)' }}>Tổng số nến</div>
                      <strong className="text-[13px]">{(dataStatus?.stats.total_bars ?? 0).toLocaleString('en-US')}</strong>
                    </div>
                    {dataStatus?.freshness && (
                      <div className="col-span-2 pt-1.5 border-t text-[10.5px]" style={{ borderColor: 'var(--tv-border)' }}>
                        <div className="flex items-center justify-between">
                          <span style={{ color: 'var(--tv-text-secondary)' }}>
                            Phiên EOD kỳ vọng: <strong>{dataStatus.freshness.expected_session_date}</strong>
                          </span>
                          <span
                            className="font-semibold"
                            style={{
                              color: dataStatus.freshness.stale_series > 0 ? '#FBC02D' : 'var(--tv-up)',
                            }}
                          >
                            {dataStatus.freshness.fresh_series}/{dataStatus.freshness.total_vn_d1_series} mới
                          </span>
                        </div>
                        {dataStatus.freshness.stale_series > 0 && (
                          <div className="text-[10px] mt-0.5" style={{ color: '#FBC02D' }}>
                            {dataStatus.freshness.stale_series} mã VN cũ (trung vị {dataStatus.freshness.median_latest_date ?? '—'})
                          </div>
                        )}
                      </div>
                    )}
                  </div>

                  {/* 1-click VN Universe Catalog & EOD Sync */}
                  <div className="grid grid-cols-2 gap-1.5">
                    <button
                      type="button"
                      disabled={syncingCatalog || Boolean(dataStatus?.offlineMode)}
                      onClick={() => void syncVnUniverseCatalog()}
                      className="flex items-center justify-center gap-1 py-1.5 px-2 rounded text-[11px] font-semibold border cursor-pointer disabled:opacity-50"
                      style={{
                        backgroundColor: 'var(--tv-bg-panel)',
                        borderColor: 'var(--tv-border)',
                        color: 'var(--tv-text-primary)',
                      }}
                      title="Quét danh mục ~1.600+ mã HOSE / HNX / UPCOM từ TradingView vào SQLite"
                    >
                      <RefreshCw className={`w-3 h-3 ${syncingCatalog ? 'animate-spin' : ''}`} />
                      <span>{syncingCatalog ? 'Đang quét...' : 'Danh mục ~1.600'}</span>
                    </button>
                    <button
                      type="button"
                      disabled={syncingEod || Boolean(dataStatus?.offlineMode)}
                      onClick={() => void syncUniverseEod('ALL_VN', 150)}
                      className="flex items-center justify-center gap-1 py-1.5 px-2 rounded text-[11px] font-semibold bg-[#089981] hover:bg-[#067f6b] text-white border-0 cursor-pointer disabled:opacity-50"
                      title="Cập nhật nến ngày EOD cho các mã VN bị cũ hơn phiên gần nhất"
                    >
                      <RefreshCw className={`w-3 h-3 ${syncingEod ? 'animate-spin' : ''}`} />
                      <span>{syncingEod ? 'Đang đồng bộ...' : 'Đồng bộ EOD VN'}</span>
                    </button>
                  </div>

                  {eodSyncProgress && syncingEod && (
                    <div
                      className="p-2 rounded text-[10.5px] space-y-1"
                      style={{ backgroundColor: 'var(--tv-bg-panel)', border: '1px solid var(--tv-border)' }}
                    >
                      <div className="flex justify-between tabular-nums">
                        <span className="truncate">
                          {eodSyncProgress.currentTicker
                            ? `Đang đồng bộ ${eodSyncProgress.currentTicker}...`
                            : 'Đang kiểm tra các mã VN cũ...'}
                        </span>
                        <span>
                          {eodSyncProgress.completed}/{eodSyncProgress.total}
                        </span>
                      </div>
                      <div className="w-full h-1.5 rounded overflow-hidden" style={{ backgroundColor: 'var(--tv-border)' }}>
                        <div
                          className="h-full bg-[#089981] transition-all duration-150"
                          style={{
                            width: `${eodSyncProgress.total > 0 ? Math.min(100, (eodSyncProgress.completed / eodSyncProgress.total) * 100) : 15}%`,
                          }}
                        />
                      </div>
                    </div>
                  )}

                  <form
                    className="space-y-1.5 border-t pt-2"
                    style={{ borderColor: 'var(--tv-border)' }}
                    onSubmit={(e) => {
                      e.preventDefault()
                      void importCafeFDirectory(cafefDirInput, cafefIncludeRaw)
                    }}
                  >
                    <label
                      htmlFor="sumi-cafef-dir-input"
                      className="block text-[11px] font-medium"
                      style={{ color: 'var(--tv-text-secondary)' }}
                    >
                      Nạp thư mục CSV CafeF (Mac / Windows)
                    </label>
                    <div className="flex items-center justify-between gap-1 text-[10px]">
                      <div className="flex gap-1">
                        {['data/raw/cafef', 'work/CafeFData'].map((preset) => (
                          <button
                            key={preset}
                            type="button"
                            className="px-1.5 py-0.5 rounded border cursor-pointer bg-transparent"
                            style={{ borderColor: 'var(--tv-border)', color: 'var(--tv-text-secondary)' }}
                            onClick={() => setCafefDirInput(preset)}
                          >
                            {preset}
                          </button>
                        ))}
                      </div>
                      <label className="inline-flex items-center gap-1 cursor-pointer" style={{ color: 'var(--tv-text-secondary)' }}>
                        <input
                          type="checkbox"
                          checked={cafefIncludeRaw}
                          onChange={(e) => setCafefIncludeRaw(e.target.checked)}
                          className="m-0"
                        />
                        <span>Kèm RAW (D/S)</span>
                      </label>
                    </div>
                    <input
                      id="sumi-cafef-dir-input"
                      type="text"
                      value={cafefDirInput}
                      onChange={(e) => setCafefDirInput(e.target.value)}
                      placeholder="VD: D:\CafeFData hoặc data/raw/cafef"
                      className="w-full px-2 py-1.5 rounded text-[11.5px] outline-none"
                      style={{
                        backgroundColor: 'var(--tv-bg-input)',
                        border: '1px solid var(--tv-border)',
                        color: 'var(--tv-text-primary)',
                      }}
                    />
                    <button
                      type="submit"
                      disabled={importingCafeF || !cafefDirInput.trim()}
                      className="w-full py-1.5 rounded text-[11.5px] font-semibold bg-[#2962FF] hover:bg-[#1E53E5] text-white border-0 cursor-pointer disabled:opacity-50"
                    >
                      {importingCafeF ? 'Đang nạp CSV vào SQLite...' : 'Nạp thư mục CafeF vào SQLite'}
                    </button>
                  </form>

                  {dataStatus?.dataDir && (
                    <div className="text-[9.5px] font-mono truncate pt-1 border-t" style={{ borderColor: 'var(--tv-border)', color: 'var(--tv-text-muted)' }} title={dataStatus.dataDir}>
                      Data: {dataStatus.dataDir} {dataStatus.yumekoDbDetected ? '· ✓ Yumeko DB' : ''}
                    </div>
                  )}
                </>
              ) : (
                <div className="space-y-2.5 text-[11px]">
                  <div
                    className="p-2 rounded space-y-1"
                    style={{ backgroundColor: 'var(--tv-bg-panel)', border: '1px solid var(--tv-border)' }}
                  >
                    <div className="flex items-center justify-between">
                      <span style={{ color: 'var(--tv-text-secondary)' }}>Thiết bị</span>
                      <span className="font-mono text-[10px]">{syncStatus?.deviceId ?? '—'}</span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span style={{ color: 'var(--tv-text-secondary)' }}>Bản sao lưu (.pre_sync.bak)</span>
                      <strong className="tabular-nums">{syncStatus?.preSyncBackupCount ?? 0} / 5</strong>
                    </div>
                    {syncStatus?.lastSyncedSha256 && (
                      <div className="flex items-center justify-between">
                        <span style={{ color: 'var(--tv-text-secondary)' }}>SHA-256 gần nhất</span>
                        <span className="font-mono text-[10px]">{syncStatus.lastSyncedSha256.slice(0, 12)}…</span>
                      </div>
                    )}
                  </div>

                  {/* Cloud Folder Bidirectional Sync (iCloud / OneDrive / Syncthing) */}
                  <div className="space-y-1.5">
                    <label className="block text-[11px] font-semibold" style={{ color: 'var(--tv-text-secondary)' }}>
                      Thư mục Đám mây 2 chiều (iCloud / OneDrive / Syncthing)
                    </label>
                    <div className="flex gap-1">
                      <input
                        type="text"
                        value={syncFolderInput}
                        onChange={(e) => setSyncFolderInput(e.target.value)}
                        placeholder="VD: ~/Library/Mobile Documents/.../SumiSync"
                        className="flex-1 px-2 py-1 rounded text-[11px] outline-none"
                        style={{
                          backgroundColor: 'var(--tv-bg-input)',
                          border: '1px solid var(--tv-border)',
                          color: 'var(--tv-text-primary)',
                        }}
                      />
                      <button
                        type="button"
                        disabled={syncBusy}
                        onClick={() => {
                          setSyncBusy(true)
                          void api
                            .syncConfigureFolder(syncFolderInput.trim() || null)
                            .then(() => refreshSyncStatus())
                            .then(() => pushToast('success', 'Đã lưu cấu hình thư mục đồng bộ đám mây'))
                            .catch((err) => pushToast('error', String(err)))
                            .finally(() => setSyncBusy(false))
                        }}
                        className="px-2 py-1 rounded text-[11px] font-semibold border cursor-pointer"
                        style={{ backgroundColor: 'var(--tv-bg-panel)', borderColor: 'var(--tv-border)', color: 'var(--tv-text-primary)' }}
                      >
                        Lưu
                      </button>
                    </div>
                    <button
                      type="button"
                      disabled={syncBusy || !syncStatus?.syncFolder}
                      onClick={() => {
                        setSyncBusy(true)
                        void api
                          .syncFolderNow()
                          .then(async (res) => {
                            await refreshSyncStatus()
                            await bootstrap()
                            pushToast(
                              'success',
                              `${res.pulledAndMerged ? 'Đã kéo gộp & đẩy' : 'Đã đẩy'} thành công tới ${res.bundlePath} (SHA ${res.pushedSha256.slice(0, 8)})`,
                            )
                          })
                          .catch((err) => pushToast('error', String(err)))
                          .finally(() => setSyncBusy(false))
                      }}
                      className="w-full flex items-center justify-center gap-1.5 py-1.5 rounded text-[11.5px] font-semibold bg-[#2962FF] hover:bg-[#1E53E5] text-white border-0 cursor-pointer disabled:opacity-50"
                    >
                      <FolderSync className="w-3.5 h-3.5" />
                      <span>{syncBusy ? 'Đang đồng bộ 2 chiều...' : 'Đồng bộ 2 chiều ngay (LWW + SHA-256)'}</span>
                    </button>
                  </div>

                  {/* Export / Import Manual .sumi.json Bundle */}
                  <div className="space-y-1.5 border-t pt-2" style={{ borderColor: 'var(--tv-border)' }}>
                    <div className="flex gap-1.5">
                      <button
                        type="button"
                        disabled={syncBusy}
                        onClick={() => {
                          setSyncBusy(true)
                          void api
                            .syncExportBundle(null)
                            .then((res) => {
                              void refreshSyncStatus()
                              pushToast(
                                'success',
                                `Đã xuất ${res.path} (${res.drawingsCount} hình vẽ · ${res.watchlistsCount} danh mục · SHA ${res.payloadSha256.slice(0, 8)})`,
                              )
                            })
                            .catch((err) => pushToast('error', String(err)))
                            .finally(() => setSyncBusy(false))
                        }}
                        className="flex-1 flex items-center justify-center gap-1 py-1.5 rounded text-[11px] font-semibold border cursor-pointer"
                        style={{ backgroundColor: 'var(--tv-bg-panel)', borderColor: 'var(--tv-border)', color: 'var(--tv-text-primary)' }}
                      >
                        <Download className="w-3 h-3" />
                        <span>Xuất .sumi.json</span>
                      </button>
                    </div>
                    <div className="flex gap-1">
                      <input
                        type="text"
                        value={importBundlePath}
                        onChange={(e) => setImportBundlePath(e.target.value)}
                        placeholder="Đường dẫn tệp .sumi.json để gộp..."
                        className="flex-1 px-2 py-1 rounded text-[11px] outline-none"
                        style={{
                          backgroundColor: 'var(--tv-bg-input)',
                          border: '1px solid var(--tv-border)',
                          color: 'var(--tv-text-primary)',
                        }}
                      />
                      <button
                        type="button"
                        disabled={syncBusy || !importBundlePath.trim()}
                        onClick={() => {
                          setSyncBusy(true)
                          void api
                            .syncImportBundle(importBundlePath.trim())
                            .then(async (rep) => {
                              await refreshSyncStatus()
                              await bootstrap()
                              pushToast(
                                'success',
                                `Đã gộp bundle (SHA ${rep.bundleSha256.slice(0, 8)}): ${rep.mergedDrawings} hình vẽ, ${rep.mergedWatchlists} WL`,
                              )
                              setImportBundlePath('')
                            })
                            .catch((err) => pushToast('error', String(err)))
                            .finally(() => setSyncBusy(false))
                        }}
                        className="flex items-center gap-1 px-2 py-1 rounded text-[11px] font-semibold bg-[#089981] text-white border-0 cursor-pointer disabled:opacity-50"
                      >
                        <Upload className="w-3 h-3" />
                        <span>Nhập</span>
                      </button>
                    </div>
                  </div>

                  {/* Pre-wired Doraemi OAuth 2.1 + PKCE Transport Status */}
                  {syncStatus?.doraemiTransport && (
                    <div
                      className="p-2 rounded text-[10px] space-y-0.5 border-t"
                      style={{ backgroundColor: 'var(--tv-bg-panel)', borderColor: 'var(--tv-border)', color: 'var(--tv-text-secondary)' }}
                    >
                      <div className="flex items-center justify-between font-semibold">
                        <span>Doraemi OAuth 2.1 + PKCE</span>
                        <span style={{ color: syncStatus.doraemiTransport.readyForCloudPush ? 'var(--tv-up)' : '#FBC02D' }}>
                          {syncStatus.doraemiTransport.readyForCloudPush
                            ? 'Sẵn sàng'
                            : syncStatus.doraemiTransport.configured
                              ? 'Đã nhận token · Chờ Migration 0006'
                              : 'Chờ Migration 0006'}
                        </span>
                      </div>
                      <div>{syncStatus.doraemiTransport.migration0006RequiredNote}</div>
                    </div>
                  )}
                </div>
              )}
            </div>
          )}
        </div>

        {/* Dark / Light Theme Toggle */}
        <button
          type="button"
          className="tv-icon-btn"
          onClick={toggleTheme}
          title={theme === 'dark' ? 'Chuyển sang Giao diện Sáng (Light)' : 'Chuyển sang Giao diện Tối (Dark)'}
        >
          {theme === 'dark' ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
        </button>

        <div className="tv-divider-v" />

        {/* Dock & Right Panel Visibility Toggles */}
        <button
          type="button"
          className="tv-icon-btn"
          data-active={bottomDockOpen}
          onClick={() => setBottomDockOpen(!bottomDockOpen)}
          title="Ẩn/Hiện Thanh giao dịch dưới (Paper Broker / Backtest / Screener)"
        >
          <PanelBottom className="w-4 h-4" />
        </button>
        <button
          type="button"
          className="tv-icon-btn"
          data-active={rightPanelOpen}
          onClick={() => setRightPanelOpen(!rightPanelOpen)}
          title="Ẩn/Hiện Bảng Watchlist & Chi tiết mã bên phải"
        >
          <PanelRight className="w-4 h-4" />
        </button>
      </div>
    </header>
  )
}
