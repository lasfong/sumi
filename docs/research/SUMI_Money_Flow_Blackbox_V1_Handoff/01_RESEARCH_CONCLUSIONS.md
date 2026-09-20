# Research Conclusions & Design Rationale

Tài liệu này tóm tắt câu hỏi nghiên cứu, thuật ngữ bắt buộc và lý do SUMI không lấy MFI/CMF/OBV/ADL làm định nghĩa của dòng tiền thật. Đọc file này để hiểu **vì sao** kiến trúc được chọn.

# 0. Executive decision

The research converged on a two-mode architecture. The preferred mode is
TRUE_FLOW, based on buyer-initiated versus seller-initiated executed
trading value. The fallback mode is OHLCV_PROXY, used only when public
data cannot reconstruct executed trade direction. These modes must
remain semantically and technically distinct.

| **Decision**        | **V1 baseline**                                                                                                          |
|---------------------|--------------------------------------------------------------------------------------------------------------------------|
| Preferred primitive | Executed aggressive Buy Value and Sell Value (buyer-/seller-initiated trades).                                           |
| Core oscillator     | BB_H = 100 × Buy_H / (Buy_H + Sell_H), equivalent to 50 × (1 + normalized order imbalance).                              |
| Horizons            | Rolling T03, T05, T10, T20, T50, T200. Default chart may show 03/05/20/50/200.                                           |
| Whole-market BB     | Aggregate Buy/Sell value across point-in-time universe first, then compute the ratio. Do not average symbol BB scores.   |
| Breadth             | Count breadth + trading-value breadth, separately from Market BB.                                                        |
| Fallback            | Continuous OHLCV technical flow proxy (True-Range/Chaikin-family pressure × Trading Value), clearly labeled OHLCV_PROXY. |
| Not allowed         | Claiming actual capital inflow/outflow, institutional/retail flow, or true aggressive buy/sell from daily OHLCV.         |
| Thresholds          | 50 is structurally meaningful. 20/30/70/80 remain research defaults, not optimized trading thresholds.                   |

GO/NO-GO rule: DEV must complete the Data Audit Gate before deciding
which mode can be implemented for each exchange, symbol history, and
date range.

# 1. Problem definition and terminology

## 1.1 What the Blackbox is intended to answer

- For a symbol: is aggressive trading pressure dominated by buyers or
  sellers, at T03/T05/T10/T20/T50/T200?

- For VNINDEX/index/universe/exchange/market: is aggregate trading
  pressure turning positive or negative, and how broad is participation?

- Can multiple horizons converge and turn together, creating a
  flow-confluence event worth further Price/Volume confirmation?

- Can the user distinguish short-horizon reversal from long-horizon
  regime without resetting every N sessions?

## 1.2 Mandatory terminology

| **Term**                       | **Definition**                                                                                                                                | **Can daily OHLCV prove it?**                                                        |
|--------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------|
| Trading Value / Turnover       | Gross value traded. Every executed trade has both a buyer and seller.                                                                         | Yes (exact if actual matched value exists; approximate from price×volume otherwise). |
| Actual capital inflow/outflow  | Cash entering/leaving an investment vehicle/account/group or primary issuance/redemption.                                                     | No.                                                                                  |
| Aggressive / initiated flow    | Executed value attributed to the side that initiated the trade (buyer taking the ask / seller hitting the bid, or equivalent classification). | No; needs transaction/quote or a trustworthy aggressor-side feed.                    |
| Order Flow Imbalance (OIB)     | Normalized difference between buyer-initiated and seller-initiated activity.                                                                  | No, unless trade direction can be reconstructed.                                     |
| Technical money-flow proxy     | Price/volume-based estimate of buying vs selling pressure.                                                                                    | Yes.                                                                                 |
| Foreign / proprietary net flow | Buy minus sell for a classified investor group.                                                                                               | Only if a source publishes the classification.                                       |
| Institutional / retail flow    | Buy/sell attributable to investor class.                                                                                                      | No, unless investor identity/classification is provided.                             |

Design rule: the UI/API must never silently collapse these concepts into
one generic “money in/out” field.

# 2. Why the research moved away from MFI/CMF as the primary core

Early design iterations evaluated MFI, CMF, ADL, OBV, Trading Value,
signed value, True-Range pressure, Force Index, VPT/PVT, Klinger and
related volume indicators. The important result is not that these
indicators are useless; it is that they remain price-volume inference
mechanisms. If executed trade direction is available, a direct
order-flow primitive is closer to the stated goal.

| **Candidate**               | **Useful information**                                                                           | **Why not TRUE_FLOW core**                                                                        |
|-----------------------------|--------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------|
| MFI                         | 0–100 volume-weighted momentum; positive/negative “money flow” based on Typical Price direction. | Daily classification is price-derived and effectively binary; it does not observe aggressor side. |
| CMF                         | Continuous accumulation/distribution based on Close location within H–L, weighted by volume.     | Still infers pressure from candle geometry; does not observe buyer-/seller-initiated trades.      |
| Twiggs/True-Range flow      | Improves gap handling and smoothing over classic CMF.                                            | Still a technical proxy, not executed order flow.                                                 |
| ADL                         | Cumulative Chaikin accumulation/distribution primitive.                                          | Redundant with the CMF family and cumulative/unbounded.                                           |
| OBV                         | Simple sign of volume by close-to-close direction.                                               | Binary price-derived sign; cumulative and unbounded.                                              |
| Force Index / VPT / Klinger | Blend return magnitude, volume and smoothing.                                                    | More transformation/parameters without solving the core data-observation problem.                 |

Decision: keep standard indicators as optional confirmation/benchmark
tools, not as the definition of TRUE_FLOW BB.
