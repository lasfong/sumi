# FINANCIAL GOLDEN CASES - hand-calc (fee 0.15% buy, 0.15%+0.1% sell, lot 100, slippage 0, next-open fill)

Moi case: 10-20 candles OHLC, signal/chi thi ro, so tay expected fill/pnl/metrics. Kernel phai khop epsilon phi.

## G-01 Long co ban thang
- Entry signal T=5 close=10.0; fill Open[6]=10.1; exit T=10 close=11.0 fill Open[11]=11.05.
- Qty 100. Gross=(11.05-10.1)*100=95. Fee buy=10.1*100*0.0015=1.515. Fee sell=11.05*100*0.0015=1.6575. Tax=11.05*100*0.001=1.105. Net=95-1.515-1.6575-1.105=90.7225. Winner (net>0).

## G-02 Long thua + breakeven rule
- Entry 20.0 fill 20.05; exit 19.5 fill 19.45. Gross=-60. Fees~3.0075+2.9175+1.945=7.87. Net=-67.87 loser.
- Breakeven gross=0 van la not-winner vi net<0 (phi).

## G-03 SHORT doi xung
- Short entry 15.0 fill 14.95; cover 13.0 fill 13.05. Gross=(14.95-13.05)*100=190. Fees tuong tu. Net>0 winner.

## G-04 SL intraday next-open (khong same-bar)
- SL dat 9.5 khi entry 10.1; bar T+1 low=9.3 nhung fill SL tai Open[T+2] (rule next-open), khong phai 9.5.

## G-05 Cut-off N-1
- Signal T=N-1 (bar cuoi): Order UNFILLED_AT_CUTOFF, khong fill, khong trade.

## G-06 T+1.5 qua weekend
- BUY Thu (T); available=0 Thu-Fri; kha dung sau 1.5 phien giao dich (bo Sat/Sun); SELL som bi chan co inline ly do.

## G-07 Le VN (ngay nghi)
- Tuong tu G-06 qua ngay le trong holiday-calendar.

## G-08 PF-inf hien N/A
- 3 trades toan thang: GP>0, GL=0 => PF=+inf hien "N/A (chua co lenh thua)" + tooltip, khong crash.

## G-09 Gate N<30
- Sample 12 trades: metrics kem canh bao "mau nho (<30), do tin cay thap".

## G-10 Sharpe 1 ngu nghia
- Chuoi daily-log returns cho truoc; Sharpe kernel == analytics == tay (rf=0, annual 252).

## G-11 Sizing tren fill-estimate
- Von 100M, risk 1%, SL 5% theo fill-est 10.1 (khong phai close 10.0): qty = floor(risk_cash / (fill_est - sl)) lam tron lot 100.

## G-12 Sweep==Batch parity
- Cung YAML, symbol, range: sweep PnL == batch PnL (epsilon lam tron), cung so trades, cung fills.
