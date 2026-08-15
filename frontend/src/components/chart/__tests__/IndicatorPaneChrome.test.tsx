import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { IndicatorPaneChrome } from '../IndicatorPaneChrome';
import type { IndicatorDocumentV1 } from '../../../features/indicators/indicatorDomain';

const doc: IndicatorDocumentV1 = {
  schemaVersion: 1,
  sessionId: 1,
  instances: [
    {
      id: 'vol-1',
      definitionId: 'volume',
      label: 'Volume',
      params: {},
      placement: 'volume',
      paneId: 'volume',
      visible: true,
      order: 0,
      styles: { volume: { color: '#58A6FF' } },
    },
    {
      id: 'vma-1',
      definitionId: 'volume_sma',
      label: 'Volume SMA',
      params: { length: 20 },
      placement: 'volume',
      paneId: 'volume',
      visible: true,
      order: 1,
      styles: { primary: { color: '#FF8A00' } },
    },
    {
      id: 'atr-1',
      definitionId: 'atr',
      label: 'Average True Range',
      params: { length: 14 },
      placement: 'oscillator',
      paneId: 'indicator:atr-1',
      visible: true,
      order: 2,
      styles: { primary: { color: '#E040FB' } },
    },
  ],
};

describe('IndicatorPaneChrome', () => {
  it('groups instances by physical paneId so raw Volume and Volume SMA share one section and ATR gets its own aligned section', () => {
    const runtime = {
      'vol-1': { status: 'ready' as const, values: { 'raw-volume': 1500000 } },
      'vma-1': { status: 'ready' as const, values: { 'primary': 1200000 } },
      'atr-1': { status: 'ready' as const, values: { 'primary': 2.45 } },
    };

    const { container } = render(
      <IndicatorPaneChrome
        document={doc}
        runtime={runtime}
        onSettings={vi.fn()}
        onToggle={vi.fn()}
        onRemove={vi.fn()}
      />
    );

    // Should render layer container
    const layer = screen.getByTestId('indicator-pane-chrome-layer');
    expect(layer).toBeInTheDocument();

    // Exactly 2 subpane sections for non-price panes (volume and indicator:atr-1)
    const sections = container.querySelectorAll('section');
    expect(sections.length).toBe(2);

    // Section 1: volume pane containing both Volume and Volume SMA controls
    const volumeSection = sections[0];
    expect(volumeSection).toHaveAttribute('data-pane-id', 'volume');
    expect(volumeSection.querySelector('[data-testid="indicator-pane-chrome-vol-1"]')).toBeInTheDocument();
    expect(volumeSection.querySelector('[data-testid="indicator-pane-chrome-vma-1"]')).toBeInTheDocument();

    // Section 2: ATR pane containing ATR controls
    const atrSection = sections[1];
    expect(atrSection).toHaveAttribute('data-pane-id', 'indicator:atr-1');
    expect(atrSection.querySelector('[data-testid="indicator-pane-chrome-atr-1"]')).toBeInTheDocument();
  });
});
