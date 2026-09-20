# 11 — Validation & Acceptance

## Core math
- Buy=Sell -> BB=50.
- Sell=0, Buy>0 -> 100.
- Buy=0, Sell>0 -> 0.
- BB always [0,100].
- T05 day 6 uses days 2–6.

## Method boundaries
- Proxy không được label True Flow.
- Source switch không mất lineage.
- Historical Proxy và forward True Flow phân biệt được.

## Data quality
- Unknown không tự gán.
- Coverage được tính.
- Missing row != zero trading.
- Stream duplicate idempotent.
- EOD aggregate reconcile với totals trong tolerance.

## SUMI-420
- Aggregate measures before ratio.
- Không average symbol BB.
- Universe version lưu rõ.
- Coverage expose.

## Source acceptance
Một source chỉ được approve `ACTIVE_TRADE_FLOW_VALUE` khi:
- semantics documented;
- units verified;
- sample manually checked;
- coverage/missingness known;
- access stable;
- license/use risk reviewed.

Tên field alone không đủ.
