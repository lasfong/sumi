# ADR-PROV-001-ADDENDUM-01 — Option A Quarantine Promotion & Precedence

**Status:** Ratified  
**Date:** 2026-10-12  
**Target Reference:** `docs/architecture/decisions/ADR-PROV-001-synthetic-policy.md`  
**Authors:** Sumi V3 Stabilization Team (Agent B & Agent D)  

---

## 1. Context & Architectural Problem

ADR-PROV-001 ratified a default-deny policy for synthetic data providers (SSI, Vnstock stubs) and established a quarantine dataset partition where synthetic candles are tagged with `source LIKE 'quarantine:%'`. Read isolation was achieved via a centralized SQLAlchemy ORM filter (`_filter_quarantine_candles`) excluding quarantine rows from canonical readers by default.

However, the `Candle` entity enforces a unique database constraint:
```sql
UNIQUE (symbol, timeframe, timestamp, adjustment_type)
```
When real market data arrives for a key currently occupied by a quarantine candle:
1. **Canonical Import**: Because canonical reads exclude quarantine rows, `preview_import` classifies incoming canonical data as new (`parsed`). Upon `accept_import`, attempting an `INSERT` triggers:
   ```text
   sqlite3.IntegrityError: UNIQUE constraint failed: candles.symbol, candles.timeframe, candles.timestamp, candles.adjustment_type
   ```
2. **Canonical Sync**: If sync inspected quarantine rows, real provider data would be misclassified as duplicate, stale, or conflicting against synthetic placeholders, preventing real data from persisting.

---

## 2. Decision: Option A In-Place Promotion & Pre-Validation Rejection

1. **Precedence Invariant**:
   Canonical real data always has absolute precedence over synthetic quarantine data. A quarantine row must never permanently reserve or block a canonical candle key.
2. **Multi-Candidate Quarantine Grouping**:
   When canonical data arrives via Import or real provider Sync, existing quarantine candles for affected symbols are queried (`include_quarantine=True`) and grouped by calendar date key `(symbol, timeframe, date, adjustment_type)` using `quarantine_groups = defaultdict(list)`.
3. **Ambiguous Candidate Pre-Validation Rejection (Finding R2)**:
   Before applying any database writes or mutation logs, an atomic pre-validation pass scans all incoming items against `quarantine_groups`.
   - If multiple quarantine candidates share the same calendar date (`len(candidates) > 1`, e.g. midnight vs non-midnight timestamps), the transaction immediately calls `db.rollback()` and raises `HTTPException(status_code=400, detail="Ambiguous quarantine candidates detected: {count} nến cách ly (quarantine) trùng ngày {date} cho mã {symbol}. Không thể thay thế tự động do dữ liệu không rõ ràng.")`.
   - This fail-closed pre-validation guarantees that ambiguous records are never arbitrarily selected, preventing silent candidate picking or `IntegrityError: UNIQUE constraint failed` collisions.
4. **Option A In-Place Promotion**:
   When exactly one quarantine candidate exists (`len(candidates) == 1`):
   - Preview classifies the item cleanly without marking it as a canonical duplicate or conflict.
   - Accept updates the existing quarantine `Candle` row **in-place** (preserving `Candle.id`):
     - Updates `open`, `high`, `low`, `close`, `volume` to canonical values.
     - Normalizes `timestamp` to midnight (for daily bars).
     - Updates `source` from `quarantine:...` to the canonical run ID (`run_id` or `sync:{sync_id}`).
   - Pre-promotion quarantine state is recorded in mutation logs (`UPDATE:{prior_source}|{prior_iso_ts}`).
5. **Synthetic Precedence Protection**:
   Synthetic sync can NEVER overwrite or downgrade canonical data. Under opt-in (`SUMI_ALLOW_SYNTHETIC_PROVIDERS=1`), if a canonical candle exists with differing OHLCV, the preview flags `conflicting` and blocks acceptance (`can_accept=False`). If OHLCV is identical, it executes an idempotent no-op (`run.status = "noop"`).

---

## 3. Alternatives Evaluated

| Alternative | Mechanics | Evaluation & Disposition |
|---|---|---|
| **Option A + Pre-Validation (Adopted)** | In-place update of unambiguous quarantine row; fail-closed HTTP 400 rejection on multiple candidates. | **Accepted**: Zero ID churn, atomic within single transaction, zero schema migrations required, preserved rollback reconstruction, complete protection against ambiguous collisions. |
| **Option B (Rejected)** | Delete quarantine row and insert canonical row. | **Rejected**: Temporarily leaves gaps, risks foreign key integrity issues, requires two SQL statements, complicates transaction locks. |
| **Option C (Rejected)** | Create separate `quarantine_candles` table. | **Rejected**: Requires Alembic schema migration and DDL modifications, violating W0-A stabilization constraints where B03 remains strictly blocked. |

---

## 4. Consequences & Invariants

- **Positive**: Real data supersedes placeholder synthetic data smoothly and deterministically.
- **Positive**: Zero schema migrations required; existing SQLite schema remains 100% compliant.
- **Positive**: Ambiguous multiple quarantine candidates fail closed with explicit diagnostic error prior to any state mutation.
- **Invariant**: All promotions must capture pre-operation quarantine state in mutation logs (`ImportRunMutation` / `SyncRunMutation`) per `ADR-TX-001-ADDENDUM-01`.

---

## 5. Verification & Test Oracles

- `backend/app/tests/test_b02_quarantine_write_collision.py`:
  - `test_q_write_01_canonical_import_supersedes_quarantine`
  - `test_q_write_02_canonical_sync_supersedes_quarantine`
  - `test_q_write_03_quarantine_must_never_overwrite_canonical`
  - `test_q_write_27_synthetic_sync_identical_ohlcv_noop_preserves_canonical`
- `backend/tests/test_w0a_final_adversarial.py`:
  - `test_08_two_quarantine_candles_same_date_in_import_fails_closed`
  - `test_09_two_quarantine_candles_same_date_in_sync_fails_closed`
  - `test_10_same_day_midnight_non_midnight_collision_fails_closed`
  - `test_11_one_unambiguous_quarantine_candle_promotes_and_rolls_back`
  - `test_14_database_state_unchanged_after_rejected_ambiguous_promotion`
