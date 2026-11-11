import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import '@testing-library/jest-dom';

import { ScannerSourceContext } from '../ScannerSourceContext';
import { buildScannerSignalMarker } from '../ReplayWorkspaceController';
import type { ReplaySourceContext } from '../../../types';


const hiddenContext: ReplaySourceContext = {
  schema_version: 1,
  source_type: 'scanner_signal',
  replay_intent: 'blind_practice',
  reveal_at_index: 10,
  revealed: false,
  signal: null,
};

const revealedContext: ReplaySourceContext = {
  ...hiddenContext,
  revealed: true,
  signal: {
    timestamp: '2024-01-11T00:00:00',
    type: 'entry',
    strategy: 'Future Integrity Strategy',
    price: 110.5,
    regime: 'bullish',
  },
};


describe('ScannerSourceContext', () => {
  it('labels blind practice without placing future signal values in visible or accessible DOM', () => {
    const { container } = render(<ScannerSourceContext context={hiddenContext} />);
    expect(screen.getByTestId('scanner-replay-intent')).toHaveTextContent('Blind practice');
    expect(screen.getByTestId('scanner-signal-hidden')).toBeInTheDocument();
    expect(container.textContent).not.toContain('2024-01-11');
    expect(container.textContent).not.toContain('Future Integrity Strategy');
    expect(container.textContent).not.toContain('110.5');
    expect(container.textContent).not.toContain('bullish');
    expect(buildScannerSignalMarker(hiddenContext, '2024-01-10')).toEqual([]);
  });

  it('reveals sanitized context and exactly one marker at the boundary', () => {
    render(<ScannerSourceContext context={revealedContext} display="details" />);
    const panel = screen.getByTestId('scanner-signal-context');
    expect(panel).toHaveAccessibleName('Blind practice revealed signal context');
    expect(panel).toHaveTextContent('Future Integrity Strategy');
    expect(panel).toHaveTextContent('entry');
    expect(panel).toHaveTextContent('bullish');
    expect(panel).toHaveTextContent('110.5');
    expect(buildScannerSignalMarker(revealedContext, '2024-01-11')).toHaveLength(1);
    expect(buildScannerSignalMarker(revealedContext, '2024-01-12')).toHaveLength(1);
  });

  it('uses an honest review label and removes context again after a server-authoritative rewind', () => {
    const reviewContext: ReplaySourceContext = { ...revealedContext, replay_intent: 'signal_review' };
    const { rerender } = render(<ScannerSourceContext context={reviewContext} />);
    expect(screen.getByTestId('scanner-replay-intent')).toHaveTextContent('Signal review');
    rerender(<ScannerSourceContext context={hiddenContext} />);
    expect(screen.getByTestId('scanner-replay-intent')).toHaveTextContent('Blind practice');
    expect(screen.queryByTestId('scanner-signal-context')).not.toBeInTheDocument();
    expect(buildScannerSignalMarker(hiddenContext, '2024-01-09')).toEqual([]);
  });
});
