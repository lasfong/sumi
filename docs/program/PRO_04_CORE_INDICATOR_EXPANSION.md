# PRO-04 — Core Indicator Expansion

## Outcome
SMA, Bollinger Bands, ATR, and backend Volume SMA are complete, correctly placed, persisted, and rendered capabilities.

## Acceptance boundary
`PRO-IND-01`–`PRO-IND-08` and `PRO-IND-11`. Use an exhaustive released-definition catalog, explicit semantic output/pane mapping, and distinguish raw Volume from Volume SMA.

## Dependencies and risks
Requires approved PRO-03 data contracts and the backend `IndicatorEngine` as calculation authority. Guard against renderer fallbacks, warm-up ambiguity, and persistence drift.

## Entry / exit
Entry only after PRO-03 approval. Exit requires backend/frontend tests, browser screenshots at required sizes, persistence/navigation evidence, and no future-data leakage.
