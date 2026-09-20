# P3-PRICE-01 — Core Price Patterns, Technical Triggers, Regimes, and Causal Support/Resistance

## Outcome
Sumi provides a deterministic, causal, versioned Price signal library covering 10 candlestick patterns (`SIG-PAT-001..010`), 6 technical indicator triggers (`SIG-TECH-001..006`), 6 market regimes (`SIG-REG-001..006`), and causal support/resistance proximity (`SIG-SR-001`). All signals run without lookahead, support safe AST evaluation in strategy rules, and report detailed matching reasons.

## Context and problem
- Canonical source: `docs/research/SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` (Phase 3, `P3-PRICE-01`).
- Mathematical specification: `docs/research/SUMI_MASTER_FUNCTIONAL_TECHNICAL_SPEC_FINAL.md` sections D.1 (Patterns), D.2 (Technical Triggers), D.4 (Regimes), D.6 (Support/Resistance), and Appendix J (`TEST-CAUSAL-003`).
- Previously, Sumi only supported Volume signals (`volume.relative_volume`, `volume.spike`). Strategies and replay analysis lacked price pattern and regime awareness.

## In scope
1. **Candle Geometry & Causal ATR** (`backend/app/domain/signals/candle_features.py`):
   - Monotonic daily bar candle geometry calculations: `body`, `range`, `body_ratio`, `wick_ratios`, `close_location`, `midpoint`.
   - Causal True Range (TR) and Wilder's smoothing Average True Range (ATR14). Zero-range / flat-bar protections.
2. **Candlestick Patterns** (`backend/app/domain/signals/patterns.py`):
   - `SIG-PAT-001`: Bullish Engulfing (`pattern.bullish_engulfing`)
   - `SIG-PAT-002`: Bearish Engulfing (`pattern.bearish_engulfing`)
   - `SIG-PAT-003`: Hammer / Bullish Pinbar (`pattern.hammer`)
   - `SIG-PAT-004`: Shooting Star / Bearish Pinbar (`pattern.shooting_star`)
   - `SIG-PAT-005`: Morning Star (`pattern.morning_star`, gap non-mandatory)
   - `SIG-PAT-006`: Evening Star (`pattern.evening_star`)
   - `SIG-PAT-007`: Piercing Line & Dark Cloud Cover (`pattern.piercing_line`, `pattern.dark_cloud_cover`)
   - `SIG-PAT-008`: Tweezer Bottom & Top (`pattern.tweezer_bottom`, `pattern.tweezer_top`) with ATR tolerance
   - `SIG-PAT-009`: Inside Bar Breakout Up & Down (`pattern.inside_bar_breakout_up`, `pattern.inside_bar_breakout_down`)
   - `SIG-PAT-010`: Bullish Any & Bearish Any composite (`pattern.any_bullish`, `pattern.any_bearish`) with child pattern explanation in `reasons`
3. **Technical Triggers** (`backend/app/domain/signals/technical.py`):
   - `SIG-TECH-001`: MACD Signal Cross (`tech.macd_signal_cross`)
   - `SIG-TECH-002`: MACD Zero Cross (`tech.macd_zero_cross`)
   - `SIG-TECH-003`: RSI Level Cross (`tech.rsi_level_cross`)
   - `SIG-TECH-004`: EMA Cross (`tech.ema_cross`)
   - `SIG-TECH-005`: Swing Break (`tech.swing_break`, strictly using confirmed swing points)
   - `SIG-TECH-006`: Composite Technical Trigger (`tech.composite`, minimum confirmations)
4. **Market Regimes** (`backend/app/domain/signals/regimes.py`):
   - `SIG-REG-001`: Uptrend (`regime.uptrend`)
   - `SIG-REG-002`: Downtrend (`regime.downtrend`)
   - `SIG-REG-003`: Sideways (`regime.sideways`)
   - `SIG-REG-004`: Pullback / Correction (`regime.pullback`)
   - `SIG-REG-005`: Recovery (`regime.recovery`)
   - `SIG-REG-006`: New High / New Low (`regime.new_high`, `regime.new_low`, strictly excluding current bar $t$ per `TEST-CAUSAL-003`)
5. **Support / Resistance** (`backend/app/domain/signals/support_resistance.py`):
   - `SIG-SR-001`: Near Support / Near Resistance (`sr.near_support`, `sr.near_resistance`), reporting matched source level
6. **Registry & AST Binding**:
   - Register definitions and dispatch in `backend/app/domain/signals/registry.py`.
   - Wire AST aliases (`pattern__*`, `tech__*`, `regime__*`, `sr__*`) in `backend/app/domain/strategy/signal_binding.py`.

## Out of scope
- P3-VSA-02 (Volume Spread Analysis, Strong/Weak Demand, Kéo Xả, Đạp Kéo).
- Phase 5 Money Flow BB (`OHLCV_PROXY`) and Provider ports.
- Modifying SQLite database or migrations (zero DB changes required).

## Invariants
- **Zero Future Leakage**: No calculation may inspect bars at index > t.
- **Historical Reference Exclusion (`TEST-CAUSAL-003`)**: Comparative extrema (e.g. rolling high/low) strictly use window t-N : t, never including bar t.
- **Database Safety**: Never mutate or connect to `backend/sumi.db` in automated tests.
- **Fail-Closed Availability**: Unavailable or invalid signals must evaluate to null/unavailable in strategy AST evaluation, never silently resolving to True under negation.

## Milestones
1. **Milestone 1**: Implement `candle_features.py` and `patterns.py` with 10 candlestick patterns and unit tests.
2. **Milestone 2**: Implement `technical.py` with 6 technical cross and confirmation triggers.
3. **Milestone 3**: Implement `regimes.py` and `support_resistance.py` with causal new-high/low and SR proximity.
4. **Milestone 4**: Update `SignalRegistry` and `SignalBindingAdapter`; run focused test suite `test_price_signals.py`.
5. **Milestone 5**: Run full gates `verify-v2.ps1` and `run-comprehensive-uat.ps1`. Obtain independent review and seal.

## Acceptance mapping
| Acceptance ID | Implementation evidence | Test/UAT evidence |
| --- | --- | --- |
| `SIG-PAT-001..010` | `backend/app/domain/signals/patterns.py` | `test_price_signals.py::test_candlestick_patterns` |
| `SIG-TECH-001..006` | `backend/app/domain/signals/technical.py` | `test_price_signals.py::test_technical_triggers` |
| `SIG-REG-001..006` | `backend/app/domain/signals/regimes.py` | `test_price_signals.py::test_regimes` |
| `SIG-SR-001` | `backend/app/domain/signals/support_resistance.py` | `test_price_signals.py::test_support_resistance` |
| `TEST-CAUSAL-001` | Append future bar test across all signals | `test_price_signals.py::test_future_invariance` |
| `TEST-CAUSAL-003` | Current bar excluded from rolling reference | `test_price_signals.py::test_new_high_low_causal_exclusion` |
| `TEST-SIG-003` | Composite reasons list active child names | `test_price_signals.py::test_composite_explanations` |

## Verification commands
```powershell
# 1. Focused price signal unit tests
pytest backend/app/tests/test_price_signals.py backend/app/tests/test_signal_binding.py backend/app/tests/test_signals.py -v

# 2. Fast technical gate
.\scripts\verify-v2.ps1

# 3. Comprehensive browser UAT
.\scripts\run-comprehensive-uat.ps1
```

## Rollback and compatibility
If issues arise, new modules (`candle_features.py`, `patterns.py`, `technical.py`, `regimes.py`, `support_resistance.py`) can be removed, and `registry.py` and `signal_binding.py` reverted without touching database or existing replay functionality.

## Risks and mitigations
- **Risk**: Too many signals in one batch.
  - **Mitigation**: Pure mathematical functional design with modular files and standardized `SignalOutputPoint` outputs.
- **Risk**: Future lookahead in swing points or rolling windows.
  - **Mitigation**: Strictly use confirmed swing points (past offset k) and exclude current bar t from all historical window slices (`t-N : t`).

## Progress log
- 2026-09-17: ExecPlan created. Research confirmed against `SUMI_MASTER_FUNCTIONAL_TECHNICAL_SPEC_FINAL.md` and `SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md`.