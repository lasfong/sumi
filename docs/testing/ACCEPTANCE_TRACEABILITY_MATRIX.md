# ACCEPTANCE TRACEABILITY MATRIX (GAP -> G/R/I/D/T)

| GAP | Acceptance | Ghi chu |
|---|---|---|
| GAP-DB-01/02/03 | G-01, G-02 | Safety gate; hash prod unchanged |
| GAP-DB-04 | G-02, G-05 | Stub deny; local-first |
| GAP-REPLAY-01/02, GAP-EXEC-08 | R-01, R-04 | Lookahead lockdown |
| GAP-FE-01/02, GAP-WS-01 | R-03, R-04, R-05 | Nav determinism + resume |
| GAP-EXEC-01..08 | T-02 (fee/T+2), G-04 | Financial truth + golden |
| GAP-API-01..11 | G-01, G-03 | Contracts snapshot, 422, envelope |
| GAP-U-01 | T-02, T-05 | T+2 inline |
| GAP-U-02 | T-01, T-02, T-04 | SL/TP/SHORT/sync |
| GAP-U-03 | T-05 | Builder-Battle khong mat rule |
| GAP-U-04 | D-06, T-05 | Undo/redo + huy LIMIT |
| GAP-U-05 | T-05 | Nav backtest/lab |
| Indicator warming/mapping (B11/B32) | I-13, I-01 | Warmup + list hien thi |
| Drawing undo/persist (B32/B51) | D-05, D-06, D-08 | Undo + persist |
| Journal/checklist (B40) | T-03 | Khong mat chart context |
| Scanner->Replay | T-05 (flow) | Intent + nguoc (tuong lai) |
| Import/sync | G-02 | Temp-DB, provenance |

G-04 (no P0/P1) chi PASS khi toan bo P0 + P1 lien quan released workflow xanh.
