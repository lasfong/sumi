import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { StrategyRuleBuilder } from '../StrategyRuleBuilder';
import * as signalsApi from '../../../api/signalsApi';
import '@testing-library/jest-dom';

vi.mock('../../../api/signalsApi', () => ({
  getSignalRegistry: vi.fn(),
}));

const mockRegistry = {
  signals: [
    {
      name: 'health.favorable',
      version: '1.0.0',
      category: 'health',
      label_vi: 'Sức Khỏe Kỹ Thuật Thuận Lợi',
      description: 'Health score >= +35.0',
      output_type: 'bool' as const,
      parameters_schema: {},
      default_parameters: {},
      dependencies: [],
      warmup_bars: 50,
      causal_delay_bars: 0,
      status: 'ACTIVE' as const,
      ast_alias: 'health__favorable',
    },
    {
      name: 'bb.direction_rising',
      version: '1.0.0',
      category: 'flow',
      label_vi: 'Dòng Tiền BB Hướng Lên',
      description: 'Dòng tiền BB tăng liên tục',
      output_type: 'bool' as const,
      parameters_schema: {},
      default_parameters: {},
      dependencies: [],
      warmup_bars: 20,
      causal_delay_bars: 0,
      status: 'ACTIVE' as const,
      ast_alias: 'bb__direction_rising',
    },
    {
      name: 'divergence.rsi_regular_bullish',
      version: '1.0.0',
      category: 'divergence',
      label_vi: 'Phân Kỳ Thường Tăng RSI',
      description: 'Đáy giá thấp hơn kèm đáy RSI cao hơn',
      output_type: 'bool' as const,
      parameters_schema: {},
      default_parameters: {},
      dependencies: [],
      warmup_bars: 30,
      causal_delay_bars: 3,
      status: 'ACTIVE' as const,
      ast_alias: 'divergence__rsi_regular_bullish',
    },
  ],
};

const renderWithClient = (ui: React.ReactElement) => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
};

describe('StrategyRuleBuilder Component (FR-CORE-006, F.4)', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(signalsApi.getSignalRegistry).mockResolvedValue(mockRegistry);
  });

  it('renders visual rule builder and generates valid AST expression', async () => {
    renderWithClient(<StrategyRuleBuilder />);

    await waitFor(() => {
      expect(screen.getByTestId('strategy-rule-builder')).toBeInTheDocument();
    });

    // Default clauses: health__favorable == True and bb__direction_rising == True
    const exprBox = screen.getByTestId('generated-ast-expression');
    expect(exprBox).toHaveTextContent('health__favorable == True and bb__direction_rising == True');
  });

  it('allows adding and removing clauses and updates expression', async () => {
    renderWithClient(<StrategyRuleBuilder />);

    await waitFor(() => {
      expect(screen.getByTestId('add-clause-btn')).toBeInTheDocument();
    });

    // Click Add Clause
    const addBtn = screen.getByTestId('add-clause-btn');
    fireEvent.click(addBtn);

    expect(screen.getByTestId('clause-row-2')).toBeInTheDocument();

    // Change signal in clause 2 to divergence__rsi_regular_bullish
    const selectClause2 = screen.getByTestId('clause-signal-select-2');
    fireEvent.change(selectClause2, { target: { value: 'divergence__rsi_regular_bullish' } });

    const exprBox = screen.getByTestId('generated-ast-expression');
    expect(exprBox).toHaveTextContent('divergence__rsi_regular_bullish == True');
  });

  it('copies generated YAML configuration on button click', async () => {
    Object.assign(navigator, {
      clipboard: {
        writeText: vi.fn().mockResolvedValue(undefined),
      },
    });

    renderWithClient(<StrategyRuleBuilder />);

    await waitFor(() => {
      expect(screen.getByTestId('copy-rule-yaml-btn')).toBeInTheDocument();
    });

    const copyBtn = screen.getByTestId('copy-rule-yaml-btn');
    fireEvent.click(copyBtn);

    expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
      expect.stringContaining('health__favorable == True and bb__direction_rising == True')
    );
  });

  it('executes rule trigger, saves to sessionStorage, invokes callback, and shows feedback banner (ST-04)', async () => {
    const onExecuteRule = vi.fn();
    const onRuleGenerated = vi.fn();

    const sessionMap = new Map<string, string>();
    const sessionMock = {
      getItem: vi.fn((key: string) => sessionMap.get(key) ?? null),
      setItem: vi.fn((key: string, value: string) => sessionMap.set(key, value)),
      removeItem: vi.fn((key: string) => sessionMap.delete(key)),
      clear: vi.fn(() => sessionMap.clear()),
    };
    Object.defineProperty(window, 'sessionStorage', {
      value: sessionMock,
      writable: true,
      configurable: true,
    });

    renderWithClient(
      <StrategyRuleBuilder onExecuteRule={onExecuteRule} onRuleGenerated={onRuleGenerated} />
    );

    await waitFor(() => {
      expect(screen.getByTestId('execute-rule-btn')).toBeInTheDocument();
      expect(screen.getByTestId('switch-to-battle-btn')).toBeInTheDocument();
    });

    const executeBtn = screen.getByTestId('execute-rule-btn');
    fireEvent.click(executeBtn);

    // Verify callback was invoked with structured payload
    expect(onExecuteRule).toHaveBeenCalledTimes(1);
    const payload = onExecuteRule.mock.calls[0][0];
    expect(payload.ruleName).toBe('Bo_Dieu_Kien_Mau');
    expect(payload.expression).toBe('health__favorable == True and bb__direction_rising == True');
    expect(payload.signalsRequired).toEqual(['health__favorable', 'bb__direction_rising']);

    // Verify sessionStorage received the payload
    expect(sessionMock.setItem).toHaveBeenCalledWith(
      'sumi_pending_strategy_rule',
      expect.stringContaining('health__favorable == True')
    );

    // Verify feedback banner is visible with AST expression
    expect(screen.getByTestId('execution-feedback-banner')).toBeInTheDocument();
    expect(screen.getByTestId('execution-feedback-banner')).toHaveTextContent(
      'Đã kích hoạt kiểm định cho quy tắc "Bo_Dieu_Kien_Mau"'
    );

    // Verify dismiss button hides the banner
    const dismissBtn = screen.getByTestId('dismiss-feedback-btn');
    fireEvent.click(dismissBtn);
    expect(screen.queryByTestId('execution-feedback-banner')).not.toBeInTheDocument();
  });

  it('switches to battle tab on switch-to-battle button click (ST-04)', async () => {
    const battleTabMock = document.createElement('button');
    battleTabMock.setAttribute('data-testid', 'lab-tab-battle');
    const clickSpy = vi.fn();
    battleTabMock.addEventListener('click', clickSpy);
    document.body.appendChild(battleTabMock);

    renderWithClient(<StrategyRuleBuilder />);

    await waitFor(() => {
      expect(screen.getByTestId('switch-to-battle-btn')).toBeInTheDocument();
    });

    const switchBtn = screen.getByTestId('switch-to-battle-btn');
    fireEvent.click(switchBtn);

    expect(clickSpy).toHaveBeenCalled();

    document.body.removeChild(battleTabMock);
  });
});
