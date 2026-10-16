import React from 'react';
import type {
  Candle,
  DecisionCreate,
  JournalEntry,
  JournalEntryCreate,
  PracticeWorkflowSnapshot,
  ReplaySession,
  ReplaySourceContext,
} from '../../types';
import type { SumiDrawing } from '../../features/drawings/drawingDomain';
import { DrawingInspector } from '../chart/DrawingInspector';
import { SignalInspector } from '../signals/SignalInspector';
import { SignalExplanationInspector } from '../signals/SignalExplanationInspector';
import { DecisionJournal } from './DecisionJournal';
import { PendingOrdersPanel } from './PendingOrdersPanel';
import { PositionPanel } from './PositionPanel';
import { PracticeJournal } from './PracticeJournal';
import { PracticeRail } from './PracticeRail';
import { PracticeScoreboard } from './PracticeScoreboard';
import { ScannerSourceContext } from './ScannerSourceContext';
import { TradeControls } from './TradeControls';

export interface ReplayRightPanelProps {
  sessionId: number;
  sessionData?: ReplaySession;
  sourceContext?: ReplaySourceContext;
  currentCandle: Candle | null;
  candleCount: number;
  selectedDrawing?: SumiDrawing;
  drawingRevision: number;
  drawingPersistenceStatus: string;
  onUpdateDrawing: (drawing: SumiDrawing) => boolean;
  onDeleteDrawing: () => void;
  showExtendedInspector: boolean;
  onToggleExtendedInspector: () => void;
  selectedInspectorSignal: string;
  onOpenSignalCatalog: () => void;
  practiceData?: PracticeWorkflowSnapshot;
  practiceLoading: boolean;
  practiceError: boolean;
  onResetPractice: () => void;
  onSubmitDecision: (decision: DecisionCreate) => Promise<{ ok: boolean; message: string }>;
  onCancelOrder?: (orderId: number) => void | Promise<void>;
  journalData?: JournalEntry[];
  journalLoading: boolean;
  journalError: boolean;
  onSaveJournal: (entry: JournalEntryCreate) => Promise<{ ok: boolean; message: string }>;
}

export const ReplayRightPanel: React.FC<ReplayRightPanelProps> = ({
  sessionId,
  sessionData,
  sourceContext,
  currentCandle,
  candleCount,
  selectedDrawing,
  drawingRevision,
  drawingPersistenceStatus,
  onUpdateDrawing,
  onDeleteDrawing,
  showExtendedInspector,
  onToggleExtendedInspector,
  selectedInspectorSignal,
  onOpenSignalCatalog,
  practiceData,
  practiceLoading,
  practiceError,
  onResetPractice,
  onSubmitDecision,
  onCancelOrder,
  journalData,
  journalLoading,
  journalError,
  onSaveJournal,
}) => {
  return (
    <PracticeRail
      key={selectedDrawing?.id ?? 'practice'}
      selectedDrawingId={selectedDrawing?.id}
      drawing={
        <DrawingInspector
          key={`${selectedDrawing?.id ?? 'none'}:${drawingRevision}`}
          selected={selectedDrawing}
          persistenceStatus={drawingPersistenceStatus}
          onApply={onUpdateDrawing}
          onDelete={onDeleteDrawing}
        />
      }
      trade={
        <div style={{ display: 'grid', gap: 8 }}>
          {/* Contextual Session Links (GAP-U-06) */}
          <div
            data-testid="practice-context-links"
            style={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              gap: 6,
              padding: '4px 8px',
              background: 'rgba(255, 255, 255, 0.03)',
              border: '1px solid var(--border-color)',
              borderRadius: 6,
              fontSize: 11,
            }}
          >
            <span style={{ color: 'var(--text-muted)' }}>Phiên #{sessionId}</span>
            <div style={{ display: 'flex', gap: 8 }}>
              <a
                href="/scanner"
                data-testid="practice-link-scanner"
                style={{ color: '#FFD166', fontWeight: 600 }}
              >
                🔎 Scanner
              </a>
              <a
                href={`/journal?session=${sessionId}`}
                data-testid="practice-link-journal"
                style={{ color: '#58A6FF', fontWeight: 600 }}
              >
                📓 Nhật ký
              </a>
              <a
                href={`/analytics?session=${sessionId}`}
                data-testid="practice-link-analytics"
                style={{ color: '#00E676', fontWeight: 600 }}
              >
                📊 Phân tích
              </a>
            </div>
          </div>

          {sourceContext && <ScannerSourceContext context={sourceContext} display="details" />}

          {practiceData ? (
            <>
              <PracticeScoreboard
                snapshot={practiceData}
                onResetPractice={onResetPractice}
              />
              <TradeControls
                snapshot={practiceData}
                onSubmitDecision={onSubmitDecision}
                disabled={practiceLoading || practiceError}
              />
              <PendingOrdersPanel orders={practiceData.orders} onCancelOrder={onCancelOrder} />
              <PositionPanel positions={practiceData.positions} snapshot={practiceData} />
            </>
          ) : (
            <div role={practiceError ? 'alert' : 'status'}>
              {practiceError
                ? 'Practice state could not be loaded.'
                : 'Loading practice state…'}
            </div>
          )}

          <div style={{ display: 'flex', gap: 6 }}>
            <button
              type="button"
              data-testid="open-signal-catalog-modal-btn"
              onClick={onOpenSignalCatalog}
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
              title="Tra cứu 72 tín hiệu định lượng chuẩn hóa"
            >
              📚 Danh Mục Tín Hiệu (72)
            </button>
            <button
              type="button"
              data-testid="toggle-extended-inspector-btn"
              onClick={onToggleExtendedInspector}
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
              title="Bật/tắt bộ soi chi tiết 72 tín hiệu, Sức khỏe kỹ thuật và Dòng tiền BB tại nến hiện tại"
            >
              {showExtendedInspector ? '🔬 Đóng Soi Chi Tiết' : '🔬 Soi Tín Hiệu Nến'}
            </button>
          </div>
          {showExtendedInspector && (
            <SignalExplanationInspector
              sessionId={sessionId}
              currentIndex={sessionData?.current_index ?? candleCount - 1}
              timeframe={sessionData?.timeframe || '1D'}
              currentTimestamp={currentCandle?.timestamp}
              initialSignalName={selectedInspectorSignal}
              onOpenCatalog={onOpenSignalCatalog}
            />
          )}
          <SignalInspector
            sessionId={sessionId}
            currentIndex={sessionData?.current_index ?? candleCount - 1}
            timeframe={sessionData?.timeframe || '1D'}
            currentTimestamp={currentCandle?.timestamp}
          />
        </div>
      }
      journal={
        practiceData ? (
          <PracticeJournal
            snapshot={practiceData}
            entries={journalData || []}
            loading={journalLoading}
            loadError={journalError}
            onSave={onSaveJournal}
          />
        ) : (
          <div>Loading practice context…</div>
        )
      }
      decisions={<DecisionJournal decisions={practiceData?.decisions || []} />}
    />
  );
};
