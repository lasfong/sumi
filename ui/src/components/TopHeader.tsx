import React, { useEffect, useRef, useState } from 'react'
import {
  Activity,
  Camera,
  CandlestickChart,
  ChevronDown,
  Clock,
  Columns2,
  Database,
  Grid2x2,
  History,
  LayoutGrid,
  Moon,
  PanelBottom,
  PanelRight,
  Rows2,
  Save,
  Search,
  Square,
  Sun,
  Trash2,
  Wifi,
  WifiOff,
} from 'lucide-react'
import { ALL_INTERVALS, QUICK_INTERVALS } from '../chart/periods'
import { useWorkspaceStore } from '../store/workspaceStore'
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
  const gridLayout = useWorkspaceStore((s) => s.gridLayout)
  const sync = useWorkspaceStore((s) => s.sync)
  const activeSlotId = useWorkspaceStore((s) => s.activeSlotId)
  const panes = useWorkspaceStore((s) => s.panes)
  const layouts = useWorkspaceStore((s) => s.layouts)
  const dataStatus = useWorkspaceStore((s) => s.dataStatus)
  const importingCafeF = useWorkspaceStore((s) => s.importingCafeF)
  const rightPanelOpen = useWorkspaceStore((s) => s.rightPanelOpen)
  const bottomDockOpen = useWorkspaceStore((s) => s.bottomDockOpen)
  const replay = useWorkspaceStore((s) => s.replay)

  const openSymbolSearch = useWorkspaceStore((s) => s.openSymbolSearch)
  const setActiveInterval = useWorkspaceStore((s) => s.setActiveInterval)
  const setActiveChartStyle = useWorkspaceStore((s) => s.setActiveChartStyle)
  const setActivePriceScale = useWorkspaceStore((s) => s.setActivePriceScale)
  const toggleActiveVnBands = useWorkspaceStore((s) => s.toggleActiveVnBands)
  const toggleActiveCountdown = useWorkspaceStore((s) => s.toggleActiveCountdown)
  const setIndicatorsModalOpen = useWorkspaceStore((s) => s.setIndicatorsModalOpen)
  const setGridLayout = useWorkspaceStore((s) => s.setGridLayout)
  const setSyncOption = useWorkspaceStore((s) => s.setSyncOption)
  const saveCurrentLayout = useWorkspaceStore((s) => s.saveCurrentLayout)
  const applySavedLayout = useWorkspaceStore((s) => s.applySavedLayout)
  const deleteSavedLayout = useWorkspaceStore((s) => s.deleteSavedLayout)
  const toggleTheme = useWorkspaceStore((s) => s.toggleTheme)
  const toggleOfflineMode = useWorkspaceStore((s) => s.toggleOfflineMode)
  const importCafeFDirectory = useWorkspaceStore((s) => s.importCafeFDirectory)
  const setRightPanelOpen = useWorkspaceStore((s) => s.setRightPanelOpen)
  const setBottomDockOpen = useWorkspaceStore((s) => s.setBottomDockOpen)
  const startReplay = useWorkspaceStore((s) => s.startReplay)
  const stopReplay = useWorkspaceStore((s) => s.stopReplay)
  const triggerExportPng = useWorkspaceStore((s) => s.triggerExportPng)

  const activePane = panes.find((p) => p.slotId === activeSlotId) ?? panes[0]
  const [openMenu, setOpenMenu] = useState<'interval' | 'style' | 'layout' | 'saved' | 'data' | null>(null)
  const [layoutNameInput, setLayoutNameInput] = useState('')
  const [cafefDirInput, setCafefDirInput] = useState('data/raw/cafef')
  const headerRef = useRef<HTMLElement | null>(null)

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

        {/* Causal Bar Replay Button */}
        <button
          type="button"
          className="tv-btn"
          data-active={Boolean(replay?.active)}
          onClick={() => {
            if (replay?.active) {
              void stopReplay()
            } else {
              void startReplay(null)
            }
          }}
          title="Phát lại nến nhân quả (Bar Replay — không rò rỉ dữ liệu tương lai)"
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

        {/* VN Ceiling/Ref/Floor Band Toggle & Countdown Toggle */}
        {isVn && (
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
              className="absolute right-0 top-[34px] w-68 rounded-md p-2.5 shadow-2xl z-50 space-y-2.5"
              style={{
                backgroundColor: 'var(--tv-bg-elevated)',
                border: '1px solid var(--tv-border)',
              }}
            >
              <div className="text-[11px] font-semibold uppercase tracking-wide" style={{ color: 'var(--tv-text-secondary)' }}>
                Khung đa biểu đồ
              </div>
              <div className="grid grid-cols-4 gap-1.5">
                {(
                  [
                    { key: '1', label: '1 ô', Icon: Square },
                    { key: '2v', label: '2 dọc', Icon: Columns2 },
                    { key: '2h', label: '2 ngang', Icon: Rows2 },
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
                    onClick={() => setGridLayout(key)}
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
              className="absolute right-0 top-[34px] w-80 rounded-md p-3 shadow-2xl z-50 space-y-2.5"
              style={{
                backgroundColor: 'var(--tv-bg-elevated)',
                border: '1px solid var(--tv-border)',
              }}
            >
              <div className="flex items-center justify-between">
                <span className="text-[11px] font-semibold uppercase tracking-wide" style={{ color: 'var(--tv-text-secondary)' }}>
                  Kho Dữ Liệu SQLite Cục Bộ
                </span>
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
              </div>

              <form
                className="space-y-1.5 border-t pt-2"
                style={{ borderColor: 'var(--tv-border)' }}
                onSubmit={(e) => {
                  e.preventDefault()
                  void importCafeFDirectory(cafefDirInput)
                }}
              >
                <label
                  htmlFor="sumi-cafef-dir-input"
                  className="block text-[11px] font-medium"
                  style={{ color: 'var(--tv-text-secondary)' }}
                >
                  Đường dẫn thư mục CSV CafeF (trên Mac / Windows)
                </label>
                <div className="flex gap-1 text-[10px]">
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
