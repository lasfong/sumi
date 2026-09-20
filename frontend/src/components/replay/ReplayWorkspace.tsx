import React, { useEffect, useState } from 'react';
import { CandleChart } from '../chart/CandleChart';
import { DrawingToolbar } from '../chart/DrawingToolbar';
import { evaluateDrawingContractRuntimeCorpus } from '../../features/drawings/drawingContractCorpus';
import { IndicatorManager } from '../chart/IndicatorManager';
import { IndicatorPaneChrome } from '../chart/IndicatorPaneChrome';
import { ChartLegendOverlay } from '../chart/ChartLegendOverlay';
import { ReplayControlDock } from './ReplayControlDock';
import { MultiChartLayout } from '../layout/MultiChartLayout';
import { DecisionJournal } from './DecisionJournal';
import { PendingOrdersPanel } from './PendingOrdersPanel';
import { PositionPanel } from './PositionPanel';
import { SessionSetup } from './SessionSetup';
import { TradeControls } from './TradeControls';
import { PracticeScoreboard } from './PracticeScoreboard';
import { PracticeRail } from './PracticeRail';
import { PracticeJournal } from './PracticeJournal';
import { KeyboardShortcutsModal } from './KeyboardShortcutsModal';
import { SessionDebriefModal } from './SessionDebriefModal';
import { SymbolSwitcherModal } from './SymbolSwitcherModal';
import { useReplayWorkspaceController } from './ReplayWorkspaceController';
import { ScannerSourceContext } from './ScannerSourceContext';
import { SessionPicker } from '../common/SessionPicker';
import { Search } from 'lucide-react';
import { formatVietnameseDate, formatVietnameseNumber, formatVietnameseVolume } from '../../utils/formatters';
import { SignalInspector } from '../signals/SignalInspector';
import { SignalCatalogModal } from '../signals/SignalCatalogModal';
import { SignalExplanationInspector } from '../signals/SignalExplanationInspector';
import { DrawingInspector } from '../chart/DrawingInspector';


export const ReplayWorkspace: React.FC = () => {
  const {
    sessionId,
    chartRef,
    symbolName,
    sessionStatus,
    sessionData,
    sourceContext,
    currentDate,
    currentCandle,
    candleCount,
    handleCreateSession,
    handleResumeSession,
    isCreatingSession,
    handleClearSession,
    onSwitchSymbol,
    indicatorDefinitions,
    indicatorDocument,
    indicatorRuntime,
    addIndicatorInstance,
    updateIndicatorInstance,
    removeIndicatorInstance,
    toggleIndicatorInstance,
    moveIndicatorInstance,
    playSpeed,
    setPlaySpeed,
    isPlaying,
    setIsPlaying,
    handlePrev,
    handleNext,
    navigationPending,
    drawing,
    selectedDrawing,
    handleDrawingTool,
    formattedCandles,
    volumeData,
    markers,
    practiceData,
    practiceLoading,
    practiceError,
    handleResetPractice,
    journalData,
    journalLoading,
    journalError,
    handleSaveJournal,
    handleSubmitDecision,
    targetTimeframe,
    setTargetTimeframe,
  } = useReplayWorkspaceController();

  const [isShortcutsOpen, setIsShortcutsOpen] = useState(false);
  const [isDebriefOpen, setIsDebriefOpen] = useState(false);
  const [isSymbolSwitcherOpen, setIsSymbolSwitcherOpen] = useState(false);
  const [isRightPanelOpen, setIsRightPanelOpen] = useState(true);
  const [isSignalCatalogOpen, setIsSignalCatalogOpen] = useState(false);
  const [showExtendedInspector, setShowExtendedInspector] = useState(false);


  useEffect(() => {
    const handleOpenShortcuts = () => setIsShortcutsOpen(true);
    const handleOpenDebrief = () => setIsDebriefOpen(true);
    const handleOpenSwitcher = () => setIsSymbolSwitcherOpen(true);
    window.addEventListener('sumi:open-shortcuts', handleOpenShortcuts);
    window.addEventListener('sumi:open-debrief', handleOpenDebrief);
    window.addEventListener('sumi:open-symbol-switcher', handleOpenSwitcher);
    return () => {
      window.removeEventListener('sumi:open-shortcuts', handleOpenShortcuts);
      window.removeEventListener('sumi:open-debrief', handleOpenDebrief);
      window.removeEventListener('sumi:open-symbol-switcher', handleOpenSwitcher);
    };
  }, []);

  if (!sessionId) {
    return (
      <SessionSetup
        onCreateSession={handleCreateSession}
        onResumeSession={handleResumeSession}
        isLoading={isCreatingSession}
      />
    );
  }

  const visibleSubpanes = indicatorDocument.instances.filter(
    (instance) => instance.visible && instance.placement !== 'price'
  ).length;
  const minimumChartHeight = 32 + 60 * (4 + visibleSubpanes);

  return (
    <div
      className="replay-workspace"
      data-testid="replay-workspace"
      style={{
        display: 'flex',
        flexDirection: 'column',
        width: '100%',
        height: '100%',
        minHeight: 0,
        overflow: 'hidden',
        background: 'var(--bg-dark)',
      }}
    >
      <div className="limited-workstation-warning" role="status">
        Limited layout below 1180px: trade submission is unavailable. Use a desktop-width workspace
        for full practice.
      </div>
      <header
        className="replay-header"
        style={{
          height: '42px',
          minHeight: '42px',
          padding: '0 12px',
          background: 'var(--bg-header)',
          backdropFilter: 'var(--backdrop-blur)',
          display: 'flex',
          flexWrap: 'nowrap',
          gap: '12px',
          justifyContent: 'space-between',
          alignItems: 'center',
          borderBottom: '1px solid var(--border-color)',
          boxShadow: '0 2px 10px rgba(0,0,0,0.4)',
          zIndex: 10,
          overflowX: 'auto',
          overflowY: 'hidden',
          whiteSpace: 'nowrap',
        }}
      >
        <div
          style={{
            display: 'flex',
            flexWrap: 'nowrap',
            alignItems: 'center',
            gap: '8px',
            flex: '1 1 auto',
            minWidth: 0,
            overflow: 'hidden',
            whiteSpace: 'nowrap',
          }}
        >
          <h2 style={{ margin: 0, fontSize: '15px', fontWeight: 600, whiteSpace: 'nowrap' }}>Sumi Replay</h2>
          <button
            type="button"
            data-testid="symbol-switcher-button"
            title="Đổi mã cổ phiếu (Phím tắt: /)"
            onClick={() => setIsSymbolSwitcherOpen(true)}
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: 5,
              padding: '2px 8px',
              background: 'rgba(41, 98, 255, 0.15)',
              color: '#58A6FF',
              borderRadius: '4px',
              fontWeight: 700,
              fontSize: '13px',
              border: '1px solid rgba(41, 98, 255, 0.4)',
              cursor: 'pointer',
              transition: 'all 0.15s ease',
            }}
          >
            <Search size={12} />
            <span>{symbolName}</span>
            <span style={{ fontSize: '10px', opacity: 0.7 }}>▾</span>
          </button>
          {sessionData && (
            <div style={{ display: 'flex', gap: '5px', fontSize: '12px', color: 'var(--text-muted)', alignItems: 'center' }}>
              <select 
                value={targetTimeframe ?? sessionData.timeframe} 
                onChange={(e) => setTargetTimeframe(e.target.value === sessionData.timeframe ? undefined : e.target.value)}
                style={{ 
                  background: 'rgba(255,255,255,0.05)', 
                  padding: '2px 6px', 
                  borderRadius: '4px', 
                  border: '1px solid rgba(255,255,255,0.1)', 
                  color: 'var(--text-main)', 
                  outline: 'none', 
                  cursor: 'pointer',
                  fontSize: '12px',
                }}
              >
                {['5m', '15m', '1H', '1D', '1W', '1M'].map(tf => (
                  <option key={tf} value={tf} style={{ background: '#1e222d' }}>
                    {tf}
                  </option>
                ))}
              </select>
              <span style={{ background: 'rgba(255,255,255,0.05)', padding: '2px 5px', borderRadius: '4px', textTransform: 'capitalize', fontSize: '11px' }}>
                {sessionData.adjustment_type}
              </span>
              <span style={{ background: 'rgba(255,255,255,0.05)', padding: '2px 5px', borderRadius: '4px', textTransform: 'capitalize', fontSize: '11px' }}>
                {sourceContext?.replay_intent?.replace('_', ' ') || sessionData.mode.replace('_', ' ')}
              </span>
            </div>
          )}
          <SessionPicker
            selectedSessionId={sessionId}
            onSelectSession={handleResumeSession}
            compact
          />
          <button
            type="button"
            data-testid="open-add-indicator"
            aria-label="Add indicator"
            onClick={() => window.dispatchEvent(new CustomEvent('sumi:indicator-add-request'))}
            style={{
              padding: '3px 8px',
              background: 'rgba(41, 98, 255, 0.1)',
              color: '#58a6ff',
              borderRadius: '4px',
              fontSize: '11px',
              fontWeight: 500,
              border: '1px solid rgba(41, 98, 255, 0.3)',
              display: 'inline-flex',
              alignItems: 'center',
              gap: '4px',
              cursor: 'pointer',
            }}
          >
            <span style={{ fontWeight: 700, fontSize: '12px', fontStyle: 'italic', fontFamily: 'serif' }}>fx</span>
            <span>Indicators</span>
          </button>
          <button
            type="button"
            data-testid="open-session-debrief"
            aria-label="Session debrief"
            title="Tổng kết phiên (D)"
            onClick={() => setIsDebriefOpen(true)}
            style={{
              padding: '3px 7px',
              background: 'rgba(255,255,255,0.05)',
              color: 'var(--text-main)',
              borderRadius: '4px',
              fontSize: '11px',
              border: '1px solid var(--border-color)',
              cursor: 'pointer',
              display: 'inline-flex',
              alignItems: 'center',
              gap: '4px',
            }}
          >
            <span>📊</span>
            <span>Debrief</span>
          </button>
          <button
            type="button"
            data-testid="open-keyboard-shortcuts"
            aria-label="Keyboard shortcuts"
            title="Phím tắt hệ thống (?)"
            onClick={() => setIsShortcutsOpen(true)}
            style={{
              padding: '3px 6px',
              background: 'rgba(255,255,255,0.05)',
              color: 'var(--text-muted)',
              borderRadius: '4px',
              fontSize: '11px',
              border: '1px solid var(--border-color)',
              cursor: 'pointer',
              display: 'inline-flex',
              alignItems: 'center',
            }}
          >
            ⌨️
          </button>
          {sessionStatus && (
            <span
              style={{
                padding: '2px 6px',
                background: 'rgba(255, 255, 255, 0.05)',
                color: 'var(--text-muted)',
                borderRadius: '4px',
                fontSize: '11px',
                textTransform: 'uppercase',
                border: '1px solid var(--border-color)',
              }}
            >
              {sessionStatus}
            </span>
          )}
          {sourceContext && <ScannerSourceContext context={sourceContext} />}

          {currentCandle && (
            <div
              style={{
                display: 'flex',
                flexWrap: 'nowrap',
                gap: '8px',
                fontSize: '11px',
                fontFamily: 'monospace',
                alignItems: 'center',
              }}
            >
              <span data-testid="replay-bar-context" style={{ color: 'var(--text-muted)' }}>
                Bar:{' '}
                <span style={{ color: 'white' }}>
                  #{practiceData?.visible_bar ?? candleCount}/{practiceData?.total_bars ?? '…'}
                </span>
              </span>
              <span data-testid="current-candle-date" style={{ color: 'var(--text-muted)' }}>
                Date: <span style={{ color: 'white' }}>{formatVietnameseDate(currentCandle.timestamp)}</span> <span style={{ fontSize: '10px' }}>(Asia/Ho_Chi_Minh)</span>
              </span>
              <span style={{ color: 'var(--text-muted)' }}>
                O: <span style={{ color: 'white' }}>{formatVietnameseNumber(currentCandle.open, 2)}</span>
              </span>
              <span style={{ color: 'var(--text-muted)' }}>
                H: <span style={{ color: 'white' }}>{formatVietnameseNumber(currentCandle.high, 2)}</span>
              </span>
              <span style={{ color: 'var(--text-muted)' }}>
                L: <span style={{ color: 'white' }}>{formatVietnameseNumber(currentCandle.low, 2)}</span>
              </span>
              <span style={{ color: 'var(--text-muted)' }}>
                C:{' '}
                <span
                  style={{
                    color:
                      currentCandle.close >= currentCandle.open
                        ? 'var(--color-buy)'
                        : 'var(--color-sell)',
                    fontWeight: 600,
                  }}
                >
                  {formatVietnameseNumber(currentCandle.close, 2)}
                </span>
              </span>
              <span style={{ color: 'var(--text-muted)' }}>
                V: <span style={{ color: 'white' }}>{formatVietnameseVolume(currentCandle.volume)}</span>
              </span>
              <span
                data-testid="replay-readiness-badge"
                style={{
                  padding: '2px 5px',
                  borderRadius: '4px',
                  fontSize: '10px',
                  background: 'rgba(0, 230, 118, 0.1)',
                  color: 'var(--color-buy)',
                  border: '1px solid rgba(0, 230, 118, 0.2)',
                  fontFamily: 'sans-serif',
                }}
              >
                Local Market Ready
              </span>
            </div>
          )}
        </div>

        <div style={{ display: 'flex', flexWrap: 'nowrap', gap: '8px', alignItems: 'center', flexShrink: 0, whiteSpace: 'nowrap', marginLeft: 'auto', background: 'var(--bg-header)', paddingLeft: '8px', zIndex: 2 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '5px' }}>
            <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>Speed:</span>
            <select
              aria-label="Playback speed"
              value={playSpeed}
              onChange={(e) => setPlaySpeed(Number(e.target.value))}
              style={{
                background: 'var(--bg-panel)',
                color: 'white',
                border: '1px solid var(--border-color)',
                borderRadius: '4px',
                padding: '3px 6px',
                fontSize: '11px',
                cursor: 'pointer',
              }}
            >
              <option value={1000}>1x</option>
              <option value={500}>2x</option>
              <option value={200}>5x</option>
              <option value={100}>10x</option>
            </select>
          </div>

          <div style={{ width: '1px', height: '16px', background: 'var(--border-color)' }}></div>

          <button
            type="button"
            data-testid="toggle-panel-button"
            onClick={() => setIsRightPanelOpen(!isRightPanelOpen)}
            style={{
              background: isRightPanelOpen ? 'rgba(41, 98, 255, 0.2)' : 'var(--bg-panel)',
              color: isRightPanelOpen ? '#58A6FF' : 'var(--text-muted)',
              border: `1px solid ${isRightPanelOpen ? 'rgba(41, 98, 255, 0.5)' : 'var(--border-color)'}`,
              borderRadius: '4px',
              fontSize: '11px',
              padding: '3px 8px',
              cursor: 'pointer',
              whiteSpace: 'nowrap',
              display: 'flex',
              alignItems: 'center',
              gap: '4px',
            }}
            title="Bật/tắt bảng điều khiển lệnh & phân tích (Right Panel)"
          >
            ◫ Panel
          </button>

          <div style={{ width: '1px', height: '16px', background: 'var(--border-color)' }}></div>

          <button
            onClick={handleClearSession}
            style={{
              background: 'var(--bg-panel)',
              color: 'var(--text-muted)',
              border: '1px solid var(--border-color)',
              borderRadius: '4px',
              fontSize: '11px',
              padding: '3px 8px',
              cursor: 'pointer',
              whiteSpace: 'nowrap',
            }}
          >
            New Session
          </button>
        </div>
      </header>

      <IndicatorManager
        definitions={indicatorDefinitions}
        document={indicatorDocument}
        runtime={indicatorRuntime}
        onAdd={addIndicatorInstance}
        onUpdate={updateIndicatorInstance}
        onRemove={removeIndicatorInstance}
        onToggle={toggleIndicatorInstance}
        onMove={moveIndicatorInstance}
      />

      <div
        className="replay-workspace-body"
        style={{ display: 'flex', flex: 1, overflow: 'hidden' }}
      >
        <DrawingToolbar
          activeTool={drawing.tool}
          onSelectTool={handleDrawingTool}
          onClearAll={drawing.clearAll}
          onUndo={drawing.undo}
          onRedo={drawing.redo}
          canUndo={drawing.canUndo}
          canRedo={drawing.canRedo}
          pendingText={!!drawing.pendingTextAnchor}
          onCommitText={drawing.commitText}
          onCancelText={drawing.cancelText}
          magnetMode={drawing.magnetMode}
          onMagnetMode={drawing.setMagnetMode}
          persistenceStatus={drawing.persistenceStatus}
        />

        <main
          className="replay-chart-region"
          style={{ flex: 3, minWidth: 0, padding: '0.5rem', display: 'flex', flexDirection: 'column' }}
        >
          <div
            className="panel"
            data-testid="replay-chart-scroll-region"
            data-minimum-subpane-height="60"
            style={{ flex: 1, padding: 0, overflowY: 'auto', overflowX: 'hidden' }}
          >
            <div
              style={{ position: 'relative', height: '100%', minHeight: minimumChartHeight }}
            >
              <MultiChartLayout layoutType="1x1">
                <CandleChart
                  ref={chartRef}
                  data={formattedCandles}
                  volumeData={volumeData}
                  markers={markers}
                  drawingDocument={drawing.document}
                  drawingTool={drawing.tool}
                  drawingSelection={drawing.selection}
                  currentDrawingTime={currentDate || '1970-01-01'}
                  onDrawingProviderEvent={drawing.providerEvent}
                  drawingMagnetMode={drawing.magnetMode}
                  minimumHeight={minimumChartHeight}
                />
              </MultiChartLayout>
              <IndicatorPaneChrome
                document={indicatorDocument}
                runtime={indicatorRuntime}
                onSettings={(id) =>
                  window.dispatchEvent(
                    new CustomEvent('sumi:indicator-settings-request', { detail: id })
                  )
                }
                onToggle={toggleIndicatorInstance}
                onRemove={removeIndicatorInstance}
              />
              <div style={{ position: 'relative', zIndex: 20 }}>
                <ChartLegendOverlay
                  symbol={symbolName}
                  timeframe={targetTimeframe ?? sessionData?.timeframe ?? '1D'}
                  currentCandle={currentCandle}
                  indicators={indicatorDocument.instances}
                  runtime={indicatorRuntime}
                  onToggle={toggleIndicatorInstance}
                  onSettings={(instance) =>
                    window.dispatchEvent(
                      new CustomEvent('sumi:indicator-settings-request', { detail: instance.id })
                    )
                  }
                  onRemove={removeIndicatorInstance}
                />
              </div>
              <div
                data-testid="indicator-order-controls"
                style={{
                  position: 'fixed',
                  top: 120,
                  right: 340,
                  zIndex: 99,
                  display: 'flex',
                  gap: 4,
                  pointerEvents: 'none',
                }}
              >
                {indicatorDocument.instances.map((instance, index) => (
                  <div key={instance.id} style={{ display: 'flex', gap: 2, pointerEvents: 'none' }}>
                    <button
                      type="button"
                      aria-label={`Move ${instance.label} up`}
                      disabled={index === 0}
                      onClick={() => moveIndicatorInstance(instance.id, -1)}
                      style={{ padding: '1px 3px', fontSize: 8, opacity: 0.15, pointerEvents: 'auto' }}
                    >
                      ↑
                    </button>
                    <button
                      type="button"
                      aria-label={`Move ${instance.label} down`}
                      disabled={index === indicatorDocument.instances.length - 1}
                      onClick={() => moveIndicatorInstance(instance.id, 1)}
                      style={{ padding: '1px 3px', fontSize: 8, opacity: 0.15, pointerEvents: 'auto' }}
                    >
                      ↓
                    </button>
                  </div>
                ))}
              </div>
              <ReplayControlDock
                isPlaying={isPlaying}
                playSpeed={playSpeed}
                onTogglePlay={() => setIsPlaying(!isPlaying)}
                onChangeSpeed={setPlaySpeed}
                onStepPrev={handlePrev}
                onStepNext={handleNext}
                navigationPending={navigationPending}
                currentBar={practiceData?.visible_bar ?? candleCount}
                totalBars={practiceData?.total_bars}
                onNewSession={handleClearSession}
              />
            </div>
            <output
              data-testid="drawing-domain-state"
              aria-label="Serialized drawing state"
              style={{ display: 'none' }}
            >
              {JSON.stringify(drawing.document)}
            </output>
            <output
              data-testid="drawing-contract-corpus-state"
              aria-label="Drawing contract corpus results"
              style={{ display: 'none' }}
            >
              {JSON.stringify(evaluateDrawingContractRuntimeCorpus())}
            </output>
            <output
              data-testid="indicator-domain-state"
              aria-label="Serialized indicator state"
              style={{ display: 'none' }}
            >
              {JSON.stringify(indicatorDocument)}
            </output>
            <output
              data-testid="indicator-runtime-state"
              aria-label="Serialized indicator runtime state"
              style={{ display: 'none' }}
            >
              {JSON.stringify(indicatorRuntime)}
            </output>
            <output
              data-testid="practice-workflow-state"
              aria-label="Projected practice workflow state"
              style={{ display: 'none' }}
            >
              {JSON.stringify(practiceData)}
            </output>
            <output
              data-testid="trade-marker-state"
              aria-label="Serialized trade marker state"
              style={{ display: 'none' }}
            >
              {JSON.stringify(markers)}
            </output>
          </div>
        </main>

        {isRightPanelOpen && (
          <PracticeRail
            key={selectedDrawing?.id ?? 'practice'}
            selectedDrawingId={selectedDrawing?.id}
            drawing={
              <DrawingInspector
                key={`${selectedDrawing?.id ?? 'none'}:${drawing.document.revision}`}
                selected={selectedDrawing}
                persistenceStatus={drawing.persistenceStatus}
                onApply={drawing.updateDrawing}
                onDelete={() => drawing.remove()}
              />
            }
            trade={
              <div style={{ display: 'grid', gap: 8 }}>
                {sourceContext && <ScannerSourceContext context={sourceContext} display="details" />}
                <div style={{ display: 'flex', gap: 6 }}>
                  <button
                    type="button"
                    data-testid="open-signal-catalog-modal-btn"
                    onClick={() => setIsSignalCatalogOpen(true)}
                    style={{
                      flex: 1,
                      padding: '6px 10px',
                      fontSize: '11px',
                      borderRadius: '4px',
                      background: 'rgba(41, 98, 255, 0.15)',
                      border: '1px solid var(--color-primary)',
                      color: 'var(--color-primary)',
                      cursor: 'pointer',
                      fontWeight: 600,
                    }}
                  >
                    📚 Danh Mục Tín Hiệu (72)
                  </button>
                  <button
                    type="button"
                    data-testid="toggle-extended-inspector-btn"
                    onClick={() => setShowExtendedInspector((prev) => !prev)}
                    style={{
                      padding: '6px 10px',
                      fontSize: '11px',
                      borderRadius: '4px',
                      background: showExtendedInspector ? 'rgba(0, 230, 118, 0.2)' : 'rgba(255, 255, 255, 0.06)',
                      border: `1px solid ${showExtendedInspector ? 'var(--color-buy)' : 'var(--border-color)'}`,
                      color: showExtendedInspector ? 'var(--color-buy)' : 'var(--text-muted)',
                      cursor: 'pointer',
                      fontWeight: 600,
                    }}
                    title="Bật/tắt bộ kiểm tra & giải thích 72 tín hiệu, Sức khỏe kỹ thuật và Dòng tiền BB"
                  >
                    {showExtendedInspector ? '🔬 Thu Gọn' : '🔬 Mở Rộng'}
                  </button>
                </div>
                {showExtendedInspector && (
                  <SignalExplanationInspector
                    sessionId={sessionId}
                    currentIndex={sessionData?.current_index ?? candleCount - 1}
                    timeframe={sessionData?.timeframe || '1D'}
                    currentTimestamp={currentCandle?.timestamp}
                    onOpenCatalog={() => setIsSignalCatalogOpen(true)}
                  />
                )}
                <SignalInspector
                  sessionId={sessionId}
                  currentIndex={sessionData?.current_index ?? candleCount - 1}
                  timeframe={sessionData?.timeframe || '1D'}
                  currentTimestamp={currentCandle?.timestamp}
                />
                {practiceData ? (
                  <>
                    <PracticeScoreboard
                      snapshot={practiceData}
                      onResetPractice={handleResetPractice}
                    />
                    <TradeControls
                      snapshot={practiceData}
                      onSubmitDecision={handleSubmitDecision}
                      disabled={practiceLoading || practiceError}
                    />
                    <PendingOrdersPanel orders={practiceData.orders} />
                    <PositionPanel positions={practiceData.positions} snapshot={practiceData} />
                  </>
                ) : (
                  <div role={practiceError ? 'alert' : 'status'}>
                    {practiceError
                      ? 'Practice state could not be loaded.'
                      : 'Loading practice state…'}
                  </div>
                )}
              </div>
            }
            journal={
              practiceData ? (
                <PracticeJournal
                  snapshot={practiceData}
                  entries={journalData || []}
                  loading={journalLoading}
                  loadError={journalError}
                  onSave={handleSaveJournal}
                />
              ) : (
                <div>Loading practice context…</div>
              )
            }
            decisions={<DecisionJournal decisions={practiceData?.decisions || []} />}
          />
        )}
      </div>

      <SessionDebriefModal
        isOpen={isDebriefOpen}
        onClose={() => setIsDebriefOpen(false)}
        sessionId={sessionId}
        symbolName={symbolName}
        practiceData={practiceData}
      />
      <KeyboardShortcutsModal
        isOpen={isShortcutsOpen}
        onClose={() => setIsShortcutsOpen(false)}
      />
      <SymbolSwitcherModal
        isOpen={isSymbolSwitcherOpen}
        onClose={() => setIsSymbolSwitcherOpen(false)}
        onSelectSymbol={onSwitchSymbol}
        currentSymbol={symbolName}
      />
      <SignalCatalogModal
        isOpen={isSignalCatalogOpen}
        onClose={() => setIsSignalCatalogOpen(false)}
      />
    </div>
  );
};
