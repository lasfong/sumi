"""Benchmark Metrics, Phase Metric Matrix, and Cross-Phase Degradation.

Implements:
- FR-CORE-010: Exact nine benchmark metrics with Master signs and N/A semantics.
- TEST-MET-001: Golden trade ledger produces exact expected metric values without intermediate rounding.
- TEST-MET-002: Edge cases (0 trades, 0 winners, 0 losers, break-even trade counts as winner).
- Master Sections 22 & 25: Master benchmark table rendering and CSV export.
- Cross-Phase Degradation: Delta, ratio, stability, and degradation analysis between evaluation phases.
"""

from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional
import csv
import io
import math

from app.domain.backtest.models import BacktestTrade


@dataclass
class BenchmarkMetricRow:
    """Exact nine benchmark metrics row per Master Spec Section 22."""
    ticker: str
    phase_name: Optional[str] = None
    start_date: Optional[str] = None
    end_date: Optional[str] = None
    initial_cash: float = 100_000_000.0
    final_cash: float = 100_000_000.0
    final_equity: float = 100_000_000.0

    # 1. Net Profit (Section 22.2): Sum of closed-trade net PnL
    net_profit: float = 0.0

    # 2. % Net Profit (Section 22.3): 100 * NetProfit / InitialAllocatedCapital
    net_profit_pct: float = 0.0

    # 3. # Trades (Section 22.4): Count of closed round-trip trades
    num_trades: int = 0

    # 4. Avg % Profit/Loss (Section 22.5): Mean return_pct of closed trades, N/A if 0 trades
    avg_profit_loss_pct: Optional[float] = None

    # 5. Avg Bars Held (Section 22.6): Mean (exit_bar - entry_bar) of closed trades, N/A if 0 trades
    avg_bars_held: Optional[float] = None

    # 6. % of Winners (Section 22.7): 100 * winners / num_trades. Break-even (return_pct >= 0) is WINNER.
    win_rate_pct: Optional[float] = None

    # 7. W. Avg % Profit (Section 22.8): Mean return_pct of winning trades, N/A if 0 winners
    win_avg_profit_pct: Optional[float] = None

    # 8. L. Avg % Loss (Section 22.9): Mean return_pct of losing trades, N/A if 0 losers. PRESERVES NEGATIVE SIGN.
    loss_avg_loss_pct: Optional[float] = None

    # Auxiliary metrics & holding integrity (Section 24, TEST-BT-005)
    num_winners: int = 0
    num_losers: int = 0
    num_breakeven: int = 0
    profit_factor: Optional[float] = None
    max_drawdown: float = 0.0
    open_position_quantity: float = 0.0
    open_position_value: float = 0.0

    def to_dict(self) -> Dict[str, Any]:
        """Serialize unrounded numeric fields for API response and JSON export."""
        return {
            "ticker": self.ticker,
            "phase_name": self.phase_name,
            "start_date": self.start_date,
            "end_date": self.end_date,
            "initial_cash": self.initial_cash,
            "final_cash": self.final_cash,
            "final_equity": self.final_equity,
            "net_profit": self.net_profit,
            "net_profit_pct": self.net_profit_pct,
            "num_trades": self.num_trades,
            "avg_profit_loss_pct": self.avg_profit_loss_pct,
            "avg_bars_held": self.avg_bars_held,
            "win_rate_pct": self.win_rate_pct,
            "win_avg_profit_pct": self.win_avg_profit_pct,
            "loss_avg_loss_pct": self.loss_avg_loss_pct,
            "num_winners": self.num_winners,
            "num_losers": self.num_losers,
            "num_breakeven": self.num_breakeven,
            "profit_factor": self.profit_factor,
            "max_drawdown": self.max_drawdown,
            "open_position_quantity": self.open_position_quantity,
            "open_position_value": self.open_position_value,
        }


def calculate_benchmark_metrics(
    trades: List[BacktestTrade],
    initial_cash: float,
    ticker: str,
    phase_name: Optional[str] = None,
    start_date: Optional[str] = None,
    end_date: Optional[str] = None,
    final_cash: Optional[float] = None,
    final_equity: Optional[float] = None,
    open_position_quantity: float = 0.0,
    open_position_value: float = 0.0,
    max_drawdown: float = 0.0,
) -> BenchmarkMetricRow:
    """Calculate exact Master 9 benchmark metrics without intermediate rounding.

    Invariants:
    - Only CLOSED trades are evaluated (TEST-BT-005).
    - Break-even trades (return_pct >= 0.0) count as WINNERS (Section 22.7).
    - Losing trade averages retain negative signs (Section 22.9).
    - Edge cases (0 trades, 0 winners, 0 losers) evaluate to None (TEST-MET-002).
    """
    closed_trades = [t for t in trades if getattr(t, "status", "") == "closed"]

    actual_final_cash = final_cash if final_cash is not None else initial_cash
    actual_final_equity = final_equity if final_equity is not None else (actual_final_cash + open_position_value)

    if not closed_trades:
        # Zero trades edge case (TEST-MET-002)
        return BenchmarkMetricRow(
            ticker=ticker,
            phase_name=phase_name,
            start_date=start_date,
            end_date=end_date,
            initial_cash=initial_cash,
            final_cash=actual_final_cash,
            final_equity=actual_final_equity,
            net_profit=0.0,
            net_profit_pct=0.0,
            num_trades=0,
            avg_profit_loss_pct=None,
            avg_bars_held=None,
            win_rate_pct=None,
            win_avg_profit_pct=None,
            loss_avg_loss_pct=None,
            num_winners=0,
            num_losers=0,
            num_breakeven=0,
            profit_factor=None,
            max_drawdown=max_drawdown,
            open_position_quantity=open_position_quantity,
            open_position_value=open_position_value,
        )

    # 1. Net Profit (Section 22.2)
    net_profit = sum(t.net_pnl for t in closed_trades)

    # 2. % Net Profit (Section 22.3)
    net_profit_pct = (100.0 * net_profit / initial_cash) if initial_cash > 0 else 0.0

    # 3. # Trades (Section 22.4)
    num_trades = len(closed_trades)

    # 4. Avg % Profit/Loss (Section 22.5)
    trade_returns = [t.pnl_percent for t in closed_trades]
    avg_profit_loss_pct = sum(trade_returns) / num_trades

    # 5. Avg Bars Held (Section 22.6)
    bars_held_list = [
        (t.exit_bar_index - t.entry_bar_index)
        if (t.exit_bar_index is not None and t.entry_bar_index is not None)
        else t.holding_bars
        for t in closed_trades
    ]
    avg_bars_held = sum(bars_held_list) / num_trades

    # 6. Winners / Losers Classification (Section 22.7, break-even is winner: return_pct >= 0)
    winners = [t for t in closed_trades if t.pnl_percent >= 0.0]
    losers = [t for t in closed_trades if t.pnl_percent < 0.0]
    breakevens = [t for t in closed_trades if t.pnl_percent == 0.0]

    num_winners = len(winners)
    num_losers = len(losers)
    num_breakeven = len(breakevens)

    win_rate_pct = 100.0 * num_winners / num_trades

    # 7. W. Avg % Profit (Section 22.8)
    win_avg_profit_pct = (sum(t.pnl_percent for t in winners) / num_winners) if num_winners > 0 else None

    # 8. L. Avg % Loss (Section 22.9 - PRESERVES NEGATIVE SIGN)
    loss_avg_loss_pct = (sum(t.pnl_percent for t in losers) / num_losers) if num_losers > 0 else None

    # Auxiliary Profit Factor
    gross_profit = sum(t.gross_pnl for t in closed_trades if t.gross_pnl > 0)
    gross_loss = sum(abs(t.gross_pnl) for t in closed_trades if t.gross_pnl < 0)
    if gross_loss > 0:
        profit_factor = gross_profit / gross_loss
    elif gross_profit > 0:
        profit_factor = float("inf")
    else:
        profit_factor = None

    return BenchmarkMetricRow(
        ticker=ticker,
        phase_name=phase_name,
        start_date=start_date,
        end_date=end_date,
        initial_cash=initial_cash,
        final_cash=actual_final_cash,
        final_equity=actual_final_equity,
        net_profit=net_profit,
        net_profit_pct=net_profit_pct,
        num_trades=num_trades,
        avg_profit_loss_pct=avg_profit_loss_pct,
        avg_bars_held=avg_bars_held,
        win_rate_pct=win_rate_pct,
        win_avg_profit_pct=win_avg_profit_pct,
        loss_avg_loss_pct=loss_avg_loss_pct,
        num_winners=num_winners,
        num_losers=num_losers,
        num_breakeven=num_breakeven,
        profit_factor=profit_factor,
        max_drawdown=max_drawdown,
        open_position_quantity=open_position_quantity,
        open_position_value=open_position_value,
    )


@dataclass
class CrossPhaseComparison:
    """Quantitative comparison and degradation between two evaluation phases."""
    ticker: str
    base_phase_name: str
    target_phase_name: str
    net_profit_delta: float
    return_delta_pct: float
    win_rate_delta_pct: Optional[float]
    profit_ratio: Optional[float]
    degradation_pct: Optional[float]
    is_degraded: bool
    drawdown_delta: float

    def to_dict(self) -> Dict[str, Any]:
        return {
            "ticker": self.ticker,
            "base_phase_name": self.base_phase_name,
            "target_phase_name": self.target_phase_name,
            "net_profit_delta": self.net_profit_delta,
            "return_delta_pct": self.return_delta_pct,
            "win_rate_delta_pct": self.win_rate_delta_pct,
            "profit_ratio": self.profit_ratio,
            "degradation_pct": self.degradation_pct,
            "is_degraded": self.is_degraded,
            "drawdown_delta": self.drawdown_delta,
        }


def compare_phases(
    base_row: BenchmarkMetricRow,
    target_row: BenchmarkMetricRow,
    degradation_threshold_pct: float = -50.0,
) -> CrossPhaseComparison:
    """Compare performance from base phase (e.g. In-Sample) to target phase (e.g. Out-of-Sample)."""
    profit_delta = target_row.net_profit - base_row.net_profit
    return_delta = target_row.net_profit_pct - base_row.net_profit_pct

    # Win rate delta
    if target_row.win_rate_pct is not None and base_row.win_rate_pct is not None:
        win_rate_delta = target_row.win_rate_pct - base_row.win_rate_pct
    else:
        win_rate_delta = None

    # Profit ratio
    if base_row.net_profit > 0:
        profit_ratio = target_row.net_profit / base_row.net_profit
    else:
        profit_ratio = None

    # Degradation percentage relative to base return
    if base_row.net_profit_pct > 0:
        degradation_pct = 100.0 * (target_row.net_profit_pct - base_row.net_profit_pct) / base_row.net_profit_pct
    elif base_row.net_profit_pct < 0 and target_row.net_profit_pct < base_row.net_profit_pct:
        # Loses became worse
        degradation_pct = -100.0 * abs(target_row.net_profit_pct - base_row.net_profit_pct) / abs(base_row.net_profit_pct)
    else:
        degradation_pct = None

    # Degradation flag: target becomes negative after positive base, or drops beyond threshold
    is_degraded = False
    if base_row.net_profit_pct > 0 and target_row.net_profit_pct < 0:
        is_degraded = True
    elif degradation_pct is not None and degradation_pct <= degradation_threshold_pct:
        is_degraded = True

    drawdown_delta = target_row.max_drawdown - base_row.max_drawdown

    return CrossPhaseComparison(
        ticker=target_row.ticker,
        base_phase_name=base_row.phase_name or "base",
        target_phase_name=target_row.phase_name or "target",
        net_profit_delta=profit_delta,
        return_delta_pct=return_delta,
        win_rate_delta_pct=win_rate_delta,
        profit_ratio=profit_ratio,
        degradation_pct=degradation_pct,
        is_degraded=is_degraded,
        drawdown_delta=drawdown_delta,
    )


@dataclass
class PhaseMetricMatrix:
    """2D benchmark metric matrix across symbols and phases with aggregate portfolio row."""
    phase_names: List[str]
    symbols: List[str]
    rows_by_symbol: Dict[str, Dict[str, BenchmarkMetricRow]]
    portfolio_by_phase: Dict[str, BenchmarkMetricRow]
    cross_phase_degradations: List[CrossPhaseComparison] = field(default_factory=list)
    consistency_score: float = 100.0  # % of non-degraded transitions

    def to_dict(self) -> Dict[str, Any]:
        return {
            "phase_names": self.phase_names,
            "symbols": self.symbols,
            "rows_by_symbol": {
                sym: {p_name: r.to_dict() for p_name, r in p_dict.items()}
                for sym, p_dict in self.rows_by_symbol.items()
            },
            "portfolio_by_phase": {
                p_name: r.to_dict() for p_name, r in self.portfolio_by_phase.items()
            },
            "cross_phase_degradations": [d.to_dict() for d in self.cross_phase_degradations],
            "consistency_score": self.consistency_score,
        }


def build_phase_metric_matrix(
    phase_names: List[str],
    metric_rows: List[BenchmarkMetricRow],
) -> PhaseMetricMatrix:
    """Assemble individual phase metric rows into a 2D matrix with portfolio summaries and degradation."""
    symbols = sorted(list({r.ticker for r in metric_rows}))
    rows_by_symbol: Dict[str, Dict[str, BenchmarkMetricRow]] = {s: {} for s in symbols}

    for r in metric_rows:
        if r.phase_name:
            rows_by_symbol[r.ticker][r.phase_name] = r

    # Compute aggregate portfolio row per phase
    portfolio_by_phase: Dict[str, BenchmarkMetricRow] = {}
    for p_name in phase_names:
        phase_rows = [rows_by_symbol[s].get(p_name) for s in symbols if p_name in rows_by_symbol[s]]
        if not phase_rows:
            continue

        tot_initial = sum(r.initial_cash for r in phase_rows)
        tot_final_cash = sum(r.final_cash for r in phase_rows)
        tot_final_equity = sum(r.final_equity for r in phase_rows)
        tot_net_profit = sum(r.net_profit for r in phase_rows)
        tot_trades = sum(r.num_trades for r in phase_rows)
        tot_open_qty = sum(r.open_position_quantity for r in phase_rows)
        tot_open_val = sum(r.open_position_value for r in phase_rows)
        tot_winners = sum(r.num_winners for r in phase_rows)
        tot_losers = sum(r.num_losers for r in phase_rows)
        tot_breakeven = sum(r.num_breakeven for r in phase_rows)

        net_profit_pct = (100.0 * tot_net_profit / tot_initial) if tot_initial > 0 else 0.0

        # Weighted averages for closed trades
        if tot_trades > 0:
            avg_profit_loss = (
                sum(r.avg_profit_loss_pct * r.num_trades for r in phase_rows if r.avg_profit_loss_pct is not None)
                / tot_trades
            )
            avg_bars = (
                sum(r.avg_bars_held * r.num_trades for r in phase_rows if r.avg_bars_held is not None)
                / tot_trades
            )
            win_rate = 100.0 * tot_winners / tot_trades
        else:
            avg_profit_loss = None
            avg_bars = None
            win_rate = None

        win_avg = (
            sum(r.win_avg_profit_pct * r.num_winners for r in phase_rows if r.win_avg_profit_pct is not None)
            / tot_winners
        ) if tot_winners > 0 else None

        loss_avg = (
            sum(r.loss_avg_loss_pct * r.num_losers for r in phase_rows if r.loss_avg_loss_pct is not None)
            / tot_losers
        ) if tot_losers > 0 else None

        portfolio_by_phase[p_name] = BenchmarkMetricRow(
            ticker="PORTFOLIO",
            phase_name=p_name,
            initial_cash=tot_initial,
            final_cash=tot_final_cash,
            final_equity=tot_final_equity,
            net_profit=tot_net_profit,
            net_profit_pct=net_profit_pct,
            num_trades=tot_trades,
            avg_profit_loss_pct=avg_profit_loss,
            avg_bars_held=avg_bars,
            win_rate_pct=win_rate,
            win_avg_profit_pct=win_avg,
            loss_avg_loss_pct=loss_avg,
            num_winners=tot_winners,
            num_losers=tot_losers,
            num_breakeven=tot_breakeven,
            open_position_quantity=tot_open_qty,
            open_position_value=tot_open_val,
        )

    # Compute cross-phase comparisons (sequential transitions between phases)
    degradations: List[CrossPhaseComparison] = []
    if len(phase_names) >= 2:
        for i in range(len(phase_names) - 1):
            base_p = phase_names[i]
            target_p = phase_names[i + 1]

            # Symbol level comparisons
            for s in symbols:
                base_r = rows_by_symbol[s].get(base_p)
                target_r = rows_by_symbol[s].get(target_p)
                if base_r and target_r:
                    degradations.append(compare_phases(base_r, target_r))

            # Portfolio level comparison
            base_port = portfolio_by_phase.get(base_p)
            target_port = portfolio_by_phase.get(target_p)
            if base_port and target_port:
                degradations.append(compare_phases(base_port, target_port))

    # Consistency score: % of non-degraded transitions
    if degradations:
        non_degraded_count = sum(1 for d in degradations if not d.is_degraded)
        consistency_score = round(100.0 * non_degraded_count / len(degradations), 2)
    else:
        consistency_score = 100.0

    return PhaseMetricMatrix(
        phase_names=phase_names,
        symbols=symbols,
        rows_by_symbol=rows_by_symbol,
        portfolio_by_phase=portfolio_by_phase,
        cross_phase_degradations=degradations,
        consistency_score=consistency_score,
    )


class BenchmarkTableRenderer:
    """Presentation formatting for the Master 9-metric benchmark table and CSV export."""

    HEADER_COLUMNS = [
        "Ticker",
        "Net Profit",
        "% Net Profit",
        "# Trades",
        "Avg % Profit/Loss",
        "Avg Bars Held",
        "% of Winners",
        "W. Avg % Profit",
        "L. Avg % Loss",
    ]

    @classmethod
    def _format_value(
        cls,
        val: Optional[float],
        is_percent: bool = False,
        is_currency: bool = False,
        decimals: int = 1,
    ) -> str:
        """Format numeric values with N/A semantics and negative sign preservation."""
        if val is None or (isinstance(val, float) and (math.isnan(val) or math.isinf(val))):
            return "N/A"

        if is_currency:
            return f"{val:,.0f}"

        if is_percent:
            sign = "-" if val < 0 else ""
            return f"{sign}{abs(val):.{decimals}f}%"

        return f"{val:.{decimals}f}"

    @classmethod
    def format_row_dict(cls, row: BenchmarkMetricRow, decimals: int = 1) -> Dict[str, str]:
        """Return row values formatted as strings mapped by Master header column name."""
        return {
            "Ticker": row.ticker,
            "Net Profit": cls._format_value(row.net_profit, is_currency=True),
            "% Net Profit": cls._format_value(row.net_profit_pct, is_percent=True, decimals=decimals),
            "# Trades": str(row.num_trades),
            "Avg % Profit/Loss": cls._format_value(row.avg_profit_loss_pct, is_percent=True, decimals=decimals),
            "Avg Bars Held": cls._format_value(row.avg_bars_held, decimals=decimals),
            "% of Winners": cls._format_value(row.win_rate_pct, is_percent=True, decimals=decimals),
            "W. Avg % Profit": cls._format_value(row.win_avg_profit_pct, is_percent=True, decimals=decimals),
            "L. Avg % Loss": cls._format_value(row.loss_avg_loss_pct, is_percent=True, decimals=decimals),
        }

    @classmethod
    def render_markdown_row(cls, row: BenchmarkMetricRow, decimals: int = 1) -> str:
        """Render a single BenchmarkMetricRow into Markdown table row cells."""
        cols = [
            row.ticker,
            cls._format_value(row.net_profit, is_currency=True),
            cls._format_value(row.net_profit_pct, is_percent=True, decimals=decimals),
            str(row.num_trades),
            cls._format_value(row.avg_profit_loss_pct, is_percent=True, decimals=decimals),
            cls._format_value(row.avg_bars_held, decimals=decimals),
            cls._format_value(row.win_rate_pct, is_percent=True, decimals=decimals),
            cls._format_value(row.win_avg_profit_pct, is_percent=True, decimals=decimals),
            cls._format_value(row.loss_avg_loss_pct, is_percent=True, decimals=decimals),
        ]
        return "| " + " | ".join(cols) + " |"

    @classmethod
    def render_markdown_table(
        cls,
        rows: List[BenchmarkMetricRow],
        title: Optional[str] = None,
        decimals: int = 1,
    ) -> str:
        """Render exact Master Spec Section 25 Markdown Table."""
        lines = []
        if title:
            lines.append(f"### {title}\n")

        lines.append("| " + " | ".join(cls.HEADER_COLUMNS) + " |")
        lines.append("| " + " | ".join(["---"] * len(cls.HEADER_COLUMNS)) + " |")

        # Stable sort: Tickers alphabetically, with PORTFOLIO at the bottom
        sorted_rows = sorted(
            rows,
            key=lambda r: (1 if r.ticker == "PORTFOLIO" else 0, r.ticker),
        )

        for r in sorted_rows:
            lines.append(cls.render_markdown_row(r, decimals=decimals))

        return "\n".join(lines)

    @classmethod
    def render_csv(cls, rows: List[BenchmarkMetricRow], formatted: bool = False, decimals: int = 1) -> str:
        """Generate CSV export matching the exact 9 columns with stable ticker sort."""
        output = io.StringIO()
        writer = csv.writer(output)
        writer.writerow(cls.HEADER_COLUMNS)

        sorted_rows = sorted(
            rows,
            key=lambda r: (1 if r.ticker == "PORTFOLIO" else 0, r.ticker),
        )

        for r in sorted_rows:
            if formatted:
                writer.writerow([
                    r.ticker,
                    cls._format_value(r.net_profit, is_currency=True),
                    cls._format_value(r.net_profit_pct, is_percent=True, decimals=decimals),
                    r.num_trades,
                    cls._format_value(r.avg_profit_loss_pct, is_percent=True, decimals=decimals),
                    cls._format_value(r.avg_bars_held, decimals=decimals),
                    cls._format_value(r.win_rate_pct, is_percent=True, decimals=decimals),
                    cls._format_value(r.win_avg_profit_pct, is_percent=True, decimals=decimals),
                    cls._format_value(r.loss_avg_loss_pct, is_percent=True, decimals=decimals),
                ])
            else:
                writer.writerow([
                    r.ticker,
                    round(r.net_profit, 2),
                    round(r.net_profit_pct, 4),
                    r.num_trades,
                    round(r.avg_profit_loss_pct, 4) if r.avg_profit_loss_pct is not None else "N/A",
                    round(r.avg_bars_held, 2) if r.avg_bars_held is not None else "N/A",
                    round(r.win_rate_pct, 4) if r.win_rate_pct is not None else "N/A",
                    round(r.win_avg_profit_pct, 4) if r.win_avg_profit_pct is not None else "N/A",
                    round(r.loss_avg_loss_pct, 4) if r.loss_avg_loss_pct is not None else "N/A",
                ])

        return output.getvalue()
