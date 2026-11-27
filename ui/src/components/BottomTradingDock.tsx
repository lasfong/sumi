import React from 'react'
import { Briefcase, ChevronDown, ChevronUp, Compass, FlaskConical, Maximize2, Minimize2, Radar } from 'lucide-react'
import { useWorkspaceStore } from '../store/workspaceStore'
import type { BottomDockTab } from '../types/market'
import { MarketBreadthRrgTab } from './dock/MarketBreadthRrgTab'
import { PaperBrokerTab } from './dock/PaperBrokerTab'
import { StockScreenerTab } from './dock/StockScreenerTab'
import { StrategyTesterTab } from './dock/StrategyTesterTab'

const DOCK_TABS: Array<{ id: BottomDockTab; label: string; Icon: React.ComponentType<{ className?: string }> }> = [
  { id: 'paper', label: 'Giao dịch Giả lập (VN T+2)', Icon: Briefcase },
  { id: 'strategy', label: 'Kiểm định Chiến lược QMV (WFO / MC)', Icon: FlaskConical },
  { id: 'screener', label: 'Bộ lọc Kỹ thuật QMV (Screener)', Icon: Radar },
  { id: 'breadth', label: 'Thị trường & RRG (Heatmap)', Icon: Compass },
]

export const BottomTradingDock: React.FC = () => {
  const bottomDockOpen = useWorkspaceStore((s) => s.bottomDockOpen)
  const bottomDockMaximized = useWorkspaceStore((s) => s.bottomDockMaximized)
  const bottomDockHeight = useWorkspaceStore((s) => s.bottomDockHeight)
  const bottomDockTab = useWorkspaceStore((s) => s.bottomDockTab)

  const setBottomDockOpen = useWorkspaceStore((s) => s.setBottomDockOpen)
  const setBottomDockMaximized = useWorkspaceStore((s) => s.setBottomDockMaximized)
  const setBottomDockTab = useWorkspaceStore((s) => s.setBottomDockTab)

  const heightStyle = !bottomDockOpen
    ? { height: 32 }
    : bottomDockMaximized
      ? { height: '62vh' }
      : { height: bottomDockHeight }

  return (
    <section
      aria-label="Thanh giao dịch, Backtest và Bộ lọc cổ phiếu"
      className="flex flex-col shrink-0 overflow-hidden"
      style={{
        ...heightStyle,
        backgroundColor: 'var(--tv-bg-panel)',
        borderTop: '1px solid var(--tv-border)',
      }}
    >
      {/* Dock Tab Bar */}
      <div
        className="flex items-center justify-between px-2 shrink-0"
        style={{ height: 32, borderBottom: bottomDockOpen ? '1px solid var(--tv-border)' : 'none' }}
      >
        <div role="tablist" aria-label="Các công cụ giao dịch" className="flex items-center h-full gap-1 overflow-x-auto">
          {DOCK_TABS.map(({ id, label, Icon }) => (
            <button
              key={id}
              type="button"
              role="tab"
              id={`dock-tab-${id}`}
              aria-selected={bottomDockOpen && bottomDockTab === id}
              className="tv-tab text-[12px]"
              onClick={() => setBottomDockTab(id)}
            >
              <Icon className="w-3.5 h-3.5" />
              <span>{label}</span>
            </button>
          ))}
        </div>

        <div className="flex items-center gap-0.5 shrink-0">
          {bottomDockOpen && (
            <button
              type="button"
              className="tv-icon-btn w-6 h-6"
              onClick={() => setBottomDockMaximized(!bottomDockMaximized)}
              title={bottomDockMaximized ? 'Thu về kích thước thường' : 'Phóng to bảng dưới'}
            >
              {bottomDockMaximized ? <Minimize2 className="w-3.5 h-3.5" /> : <Maximize2 className="w-3.5 h-3.5" />}
            </button>
          )}
          <button
            type="button"
            className="tv-icon-btn w-6 h-6"
            onClick={() => setBottomDockOpen(!bottomDockOpen)}
            title={bottomDockOpen ? 'Thu gọn bảng dưới' : 'Mở rộng bảng dưới'}
          >
            {bottomDockOpen ? <ChevronDown className="w-4 h-4" /> : <ChevronUp className="w-4 h-4" />}
          </button>
        </div>
      </div>

      {/* Active Tab Panel */}
      {bottomDockOpen && (
        <div role="tabpanel" aria-labelledby={`dock-tab-${bottomDockTab}`} className="flex-1 min-h-0 overflow-hidden">
          {bottomDockTab === 'paper' && <PaperBrokerTab />}
          {bottomDockTab === 'strategy' && <StrategyTesterTab />}
          {bottomDockTab === 'screener' && <StockScreenerTab />}
          {bottomDockTab === 'breadth' && <MarketBreadthRrgTab />}
        </div>
      )}
    </section>
  )
}
