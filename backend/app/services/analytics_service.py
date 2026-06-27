from sqlalchemy.orm import Session
from sqlalchemy import func
from app.models.trade import Trade
from app.schemas.analytics_schema import AnalyticsResponse, SetupPerformance

class AnalyticsService:
    @staticmethod
    def get_analytics(db: Session, session_id: int) -> AnalyticsResponse:
        trades = db.query(Trade).filter(Trade.session_id == session_id, Trade.exit_date.isnot(None)).order_by(Trade.exit_date).all()
        
        total_trades = len(trades)
        if total_trades == 0:
            return AnalyticsResponse(
                total_trades=0, win_rate=0.0, total_net_pnl=0.0, 
                average_win=0.0, average_loss=0.0, profit_factor=0.0
            )
            
        winning_trades = [t for t in trades if t.net_pnl and t.net_pnl > 0]
        losing_trades = [t for t in trades if t.net_pnl and t.net_pnl <= 0]
        
        win_rate = len(winning_trades) / total_trades
        total_net_pnl = sum(t.net_pnl or 0 for t in trades)
        
        gross_profit = sum(t.net_pnl or 0 for t in winning_trades)
        gross_loss = abs(sum(t.net_pnl or 0 for t in losing_trades))
        
        average_win = gross_profit / len(winning_trades) if winning_trades else 0
        average_loss = gross_loss / len(losing_trades) if losing_trades else 0
        
        profit_factor = (gross_profit / gross_loss) if gross_loss > 0 else float('inf')
        
        largest_win = max([t.net_pnl or 0 for t in trades]) if trades else 0
        largest_loss = min([t.net_pnl or 0 for t in trades]) if trades else 0
        
        # Calculate max drawdown
        max_drawdown = 0.0
        peak = 0.0
        cumulative_pnl = 0.0
        for t in trades:
            cumulative_pnl += (t.net_pnl or 0)
            if cumulative_pnl > peak:
                peak = cumulative_pnl
            drawdown = peak - cumulative_pnl
            if drawdown > max_drawdown:
                max_drawdown = drawdown
                
        # Calculate setup performance
        setup_stats = {}
        for t in trades:
            setup = t.setup_type or "Uncategorized"
            if setup not in setup_stats:
                setup_stats[setup] = {"trades": 0, "wins": 0, "net_pnl": 0.0}
            setup_stats[setup]["trades"] += 1
            if t.net_pnl and t.net_pnl > 0:
                setup_stats[setup]["wins"] += 1
            setup_stats[setup]["net_pnl"] += (t.net_pnl or 0)
            
        setup_performance = []
        for setup, stats in setup_stats.items():
            setup_performance.append(SetupPerformance(
                setup_type=setup,
                trades=stats["trades"],
                win_rate=stats["wins"] / stats["trades"] if stats["trades"] > 0 else 0,
                net_pnl=stats["net_pnl"]
            ))
        
        # Calculate R-multiple if stop_loss and entry_price are present
        r_multiples = []
        for t in trades:
            if t.initial_stop_loss and t.entry_price and t.entry_price != t.initial_stop_loss:
                risk = abs(t.entry_price - t.initial_stop_loss)
                pnl = t.net_pnl or 0
                r_mult = pnl / (risk * t.quantity) if t.quantity > 0 else 0
                r_multiples.append(r_mult)
                
        average_r = sum(r_multiples) / len(r_multiples) if r_multiples else None
        expectancy = (win_rate * average_win) - ((1 - win_rate) * average_loss)
        
        import math
        
        # Calculate Sharpe, Sortino and Equity Curve
        equity_curve = []
        returns = []
        sharpe_ratio = 0.0
        sortino_ratio = 0.0
        sqn = 0.0
        
        if total_trades > 0:
            # Simple assumption: risk free rate is 0
            # Calculate standard deviation of trade returns
            avg_pnl = total_net_pnl / total_trades
            
            variance = sum(((t.net_pnl or 0) - avg_pnl) ** 2 for t in trades) / total_trades
            std_dev = math.sqrt(variance) if variance > 0 else 1.0
            
            # SQN (System Quality Number) = sqrt(trades) * (Average R / StdDev R)
            # Here using absolute PnL for simplicity if R is not available
            sqn = (math.sqrt(total_trades) * avg_pnl) / std_dev if std_dev > 0 else 0
            
            # Sharpe = avg_return / std_dev
            sharpe_ratio = avg_pnl / std_dev if std_dev > 0 else 0
            
            # Sortino
            downside_variance = sum(min(0, t.net_pnl or 0) ** 2 for t in trades) / total_trades
            downside_std_dev = math.sqrt(downside_variance) if downside_variance > 0 else 1.0
            sortino_ratio = avg_pnl / downside_std_dev if downside_std_dev > 0 else 0
            
            # Build Equity Curve
            current_equity = 100000.0 # Base
            peak_equity = current_equity
            
            for t in trades:
                current_equity += (t.net_pnl or 0)
                if current_equity > peak_equity:
                    peak_equity = current_equity
                drawdown = peak_equity - current_equity
                
                timestamp_str = t.exit_date.isoformat() if t.exit_date else ""
                equity_curve.append({
                    "timestamp": timestamp_str,
                    "equity": current_equity,
                    "drawdown": drawdown
                })
        
        return AnalyticsResponse(
            total_trades=total_trades,
            win_rate=win_rate,
            total_net_pnl=total_net_pnl,
            average_win=average_win,
            average_loss=average_loss,
            profit_factor=profit_factor,
            average_r=average_r,
            expectancy=expectancy,
            largest_win=largest_win,
            largest_loss=largest_loss,
            max_drawdown=max_drawdown,
            sharpe_ratio=sharpe_ratio,
            sortino_ratio=sortino_ratio,
            sqn=sqn,
            setup_performance=setup_performance,
            equity_curve=equity_curve
        )
