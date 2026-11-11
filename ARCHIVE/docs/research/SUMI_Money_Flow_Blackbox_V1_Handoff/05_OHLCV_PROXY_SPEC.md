# OHLCV_PROXY Fallback Specification

Dùng **chỉ khi TRUE_FLOW không khả dụng**. Đây là technical money-flow proxy từ OHLCV/Trading Value và bắt buộc phải mang metadata `flow_method=OHLCV_PROXY`; không được gọi là actual inflow/outflow.

# 9. OHLCV_PROXY fallback specification

Use this mode only when executed buy/sell flow cannot be obtained or
reliably reconstructed. It is a technical pressure proxy, not actual
order flow.

## 9.1 Proxy primitive

> true_high\[t\] = max(high\[t\], close\[t-1\])  
> true_low\[t\] = min(low\[t\], close\[t-1\])  
>   
> pressure\[t\] = (2\*close\[t\] - true_high\[t\] - true_low\[t\]) /
> (true_high\[t\] - true_low\[t\])  
> \# pressure in \[-1, +1\]; if denominator=0 -\> 0  
>   
> trading_value\[t\] = actual_matched_value\[t\] if available  
> else typical_price\[t\] \* volume\[t\]  
>   
> signed_pressure_value\[t\] = pressure\[t\] \* trading_value\[t\]

## 9.2 Proxy BB horizon

> proxy_flow_H = Σ_H signed_pressure_value / Σ_H trading_value  
> ProxyBB_H = 50 × (1 + proxy_flow_H)

Why this proxy: it is continuous (unlike binary MFI/OBV direction),
incorporates gap information through previous Close, remains bounded, is
value-weighted, and aggregates cleanly from symbol to market. It is
still price-derived, therefore its API/UI label must say \`Technical
Flow Proxy\`.

## 9.3 Proxy data priority

| **Priority** | **Value source**               | **Notes**                                                                                                     |
|--------------|--------------------------------|---------------------------------------------------------------------------------------------------------------|
| 1            | Actual matched trading value   | Preferred. Exclude or separately flag put-through/negotiated trades unless research explicitly includes them. |
| 2            | Typical Price × matched volume | Fallback estimate when value is absent.                                                                       |
| 3            | Typical Price × total volume   | Last resort if matched vs negotiated volume cannot be separated; lower quality flag.                          |

## 9.4 MFI/CMF roles in proxy research

- MFI is a useful benchmark/confirmation oscillator and naturally uses
  Typical Price × Volume, but is not the proxy core because its daily
  positive/negative classification is price-derived and binary.
  \[R2\]\[R3\]

- CMF is a useful benchmark for accumulation/distribution logic. The
  proxy pressure is conceptually in the Chaikin family but uses a
  True-Range-style daily range to account for gaps. \[R4\]

- OBV and ADL are not required in BB V1. They may remain available
  elsewhere in the technical indicator library.
