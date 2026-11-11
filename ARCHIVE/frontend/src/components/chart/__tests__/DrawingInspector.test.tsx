import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { DrawingInspector } from '../DrawingInspector';
import type { SumiDrawing } from '../../../features/drawings/drawingDomain';

const rectangle: SumiDrawing = {
  id: '123e4567-e89b-42d3-a456-426614174000', tool: 'rectangle', paneId: 'price', order: 0, visible: true, locked: false,
  anchors: [{ time: '2026-07-15', price: 100 }, { time: '2026-07-16', price: 120 }],
  style: { lineColor: '#e056fd', lineWidth: 2, lineStyle: 'solid', fillColor: '#e056fd', fillOpacity: 0.12 }, geometry: { kind: 'rectangle' },
};

describe('DrawingInspector', () => {
  it('shows complete labelled values and submits style, visibility and lock as one semantic draft', () => {
    const apply = vi.fn((drawing: SumiDrawing) => Boolean(drawing)); render(<DrawingInspector selected={rectangle} persistenceStatus="ready" onApply={apply} onDelete={vi.fn()} />);
    expect(screen.getByTestId('selected-drawing-id')).toHaveAttribute('title', rectangle.id);
    expect(screen.getByLabelText('Anchor 1 time')).toHaveValue('2026-07-15'); expect(screen.getByLabelText('Anchor 2 price')).toHaveValue(120);
    fireEvent.change(screen.getByTestId('drawing-line-width'), { target: { value: '4' } }); fireEvent.change(screen.getByTestId('drawing-line-style'), { target: { value: 'dashed' } });
    fireEvent.change(screen.getByTestId('drawing-fill-opacity'), { target: { value: '0.3' } }); fireEvent.click(screen.getByTestId('drawing-visible')); fireEvent.click(screen.getByTestId('drawing-locked'));
    fireEvent.click(screen.getByTestId('apply-drawing-settings')); expect(apply).toHaveBeenCalledOnce();
    expect(apply.mock.calls[0][0]).toMatchObject({ visible: false, locked: true, style: { lineWidth: 4, lineStyle: 'dashed', fillOpacity: 0.3 } });
  });
  it('blocks invalid leftward Ray drafts and isolates editing keys from chart commands', () => {
    const ray: SumiDrawing = { ...rectangle, tool: 'ray', anchors: [{ time: '2026-07-15', price: 100 }, { time: '2026-07-16', price: 120 }], geometry: { kind: 'ray' } };
    const chartKey = vi.fn(); window.addEventListener('keydown', chartKey); render(<DrawingInspector selected={ray} persistenceStatus="ready" onApply={vi.fn(() => true)} onDelete={vi.fn()} />);
    const secondDate = screen.getByLabelText('Anchor 2 time'); fireEvent.change(secondDate, { target: { value: '2026-07-14' } });
    expect(screen.getByTestId('apply-drawing-settings')).toBeDisabled(); expect(screen.getByTestId('drawing-inspector-validation')).toBeVisible();
    fireEvent.keyDown(secondDate, { key: 'Delete', code: 'Delete' }); fireEvent.keyDown(secondDate, { key: 'Escape', code: 'Escape' }); expect(chartKey).not.toHaveBeenCalled();
    window.removeEventListener('keydown', chartKey);
  });
});
