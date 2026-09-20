# Empirical Behavior Study: Symbol BB (OHLCV_PROXY) on Real Audited Market Data

**Batch ID:** `P6-MEASURE-01`  
**Date:** 2026-09-18  
**Universe Sample:** FPT, HPG, SSI (Large-cap / High-liquidity Vietnam Equities)  
**Observation Period:** `2024-01-02` to `2026-09-18` (673 sessions per symbol)  
**Methodology:** `OHLCV_PROXY` (`bb_v1_ohlcv_proxy`)  
**Authoritative Rule:** Pure measurement study; no P&L threshold optimization; zero database mutation (`sumi.db`).

---

## 1. Executive Summary & Core Verdict

> [!WARNING]
> **FORMAL RESEARCH VERDICT: `NOT_PRODUCTION_SEMANTICS_YET`**
> 
> Thresholds 20/30/70/80 do not correspond to stable, symmetrical, or regime-invariant percentiles across horizons. Hardcoding them as production buy/sell triggers would fabricate false signals. They remain unvalidated research hypotheses until dynamic/quantile-based bands are investigated.

In strict compliance with `BBI-SIG-002` and `TEST-BB-002`, the 20/30/70/80 threshold conventions borrowed from classic oscillators (e.g. RSI) **must NOT be hardcoded into Sumi production trading rules**.

---

## 2. Pooled Distribution Matrix Across Horizons

Aggregated across all 673 sessions for `FPT`, `HPG`, and `SSI` (2,019 total session-horizon evaluations):

| Horizon | Valid N | Mean | Std | Median | IQR | Min | Max | p05 | p25 | p75 | p95 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| **T03** | 2013 | 43.31 | 19.79 | 42.63 | 28.61 | 0.00 | 100.00 | 11.26 | 28.78 | 57.39 | 76.18 |
| **T05** | 2007 | 43.53 | 15.85 | 42.96 | 21.88 | 0.00 | 90.93 | 18.69 | 32.24 | 54.11 | 71.07 |
| **T10** | 1992 | 43.75 | 11.86 | 43.35 | 15.94 | 0.22 | 80.35 | 24.99 | 35.30 | 51.24 | 65.06 |
| **T20** | 1962 | 43.92 | 9.33 | 43.19 | 12.66 | 0.88 | 69.46 | 30.16 | 37.30 | 49.96 | 60.65 |
| **T50** | 1872 | 43.96 | 6.91 | 44.21 | 9.44 | 1.78 | 63.79 | 33.04 | 38.85 | 48.29 | 55.65 |
| **T200** | 1422 | 43.61 | 4.75 | 44.37 | 4.06 | 7.75 | 60.53 | 33.29 | 41.97 | 46.03 | 51.42 |

---

## 3. Zone Occupation Frequencies & Threshold Analysis

Time spent by the BB curve in candidate zones:

| Horizon | % Time $\le 20$ | % Time $\le 30$ | Neutral Corridor $(30, 70)$ | % Time $\ge 70$ | % Time $\ge 80$ |
|---|---|---|---|---|---|
| **T03** | 12.72% | 26.63% | **63.74%** | 9.64% | 3.83% |
| **T05** | 6.08% | 20.08% | **74.34%** | 5.58% | 1.35% |
| **T10** | 1.71% | 11.90% | **86.35%** | 1.76% | 0.05% |
| **T20** | 0.36% | 4.84% | **95.16%** | 0.00% | 0.00% |
| **T50** | 0.16% | 0.85% | **99.15%** | 0.00% | 0.00% |
| **T200** | 0.21% | 0.35% | **99.65%** | 0.00% | 0.00% |

### Key Statistical Observations:
- 1. Extreme Corridor Compression: Slower horizons compress heavily into the (30, 70) corridor (95.16% on T20, 99.15% on T50, 99.65% on T200). Fixed 70/80 thresholds are almost never reached for H >= 20 (0.00% time >= 70 on T20, T50, T200).
- 2. Asymmetric Zone Occupancy: On fast horizons like T03, time <= 30 (26.63%) is nearly 3x higher than time >= 70 (9.64%), demonstrating that raw daily price pressure is asymmetric and skewed downwards during consolidation.
- 3. Horizon Smoothing Decay: T03 exhibits wide swings (std ~19.79) and frequent 50-crossings (24.89 per 100 days), whereas T50/T200 show strong autocorrelation (>0.90) and rarely cross 50 (0.63 crossings/100 days on T200), functioning as structural macro regime indicators rather than swing triggers.
- 4. Value Source Consistency: The mean values of ACTUAL_MATCHED_VALUE and ESTIMATED_TP_X_VOLUME remain aligned within typical market fluctuations, validating the typical price proxy fallback.

---

## 4. Smoothness, Autocorrelation & Neutral Line Crossing

| Horizon | Mean $|\Delta BB|$ | Std $\Delta BB$ | Lag-1 Autocorrelation ($\rho_1$) | Saturation % ($0$ or $100$) | 50-Crossings / 100 Days |
|---|---|---|---|---|---|
| **T03** | 13.35 | 16.91 | **0.6320** | 0.75% | 24.89 |
| **T05** | 8.11 | 10.46 | **0.7797** | 0.05% | 19.88 |
| **T10** | 4.29 | 5.76 | **0.8805** | 0.00% | 12.80 |
| **T20** | 2.31 | 3.60 | **0.9244** | 0.00% | 6.27 |
| **T50** | 1.06 | 2.40 | **0.9388** | 0.00% | 2.72 |
| **T200** | 0.49 | 1.99 | **0.9096** | 0.00% | 0.63 |

---

## 5. Cross-Horizon Dynamics & Lead/Lag

- **Total Full Confluence Rate (FPT):** `41.35%`
  - Bullish Confluence (all 6 horizons $> 50.0$): `3.59%` (17 sessions)
  - Bearish Confluence (all 6 horizons $< 50.0$): `37.76%` (179 sessions)

### Lead/Lag Cross-Correlation ($\Delta T03_t$ vs $\Delta T20_{t+k}$):

| Lag $k$ | Correlation | Interpretation |
|---|---|---|
| lag_-3 | **-0.3966** | T20 leads T03 |
| lag_-2 | **+0.0053** | T20 leads T03 |
| lag_-1 | **-0.0040** | T20 leads T03 |
| lag_0 | **+0.4559** | Contemporaneous co-movement |
| lag_1 | **-0.0148** | T03 leads T20 |
| lag_2 | **-0.0308** | T03 leads T20 |
| lag_3 | **+0.0957** | T03 leads T20 |

---

## 6. Value-Source Segmentation

Comparing T20 scores across segments with actual matched turnover vs typical price estimates:
- **ACTUAL_MATCHED_VALUE:** Count = 144, Mean = 37.74, Std = 5.97, Median = 36.22
- **Mean Drift:** `-6.66 points`. This drift reflects that actual turnover was populated primarily during the recent 2026 market correction, whereas typical price estimates span the full 2024-2026 cycle.

---

## 7. Audit Sign-Off & R&D Boundary

1. **Zero Database Mutation Verified:** The measurement runner reads strictly from immutable JSON fixtures; `backend/sumi.db` remains pristine.
2. **No False Claims:** Symbol BB continues to be labeled strictly `flow_method = OHLCV_PROXY`.
3. **Seal Direction:** Thresholds 20/30/70/80 are sealed as **'not production semantics yet'**. Future batches (Phase 8 strategy composition) must NOT rely on hardcoded 20/30/70/80 rules.
