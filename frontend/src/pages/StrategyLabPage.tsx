import React, { useState, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { getAvailableStrategies, runBacktest } from '../api/backtestApi';
import type { BacktestRequest, BacktestResponse, AvailableStrategy } from '../api/backtestApi';
import {
  cancelParameterSweep,
  deleteStrategyLabRun,
  listStrategyLabRuns,
  runParameterSweep,
  saveStrategyLabRun,
} from '../api/strategyLabApi';
import type {
  StrategyLabRun,
  SweepVariant,
  TypedParameterDefinition,
  SweepParameter,
} from '../api/strategyLabApi';
import { MetricResultValue } from '../components/analytics/MetricResultValue';

interface LabResult {
  filename: string;
  name: string;
  response: BacktestResponse;
  oosResponse?: BacktestResponse | null;
}

interface LabHistoryEntry {
  id: string;
  type: 'comparison' | 'sweep';
  createdAt: string;
  label: string;
  requestConfig?: Record<string, unknown>;
  results?: LabResult[];
  sweepResults?: SweepVariant[];
}

const HISTORY_KEY = 'sumi.strategyLab.history.v1';

const generateSweepId = (): string => {
  return `sweep-${Date.now()}-${Math.random().toString(16).slice(2, 8)}`;
};

const loadHistory = (): LabHistoryEntry[] => {
  if (typeof window === 'undefined') return [];
  try {
    const raw = window.localStorage.getItem(HISTORY_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
};

const saveHistory = (entries: LabHistoryEntry[]) => {
  if (typeof window !== 'undefined') {
    try {
      window.localStorage.setItem(HISTORY_KEY, JSON.stringify(entries.slice(0, 20)));
    } catch {
      try {
        window.localStorage.setItem(HISTORY_KEY, JSON.stringify(entries.slice(0, 5)));
      } catch {
        window.localStorage.removeItem(HISTORY_KEY);
      }
    }
  }
};

export const StrategyLabPage: React.FC = () => {
  const [symbolsInput, setSymbolsInput] = useState('FPT, SSI, VCI');
  const [startDate, setStartDate] = useState('2020-01-01');
  const [endDate, setEndDate] = useState('2022-12-31');
  const [enableOos, setEnableOos] = useState(false);
  const [oosStartDate, setOosStartDate] = useState('2023-01-01');
  const [oosEndDate, setOosEndDate] = useState('2023-12-31');
  const [initialCash, setInitialCash] = useState(100000000);
  const [benchmarkSymbol, setBenchmarkSymbol] = useState('VNINDEX');
  const [selectedFilenames, setSelectedFilenames] = useState<string[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const [isSweeping, setIsSweeping] = useState(false);
  const [activeSweepId, setActiveSweepId] = useState<string | null>(null);
  const [results, setResults] = useState<LabResult[]>([]);

  // Typed parameter selection state
  const [selectedSweepStrategyFilename, setSelectedSweepStrategyFilename] = useState<string>('');
  const [selectedTargetKey, setSelectedTargetKey] = useState<string>('');
  const [selectedParamKey, setSelectedParamKey] = useState<string>('length');
  const [sweepValues, setSweepValues] = useState('5, 10, 20');
  const [maxVariants, setMaxVariants] = useState(12);

  const [sweepResults, setSweepResults] = useState<SweepVariant[]>([]);
  const [history, setHistory] = useState<LabHistoryEntry[]>(loadHistory);
  const [hiddenSavedRunIds, setHiddenSavedRunIds] = useState<number[]>([]);
  const [error, setError] = useState<string | null>(null);

  const { data: strategies, isLoading: isLoadingStrategies } = useQuery({
    queryKey: ['strategies'],
    queryFn: getAvailableStrategies,
  });

  const { data: savedRuns } = useQuery({
    queryKey: ['strategy-lab-runs'],
    queryFn: () => listStrategyLabRuns(50),
  });

  // Base strategy for parameter sweep
  const activeSweepStrategy = useMemo(() => {
    if (!strategies || strategies.length === 0) return null;
    if (selectedSweepStrategyFilename) {
      const found = strategies.find(s => s.filename === selectedSweepStrategyFilename);
      if (found) return found;
    }
    if (selectedFilenames.length > 0) {
      const found = strategies.find(s => s.filename === selectedFilenames[0]);
      if (found) return found;
    }
    return strategies[0];
  }, [strategies, selectedSweepStrategyFilename, selectedFilenames]);

  // Extract typed parameters for the selected base strategy
  const typedParameterOptions = useMemo<TypedParameterDefinition[]>(() => {
    if (!activeSweepStrategy || !activeSweepStrategy.config) return [];
    const config = activeSweepStrategy.config;
    const list: TypedParameterDefinition[] = [];

    // Indicators
    const indicators = (config.indicators || []) as Array<Record<string, unknown>>;
    indicators.forEach((ind, idx) => {
      const indName = (ind.name as string) || `indicator_${idx}`;
      const indType = ((ind.type as string) || '').toLowerCase();

      if (['sma', 'ema', 'rsi', 'atr', 'cci', 'mfi', 'adx'].includes(indType)) {
        list.push({
          target_type: 'indicator',
          target_name: indName,
          parameter: 'length',
          label: `${indName} (${indType.toUpperCase()}) — Length`,
          type: 'int',
          current_value: ind.length as number | undefined ?? 14,
          path: `indicators[${idx}].length`,
        });
      } else if (indType === 'macd') {
        ['fast', 'slow', 'signal'].forEach((p) => {
          list.push({
            target_type: 'indicator',
            target_name: indName,
            parameter: p,
            label: `${indName} (MACD) — ${p.toUpperCase()}`,
            type: 'int',
            current_value: ind[p] as number | undefined ?? (p === 'fast' ? 12 : p === 'slow' ? 26 : 9),
            path: `indicators[${idx}].${p}`,
          });
        });
      } else if (indType === 'bbands') {
        list.push({
          target_type: 'indicator',
          target_name: indName,
          parameter: 'length',
          label: `${indName} (BBANDS) — Length`,
          type: 'int',
          current_value: ind.length as number | undefined ?? 20,
          path: `indicators[${idx}].length`,
        });
        list.push({
          target_type: 'indicator',
          target_name: indName,
          parameter: 'std',
          label: `${indName} (BBANDS) — Std Dev`,
          type: 'float',
          current_value: ind.std as number | undefined ?? 2.0,
          path: `indicators[${idx}].std`,
        });
      } else if (indType === 'stoch') {
        ['k', 'd', 'smooth_k'].forEach((p) => {
          list.push({
            target_type: 'indicator',
            target_name: indName,
            parameter: p,
            label: `${indName} (STOCH) — ${p.toUpperCase()}`,
            type: 'int',
            current_value: ind[p] as number | undefined ?? (p === 'k' ? 14 : 3),
            path: `indicators[${idx}].${p}`,
          });
        });
      } else if (indType === 'kc') {
        list.push({
          target_type: 'indicator',
          target_name: indName,
          parameter: 'length',
          label: `${indName} (KC) — Length`,
          type: 'int',
          current_value: ind.length as number | undefined ?? 20,
          path: `indicators[${idx}].length`,
        });
        list.push({
          target_type: 'indicator',
          target_name: indName,
          parameter: 'scalar',
          label: `${indName} (KC) — Multiplier`,
          type: 'float',
          current_value: ind.scalar as number | undefined ?? 2.0,
          path: `indicators[${idx}].scalar`,
        });
      } else if (indType === 'psar') {
        ['af0', 'af', 'max_af'].forEach((p) => {
          list.push({
            target_type: 'indicator',
            target_name: indName,
            parameter: p,
            label: `${indName} (PSAR) — ${p}`,
            type: 'float',
            current_value: ind[p] as number | undefined ?? (p === 'max_af' ? 0.2 : 0.02),
            path: `indicators[${idx}].${p}`,
          });
        });
      } else if (indType === 'supertrend') {
        list.push({
          target_type: 'indicator',
          target_name: indName,
          parameter: 'length',
          label: `${indName} (SUPERTREND) — Length`,
          type: 'int',
          current_value: ind.length as number | undefined ?? 10,
          path: `indicators[${idx}].length`,
        });
        list.push({
          target_type: 'indicator',
          target_name: indName,
          parameter: 'multiplier',
          label: `${indName} (SUPERTREND) — Multiplier`,
          type: 'float',
          current_value: ind.multiplier as number | undefined ?? 3.0,
          path: `indicators[${idx}].multiplier`,
        });
      } else if (indType === 'ichimoku') {
        ['tenkan', 'kijun', 'senkou'].forEach((p) => {
          list.push({
            target_type: 'indicator',
            target_name: indName,
            parameter: p,
            label: `${indName} (ICHIMOKU) — ${p.toUpperCase()}`,
            type: 'int',
            current_value: ind[p] as number | undefined ?? (p === 'tenkan' ? 9 : p === 'kijun' ? 26 : 52),
            path: `indicators[${idx}].${p}`,
          });
        });
      } else if (indType === 'relative_strength') {
        list.push({
          target_type: 'indicator',
          target_name: indName,
          parameter: 'length',
          label: `${indName} (RS) — Length`,
          type: 'int',
          current_value: ind.length as number | undefined ?? 55,
          path: `indicators[${idx}].length`,
        });
      }
    });

    // Position Sizing
    list.push({
      target_type: 'position_sizing',
      target_name: 'position_sizing',
      parameter: 'quantity',
      label: 'Position Sizing — Fixed Lot Quantity',
      type: 'int',
      current_value: (config.position_sizing as Record<string, unknown>)?.quantity as number | undefined ?? 100,
      path: 'position_sizing.quantity',
    });
    list.push({
      target_type: 'position_sizing',
      target_name: 'position_sizing',
      parameter: 'percent',
      label: 'Position Sizing — Equity %',
      type: 'float',
      current_value: (config.position_sizing as Record<string, unknown>)?.percent as number | undefined ?? 10.0,
      path: 'position_sizing.percent',
    });

    // Risk Management
    list.push({
      target_type: 'risk_management',
      target_name: 'risk_management',
      parameter: 'stop_loss_pct',
      label: 'Risk Management — Stop Loss %',
      type: 'float',
      current_value: (config.risk_management as Record<string, unknown>)?.stop_loss_pct as number | undefined,
      path: 'risk_management.stop_loss_pct',
    });
    list.push({
      target_type: 'risk_management',
      target_name: 'risk_management',
      parameter: 'take_profit_pct',
      label: 'Risk Management — Take Profit %',
      type: 'float',
      current_value: (config.risk_management as Record<string, unknown>)?.take_profit_pct as number | undefined,
      path: 'risk_management.take_profit_pct',
    });

    return list;
  }, [activeSweepStrategy]);

  // Unique component targets in active strategy
  const availableTargets = useMemo(() => {
    const targets = new Set<string>();
    typedParameterOptions.forEach(opt => targets.add(opt.target_name));
    return Array.from(targets);
  }, [typedParameterOptions]);

  // Active target
  const currentTargetName = selectedTargetKey || availableTargets[0] || '';

  // Available parameters for the selected target
  const availableParamsForTarget = useMemo(() => {
    return typedParameterOptions.filter(opt => opt.target_name === currentTargetName);
  }, [typedParameterOptions, currentTargetName]);

  // Active parameter
  const currentParamDef = useMemo(() => {
    return (
      availableParamsForTarget.find(opt => opt.parameter === selectedParamKey) ||
      availableParamsForTarget[0] ||
      typedParameterOptions[0]
    );
  }, [availableParamsForTarget, selectedParamKey, typedParameterOptions]);

  const toggleStrategy = (filename: string) => {
    setSelectedFilenames((current) => (
      current.includes(filename)
        ? current.filter(item => item !== filename)
        : [...current, filename]
    ));
  };

  const validateDates = () => {
    if (!startDate || !endDate) {
      return 'Start date and end date are required.';
    }
    if (startDate >= endDate) {
      return `In-sample start date (${startDate}) must be before end date (${endDate}).`;
    }
    if (enableOos) {
      if (!oosStartDate || !oosEndDate) {
        return 'Out-of-sample start and end dates are required when OOS is enabled.';
      }
      if (oosStartDate >= oosEndDate) {
        return `Out-of-sample start date (${oosStartDate}) must be before end date (${oosEndDate}).`;
      }
      if (!(endDate <= oosStartDate || oosEndDate <= startDate)) {
        return `In-sample [${startDate} to ${endDate}] and Out-of-sample [${oosStartDate} to ${oosEndDate}] periods must not overlap.`;
      }
    }
    return null;
  };

  const handleRun = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!strategies) return;

    const dateErr = validateDates();
    if (dateErr) {
      setError(dateErr);
      return;
    }

    const selected = strategies.filter(strategy => selectedFilenames.includes(strategy.filename));
    if (selected.length === 0) {
      setError('Select at least one strategy.');
      return;
    }

    const symbols = symbolsInput.split(',').map(item => item.trim().toUpperCase()).filter(Boolean);
    if (symbols.length === 0) {
      setError('Enter at least one symbol.');
      return;
    }

    setIsRunning(true);
    setError(null);
    try {
      const nextResults: LabResult[] = [];
      for (const strategy of selected) {
        // In-sample backtest
        const isRequest: BacktestRequest = {
          start_date: startDate,
          end_date: endDate,
          initial_cash: initialCash,
          benchmark_symbol: benchmarkSymbol.trim().toUpperCase() || undefined,
          strategy: strategy.config,
        };
        if (symbols.length > 1) {
          isRequest.symbols = symbols;
        } else {
          isRequest.symbol = symbols[0];
        }
        const isResponse = await runBacktest(isRequest);

        // Optional Out-of-Sample backtest
        let oosResponse: BacktestResponse | null = null;
        if (enableOos) {
          const oosRequest: BacktestRequest = {
            start_date: oosStartDate,
            end_date: oosEndDate,
            initial_cash: initialCash,
            benchmark_symbol: benchmarkSymbol.trim().toUpperCase() || undefined,
            strategy: strategy.config,
          };
          if (symbols.length > 1) {
            oosRequest.symbols = symbols;
          } else {
            oosRequest.symbol = symbols[0];
          }
          oosResponse = await runBacktest(oosRequest);
        }

        nextResults.push({
          filename: strategy.filename,
          name: strategy.name,
          response: isResponse,
          oosResponse,
        });
      }
      setResults(nextResults);
      addHistory({
        type: 'comparison',
        label: `${selected.length} strategy comparison`,
        results: nextResults,
      }, {
        symbols,
        start_date: startDate,
        end_date: endDate,
        oos_start_date: enableOos ? oosStartDate : undefined,
        oos_end_date: enableOos ? oosEndDate : undefined,
        initial_cash: initialCash,
        benchmark_symbol: benchmarkSymbol,
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Strategy comparison failed.');
    } finally {
      setIsRunning(false);
    }
  };

  const handleSweep = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!strategies || !activeSweepStrategy) {
      setError('Select a base strategy.');
      return;
    }

    const dateErr = validateDates();
    if (dateErr) {
      setError(dateErr);
      return;
    }

    const symbols = symbolsInput.split(',').map(item => item.trim().toUpperCase()).filter(Boolean);
    const values = sweepValues
      .split(',')
      .map(item => parseSweepValue(item.trim()))
      .filter(item => item !== '');

    if (symbols.length === 0 || values.length === 0) {
      setError('Enter symbols and at least one sweep value.');
      return;
    }

    if (!currentParamDef) {
      setError('Please select a valid parameter to sweep.');
      return;
    }

    const sweepId = generateSweepId();
    setActiveSweepId(sweepId);
    setIsSweeping(true);
    setError(null);

    const sweepParam: SweepParameter = {
      target_type: currentParamDef.target_type,
      target_name: currentParamDef.target_name,
      parameter: currentParamDef.parameter,
      path: currentParamDef.path,
      values,
    };

    try {
      const request = {
        sweep_id: sweepId,
        start_date: startDate,
        end_date: endDate,
        oos_start_date: enableOos ? oosStartDate : undefined,
        oos_end_date: enableOos ? oosEndDate : undefined,
        initial_cash: initialCash,
        benchmark_symbol: benchmarkSymbol.trim().toUpperCase() || undefined,
        strategy: activeSweepStrategy.config,
        sweep: [sweepParam],
        max_variants: Math.min(Math.max(Number(maxVariants) || 12, 1), 50),
      };

      const response = await runParameterSweep(
        symbols.length > 1 ? { ...request, symbols } : { ...request, symbol: symbols[0] }
      );

      if (response.status === 'failed') {
        setError(response.message || 'Parameter sweep failed.');
        setSweepResults([]);
      } else if (response.status === 'cancelled') {
        setError('Parameter sweep was cancelled.');
        setSweepResults(response.variants || []);
      } else {
        setSweepResults(response.variants);
        addHistory({
          type: 'sweep',
          label: `${activeSweepStrategy.name} sweep: ${currentParamDef.label}`,
          sweepResults: response.variants,
        }, {
          symbols,
          start_date: startDate,
          end_date: endDate,
          oos_start_date: enableOos ? oosStartDate : undefined,
          oos_end_date: enableOos ? oosEndDate : undefined,
          initial_cash: initialCash,
          benchmark_symbol: benchmarkSymbol,
          sweep_target: currentParamDef.target_name,
          sweep_param: currentParamDef.parameter,
          sweep_values: values,
        });
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Parameter sweep failed.');
    } finally {
      setIsSweeping(false);
      setActiveSweepId(null);
    }
  };

  const handleCancelSweep = async () => {
    if (!activeSweepId) return;
    try {
      await cancelParameterSweep(activeSweepId);
    } catch {
      // Ignore cancellation error
    }
  };

  const selectAll = () => {
    setSelectedFilenames((strategies || []).map(strategy => strategy.filename));
  };

  const clearSelection = () => {
    setSelectedFilenames([]);
  };

  const addHistory = (
    entry: Omit<LabHistoryEntry, 'id' | 'createdAt'>,
    requestConfig: Record<string, unknown> = {}
  ) => {
    setHistory((current) => {
      const next = [
        {
          ...entry,
          id: `${Date.now()}-${Math.random().toString(16).slice(2)}`,
          createdAt: new Date().toISOString(),
          requestConfig,
        },
        ...current,
      ].slice(0, 20);
      saveHistory(next);
      return next;
    });
    void saveStrategyLabRun({
      run_type: entry.type,
      label: entry.label,
      request_config: requestConfig,
      result_payload: {
        results: entry.results,
        sweepResults: entry.sweepResults,
      },
      metrics: {},
    });
  };

  const restoreHistory = (entry: LabHistoryEntry) => {
    if (entry.results) setResults(entry.results);
    if (entry.sweepResults) setSweepResults(entry.sweepResults);
    if (entry.requestConfig) {
      if (entry.requestConfig.start_date) setStartDate(entry.requestConfig.start_date as string);
      if (entry.requestConfig.end_date) setEndDate(entry.requestConfig.end_date as string);
      if (entry.requestConfig.oos_start_date && entry.requestConfig.oos_end_date) {
        setEnableOos(true);
        setOosStartDate(entry.requestConfig.oos_start_date as string);
        setOosEndDate(entry.requestConfig.oos_end_date as string);
      }
      if (entry.requestConfig.symbols && Array.isArray(entry.requestConfig.symbols)) {
        setSymbolsInput(entry.requestConfig.symbols.join(', '));
      }
    }
    setError(null);
  };

  const clearHistory = () => {
    setHistory([]);
    saveHistory([]);
    setHiddenSavedRunIds((savedRuns || []).map(run => run.id));
    void Promise.all((savedRuns || []).map(run => deleteStrategyLabRun(run.id)));
  };

  const getTrades = (response: BacktestResponse) =>
    response.summary?.total_trades ?? response.analytics?.total_trades ?? 0;
  const getNetPnl = (response: BacktestResponse) =>
    response.summary?.total_net_pnl ?? response.analytics?.total_net_pnl ?? 0;
  const getExpectancy = (response: BacktestResponse) => response.analytics?.expectancy ?? null;

  // Minimum 5 trades required for ranking eligibility (PRO-STRAT-05)
  const isRankingEligible = (response: BacktestResponse) =>
    response.status === 'succeeded' &&
    getTrades(response) >= 5 &&
    response.analytics?.metrics?.total_net_pnl?.status === 'valid';

  const formatMoney = (value?: number | null) =>
    value != null
      ? value.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })
      : '0.00';
  const formatPercent = (value?: number) => (value != null ? `${(value * 100).toFixed(2)}%` : '0.00%');

  const parseSweepValue = (value: string) => {
    if (value === '') return '';
    if (value.toLowerCase() === 'true') return true;
    if (value.toLowerCase() === 'false') return false;
    const numeric = Number(value);
    return Number.isNaN(numeric) ? value : numeric;
  };

  const getRobustnessBadgeStyle = (badge?: string) => {
    switch (badge) {
      case 'Robust':
        return { background: 'rgba(0, 230, 118, 0.15)', color: 'var(--color-buy)', border: '1px solid rgba(0, 230, 118, 0.4)' };
      case 'Overfitted':
        return { background: 'rgba(255, 23, 68, 0.15)', color: 'var(--color-sell)', border: '1px solid rgba(255, 23, 68, 0.4)' };
      case 'Low Sample':
        return { background: 'rgba(255, 145, 0, 0.15)', color: '#FF9100', border: '1px solid rgba(255, 145, 0, 0.4)' };
      case 'Degraded':
        return { background: 'rgba(255, 209, 102, 0.15)', color: '#FFD166', border: '1px solid rgba(255, 209, 102, 0.4)' };
      default:
        return { background: 'rgba(255, 255, 255, 0.05)', color: 'var(--text-muted)', border: '1px solid rgba(255, 255, 255, 0.1)' };
    }
  };

  const sortedResults = [...results].sort((left, right) => {
    const eligibility = Number(isRankingEligible(right.response)) - Number(isRankingEligible(left.response));
    return eligibility || getNetPnl(right.response) - getNetPnl(left.response);
  });

  const bestFilename = sortedResults.find(item => isRankingEligible(item.response))?.filename;

  const savedHistory = (savedRuns || [])
    .filter(run => !hiddenSavedRunIds.includes(run.id))
    .map((run: StrategyLabRun): LabHistoryEntry => ({
      id: `server-${run.id}`,
      type: run.run_type,
      createdAt: run.created_at,
      label: run.label,
      requestConfig: run.request_config,
      results: run.result_payload.results as LabResult[] | undefined,
      sweepResults: run.result_payload.sweepResults as SweepVariant[] | undefined,
    }));
  const displayHistory = [...history, ...savedHistory];

  return (
    <div className="animate-fade-in" style={{ maxWidth: '1280px', margin: '0 auto', paddingBottom: '40px' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.5rem' }}>
        <div>
          <h2
            style={{
              margin: 0,
              fontSize: '28px',
              background: 'linear-gradient(90deg, #fff, #8B949E)',
              WebkitBackgroundClip: 'text',
              WebkitTextFillColor: 'transparent',
            }}
          >
            Strategy Lab
          </h2>
          <p style={{ margin: '4px 0 0', color: 'var(--text-muted)', fontSize: '13px' }}>
            Declarative strategy research, bounded parameter sweeps & out-of-sample robustness validation.
          </p>
        </div>
      </div>

      {/* Strategy Selection Catalog (Rendered First for Clean Access) */}
      <div className="glass-panel" style={{ padding: '20px', marginBottom: '24px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
          <div>
            <h3 style={{ margin: 0, fontSize: '16px' }}>Strategies</h3>
            <span style={{ color: 'var(--text-muted)', fontSize: '12px' }}>
              Select strategies to compare or pick a base strategy for parameter tuning.
            </span>
          </div>
          <div style={{ display: 'flex', gap: '8px' }}>
            <button type="button" onClick={selectAll} style={{ padding: '6px 10px', fontSize: '12px' }}>
              Select All
            </button>
            <button type="button" onClick={clearSelection} style={{ padding: '6px 10px', fontSize: '12px' }}>
              Clear
            </button>
          </div>
        </div>

        {strategies && strategies.length > 0 ? (
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(240px, 1fr))', gap: '12px' }}>
            {strategies.map((strategy: AvailableStrategy) => {
              const isSelected = selectedFilenames.includes(strategy.filename);
              return (
                <label
                  key={strategy.filename}
                  className="glass-panel"
                  style={{
                    padding: '12px',
                    display: 'flex',
                    gap: '10px',
                    alignItems: 'flex-start',
                    cursor: 'pointer',
                    borderColor: isSelected ? 'var(--color-primary)' : 'var(--border-color)',
                  }}
                >
                  <input
                    type="checkbox"
                    checked={isSelected}
                    onChange={() => toggleStrategy(strategy.filename)}
                  />
                  <div style={{ flex: 1 }}>
                    <strong style={{ display: 'block', fontSize: '14px' }}>{strategy.name}</strong>
                    <div style={{ display: 'flex', gap: '6px', marginTop: '4px', alignItems: 'center' }}>
                      <span
                        style={{
                          fontSize: '10px',
                          padding: '1px 5px',
                          background: 'rgba(255,255,255,0.06)',
                          borderRadius: '3px',
                          color: 'var(--text-muted)',
                        }}
                      >
                        v{(strategy.config as Record<string, unknown>)?.version as string || '1.0'}
                      </span>
                      <span style={{ color: 'var(--text-muted)', fontSize: '11px' }}>{strategy.filename}</span>
                    </div>
                  </div>
                </label>
              );
            })}
          </div>
        ) : (
          <p style={{ color: 'var(--text-muted)', fontSize: '14px', margin: 0 }}>No strategies available.</p>
        )}
      </div>

      {error && (
        <div
          className="glass-panel"
          style={{ borderColor: 'var(--color-sell)', padding: '16px', marginBottom: '24px', background: 'rgba(255, 23, 68, 0.05)' }}
        >
          <p style={{ color: 'var(--color-sell)', margin: 0, fontSize: '14px' }}>{error}</p>
        </div>
      )}

      {/* Main Configuration Panel */}
      <div className="glass-panel" style={{ padding: '24px', marginBottom: '24px' }}>
        <form onSubmit={handleRun}>
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))',
              gap: '16px',
              marginBottom: '16px',
            }}
          >
            <div>
              <label htmlFor="lab-symbols" style={{ display: 'block', marginBottom: '6px', color: 'var(--text-muted)', fontSize: '13px' }}>
                Symbols
              </label>
              <input
                id="lab-symbols"
                type="text"
                value={symbolsInput}
                onChange={e => setSymbolsInput(e.target.value.toUpperCase())}
                style={{ width: '100%', padding: '10px' }}
                required
              />
            </div>
            <div>
              <label htmlFor="lab-cash" style={{ display: 'block', marginBottom: '6px', color: 'var(--text-muted)', fontSize: '13px' }}>
                Initial Cash (VND)
              </label>
              <input
                id="lab-cash"
                type="number"
                value={initialCash}
                onChange={e => setInitialCash(Number(e.target.value))}
                style={{ width: '100%', padding: '10px' }}
                required
              />
            </div>
            <div>
              <label htmlFor="lab-benchmark" style={{ display: 'block', marginBottom: '6px', color: 'var(--text-muted)', fontSize: '13px' }}>
                Benchmark Symbol
              </label>
              <input
                id="lab-benchmark"
                type="text"
                value={benchmarkSymbol}
                onChange={e => setBenchmarkSymbol(e.target.value.toUpperCase())}
                style={{ width: '100%', padding: '10px' }}
              />
            </div>
          </div>

          {/* Date Range Evaluation Grid: In-Sample & Out-of-Sample */}
          <div
            style={{
              padding: '16px',
              background: 'rgba(255, 255, 255, 0.02)',
              border: '1px solid var(--border-color)',
              borderRadius: '8px',
              marginBottom: '20px',
            }}
          >
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '12px' }}>
              <span style={{ fontWeight: 600, fontSize: '14px', color: 'var(--text-main)' }}>
                Evaluation Periods (In-Sample / Out-of-Sample Split)
              </span>
              <label style={{ display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer', fontSize: '13px', color: 'var(--text-muted)' }}>
                <input
                  id="lab-enable-oos"
                  type="checkbox"
                  checked={enableOos}
                  onChange={e => setEnableOos(e.target.checked)}
                />
                Enable Out-of-Sample (OOS) Validation
              </label>
            </div>

            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '16px' }}>
              <div>
                <label htmlFor="lab-start" style={{ display: 'block', marginBottom: '6px', color: 'var(--text-muted)', fontSize: '12px' }}>
                  In-Sample Start Date
                </label>
                <input
                  id="lab-start"
                  type="date"
                  value={startDate}
                  onChange={e => setStartDate(e.target.value)}
                  style={{ width: '100%', padding: '8px' }}
                  required
                />
              </div>
              <div>
                <label htmlFor="lab-end" style={{ display: 'block', marginBottom: '6px', color: 'var(--text-muted)', fontSize: '12px' }}>
                  In-Sample End Date
                </label>
                <input
                  id="lab-end"
                  type="date"
                  value={endDate}
                  onChange={e => setEndDate(e.target.value)}
                  style={{ width: '100%', padding: '8px' }}
                  required
                />
              </div>

              {enableOos && (
                <>
                  <div>
                    <label htmlFor="lab-oos-start" style={{ display: 'block', marginBottom: '6px', color: '#58A6FF', fontSize: '12px' }}>
                      Out-of-Sample (OOS) Start
                    </label>
                    <input
                      id="lab-oos-start"
                      type="date"
                      value={oosStartDate}
                      onChange={e => setOosStartDate(e.target.value)}
                      style={{ width: '100%', padding: '8px', borderColor: 'rgba(88, 166, 255, 0.4)' }}
                      required={enableOos}
                    />
                  </div>
                  <div>
                    <label htmlFor="lab-oos-end" style={{ display: 'block', marginBottom: '6px', color: '#58A6FF', fontSize: '12px' }}>
                      Out-of-Sample (OOS) End
                    </label>
                    <input
                      id="lab-oos-end"
                      type="date"
                      value={oosEndDate}
                      onChange={e => setOosEndDate(e.target.value)}
                      style={{ width: '100%', padding: '8px', borderColor: 'rgba(88, 166, 255, 0.4)' }}
                      required={enableOos}
                    />
                  </div>
                </>
              )}
            </div>
          </div>

          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '12px' }}>
            <button
              id="btn-compare-strategies"
              type="submit"
              className="btn-primary"
              style={{ height: '42px', minWidth: '180px' }}
              disabled={isRunning || isSweeping || isLoadingStrategies}
            >
              {isRunning ? 'Running Comparison...' : 'Compare Strategies'}
            </button>
          </div>
        </form>
      </div>

      {/* Typed Parameter Sweep Panel (PRO-STRAT-01, PRO-STRAT-04) */}
      <div className="glass-panel" style={{ padding: '24px', marginBottom: '24px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
          <div>
            <h3 style={{ margin: 0, fontSize: '16px' }}>Parameter Sweep & Robustness Tuning</h3>
            <span style={{ color: 'var(--text-muted)', fontSize: '12px' }}>
              Test multiple parameter variations deterministically with In-Sample and Out-of-Sample validation.
            </span>
          </div>
        </div>

        <form onSubmit={handleSweep}>
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))',
              gap: '16px',
              marginBottom: '16px',
            }}
          >
            {/* Strategy base selector */}
            <div>
              <label htmlFor="sweep-base-strategy" style={{ display: 'block', marginBottom: '6px', color: 'var(--text-muted)', fontSize: '13px' }}>
                Base Strategy
              </label>
              <select
                id="sweep-base-strategy"
                value={activeSweepStrategy?.filename || ''}
                onChange={e => setSelectedSweepStrategyFilename(e.target.value)}
                style={{ width: '100%', padding: '9px', background: 'var(--bg-panel-solid)', color: 'var(--text-main)', border: '1px solid var(--border-color)', borderRadius: '4px' }}
              >
                {(strategies || []).map(s => (
                  <option key={s.filename} value={s.filename}>
                    {s.name}
                  </option>
                ))}
              </select>
            </div>

            {/* Target Component Dropdown (PRO-STRAT-01) */}
            <div>
              <label htmlFor="sweep-target" style={{ display: 'block', marginBottom: '6px', color: 'var(--text-muted)', fontSize: '13px' }}>
                Target Component
              </label>
              <select
                id="sweep-target"
                value={currentTargetName}
                onChange={e => {
                  setSelectedTargetKey(e.target.value);
                  const firstParam = typedParameterOptions.find(opt => opt.target_name === e.target.value);
                  if (firstParam) setSelectedParamKey(firstParam.parameter);
                }}
                style={{ width: '100%', padding: '9px', background: 'var(--bg-panel-solid)', color: 'var(--text-main)', border: '1px solid var(--border-color)', borderRadius: '4px' }}
              >
                {availableTargets.map(target => (
                  <option key={target} value={target}>
                    {target}
                  </option>
                ))}
              </select>
            </div>

            {/* Parameter Dropdown (PRO-STRAT-01) */}
            <div>
              <label htmlFor="sweep-param" style={{ display: 'block', marginBottom: '6px', color: 'var(--text-muted)', fontSize: '13px' }}>
                Parameter
              </label>
              <select
                id="sweep-param"
                value={currentParamDef?.parameter || selectedParamKey}
                onChange={e => setSelectedParamKey(e.target.value)}
                style={{ width: '100%', padding: '9px', background: 'var(--bg-panel-solid)', color: 'var(--text-main)', border: '1px solid var(--border-color)', borderRadius: '4px' }}
              >
                {availableParamsForTarget.map(opt => (
                  <option key={opt.parameter} value={opt.parameter}>
                    {opt.label.split('—')[1]?.trim() || opt.parameter} ({opt.type})
                  </option>
                ))}
              </select>
            </div>

            {/* Sweep Values Input */}
            <div>
              <label htmlFor="sweep-values" style={{ display: 'block', marginBottom: '6px', color: 'var(--text-muted)', fontSize: '13px' }}>
                Sweep Values
              </label>
              <input
                id="sweep-values"
                type="text"
                value={sweepValues}
                onChange={e => setSweepValues(e.target.value)}
                placeholder="e.g. 5, 10, 20, 50"
                style={{ width: '100%', padding: '8px' }}
                required
              />
            </div>

            {/* Max Variants Bound (PRO-STRAT-04) */}
            <div>
              <label htmlFor="sweep-max" style={{ display: 'block', marginBottom: '6px', color: 'var(--text-muted)', fontSize: '13px' }}>
                Max Variants (1–50)
              </label>
              <input
                id="sweep-max"
                type="number"
                min={1}
                max={50}
                value={maxVariants}
                onChange={e => setMaxVariants(Number(e.target.value))}
                style={{ width: '100%', padding: '8px' }}
                required
              />
            </div>
          </div>

          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: '12px' }}>
            <div style={{ color: 'var(--text-muted)', fontSize: '12px' }}>
              Active Target: <span style={{ color: 'var(--text-main)', fontWeight: 500 }}>{currentParamDef?.label || 'None'}</span>
              {' '}(Default: {String(currentParamDef?.current_value ?? 'N/A')})
            </div>

            <div style={{ display: 'flex', gap: '10px' }}>
              {isSweeping && (
                <button
                  id="btn-cancel-sweep"
                  type="button"
                  onClick={handleCancelSweep}
                  style={{
                    height: '40px',
                    padding: '0 16px',
                    background: 'rgba(255, 23, 68, 0.2)',
                    color: 'var(--color-sell)',
                    border: '1px solid var(--color-sell)',
                    borderRadius: '4px',
                    cursor: 'pointer',
                    fontWeight: 600,
                  }}
                >
                  Cancel Sweep
                </button>
              )}
              <button
                id="btn-run-sweep"
                type="submit"
                className="btn-primary"
                style={{ height: '40px', minWidth: '140px' }}
                disabled={isSweeping || isRunning || isLoadingStrategies}
              >
                {isSweeping ? 'Sweeping...' : 'Run Sweep'}
              </button>
            </div>
          </div>
        </form>
      </div>

      {/* Comparison Results Section */}
      {sortedResults.length > 0 && (
        <div className="glass-panel" style={{ padding: '20px', marginBottom: '24px' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
            <h3 style={{ margin: 0, fontSize: '16px' }}>Comparison</h3>
            <span style={{ color: 'var(--text-muted)', fontSize: '12px' }}>
              Minimum 5 closed trades required for winning ranking eligibility.
            </span>
          </div>

          <div style={{ overflowX: 'auto' }}>
            <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '13px' }}>
              <thead>
                <tr style={{ borderBottom: '1px solid rgba(255,255,255,0.1)', color: 'var(--text-muted)', textAlign: 'left' }}>
                  <th style={{ padding: '10px 8px', fontWeight: 500 }}>Strategy</th>
                  <th style={{ padding: '10px 8px', fontWeight: 500 }}>Status</th>
                  <th style={{ padding: '10px 8px', fontWeight: 500, textAlign: 'right' }}>IS Trades</th>
                  <th style={{ padding: '10px 8px', fontWeight: 500, textAlign: 'right' }}>IS Win %</th>
                  <th style={{ padding: '10px 8px', fontWeight: 500, textAlign: 'right' }}>IS Net PnL</th>
                  <th style={{ padding: '10px 8px', fontWeight: 500, textAlign: 'right' }}>IS Profit Factor</th>
                  {enableOos && (
                    <>
                      <th style={{ padding: '10px 8px', fontWeight: 500, textAlign: 'right', color: '#58A6FF' }}>OOS Trades</th>
                      <th style={{ padding: '10px 8px', fontWeight: 500, textAlign: 'right', color: '#58A6FF' }}>OOS Net PnL</th>
                      <th style={{ padding: '10px 8px', fontWeight: 500, textAlign: 'right', color: '#58A6FF' }}>OOS PF</th>
                    </>
                  )}
                  <th style={{ padding: '10px 8px', fontWeight: 500, textAlign: 'right' }}>Expectancy</th>
                </tr>
              </thead>
              <tbody>
                {sortedResults.map((item) => {
                  const netPnl = getNetPnl(item.response);
                  const isEligible = isRankingEligible(item.response);
                  const isBest = item.filename === bestFilename;
                  const tradeCount = getTrades(item.response);

                  const oosTrades = item.oosResponse ? getTrades(item.oosResponse) : null;
                  const oosNetPnl = item.oosResponse ? getNetPnl(item.oosResponse) : null;
                  const oosPf = item.oosResponse?.analytics?.profit_factor;

                  return (
                    <tr key={item.filename} style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}>
                      <td style={{ padding: '12px 8px', fontWeight: 600 }}>
                        {item.name}
                        {isBest && (
                          <span
                            style={{
                              marginLeft: '8px',
                              color: 'var(--color-buy)',
                              fontSize: '11px',
                              background: 'rgba(0, 230, 118, 0.12)',
                              padding: '2px 6px',
                              borderRadius: '3px',
                            }}
                          >
                            Best eligible
                          </span>
                        )}
                        {!isEligible && (
                          <span
                            style={{
                              marginLeft: '8px',
                              color: tradeCount < 5 ? '#FF9100' : 'var(--text-muted)',
                              fontSize: '11px',
                              background: 'rgba(255, 255, 255, 0.05)',
                              padding: '2px 6px',
                              borderRadius: '3px',
                            }}
                          >
                            {tradeCount < 5 ? 'Not rankable (Low sample)' : 'Not rankable'}
                          </span>
                        )}
                      </td>
                      <td
                        style={{
                          padding: '12px 8px',
                          textTransform: 'uppercase',
                          color: item.response.status === 'failed' ? 'var(--color-sell)' : 'var(--text-main)',
                          fontSize: '12px',
                        }}
                      >
                        {item.response.status || 'succeeded'}
                      </td>
                      <td style={{ padding: '12px 8px', textAlign: 'right' }}>{tradeCount}</td>
                      <td style={{ padding: '12px 8px', textAlign: 'right' }}>
                        <MetricResultValue metric={item.response.analytics?.metrics?.win_rate} format={formatPercent} />
                      </td>
                      <td
                        style={{
                          padding: '12px 8px',
                          textAlign: 'right',
                          fontWeight: 600,
                          color: netPnl >= 0 ? 'var(--color-buy)' : 'var(--color-sell)',
                        }}
                      >
                        {formatMoney(netPnl)}
                      </td>
                      <td style={{ padding: '12px 8px', textAlign: 'right' }}>
                        <MetricResultValue metric={item.response.analytics?.metrics?.profit_factor} />
                      </td>
                      {enableOos && (
                        <>
                          <td style={{ padding: '12px 8px', textAlign: 'right', color: '#58A6FF' }}>
                            {oosTrades != null ? oosTrades : '—'}
                          </td>
                          <td
                            style={{
                              padding: '12px 8px',
                              textAlign: 'right',
                              fontWeight: 600,
                              color: oosNetPnl != null ? (oosNetPnl >= 0 ? 'var(--color-buy)' : 'var(--color-sell)') : 'var(--text-muted)',
                            }}
                          >
                            {oosNetPnl != null ? formatMoney(oosNetPnl) : '—'}
                          </td>
                          <td style={{ padding: '12px 8px', textAlign: 'right', color: '#58A6FF' }}>
                            {oosPf != null ? oosPf.toFixed(2) : '—'}
                          </td>
                        </>
                      )}
                      <td style={{ padding: '12px 8px', textAlign: 'right' }}>{formatMoney(getExpectancy(item.response))}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* Sweep Results Section with Robustness Metrics (PRO-STRAT-05, PRO-STRAT-06) */}
      {sweepResults.length > 0 && (
        <div className="glass-panel" style={{ padding: '20px', marginBottom: '24px' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
            <div>
              <h3 style={{ margin: 0, fontSize: '16px' }}>Sweep Results</h3>
              <span style={{ color: 'var(--text-muted)', fontSize: '12px' }}>
                Sorted by ranking eligibility, robustness score, and In-Sample Net PnL.
              </span>
            </div>
            <div style={{ display: 'flex', gap: '12px', fontSize: '12px', color: 'var(--text-muted)' }}>
              <span>Assumptions: 100-share lot, T+2, 0.15% fee, 0.10% tax</span>
            </div>
          </div>

          <div style={{ overflowX: 'auto' }}>
            <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '13px' }}>
              <thead>
                <tr style={{ borderBottom: '1px solid rgba(255,255,255,0.1)', color: 'var(--text-muted)', textAlign: 'left' }}>
                  <th style={{ padding: '10px 8px', fontWeight: 500 }}>Variant</th>
                  <th style={{ padding: '10px 8px', fontWeight: 500 }}>Robustness</th>
                  <th style={{ padding: '10px 8px', fontWeight: 500, textAlign: 'right' }}>IS Trades</th>
                  <th style={{ padding: '10px 8px', fontWeight: 500, textAlign: 'right' }}>IS Win %</th>
                  <th style={{ padding: '10px 8px', fontWeight: 500, textAlign: 'right' }}>IS Net PnL</th>
                  <th style={{ padding: '10px 8px', fontWeight: 500, textAlign: 'right' }}>IS PF</th>
                  {sweepResults.some(r => r.metrics.out_of_sample) && (
                    <>
                      <th style={{ padding: '10px 8px', fontWeight: 500, textAlign: 'right', color: '#58A6FF' }}>OOS Trades</th>
                      <th style={{ padding: '10px 8px', fontWeight: 500, textAlign: 'right', color: '#58A6FF' }}>OOS Net PnL</th>
                      <th style={{ padding: '10px 8px', fontWeight: 500, textAlign: 'right', color: '#58A6FF' }}>OOS PF</th>
                      <th style={{ padding: '10px 8px', fontWeight: 500, textAlign: 'right', color: '#58A6FF' }}>Stability</th>
                    </>
                  )}
                  <th style={{ padding: '10px 8px', fontWeight: 500, textAlign: 'right' }}>Expectancy</th>
                </tr>
              </thead>
              <tbody>
                {sweepResults.map((item) => {
                  const badge = item.metrics.robustness?.badge || item.metrics.robustness_badge;
                  const score = item.metrics.robustness?.score ?? item.metrics.robustness_score;
                  const isEligible = item.metrics.ranking_eligible;
                  const oos = item.metrics.out_of_sample;
                  const stability = item.metrics.robustness?.stability_ratio;

                  return (
                    <tr key={item.label} style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}>
                      <td style={{ padding: '12px 8px', fontWeight: 600 }}>
                        {item.label}
                        {!isEligible && (
                          <span
                            style={{
                              marginLeft: '8px',
                              color: item.metrics.total_trades < 5 ? '#FF9100' : 'var(--text-muted)',
                              fontSize: '11px',
                              background: 'rgba(255, 255, 255, 0.05)',
                              padding: '2px 6px',
                              borderRadius: '3px',
                            }}
                          >
                            {item.metrics.total_trades < 5 ? 'Not rankable (Low sample)' : 'Not rankable'}
                          </span>
                        )}
                      </td>
                      <td style={{ padding: '12px 8px' }}>
                        {badge && (
                          <span
                            style={{
                              fontSize: '11px',
                              fontWeight: 600,
                              padding: '2px 8px',
                              borderRadius: '4px',
                              ...getRobustnessBadgeStyle(badge),
                            }}
                          >
                            {badge} {score != null && `(${score.toFixed(0)})`}
                          </span>
                        )}
                      </td>
                      <td style={{ padding: '12px 8px', textAlign: 'right' }}>{item.metrics.total_trades}</td>
                      <td style={{ padding: '12px 8px', textAlign: 'right' }}>
                        <MetricResultValue metric={item.metrics.metric_results?.win_rate} format={formatPercent} />
                      </td>
                      <td
                        style={{
                          padding: '12px 8px',
                          textAlign: 'right',
                          color: item.metrics.net_pnl >= 0 ? 'var(--color-buy)' : 'var(--color-sell)',
                          fontWeight: 600,
                        }}
                      >
                        {formatMoney(item.metrics.net_pnl)}
                      </td>
                      <td style={{ padding: '12px 8px', textAlign: 'right' }}>
                        <MetricResultValue metric={item.metrics.metric_results?.profit_factor} />
                      </td>
                      {sweepResults.some(r => r.metrics.out_of_sample) && (
                        <>
                          <td style={{ padding: '12px 8px', textAlign: 'right', color: '#58A6FF' }}>
                            {oos ? oos.total_trades : '—'}
                          </td>
                          <td
                            style={{
                              padding: '12px 8px',
                              textAlign: 'right',
                              fontWeight: 600,
                              color: oos ? (oos.net_pnl >= 0 ? 'var(--color-buy)' : 'var(--color-sell)') : 'var(--text-muted)',
                            }}
                          >
                            {oos ? formatMoney(oos.net_pnl) : '—'}
                          </td>
                          <td style={{ padding: '12px 8px', textAlign: 'right', color: '#58A6FF' }}>
                            {oos?.profit_factor != null ? oos.profit_factor.toFixed(2) : '—'}
                          </td>
                          <td style={{ padding: '12px 8px', textAlign: 'right', color: '#58A6FF' }}>
                            {stability != null ? `${(stability * 100).toFixed(0)}%` : '—'}
                          </td>
                        </>
                      )}
                      <td style={{ padding: '12px 8px', textAlign: 'right' }}>{formatMoney(item.metrics.expectancy)}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* Run History Section (PRO-STRAT-07) */}
      {displayHistory.length > 0 && (
        <div className="glass-panel" style={{ padding: '20px' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
            <h3 style={{ margin: 0, fontSize: '16px' }}>Run History</h3>
            <button type="button" onClick={clearHistory} style={{ padding: '6px 10px', fontSize: '12px' }}>
              Clear History
            </button>
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(260px, 1fr))', gap: '12px' }}>
            {displayHistory.map((entry) => (
              <button
                key={entry.id}
                type="button"
                onClick={() => restoreHistory(entry)}
                className="glass-panel"
                style={{ padding: '12px', textAlign: 'left', cursor: 'pointer' }}
              >
                <strong style={{ display: 'block', marginBottom: '4px', fontSize: '13px' }}>{entry.label}</strong>
                <div style={{ color: 'var(--text-muted)', fontSize: '12px', display: 'flex', justifyContent: 'space-between' }}>
                  <span>{entry.type.toUpperCase()}</span>
                  <span>{new Date(entry.createdAt).toLocaleString()}</span>
                </div>
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};
