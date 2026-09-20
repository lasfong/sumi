import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { KeyboardShortcutsModal } from '../KeyboardShortcutsModal';

describe('KeyboardShortcutsModal', () => {
  it('renders nothing when closed', () => {
    const { container } = render(<KeyboardShortcutsModal isOpen={false} onClose={vi.fn()} />);
    expect(container.firstChild).toBeNull();
  });

  it('renders categories and closes on Escape or button click', () => {
    const onClose = vi.fn();
    render(<KeyboardShortcutsModal isOpen={true} onClose={onClose} />);

    expect(screen.getByRole('dialog', { name: 'Bảng phím tắt Trading Lab' })).toBeInTheDocument();
    expect(screen.getByText('Phím tắt Trading Lab (Pro Hotkeys)')).toBeInTheDocument();
    expect(screen.getByText('Điều khiển tua nến (Replay)')).toBeInTheDocument();
    expect(screen.getByText('Giao dịch & Kỷ luật (Trading)')).toBeInTheDocument();

    // Test Esc key
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(1);

    // Test close button
    fireEvent.click(screen.getByRole('button', { name: 'Đã hiểu (Esc)' }));
    expect(onClose).toHaveBeenCalledTimes(2);
  });
});
