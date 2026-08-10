# PRO-10 — Market Data Provider Decision

## Outcome
An evidence-backed ADR approves or rejects network data integration before implementation begins.

## Acceptance boundary
Terms/license/attribution, auth/secrets, Daily/Weekly coverage, timestamp/timezone/corporate-action semantics, throttling/failure/correction/reproducibility, provider boundary, security, runtime impact, and fallback evidence.

## Dependencies / exit
Requires PRO-03 import/catalog contracts. Exit is either an approved ADR unlocking PRO-11 or a rejection that retains local file import as the supported path. No provider code belongs in this spike.
