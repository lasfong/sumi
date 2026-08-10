# PRO-07 — Ichimoku Contract

## Outcome
Ichimoku is released without look-ahead: calculation timestamps and displaced display timestamps remain distinct and replay-safe.

## Acceptance boundary
All `PRO-IND` criteria, especially `PRO-IND-10`; Tenkan, Kijun, Chikou, cloud displacement, warm-up, gaps, start/end, replay, rewind, and resume are specified and tested.

## Dependencies / exit
Requires the stable indicator definition/data contracts from PRO-03–PRO-06. Exit requires payload-boundary assertions, browser evidence, and full gates.
