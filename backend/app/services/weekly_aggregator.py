import json
from datetime import datetime, date, time, timedelta
from typing import List, Dict, Tuple, Optional, Set
from sqlalchemy.orm import Session

from app.models.candle import Candle
from app.models.import_run import WeeklyCandleProvenance

class WeeklyAggregator:
    RULE_VERSION = "VN_TRADING_WEEK_V1"

    @staticmethod
    def derive_weekly_candles(
        db: Session,
        symbols: Set[str],
        adjustment_types: Set[str]
    ) -> int:
        """
        Derives and updates Weekly (1W) candles from accepted Daily (1D) candles using VN_TRADING_WEEK_V1.
        Returns the count of weekly candles updated/inserted.
        """
        updated_count = 0

        for symbol in symbols:
            for adj_type in adjustment_types:
                # Fetch all 1D candles for (symbol, adj_type)
                daily_candles = (
                    db.query(Candle)
                    .filter(
                        Candle.symbol == symbol,
                        Candle.timeframe == "1D",
                        Candle.adjustment_type == adj_type
                    )
                    .order_by(Candle.timestamp.asc())
                    .all()
                )

                if not daily_candles:
                    # Clean up all 1W candles and provenance if no 1D candles remain
                    db.query(Candle).filter(
                        Candle.symbol == symbol,
                        Candle.timeframe == "1W",
                        Candle.adjustment_type == adj_type
                    ).delete()
                    db.query(WeeklyCandleProvenance).filter(
                        WeeklyCandleProvenance.symbol == symbol,
                        WeeklyCandleProvenance.adjustment_type == adj_type
                    ).delete()
                    continue

                # Group daily candles by week (Monday-start)
                weeks_map: Dict[date, List[Candle]] = {}
                for c in daily_candles:
                    c_date = c.timestamp.date() if isinstance(c.timestamp, datetime) else c.timestamp
                    # Only accept Monday-Friday (0-4)
                    if c_date.weekday() < 5:
                        week_start = c_date - timedelta(days=c_date.weekday())
                        if week_start not in weeks_map:
                            weeks_map[week_start] = []
                        weeks_map[week_start].append(c)

                # Query existing 1W provenance
                existing_provs = (
                    db.query(WeeklyCandleProvenance)
                    .filter(
                        WeeklyCandleProvenance.symbol == symbol,
                        WeeklyCandleProvenance.adjustment_type == adj_type
                    )
                    .all()
                )
                existing_weeks = {p.week_start_date: p for p in existing_provs}

                active_week_starts = set()

                for week_start, members in weeks_map.items():
                    week_start_str = week_start.strftime("%Y-%m-%d")
                    active_week_starts.add(week_start_str)

                    members_sorted = sorted(
                        members,
                        key=lambda x: x.timestamp.date() if isinstance(x.timestamp, datetime) else x.timestamp
                    )

                    first_c = members_sorted[0]
                    last_c = members_sorted[-1]

                    w_open = float(first_c.open)
                    w_high = max(float(c.high) for c in members_sorted)
                    w_low = min(float(c.low) for c in members_sorted)
                    w_close = float(last_c.close)
                    w_volume = sum(float(c.volume) for c in members_sorted)

                    last_date = last_c.timestamp.date() if isinstance(last_c.timestamp, datetime) else last_c.timestamp
                    w_timestamp = datetime.combine(last_date, time.min)

                    member_keys = [
                        (c.timestamp.date() if isinstance(c.timestamp, datetime) else c.timestamp).strftime("%Y-%m-%d")
                        for c in members_sorted
                    ]
                    source_runs = list(set([c.source for c in members_sorted if c.source]))

                    # Find existing weekly candle for this symbol/adj_type/week_start
                    prov = existing_weeks.get(week_start_str)

                    # Delete any existing 1W candle for this symbol/adj_type whose date falls in this week
                    week_end = week_start + timedelta(days=6)
                    old_candles = (
                        db.query(Candle)
                        .filter(
                            Candle.symbol == symbol,
                            Candle.timeframe == "1W",
                            Candle.adjustment_type == adj_type
                        )
                        .all()
                    )
                    for ow in old_candles:
                        ow_d = ow.timestamp.date() if isinstance(ow.timestamp, datetime) else ow.timestamp
                        if week_start <= ow_d <= week_end and ow_d != last_date:
                            db.delete(ow)

                    weekly_candle = (
                        db.query(Candle)
                        .filter(
                            Candle.symbol == symbol,
                            Candle.timeframe == "1W",
                            Candle.adjustment_type == adj_type,
                            Candle.timestamp == w_timestamp
                        )
                        .first()
                    )

                    if not weekly_candle:
                        weekly_candle = Candle(
                            symbol=symbol,
                            timeframe="1W",
                            timestamp=w_timestamp,
                            open=w_open,
                            high=w_high,
                            low=w_low,
                            close=w_close,
                            volume=w_volume,
                            source="VN_TRADING_WEEK_V1",
                            adjustment_type=adj_type
                        )
                        db.add(weekly_candle)
                    else:
                        weekly_candle.open = w_open
                        weekly_candle.high = w_high
                        weekly_candle.low = w_low
                        weekly_candle.close = w_close
                        weekly_candle.volume = w_volume

                    if not prov:
                        prov = WeeklyCandleProvenance(
                            symbol=symbol,
                            adjustment_type=adj_type,
                            week_start_date=week_start_str,
                            weekly_timestamp=w_timestamp,
                            rule_version=WeeklyAggregator.RULE_VERSION,
                            daily_member_keys_json=json.dumps(member_keys),
                            source_run_ids_json=json.dumps(source_runs)
                        )
                        db.add(prov)
                    else:
                        prov.weekly_timestamp = w_timestamp
                        prov.daily_member_keys_json = json.dumps(member_keys)
                        prov.source_run_ids_json = json.dumps(source_runs)

                    updated_count += 1

                # Clean up inactive weeks
                for week_start_str, prov in existing_weeks.items():
                    if week_start_str not in active_week_starts:
                        db.query(Candle).filter(
                            Candle.symbol == symbol,
                            Candle.timeframe == "1W",
                            Candle.adjustment_type == adj_type,
                            Candle.timestamp == prov.weekly_timestamp
                        ).delete()
                        db.delete(prov)

        db.flush()
        return updated_count
