import math
import hashlib
from datetime import datetime, date, timedelta
from typing import List, Dict, Tuple, Optional, Any, Set
import pandas as pd

class ClassifiedRow:
    def __init__(
        self,
        row_index: int,
        symbol: str,
        timeframe: str,
        timestamp: date,
        adjustment_type: str,
        open: Optional[float],
        high: Optional[float],
        low: Optional[float],
        close: Optional[float],
        volume: Optional[float],
        classification: str,
        reject_reason: Optional[str] = None
    ):
        self.row_index = row_index
        self.symbol = symbol
        self.timeframe = timeframe
        self.timestamp = timestamp
        self.adjustment_type = adjustment_type
        self.open = open
        self.high = high
        self.low = low
        self.close = close
        self.volume = volume
        self.classification = classification
        self.reject_reason = reject_reason

    def to_dict(self) -> Dict[str, Any]:
        return {
            "row_index": self.row_index,
            "symbol": self.symbol,
            "timeframe": self.timeframe,
            "timestamp": self.timestamp.isoformat() if isinstance(self.timestamp, (date, datetime)) else str(self.timestamp),
            "adjustment_type": self.adjustment_type,
            "open": self.open,
            "high": self.high,
            "low": self.low,
            "close": self.close,
            "volume": self.volume,
            "classification": self.classification,
            "reject_reason": self.reject_reason,
        }


ALLOWED_SOURCE_TYPES: Set[str] = {"cafef", "manual_upload", "local_file", "vnstock", "sumi_demo"}
ALLOWED_TIMEFRAMES: Set[str] = {"1D", "D", "1W", "W"}
ALLOWED_ADJUSTMENT_TYPES: Set[str] = {"unadjusted", "adjusted", "split"}
ALLOWED_TIMEZONES: Set[str] = {"Asia/Ho_Chi_Minh", "UTC", "Asia/Saigon"}


class ImportClassifier:
    @staticmethod
    def validate_metadata(
        source_type: str,
        timeframe: str,
        adjustment_type: str,
        timezone_str: str
    ) -> Optional[str]:
        if source_type not in ALLOWED_SOURCE_TYPES:
            return f"Nguồn dữ liệu không hợp lệ hoặc không được hỗ trợ: {source_type}"
        if timeframe not in ALLOWED_TIMEFRAMES:
            return f"Khung thời gian không hợp lệ hoặc không được hỗ trợ: {timeframe}"
        if adjustment_type not in ALLOWED_ADJUSTMENT_TYPES:
            return f"Kiểu điều chỉnh không hợp lệ hoặc không được hỗ trợ: {adjustment_type}"
        if timezone_str not in ALLOWED_TIMEZONES:
            return f"Múi giờ không hợp lệ hoặc không được hỗ trợ: {timezone_str}"
        return None

    @staticmethod
    def compute_sha256(content: bytes) -> str:
        return hashlib.sha256(content).hexdigest()

    @staticmethod
    def compute_semantic_checksum(rows: List[ClassifiedRow]) -> str:
        """Compute SHA256 over normalized, valid candidate rows."""
        valid_rows = [r for r in rows if r.classification in ('parsed', 'duplicate')]
        valid_rows_sorted = sorted(valid_rows, key=lambda x: (x.symbol, x.timestamp))
        lines = []
        for r in valid_rows_sorted:
            lines.append(f"{r.symbol}|{r.timeframe}|{r.timestamp}|{r.adjustment_type}|{r.open}|{r.high}|{r.low}|{r.close}|{r.volume}")
        raw_str = "\n".join(lines)
        return hashlib.sha256(raw_str.encode('utf-8')).hexdigest()

    @staticmethod
    def classify_records(
        raw_df: pd.DataFrame,
        existing_candles_map: Dict[Tuple[str, str, date, str], Tuple[float, float, float, float, float]],
        timeframe: str = "1D",
        adjustment_type: str = "unadjusted"
    ) -> Tuple[List[ClassifiedRow], Dict[str, int], bool, Optional[str]]:
        """
        Classifies raw rows into parsed, rejected, duplicate, conflicting, missing, out_of_order.
        
        Returns:
            (items, counts, can_accept, block_reason)
        """
        items: List[ClassifiedRow] = []
        seen_file_keys: Dict[Tuple[str, str, date, str], Tuple[float, float, float, float, float]] = {}
        last_timestamp_by_symbol: Dict[str, date] = {}
        out_of_order_symbols: Set[str] = set()

        counts = {
            "parsed": 0,
            "rejected": 0,
            "duplicate": 0,
            "conflicting": 0,
            "missing": 0,
            "out_of_order": 0,
            "accepted": 0
        }

        if raw_df is None or raw_df.empty:
            return [], counts, False, "Tập tin rỗng hoặc không có dữ liệu hợp lệ"

        # Check required columns
        req_cols = ['symbol', 'timestamp', 'open', 'high', 'low', 'close', 'volume']
        missing_cols = [c for c in req_cols if c not in raw_df.columns]
        if missing_cols:
            return [], counts, False, f"Thiếu các cột bắt buộc: {', '.join(missing_cols)}"

        symbol_valid_dates: Dict[str, Set[date]] = {}

        for idx, row in raw_df.iterrows():
            row_num = idx + 1 if isinstance(idx, int) else 1

            raw_sym = row.get('symbol')
            raw_ts = row.get('timestamp')
            raw_o = row.get('open')
            raw_h = row.get('high')
            raw_l = row.get('low')
            raw_c = row.get('close')
            raw_v = row.get('volume')

            # 1. Symbol check
            if pd.isna(raw_sym) or not str(raw_sym).strip():
                items.append(ClassifiedRow(row_num, "", timeframe, date.today(), adjustment_type, None, None, None, None, None, "rejected", "Mã chứng khoán bị trống"))
                counts["rejected"] += 1
                continue
            
            sym = str(raw_sym).strip().upper()
            if not sym.isalnum() and not all(c in "._-" or c.isalnum() for c in sym) or len(sym) > 20:
                items.append(ClassifiedRow(row_num, sym, timeframe, date.today(), adjustment_type, None, None, None, None, None, "rejected", f"Mã chứng khoán không đúng định dạng ({sym})"))
                counts["rejected"] += 1
                continue


            # 2. Timestamp check
            parsed_date: Optional[date] = None
            if isinstance(raw_ts, (datetime, date)):
                parsed_date = raw_ts.date() if isinstance(raw_ts, datetime) else raw_ts
            elif isinstance(raw_ts, str):
                try:
                    parsed_date = pd.to_datetime(raw_ts).date()
                except Exception:
                    pass
            elif isinstance(raw_ts, (int, float)) and not pd.isna(raw_ts):
                ts_str = str(int(raw_ts))
                if len(ts_str) == 8:
                    try:
                        parsed_date = datetime.strptime(ts_str, "%Y%m%d").date()
                    except Exception:
                        pass
                else:
                    try:
                        parsed_date = pd.to_datetime(raw_ts, unit='s').date()
                    except Exception:
                        pass

            if parsed_date is None:
                items.append(ClassifiedRow(row_num, sym, timeframe, date.today(), adjustment_type, None, None, None, None, None, "rejected", f"Ngày giao dịch không hợp lệ ({raw_ts})"))
                counts["rejected"] += 1
                continue

            # Weekend check (Saturday=5, Sunday=6)
            if parsed_date.weekday() >= 5:
                items.append(ClassifiedRow(row_num, sym, timeframe, parsed_date, adjustment_type, None, None, None, None, None, "rejected", f"Ngày giao dịch rơi vào cuối tuần ({parsed_date.strftime('%Y-%m-%d')})"))
                counts["rejected"] += 1
                continue

            # 3. OHLCV check
            try:
                o_val = float(raw_o)
                h_val = float(raw_h)
                l_val = float(raw_l)
                c_val = float(raw_c)
                v_val = float(raw_v)
            except (ValueError, TypeError):
                items.append(ClassifiedRow(row_num, sym, timeframe, parsed_date, adjustment_type, None, None, None, None, None, "rejected", "Dữ liệu giá OHLCV không phải số"))
                counts["rejected"] += 1
                continue

            if any(math.isnan(x) or math.isinf(x) for x in (o_val, h_val, l_val, c_val, v_val)):
                items.append(ClassifiedRow(row_num, sym, timeframe, parsed_date, adjustment_type, None, None, None, None, None, "rejected", "Dữ liệu giá OHLCV chứa giá trị NaN/Inf"))
                counts["rejected"] += 1
                continue

            if o_val <= 0 or h_val <= 0 or l_val <= 0 or c_val <= 0 or v_val < 0:
                items.append(ClassifiedRow(row_num, sym, timeframe, parsed_date, adjustment_type, o_val, h_val, l_val, c_val, v_val, "rejected", "Giá phải dương và khối lượng không âm"))
                counts["rejected"] += 1
                continue

            if l_val > h_val:
                items.append(ClassifiedRow(row_num, sym, timeframe, parsed_date, adjustment_type, o_val, h_val, l_val, c_val, v_val, "rejected", f"Giá thấp nhất ({l_val}) lớn hơn giá cao nhất ({h_val})"))
                counts["rejected"] += 1
                continue

            if not (l_val <= o_val <= h_val) or not (l_val <= c_val <= h_val):
                items.append(ClassifiedRow(row_num, sym, timeframe, parsed_date, adjustment_type, o_val, h_val, l_val, c_val, v_val, "rejected", "Giá Mở/Đóng nằm ngoài khoảng [Thấp nhất, Cao nhất]"))
                counts["rejected"] += 1
                continue

            # 4. Out of order check
            is_out_of_order = False
            if sym in last_timestamp_by_symbol:
                if parsed_date <= last_timestamp_by_symbol[sym]:
                    is_out_of_order = True
            
            if not is_out_of_order:
                last_timestamp_by_symbol[sym] = parsed_date
            else:
                out_of_order_symbols.add(sym)

            # 5. Check duplicate or conflict within file or against DB
            candle_tuple = (o_val, h_val, l_val, c_val, v_val)
            key = (sym, timeframe, parsed_date, adjustment_type)

            if is_out_of_order:
                items.append(ClassifiedRow(row_num, sym, timeframe, parsed_date, adjustment_type, o_val, h_val, l_val, c_val, v_val, "out_of_order", f"Ngày {parsed_date} bị ngược thứ tự thời gian"))
                counts["out_of_order"] += 1
            elif key in seen_file_keys:
                prev_tuple = seen_file_keys[key]
                if prev_tuple == candle_tuple:
                    items.append(ClassifiedRow(row_num, sym, timeframe, parsed_date, adjustment_type, o_val, h_val, l_val, c_val, v_val, "duplicate", "Dữ liệu trùng lặp trong tập tin"))
                    counts["duplicate"] += 1
                else:
                    items.append(ClassifiedRow(row_num, sym, timeframe, parsed_date, adjustment_type, o_val, h_val, l_val, c_val, v_val, "conflicting", "Dữ liệu xung đột trong cùng tập tin"))
                    counts["conflicting"] += 1
            elif key in existing_candles_map:
                existing_tuple = existing_candles_map[key]
                if existing_tuple == candle_tuple:
                    items.append(ClassifiedRow(row_num, sym, timeframe, parsed_date, adjustment_type, o_val, h_val, l_val, c_val, v_val, "duplicate", "Dữ liệu trùng khớp với dữ liệu đã lưu trong hệ thống"))
                    counts["duplicate"] += 1
                else:
                    items.append(ClassifiedRow(row_num, sym, timeframe, parsed_date, adjustment_type, o_val, h_val, l_val, c_val, v_val, "conflicting", f"Dữ liệu xung đột với nến đã tồn tại (Đã có: O={existing_tuple[0]}, H={existing_tuple[1]}, L={existing_tuple[2]}, C={existing_tuple[3]}, V={existing_tuple[4]})"))
                    counts["conflicting"] += 1
            else:
                seen_file_keys[key] = candle_tuple
                items.append(ClassifiedRow(row_num, sym, timeframe, parsed_date, adjustment_type, o_val, h_val, l_val, c_val, v_val, "parsed"))
                counts["parsed"] += 1
                if sym not in symbol_valid_dates:
                    symbol_valid_dates[sym] = set()
                symbol_valid_dates[sym].add(parsed_date)

        # 6. Check missing weekday gaps per symbol
        for sym, dates_set in symbol_valid_dates.items():
            if len(dates_set) >= 2:
                min_d = min(dates_set)
                max_d = max(dates_set)
                curr = min_d + timedelta(days=1)
                while curr < max_d:
                    if curr.weekday() < 5:  # Mon-Fri
                        key = (sym, timeframe, curr, adjustment_type)
                        if curr not in dates_set and key not in existing_candles_map:
                            items.append(ClassifiedRow(
                                row_index=0,
                                symbol=sym,
                                timeframe=timeframe,
                                timestamp=curr,
                                adjustment_type=adjustment_type,
                                open=None, high=None, low=None, close=None, volume=None,
                                classification="missing",
                                reject_reason=f"Thiếu ngày giao dịch {curr.strftime('%Y-%m-%d')} trong chuỗi thời gian"
                            ))
                            counts["missing"] += 1
                    curr += timedelta(days=1)

        # Evaluate acceptance status
        can_accept = True
        block_reasons = []

        if counts["rejected"] > 0:
            can_accept = False
            block_reasons.append(f"Có {counts['rejected']} dòng dữ liệu không hợp lệ hoặc sai định dạng")
        if counts["conflicting"] > 0:
            can_accept = False
            block_reasons.append(f"Có {counts['conflicting']} dòng dữ liệu xung đột với giá trị đã lưu")
        if counts["out_of_order"] > 0:
            can_accept = False
            block_reasons.append(f"Có {counts['out_of_order']} dòng dữ liệu sai thứ tự thời gian")
        if counts["parsed"] == 0 and counts["duplicate"] == 0:
            can_accept = False
            block_reasons.append("Không có dữ liệu hợp lệ nào để nhập")

        block_reason_str = "; ".join(block_reasons) if block_reasons else None
        return items, counts, can_accept, block_reason_str
