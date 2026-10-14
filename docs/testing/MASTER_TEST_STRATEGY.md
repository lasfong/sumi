# MASTER TEST STRATEGY - Sumi V3

## Tang
1. Unit/domain (pytest, vitest): indicator engine, settlement calendar, metrics, signal registry, drawing geometry.
2. Integration BE (temp-DB): import preview/accept/rollback, weekly derive, replay cursor, practice lifecycle, backtest kernel.
3. Contract/API: openapi snapshot Pydantic-TS, BB envelope, 422 normalize, error envelope, dead-route absent.
4. WS: handshake/speed/validate/echo/frames + chaos matrix.
5. Component FE: MappingResult states, TradeControls validation, PositionPanel SHORT, PendingOrders cancel.
6. E2E UAT (Playwright 1440x1000 + 1280x800): 31 cu + P0-U asserts moi.
7. Regression: parity sweep==batch, golden financial, lookahead suite, legacy-loss.
8. Data/deterministic: temp-DB only, seed deterministic, hash prod unchanged, no console/page errors.
9. Visual: screenshots retained pass+fail, diff review.

## Gate
- Per-commit: lint + unit nhanh.
- Per-batch: ExecPlan tests + verify-v2.ps1.
- Per-merge: full pytest/vitest + contract snapshot + UAT lien quan.
- Release: full 31 + new asserts + screenshots + hash invariant + G/R/I/D/T green.

## Nguyen tac
- Khong mutate backend/sumi.db; UAT temp-DB.
- Khong weaken test de xanh; do -> STOP.
- Moi GAP co verification trong GAP_REGISTER; moi batch dan GAP IDs.
