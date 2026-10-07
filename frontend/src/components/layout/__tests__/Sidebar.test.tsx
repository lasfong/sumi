import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { Sidebar } from '../Sidebar';
import { useReplayStore } from '../../../store/replayStore';

describe('Sidebar Navigation & Settings Modal (GL-01, GL-02, GL-03)', () => {
  beforeEach(() => {
    // Reset session in replayStore
    useReplayStore.setState({ sessionId: null });
  });

  it('renders all functional group headers (GL-01)', () => {
    render(
      <MemoryRouter initialEntries={['/']}>
        <Sidebar isCollapsed={false} />
      </MemoryRouter>
    );

    // Group 1: Manual practice
    const manualGroup = screen.getByTestId('nav-group-manual');
    expect(manualGroup).toBeInTheDocument();
    expect(manualGroup).toHaveTextContent('LUYỆN TẬP THỦ CÔNG');

    // Group 2: Automated testing & Lab
    const autoGroup = screen.getByTestId('nav-group-auto');
    expect(autoGroup).toBeInTheDocument();
    expect(autoGroup).toHaveTextContent('KIỂM ĐỊNH TỰ ĐỘNG & NGHIÊN CỨU');

    // Group 3: System utilities
    const systemGroup = screen.getByTestId('nav-group-system');
    expect(systemGroup).toBeInTheDocument();
    expect(systemGroup).toHaveTextContent('TIỆN ÍCH HỆ THỐNG');
  });

  it('renders all required navigation items with correct paths (GL-01)', () => {
    render(
      <MemoryRouter initialEntries={['/']}>
        <Sidebar isCollapsed={false} />
      </MemoryRouter>
    );

    // NHÓM 1: LUYỆN TẬP THỦ CÔNG
    const replayLink = screen.getByTestId('nav-item-replay');
    expect(replayLink).toBeInTheDocument();
    expect(replayLink).toHaveAttribute('href', '/replay');
    expect(replayLink).toHaveTextContent('Trading Lab');
    expect(replayLink).toHaveTextContent('Core');

    const journalLink = screen.getByTestId('nav-item-journal');
    expect(journalLink).toBeInTheDocument();
    expect(journalLink).toHaveAttribute('href', '/journal');
    expect(journalLink).toHaveTextContent('Nhật Ký Giao Dịch');

    const analyticsLink = screen.getByTestId('nav-item-analytics');
    expect(analyticsLink).toBeInTheDocument();
    expect(analyticsLink).toHaveAttribute('href', '/analytics');
    expect(analyticsLink).toHaveTextContent('Phân Tích Hiệu Suất');

    // NHÓM 2: KIỂM ĐỊNH TỰ ĐỘNG & NGHIÊN CỨU
    const strategyLabLink = screen.getByTestId('nav-item-strategy-lab');
    expect(strategyLabLink).toBeInTheDocument();
    expect(strategyLabLink).toHaveAttribute('href', '/strategy-lab');
    expect(strategyLabLink).toHaveTextContent('Strategy Tester');
    expect(strategyLabLink).toHaveTextContent('V3');

    const scannerLink = screen.getByTestId('nav-item-scanner');
    expect(scannerLink).toBeInTheDocument();
    expect(scannerLink).toHaveAttribute('href', '/scanner');
    expect(scannerLink).toHaveTextContent('Bộ Quét Tín Hiệu');

    const catalogLink = screen.getByTestId('nav-item-signal-catalog');
    expect(catalogLink).toBeInTheDocument();
    expect(catalogLink).toHaveAttribute('href', '/strategy-lab?tab=catalog');
    expect(catalogLink).toHaveTextContent('Thư Viện Tín Hiệu');

    const builderLink = screen.getByTestId('nav-item-rule-builder');
    expect(builderLink).toBeInTheDocument();
    expect(builderLink).toHaveAttribute('href', '/strategy-lab?tab=builder');
    expect(builderLink).toHaveTextContent('Soạn Quy Tắc');

    // TIỆN ÍCH HỆ THỐNG
    const importLink = screen.getByTestId('nav-item-import');
    expect(importLink).toBeInTheDocument();
    expect(importLink).toHaveAttribute('href', '/import');
    expect(importLink).toHaveTextContent('Nạp Dữ Liệu');

    // Settings button
    const settingsBtn = screen.getByTestId('sidebar-settings-button');
    expect(settingsBtn).toBeInTheDocument();
    expect(settingsBtn).toHaveTextContent('Cài Đặt');
  });

  it('renders version badge as v3.0.0 with data-testid (GL-03)', () => {
    render(
      <MemoryRouter initialEntries={['/']}>
        <Sidebar isCollapsed={false} />
      </MemoryRouter>
    );

    const badge = screen.getByTestId('version-badge');
    expect(badge).toBeInTheDocument();
    expect(badge).toHaveTextContent('v3.0.0');
  });

  it('opens Settings modal on click and closes via close button (GL-02)', () => {
    render(
      <MemoryRouter initialEntries={['/']}>
        <Sidebar isCollapsed={false} />
      </MemoryRouter>
    );

    // Initial state: modal is closed
    expect(screen.queryByTestId('settings-modal')).not.toBeInTheDocument();

    // Click Settings button
    const settingsBtn = screen.getByTestId('sidebar-settings-button');
    fireEvent.click(settingsBtn);

    // Modal is opened
    const modal = screen.getByTestId('settings-modal');
    expect(modal).toBeInTheDocument();

    // Verify system config content
    expect(screen.getByText('Cấu Hình Hệ Thống (System Configuration)')).toBeInTheDocument();
    expect(screen.getByText('SQLite (backend/sumi.db)')).toBeInTheDocument();
    expect(screen.getByText('Local-First (Offline Ready)')).toBeInTheDocument();
    expect(screen.getByText('http://localhost:8000')).toBeInTheDocument();

    // Verify shortcuts content
    expect(screen.getByText('Bảng Phím Tắt Hệ Thống (Keyboard Shortcuts)')).toBeInTheDocument();
    expect(screen.getByText('Space')).toBeInTheDocument();
    expect(screen.getByText('B')).toBeInTheDocument();
    expect(screen.getByText('S')).toBeInTheDocument();
    expect(screen.getByText('1 - 9')).toBeInTheDocument();

    // Verify version and environment
    expect(screen.getByText('Phiên Bản & Môi Trường (Version & Environment)')).toBeInTheDocument();
    expect(screen.getAllByText('v3.0.0').length).toBeGreaterThanOrEqual(1);

    // Close via close button
    const closeBtn = screen.getByTestId('close-settings-btn');
    fireEvent.click(closeBtn);

    // Modal is closed
    expect(screen.queryByTestId('settings-modal')).not.toBeInTheDocument();
  });

  it('closes Settings modal on Escape key press (GL-02)', () => {
    render(
      <MemoryRouter initialEntries={['/']}>
        <Sidebar isCollapsed={false} />
      </MemoryRouter>
    );

    // Click Settings button
    fireEvent.click(screen.getByTestId('sidebar-settings-button'));
    expect(screen.getByTestId('settings-modal')).toBeInTheDocument();

    // Press Escape
    fireEvent.keyDown(window, { key: 'Escape' });

    // Modal is closed
    expect(screen.queryByTestId('settings-modal')).not.toBeInTheDocument();
  });

  it('closes Settings modal on backdrop click (GL-02)', () => {
    render(
      <MemoryRouter initialEntries={['/']}>
        <Sidebar isCollapsed={false} />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByTestId('sidebar-settings-button'));
    const modalBackdrop = screen.getByTestId('settings-modal');
    expect(modalBackdrop).toBeInTheDocument();

    // Click backdrop
    fireEvent.click(modalBackdrop);
    expect(screen.queryByTestId('settings-modal')).not.toBeInTheDocument();
  });

  it('renders collapsed state correctly with minimal indicators', () => {
    render(
      <MemoryRouter initialEntries={['/']}>
        <Sidebar isCollapsed={true} />
      </MemoryRouter>
    );

    const sidebar = screen.getByTestId('app-sidebar');
    expect(sidebar).toHaveClass('collapsed');

    // Group titles should display collapsed placeholder
    const manualGroup = screen.getByTestId('nav-group-manual');
    expect(manualGroup).toHaveTextContent('•••');

    // Brand title is hidden
    expect(screen.queryByText('Sumi')).not.toBeInTheDocument();
  });

  it('preserves active sessionId in manual practice links', () => {
    useReplayStore.setState({ sessionId: 456 });

    render(
      <MemoryRouter initialEntries={['/']}>
        <Sidebar isCollapsed={false} />
      </MemoryRouter>
    );

    const replayLink = screen.getByTestId('nav-item-replay');
    expect(replayLink).toHaveAttribute('href', '/replay?session=456');

    const journalLink = screen.getByTestId('nav-item-journal');
    expect(journalLink).toHaveAttribute('href', '/journal?session=456');

    const analyticsLink = screen.getByTestId('nav-item-analytics');
    expect(analyticsLink).toHaveAttribute('href', '/analytics?session=456');

    // Auto testing links do not append session
    const strategyLabLink = screen.getByTestId('nav-item-strategy-lab');
    expect(strategyLabLink).toHaveAttribute('href', '/strategy-lab');
  });
});
