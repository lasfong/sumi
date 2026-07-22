import React from 'react';
import { CandleChart } from '../chart/CandleChart';
import { DrawingToolbar } from '../chart/DrawingToolbar';
import { DrawingInspector } from '../chart/DrawingInspector';
import { evaluateDrawingContractRuntimeCorpus } from '../../features/drawings/drawingContractCorpus';
import { IndicatorManager } from '../chart/IndicatorManager';
import { IndicatorPaneChrome } from '../chart/IndicatorPaneChrome';
import { MultiChartLayout } from '../layout/MultiChartLayout';
import { DecisionJournal } from './DecisionJournal';
import { PendingOrdersPanel } from './PendingOrdersPanel';
import { PositionPanel } from './PositionPanel';
import { SessionSetup } from './SessionSetup';
import { TradeControls } from './TradeControls';
import { PracticeJournal } from './PracticeJournal';
import { PracticeRail } from './PracticeRail';
import { useReplayWorkspaceController } from './ReplayWorkspaceController';

export const ReplayWorkspace: React.FC = () => {
  const {
    sessionId, chartRef, symbolName, sessionStatus, signalSource, signalDate, currentDate, currentCandle, candleCount,
    handleCreateSession, handleResumeSession, isCreatingSession, handleClearSession,
    indicatorDefinitions, indicatorDocument, indicatorRuntime, addIndicatorInstance, updateIndicatorInstance,
    removeIndicatorInstance, toggleIndicatorInstance, moveIndicatorInstance,
    playSpeed, setPlaySpeed, isPlaying, setIsPlaying, handlePrev, handleNext, navigationPending,
    drawing, selectedDrawing, handleDrawingTool, formattedCandles, volumeData, markers,
    practiceData, practiceLoading, practiceError,
    journalData, journalLoading, journalError, handleSaveJournal, handleSubmitDecision,
  } = useReplayWorkspaceController();
  if (!sessionId) {
    return <SessionSetup onCreateSession={handleCreateSession} onResumeSession={handleResumeSession} isLoading={isCreatingSession} />;
  }
  const visibleSubpanes = indicatorDocument.instances.filter(instance => instance.visible && instance.placement !== 'price').length;
  const minimumChartHeight = 32 + (60 * (4 + visibleSubpanes));

  return (
    <div className="replay-workspace" style={{ display: 'flex', flexDirection: 'column', width: '100%', height: '100%', minHeight: 0, overflow: 'hidden', background: 'var(--bg-dark)' }}>
      <div className="limited-workstation-warning" role="status">Limited layout below 1180px: trade submission is unavailable. Use a desktop-width workspace for full practice.</div>
      <header style={{
        padding: '10px 12px', background: 'var(--bg-header)', backdropFilter: 'var(--backdrop-blur)',
        display: 'flex', flexWrap: 'wrap', gap: '8px 16px', justifyContent: 'space-between', alignItems: 'center',
        borderBottom: '1px solid var(--border-color)',
        boxShadow: '0 4px 20px rgba(0,0,0,0.5)', zIndex: 10
      }}>
        <div style={{ display: 'flex', flexWrap: 'wrap', alignItems: 'center', gap: '8px 12px', minWidth: 0 }}>
          <h2 style={{ margin: 0, fontSize: '18px', fontWeight: 600 }}>Sumi Replay</h2>
          <span style={{ padding: '4px 8px', background: 'rgba(41, 98, 255, 0.1)', color: 'var(--color-primary)', borderRadius: '4px', fontWeight: 600, fontSize: '14px', border: '1px solid rgba(41, 98, 255, 0.3)' }}>
            {symbolName}
          </span>
          <span style={{ color: 'var(--text-muted)', fontSize: '13px' }}>Session #{sessionId}</span>
          {sessionStatus && (
            <span style={{ padding: '4px 8px', background: 'rgba(255, 255, 255, 0.05)', color: 'var(--text-muted)', borderRadius: '4px', fontSize: '12px', textTransform: 'uppercase', border: '1px solid var(--border-color)' }}>
              {sessionStatus}
            </span>
          )}
          {signalSource && (
            <span style={{ padding: '4px 8px', background: 'rgba(255, 209, 102, 0.12)', color: '#FFD166', borderRadius: '4px', fontWeight: 600, fontSize: '12px', border: '1px solid rgba(255, 209, 102, 0.35)' }}>
              Scanner Signal
            </span>
          )}

          {currentCandle && (
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px', fontSize: '12px', fontFamily: 'monospace' }}>
              <span data-testid="replay-bar-context" style={{ color: 'var(--text-muted)' }}>Bar: <span style={{ color: 'white' }}>#{practiceData?.visible_bar ?? candleCount}/{practiceData?.total_bars ?? '…'}</span></span>
              <span data-testid="current-candle-date" style={{ color: 'var(--text-muted)' }}>Date: <span style={{ color: 'white' }}>{currentDate}</span></span>
              <span style={{ color: 'var(--text-muted)' }}>O: <span style={{ color: 'white' }}>{currentCandle.open.toLocaleString()}</span></span>
              <span style={{ color: 'var(--text-muted)' }}>H: <span style={{ color: 'white' }}>{currentCandle.high.toLocaleString()}</span></span>
              <span style={{ color: 'var(--text-muted)' }}>L: <span style={{ color: 'white' }}>{currentCandle.low.toLocaleString()}</span></span>
              <span style={{ color: 'var(--text-muted)' }}>C: <span style={{ color: currentCandle.close >= currentCandle.open ? 'var(--color-buy)' : 'var(--color-sell)', fontWeight: 600 }}>{currentCandle.close.toLocaleString()}</span></span>
              <span style={{ color: 'var(--text-muted)' }}>V: <span style={{ color: 'white' }}>{currentCandle.volume.toLocaleString()}</span></span>
            </div>
          )}
        </div>

        <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px', alignItems: 'center' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <select
              value={playSpeed}
              onChange={(e) => setPlaySpeed(Number(e.target.value))}
              style={{ background: 'var(--bg-panel)', color: 'white', border: '1px solid var(--border-color)', borderRadius: '4px', padding: '4px', fontSize: '12px' }}
            >
              <option value={1000}>1x</option>
              <option value={500}>2x</option>
              <option value={200}>5x</option>
              <option value={100}>10x</option>
            </select>

            <button
              onClick={() => setIsPlaying(!isPlaying)}
              style={{
                background: isPlaying ? 'rgba(255, 23, 68, 0.2)' : 'rgba(0, 230, 118, 0.2)',
                color: isPlaying ? '#FF1744' : '#00E676',
                border: `1px solid ${isPlaying ? '#FF1744' : '#00E676'}`,
                fontSize: '12px', padding: '6px 12px', display: 'flex', alignItems: 'center', gap: '4px'
              }}
            >
              {isPlaying ? (
                <><svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><path d="M6 4h4v16H6zm8 0h4v16h-4z"/></svg> Pause</>
              ) : (
                <><svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg> Auto-Play</>
              )}
            </button>
          </div>

          <div style={{ width: '1px', height: '24px', background: 'var(--border-color)' }}></div>

          <button
            onClick={handleClearSession}
            style={{ background: 'var(--bg-panel)', color: 'var(--text-muted)', border: '1px solid var(--border-color)', fontSize: '12px', padding: '6px 10px' }}
          >
            New Session
          </button>

          <div style={{ display: 'flex', border: '1px solid var(--border-color)', borderRadius: '6px', overflow: 'hidden' }}>
            <button onClick={() => handlePrev(5)} disabled={navigationPending || isPlaying} style={{ background: 'var(--bg-panel)', color: 'white', fontSize: '13px', padding: '6px 12px', border: 'none', borderRight: '1px solid var(--border-color)', borderRadius: 0 }}>-5</button>
            <button onClick={() => handlePrev(1)} disabled={navigationPending || isPlaying} style={{ background: 'var(--bg-panel)', color: 'white', fontSize: '13px', padding: '6px 12px', border: 'none', borderRadius: 0 }}>← Prev</button>
          </div>
          <div style={{ display: 'flex', border: '1px solid var(--color-primary)', borderRadius: '6px', overflow: 'hidden', boxShadow: '0 0 10px rgba(41,98,255,0.2)' }}>
            <button onClick={() => handleNext(1)} disabled={navigationPending || isPlaying} style={{ background: 'var(--color-primary)', color: 'white', fontSize: '13px', padding: '6px 12px', border: 'none', borderRight: '1px solid rgba(255,255,255,0.2)', borderRadius: 0 }}>Next →</button>
            <button onClick={() => handleNext(5)} disabled={navigationPending || isPlaying} style={{ background: 'var(--color-primary)', color: 'white', fontSize: '13px', padding: '6px 12px', border: 'none', borderRadius: 0 }}>+5</button>
          </div>
        </div>
      </header>

      <IndicatorManager definitions={indicatorDefinitions} document={indicatorDocument} runtime={indicatorRuntime}
        onAdd={addIndicatorInstance} onUpdate={updateIndicatorInstance} onRemove={removeIndicatorInstance}
        onToggle={toggleIndicatorInstance} onMove={moveIndicatorInstance} />

      <div className="replay-workspace-body" style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
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

        <main className="replay-chart-region" style={{ flex: 3, minWidth: 0, padding: '0.5rem', display: 'flex', flexDirection: 'column' }}>
          <div className="panel" data-testid="replay-chart-scroll-region" data-minimum-subpane-height="60" style={{ flex: 1, padding: 0, overflowY: 'auto', overflowX: 'hidden' }}>
            <div style={{ position: 'relative', height: '100%', minHeight: minimumChartHeight }}>
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
              <IndicatorPaneChrome document={indicatorDocument} runtime={indicatorRuntime}
                onSettings={id => window.dispatchEvent(new CustomEvent('sumi:indicator-settings-request', { detail: id }))}
                onToggle={toggleIndicatorInstance} onRemove={removeIndicatorInstance} />
            </div>
            <output data-testid="drawing-domain-state" aria-label="Serialized drawing state" style={{ display: 'none' }}>{JSON.stringify(drawing.document)}</output>
            <output data-testid="drawing-contract-corpus-state" aria-label="Drawing contract corpus results" style={{ display: 'none' }}>{JSON.stringify(evaluateDrawingContractRuntimeCorpus())}</output>
            <output data-testid="indicator-domain-state" aria-label="Serialized indicator state" style={{ display: 'none' }}>{JSON.stringify(indicatorDocument)}</output>
            <output data-testid="indicator-runtime-state" aria-label="Serialized indicator runtime state" style={{ display: 'none' }}>{JSON.stringify(indicatorRuntime)}</output>
            <output data-testid="practice-workflow-state" aria-label="Projected practice workflow state" style={{ display: 'none' }}>{JSON.stringify(practiceData)}</output>
            <output data-testid="trade-marker-state" aria-label="Serialized trade marker state" style={{ display: 'none' }}>{JSON.stringify(markers)}</output>
          </div>
        </main>

        <PracticeRail key={selectedDrawing?.id ?? 'practice'} selectedDrawingId={selectedDrawing?.id}
          drawing={<DrawingInspector key={`${selectedDrawing?.id ?? 'none'}:${drawing.document.revision}`} selected={selectedDrawing} persistenceStatus={drawing.persistenceStatus} onApply={drawing.updateDrawing} onDelete={() => drawing.remove()} />}
          trade={<div style={{ display: 'grid', gap: 8 }}>
            {signalSource && <div className="panel" style={{ padding: '12px', borderColor: 'rgba(255, 209, 102, 0.35)' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', gap: '8px', alignItems: 'center', marginBottom: '8px' }}>
                <h3 style={{ margin: 0, fontSize: '14px', color: '#FFD166' }}>Scanner Signal</h3>
                <span style={{ color: 'var(--text-muted)', fontSize: '12px' }}>{signalDate}</span>
              </div>
              <div style={{ display: 'grid', gap: '6px', fontSize: '12px' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', gap: '8px' }}>
                  <span style={{ color: 'var(--text-muted)' }}>Strategy</span>
                  <span style={{ textAlign: 'right' }}>{signalSource.strategy || 'n/a'}</span>
                </div>
                <div style={{ display: 'flex', justifyContent: 'space-between', gap: '8px' }}>
                  <span style={{ color: 'var(--text-muted)' }}>Signal</span>
                  <span style={{ textTransform: 'uppercase' }}>{signalSource.signal_type || 'n/a'}</span>
                </div>
                <div style={{ display: 'flex', justifyContent: 'space-between', gap: '8px' }}>
                  <span style={{ color: 'var(--text-muted)' }}>Regime</span>
                  <span>{signalSource.regime || 'n/a'}</span>
                </div>
                <div style={{ display: 'flex', justifyContent: 'space-between', gap: '8px' }}>
                  <span style={{ color: 'var(--text-muted)' }}>Signal Price</span>
                  <span style={{ fontFamily: 'monospace' }}>{signalSource.price?.toLocaleString() || 'n/a'}</span>
                </div>
              </div>
            </div>}
            {practiceData ? <><TradeControls snapshot={practiceData} onSubmitDecision={handleSubmitDecision} disabled={practiceLoading || practiceError} /><PendingOrdersPanel orders={practiceData.orders} /><PositionPanel positions={practiceData.positions} snapshot={practiceData} /></> : <div role={practiceError ? 'alert' : 'status'}>{practiceError ? 'Practice state could not be loaded.' : 'Loading practice state…'}</div>}
          </div>}
          journal={practiceData ? <PracticeJournal snapshot={practiceData} entries={journalData || []} loading={journalLoading} loadError={journalError} onSave={handleSaveJournal} /> : <div>Loading practice context…</div>}
          decisions={<DecisionJournal decisions={practiceData?.decisions || []} />}
        />
      </div>
    </div>
  );
};
