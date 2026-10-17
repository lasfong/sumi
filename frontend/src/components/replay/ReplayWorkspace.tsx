import React, { useEffect, useState } from 'react';
import { CandleChart } from '../chart/CandleChart';
import { DrawingToolbar } from '../chart/DrawingToolbar';
import { evaluateDrawingContractRuntimeCorpus } from '../../features/drawings/drawingContractCorpus';
import { IndicatorManager } from '../chart/IndicatorManager';
import { IndicatorPaneChrome } from '../chart/IndicatorPaneChrome';
import { ChartLegendOverlay } from '../chart/ChartLegendOverlay';
import { ReplayControlDock } from './ReplayControlDock';
import { ReplayHeader } from './ReplayHeader';
import { ReplayRightPanel } from './ReplayRightPanel';
import { MultiChartLayout } from '../layout/MultiChartLayout';
import { SessionSetup } from './SessionSetup';
import { KeyboardShortcutsModal } from './KeyboardShortcutsModal';
import { SessionDebriefModal } from './SessionDebriefModal';
import { SymbolSwitcherModal } from './SymbolSwitcherModal';
import { useReplayWorkspaceController } from './ReplayWorkspaceController';
import { SignalCatalogModal } from '../signals/SignalCatalogModal';

const IS_DEV_OR_TEST = Boolean(import.meta.env.DEV || import.meta.env.MODE === 'test');

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
    invalidatedSession,
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
    wsStatus,
    wsReconnectAttempt,
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
    handleCancelOrder,
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
  const [selectedInspectorSignal, setSelectedInspectorSignal] = useState<string>('health.score');

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
        invalidatedSession={invalidatedSession}
      />
    );
  }

  const visibleSubpanes = indicatorDocument.instances.filter(
    (instance) => instance.visible && instance.placement !== 'price',
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
        Limited layout below 1280px: trade submission is unavailable. Use a desktop-width workspace (≥1280px)
        for full practice.
      </div>

      <ReplayHeader
        sessionId={sessionId}
        symbolName={symbolName}
        sessionStatus={sessionStatus}
        sessionData={sessionData}
        sourceContext={sourceContext}
        currentCandle={currentCandle}
        candleCount={candleCount}
        practiceData={practiceData}
        targetTimeframe={targetTimeframe}
        onChangeTimeframe={setTargetTimeframe}
        onResumeSession={handleResumeSession}
        onOpenSymbolSwitcher={() => setIsSymbolSwitcherOpen(true)}
        onOpenDebrief={() => setIsDebriefOpen(true)}
        onOpenShortcuts={() => setIsShortcutsOpen(true)}
        playSpeed={playSpeed}
        onChangeSpeed={setPlaySpeed}
        isRightPanelOpen={isRightPanelOpen}
        onToggleRightPanel={() => setIsRightPanelOpen(!isRightPanelOpen)}
        onClearSession={handleClearSession}
        wsStatus={wsStatus}
        wsReconnectAttempt={wsReconnectAttempt}
      />

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
                    new CustomEvent('sumi:indicator-settings-request', { detail: id }),
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
                      new CustomEvent('sumi:indicator-settings-request', { detail: instance.id }),
                    )
                  }
                  onRemove={removeIndicatorInstance}
                />
              </div>
              <div
                data-testid="indicator-order-controls"
                style={{
                  position: 'absolute',
                  top: 10,
                  right: 76,
                  zIndex: 21,
                  display: 'flex',
                  flexWrap: 'wrap',
                  gap: 4,
                  pointerEvents: 'none',
                }}
              >
                {indicatorDocument.instances.map((instance, index) => (
                  <div
                    key={instance.id}
                    style={{
                      display: 'inline-flex',
                      alignItems: 'center',
                      gap: 2,
                      padding: '2px 5px',
                      borderRadius: 4,
                      background: 'rgba(19, 23, 34, 0.85)',
                      border: '1px solid rgba(255, 255, 255, 0.14)',
                      pointerEvents: 'auto',
                    }}
                  >
                    <span style={{ fontSize: 10, color: '#9DA7B3', marginRight: 2 }}>
                      {instance.label}
                    </span>
                    <button
                      type="button"
                      aria-label={`Move ${instance.label} up`}
                      title={`Di chuyển ${instance.label} lên trên`}
                      disabled={index === 0}
                      onClick={() => moveIndicatorInstance(instance.id, -1)}
                      style={{
                        padding: '1px 5px',
                        fontSize: 10,
                        opacity: index === 0 ? 0.35 : 0.9,
                        background: 'rgba(255, 255, 255, 0.08)',
                        color: '#F0F6FC',
                        borderRadius: 3,
                        pointerEvents: 'auto',
                      }}
                    >
                      ↑
                    </button>
                    <button
                      type="button"
                      aria-label={`Move ${instance.label} down`}
                      title={`Di chuyển ${instance.label} xuống dưới`}
                      disabled={index === indicatorDocument.instances.length - 1}
                      onClick={() => moveIndicatorInstance(instance.id, 1)}
                      style={{
                        padding: '1px 5px',
                        fontSize: 10,
                        opacity: index === indicatorDocument.instances.length - 1 ? 0.35 : 0.9,
                        background: 'rgba(255, 255, 255, 0.08)',
                        color: '#F0F6FC',
                        borderRadius: 3,
                        pointerEvents: 'auto',
                      }}
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
            {IS_DEV_OR_TEST && (
              <>
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
              </>
            )}
          </div>
        </main>

        {isRightPanelOpen && (
          <ReplayRightPanel
            sessionId={sessionId}
            sessionData={sessionData}
            sourceContext={sourceContext}
            currentCandle={currentCandle}
            candleCount={candleCount}
            selectedDrawing={selectedDrawing}
            drawingRevision={drawing.document.revision}
            drawingPersistenceStatus={drawing.persistenceStatus}
            onUpdateDrawing={drawing.updateDrawing}
            onDeleteDrawing={() => drawing.remove()}
            showExtendedInspector={showExtendedInspector}
            onToggleExtendedInspector={() => setShowExtendedInspector((prev) => !prev)}
            selectedInspectorSignal={selectedInspectorSignal}
            onOpenSignalCatalog={() => setIsSignalCatalogOpen(true)}
            practiceData={practiceData}
            practiceLoading={practiceLoading}
            practiceError={practiceError}
            onResetPractice={handleResetPractice}
            onSubmitDecision={handleSubmitDecision}
            onCancelOrder={handleCancelOrder}
            journalData={journalData}
            journalLoading={journalLoading}
            journalError={journalError}
            onSaveJournal={handleSaveJournal}
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
        selectedSignalName={selectedInspectorSignal}
        onSelectSignal={(sig) => {
          setSelectedInspectorSignal(sig.name);
          setShowExtendedInspector(true);
          setIsSignalCatalogOpen(false);
        }}
      />
    </div>
  );
};
