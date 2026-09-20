# P2-MKT-02 Review & Verification Seal

## 1. Metadata
- **Task ID**: `P2-MKT-02`
- **Task Name**: Vietnam Market-Rule Profile
- **Review Date**: 2026-09-17
- **Status**: ACCEPTED
- **Working Root**: `e:\Workspace\sumi`
- **Authoritative ExecPlan**: `docs/exec-plans/P2_MKT_02_VIETNAM_MARKET_RULE_PROFILE.md`
- **Reference Spec**: `docs/research/SUMI_MASTER_FUNCTIONAL_TECHNICAL_SPEC_FINAL.md` (§19, §23, §24, Appendix H, Appendix J.1)

---

## 2. Deliverable Scope & Code Digest

### 2.1 Pure Market Domain Package (`backend/app/domain/market/`)
1. `backend/app/domain/market/__init__.py`: Public package exports for domain calendar, rules, settlement, and provider.
2. `backend/app/domain/market/calendar.py`: `TradingCalendar` providing:
   - Immutable holiday fixtures (`VIETNAM_HOLIDAYS_V1` covering 2020–2026+ statutory holidays including Lunar New Year Tet, Hung Kings, Victory/Labor Days, National Days).
   - Pure trading-session navigation: `is_trading_day`, `next_trading_day`, `previous_trading_day`, `add_trading_days`, `trading_days_between`.
   - Eliminates calendar-day `timedelta(days=N)` approximations (`TEST-BT-003`).
3. `backend/app/domain/market/rules.py`: `MarketRules` formalizing:
   - Exchange price bands: HOSE (±7.0%), HNX (±10.0%), UPCoM (±15.0%).
   - Tiered tick sizes per Circular 120/2020/TT-BTC: HOSE (<10k: 10 VND, 10k–50k: 50 VND, ≥50k: 100 VND); HNX & UPCoM: uniform 100 VND.
   - Statutory tick rounding: Ceiling rounds down, Floor rounds up (with IEEE-754 precision safeguards).
   - Board lot sizing: 100 shares standard board lot, floor rounding, odd lots reject/clip cleanly.
   - Locked limit detection (`TEST-BT-004`): Identifies locked ceiling (tím) and locked floor (sàn) bars (e.g. `open == high == low == close == limit_price`) and rejects impossible counter-liquidity fills.
4. `backend/app/domain/market/settlement.py`: `SettlementEngine` and `HoldingLot`:
   - Enforces cash-equity T+2 / T+1.5 afternoon session availability (`TEST-BT-002`).
   - Conservative 1D daily ambiguity modeling: Morning open exits blocked; afternoon close exits and T+3 full tradable day exits allowed.
   - FIFO lot allocation for partial sales.
5. `backend/app/domain/market/provider.py`: Named execution profiles (`vietnam_default_conservative`, `vietnam_standard_t2`, `vietnam_legacy_t3`, `VN_EQUITY_DAILY_V1`) and `MarketRuleProvider`.

### 2.2 Integration Modules
1. `backend/app/domain/backtest/execution.py`: Integrated `MarketRuleProvider` and `SettlementEngine`:
   - Tracks discrete `HoldingLot` per position.
   - Validates settlement trading sessions before queueing or filling exits.
   - Rejects locked-limit fills at Bar Open in conservative profiles.
   - Suppresses intraday Stop Loss on floor-locked bars (trapped liquidity).
   - Preserves phase-end unsellable positions as open holdings without synthetic liquidation (`TEST-BT-005`).
2. `backend/app/schemas/analytics_trust_schema.py`: Extended `ExecutionAssumptions` with `market_rule_profile`, `calendar_version`, `price_band_pct`, and `settlement_cycle`, preserving 100% backward compatibility.
3. `backend/app/services/backtest_service.py`: Wires profile resolution from configuration into kernel execution.

---

## 3. Independent Verification Swarm Findings

### 3.1 Adversarial Stress Testing (`challenger_mkt_1` & `challenger_mkt_2`)
- **Empirical Pricing Limits Trial**: Executed 1,500 continuous randomized trials of exchange price bands and tick sizes with zero boundary breaches.
- **Consecutive Floor-Locked Sequences**: Evaluated multi-bar consecutive floor-locked bars; confirmed that counter-liquidity sells were conservatively rejected, intraday SL fills suppressed, and positions properly retained as held/trapped.
- **Holiday Settlement Transitions**: Tested Lunar New Year (Tet 2024: 7 consecutive non-trading days) and weekend crossovers; confirmed settlement strictly counts trading sessions, not calendar days.

### 3.2 Regulatory & Requirements Review (`reviewer_mkt_1` & `reviewer_mkt_2`)
- Verified exact compliance with Circular 120/2020/TT-BTC and VSD Decision 109/QĐ-VSD.
- Verified all acceptance criteria:
  - `TEST-BT-002` (T+2 settlement): PASS
  - `TEST-BT-003` (Trading calendar & holiday navigation): PASS
  - `TEST-BT-004` (Locked limits & price bands): PASS
  - `TEST-BT-005` (Phase-end holding integrity): PASS

### 3.3 Forensic Audit (`auditor_mkt_1`)
- **Verdict**: CLEAN
- **Database Invariant**: Production database `backend/sumi.db` SHA256 matches baseline `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` (0 mutations).
- **Working Tree Invariant**: Exactly 159 staged deletions in `docs/` remain preserved intact in the git index.
- **Anti-Cheat Verification**: Confirmed zero hardcoded test outputs, zero facade patterns, and zero pre-populated outputs.

---

## 4. Verification Evidence & Test Execution

### 4.1 Dedicated Market Rules Test Suite
- **Command**: `.\.venv\Scripts\python.exe -m pytest app\tests\test_market_rules.py -v`
- **Result**: 26 passed, 0 failed in 0.08s.

### 4.2 Full Technical Gate (`scripts/verify-v2.ps1`)
- **Command**: `powershell -ExecutionPolicy Bypass -File .\scripts\verify-v2.ps1`
- **Result**: Exit code 0 (All passed).
  - Backend pytest: 253 passed in 15.09s (all 227 baseline + 26 market rules tests).
  - Alembic migrations: Upgrade to head succeeded.
  - Frontend lint: ESLint clean, 0 errors.
  - Frontend tests: 32 test files passed, 210 tests passed in 19.73s.
  - Frontend build: Vite production build succeeded in 1.16s.

### 4.3 Comprehensive Browser E2E UAT (`scripts/run-comprehensive-uat.ps1`)
- **Command**: `powershell -ExecutionPolicy Bypass -File .\scripts\run-comprehensive-uat.ps1`
- **Result**: Exit code 0 (31/31 passed, 100.0%, 0 failed).
  - Strategy Lab & Backtesting: `TC-STR-01` through `TC-STR-06` passed.
  - Practice Lab & Orderflow: `TC-LAB-01` through `TC-LAB-07` passed.
  - System Boundaries: `TC-SYS-01` through `TC-SYS-05` passed (zero runtime/console errors, zero DB mutation).
  - Machine-readable artifact: `test-results/comprehensive-uat/report.json`.

### 4.4 Programmatic State Verification
- **Command**: `node scripts/verify-dev-program.mjs`
- **Result**: PASS (status `running`, `BOOTSTRAP: accepted`, `P2-BT-01: accepted`, `P2-MKT-02: accepted`).

---

## 5. Reviewer Seal
The implementation of `P2-MKT-02` satisfies all statutory market rules, settlement session invariants, and test gates. The batch is certified and accepted into the Sumi DEV Program baseline.
