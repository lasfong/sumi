import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { IndicatorRenderRegistry } from '../IndicatorRenderRegistry';
import { IndicatorPaneChrome } from '../IndicatorPaneChrome';
import { ChartLegendOverlay } from '../ChartLegendOverlay';
import { ReplayHeader } from '../../replay/ReplayHeader';
import type { IndicatorDocumentV1, IndicatorInstanceV1 } from '../../../features/indicators/indicatorDomain';
import type { IndicatorRuntimeState } from '../IndicatorManager';

const rsiInstance: IndicatorInstanceV1 = {
  id: 'inst-rsi-14',
  definitionId: 'rsi',
  label: 'RSI',
  params: { length: 14 },
  placement: 'oscillator',
  paneId: 'indicator:rsi-14',
  visible: true,
  order: 0,
  styles: {
    primary: { color: '#58A6FF' },
  },
};

const docWithRsi: IndicatorDocumentV1 = {
  schemaVersion: 1,
  sessionId: 42,
  instances: [rsiInstance],
};

describe('B32 — ChartKernel Facade, MappingResult Contract & ReplayHeader Decomposition', () => {
  it('surfaces UNMAPPED_COLUMN with diagnostic message even when backend meta.status is OK', () => {
    const result = IndicatorRenderRegistry.mapBackendDataWithResult(
      [{ timestamp: '2026-01-02', WRONG_RSI_COL: 55.2 }],
      rsiInstance,
      {
        status: 'OK',
        warmup_bars: 14,
        available_bars: 20,
        is_projected_clamped: false,
      },
    );

    expect(result.status).toBe('UNMAPPED_COLUMN');
    expect(result.series).toEqual([]);
    expect(result.message).toContain('Indicator column mapping missing for rsi');
    expect(result.message).toContain('WRONG_RSI_COL');
  });

  it('surfaces warming progress (available/warmup) and mapping-missing alerts in IndicatorPaneChrome and ChartLegendOverlay', () => {
    const warmingRuntime: Record<string, IndicatorRuntimeState> = {
      [rsiInstance.id]: {
        status: 'warming',
        mappingStatus: 'EMPTY_WARMING',
        warmupBars: 50,
        availableBars: 11,
        values: {},
      },
    };

    const { rerender } = render(
      <div>
        <IndicatorPaneChrome
          document={docWithRsi}
          runtime={warmingRuntime}
          onSettings={vi.fn()}
          onToggle={vi.fn()}
          onRemove={vi.fn()}
        />
        <ChartLegendOverlay
          symbol="FPT"
          timeframe="1D"
          currentCandle={null}
          indicators={[rsiInstance]}
          runtime={warmingRuntime}
          onToggle={vi.fn()}
          onSettings={vi.fn()}
          onRemove={vi.fn()}
        />
      </div>,
    );

    expect(screen.getAllByText('warming (11/50)').length).toBeGreaterThanOrEqual(2);

    const mappingErrorRuntime: Record<string, IndicatorRuntimeState> = {
      [rsiInstance.id]: {
        status: 'error',
        errorKind: 'mapping',
        mappingStatus: 'UNMAPPED_COLUMN',
        error: 'Indicator column mapping missing for rsi (WRONG_COL)',
        values: {},
      },
    };

    rerender(
      <div>
        <IndicatorPaneChrome
          document={docWithRsi}
          runtime={mappingErrorRuntime}
          onSettings={vi.fn()}
          onToggle={vi.fn()}
          onRemove={vi.fn()}
        />
        <ChartLegendOverlay
          symbol="FPT"
          timeframe="1D"
          currentCandle={null}
          indicators={[rsiInstance]}
          runtime={mappingErrorRuntime}
          onToggle={vi.fn()}
          onSettings={vi.fn()}
          onRemove={vi.fn()}
        />
      </div>,
    );

    expect(screen.getByTestId(`indicator-mapping-warning-${rsiInstance.id}`)).toHaveTextContent(
      'Indicator column mapping missing for rsi (WRONG_COL)',
    );
    expect(screen.getByTestId(`legend-mapping-warning-${rsiInstance.id}`)).toHaveTextContent(
      'Indicator column mapping missing for rsi (WRONG_COL)',
    );
  });

  it('renders ReplayHeader with WebSocket offline/reconnecting status and completed session badges', () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });

    const { rerender } = render(
      <QueryClientProvider client={queryClient}>
        <ReplayHeader
          sessionId={42}
          symbolName="FPT"
          sessionStatus="active"
          currentCandle={null}
          candleCount={10}
          onChangeTimeframe={vi.fn()}
          onResumeSession={vi.fn()}
          onOpenSymbolSwitcher={vi.fn()}
          onOpenDebrief={vi.fn()}
          onOpenShortcuts={vi.fn()}
          playSpeed={1000}
          onChangeSpeed={vi.fn()}
          isRightPanelOpen={true}
          onToggleRightPanel={vi.fn()}
          onClearSession={vi.fn()}
          wsStatus="OFFLINE"
          wsReconnectAttempt={0}
        />
      </QueryClientProvider>,
    );

    expect(screen.getByTestId('replay-ws-status-badge')).toHaveTextContent(
      'Offline — waiting for network',
    );

    rerender(
      <QueryClientProvider client={queryClient}>
        <ReplayHeader
          sessionId={42}
          symbolName="FPT"
          sessionStatus="completed"
          currentCandle={null}
          candleCount={10}
          onChangeTimeframe={vi.fn()}
          onResumeSession={vi.fn()}
          onOpenSymbolSwitcher={vi.fn()}
          onOpenDebrief={vi.fn()}
          onOpenShortcuts={vi.fn()}
          playSpeed={1000}
          onChangeSpeed={vi.fn()}
          isRightPanelOpen={true}
          onToggleRightPanel={vi.fn()}
          onClearSession={vi.fn()}
          wsStatus="RECONNECTING"
          wsReconnectAttempt={2}
        />
      </QueryClientProvider>,
    );

    expect(screen.getByTestId('replay-ws-status-badge')).toHaveTextContent('Reconnecting (2)…');
    expect(screen.getByTestId('replay-session-status-badge')).toHaveTextContent('completed');
  });
});
