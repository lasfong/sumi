import logging
import math
import pandas as pd
import zipfile
from typing import List, Tuple, Set, Optional
from io import BytesIO
from sqlalchemy.orm import Session

from app.models.candle import Candle
from app.models.symbol import Symbol
from app.schemas.import_schema import ImportResponse, ImportWarning

logger = logging.getLogger(__name__)

class CafeFImporter:
    PRICE_SCALE_FACTOR = 1000.0
    # CafeF SolieuGD exports split/dividend-adjusted prices in kVND rounded to 4 decimal places (0.1 VND).
    # <Open>, <High>, <Low> use a 4-decimal truncated cumulative factor k_4dp = floor(1e4 * k_exact) / 1e4,
    # whereas <Close> uses k_exact. On sessions closing at intraday high/low prior to a corporate action,
    # Close exceeds High (or falls below Low) by up to P_unadj * 1e-4 kVND (<= 6 bps of High, max 25 VND).
    SPLIT_FACTOR_TRUNCATION_REL_TOL = 0.0006
    SPLIT_FACTOR_TRUNCATION_ABS_MAX_VND = 25.0
    SINGLE_ULP_ABS_TOL_VND = 0.100001
    NORMALIZED_PROVENANCE_PREFIX = "[NORMALIZED_OHLC_PRECISION]"

    KNOWN_INDEX_SYMBOLS: Set[str] = {
        "VNINDEX", "VN-INDEX", "HNX-INDEX", "HNXINDEX", "UPCOM-INDEX", "UPCOMINDEX",
        "VN30", "VN100", "VNALL", "VNMID", "VNSML",
        "HNX30", "HNXLCAP", "HNXSMCAP", "HNXFIN", "HNXCON", "HNXMAN",
        "VNX50", "VNXALL", "VN30F1M", "VN30F2M",
        "VNSI", "VNHEAL", "VNFIN", "VNREAL", "VNMAT", "VNIT", "VNIND", "VNENE",
        "VNDIAMOND", "VNFINLEAD", "VNFINSELECT",
    }

    @classmethod
    def is_index_symbol(cls, symbol: Optional[str]) -> bool:
        if not symbol:
            return False
        s = str(symbol).strip().upper()
        if s.startswith("^") or "INDEX" in s:
            return True
        return s in cls.KNOWN_INDEX_SYMBOLS

    @classmethod
    def is_normalized_provenance(cls, reject_reason: Optional[str]) -> bool:
        return bool(reject_reason and str(reject_reason).startswith(cls.NORMALIZED_PROVENANCE_PREFIX))

    @classmethod
    def evaluate_and_normalize_ohlc(
        cls,
        symbol: Optional[str],
        o_val: float,
        h_val: float,
        l_val: float,
        c_val: float,
        v_val: float = 0.0,
        raw_kvnd: Optional[Tuple[float, float, float, float]] = None,
    ) -> Tuple[float, float, float, float, bool, Optional[str]]:
        """
        Evaluate whether an OHLC envelope discrepancy (Open or Close outside [Low, High])
        is a deterministic CafeF split-adjusted 4-decimal truncation/rounding artifact,
        and if so, safely normalize High = max(O, H, L, C) and Low = min(O, H, L, C)
        without modifying Open, Close, or Volume.

        Returns:
            (open_val, norm_high, norm_low, close_val, is_corrected, provenance_reason)
        """
        if any(math.isnan(x) or math.isinf(x) for x in (o_val, h_val, l_val, c_val, v_val)):
            return o_val, h_val, l_val, c_val, False, None

        if o_val <= 0 or h_val <= 0 or l_val <= 0 or c_val <= 0 or v_val < 0:
            return o_val, h_val, l_val, c_val, False, None

        if l_val > h_val:
            return o_val, h_val, l_val, c_val, False, None

        if (l_val <= o_val <= h_val) and (l_val <= c_val <= h_val):
            return o_val, h_val, l_val, c_val, False, None

        # Rule 1: Only equity symbols undergo CafeF split-adjusted kVND truncation.
        # Market indices (VNINDEX, VN30, HNX-INDEX, etc.) are unadjusted point series.
        if cls.is_index_symbol(symbol):
            return o_val, h_val, l_val, c_val, False, None

        # Determine whether caller passed VND-scaled prices or unscaled kVND prices
        if raw_kvnd is not None:
            scale_to_vnd = 1.0
        elif max(o_val, h_val, l_val, c_val) < 1000.0 and any(
            abs(v * 100.0 - round(v * 100.0)) > 1e-3 for v in (o_val, h_val, l_val, c_val)
        ):
            scale_to_vnd = cls.PRICE_SCALE_FACTOR
        else:
            scale_to_vnd = 1.0

        o_vnd = round(o_val * scale_to_vnd, 4)
        h_vnd = round(h_val * scale_to_vnd, 4)
        l_vnd = round(l_val * scale_to_vnd, 4)
        c_vnd = round(c_val * scale_to_vnd, 4)

        # Rule 2: Must exhibit split-adjusted sub-10-VND fractional precision
        # (3rd or 4th decimal place in kVND, i.e., not all integer multiples of 10 VND).
        # Unadjusted Vietnam equity prices are quoted in 10/50/100 VND ticks and are never altered.
        has_sub_10vnd = any(
            abs(v - round(v / 10.0) * 10.0) > 1e-3 for v in (o_vnd, h_vnd, l_vnd, c_vnd)
        )
        if not has_sub_10vnd:
            return o_val, h_val, l_val, c_val, False, None

        # Compute envelope overshoot in VND
        open_over_vnd = round(max(0.0, o_vnd - h_vnd, l_vnd - o_vnd), 4)
        close_over_vnd = round(max(0.0, c_vnd - h_vnd, l_vnd - c_vnd), 4)
        delta_vnd = max(open_over_vnd, close_over_vnd)

        if delta_vnd <= 0.0:
            return o_val, h_val, l_val, c_val, False, None

        # Rule 3: In CafeF SolieuGD, <Open>, <High>, <Low> share the same 4-decimal truncated
        # adjustment factor, so Open can only differ from [Low, High] by at most 1 ULP (0.1 VND).
        if open_over_vnd > cls.SINGLE_ULP_ABS_TOL_VND:
            return o_val, h_val, l_val, c_val, False, None

        # Rule 4: <Close> uses the un-truncated adjustment factor vs 4-decimal truncated factor
        # on <High>/<Low>, bounded by either 1 ULP (0.1 VND) or (<= 6 bps of High AND <= 25.0 VND).
        within_ulp = delta_vnd <= cls.SINGLE_ULP_ABS_TOL_VND
        within_factor_truncation = (
            (delta_vnd / h_vnd) <= cls.SPLIT_FACTOR_TRUNCATION_REL_TOL
            and delta_vnd <= cls.SPLIT_FACTOR_TRUNCATION_ABS_MAX_VND
        )
        if not (within_ulp or within_factor_truncation):
            return o_val, h_val, l_val, c_val, False, None

        # Rule 5: Sub-tick step check — discrepancy must be strictly less than half of the
        # smallest positive non-violating intraday step on the bar.
        if open_over_vnd == 0.0:
            non_violating_steps = [
                d for d in (h_vnd - l_vnd, h_vnd - o_vnd, o_vnd - l_vnd) if d > 1e-6
            ]
        else:
            non_violating_steps = [d for d in (h_vnd - l_vnd,) if d > 1e-6]

        if non_violating_steps:
            min_step_vnd = min(non_violating_steps)
            if delta_vnd >= 0.5 * min_step_vnd:
                return o_val, h_val, l_val, c_val, False, None

        # Safe deterministic envelope normalization: expand High/Low to include Open/Close.
        # Open, Close, and Volume are strictly preserved.
        norm_h = round(max(o_val, h_val, l_val, c_val), 4)
        norm_l = round(min(o_val, h_val, l_val, c_val), 4)
        norm_h_vnd = round(max(o_vnd, h_vnd, l_vnd, c_vnd), 4)
        norm_l_vnd = round(min(o_vnd, h_vnd, l_vnd, c_vnd), 4)

        if raw_kvnd is not None:
            rk_o, rk_h, rk_l, rk_c = raw_kvnd
        else:
            rk_o = round(o_vnd / cls.PRICE_SCALE_FACTOR, 4)
            rk_h = round(h_vnd / cls.PRICE_SCALE_FACTOR, 4)
            rk_l = round(l_vnd / cls.PRICE_SCALE_FACTOR, 4)
            rk_c = round(c_vnd / cls.PRICE_SCALE_FACTOR, 4)

        reason = (
            f"{cls.NORMALIZED_PROVENANCE_PREFIX} Chuẩn hóa biên OHLC do sai số làm tròn 4 chữ số thập phân "
            f"giá điều chỉnh CafeF: raw_kvnd=({rk_o:g}, {rk_h:g}, {rk_l:g}, {rk_c:g}), "
            f"raw_open={o_vnd:g}, raw_high={h_vnd:g}, raw_low={l_vnd:g}, raw_close={c_vnd:g} "
            f"-> high={norm_h_vnd:g}, low={norm_l_vnd:g} (delta_vnd={delta_vnd:g})"
        )
        return o_val, norm_h, norm_l, c_val, True, reason

    @classmethod
    def parse_file(cls, file_content: bytes, filename: str) -> pd.DataFrame:
        try:
            df = pd.read_csv(BytesIO(file_content))
        except Exception:
            df = pd.read_csv(BytesIO(file_content), on_bad_lines='skip', engine='python')

        col_map = {}
        for col in df.columns:
            lower_col = col.lower().strip('<>')
            if 'ticker' in lower_col or 'symbol' in lower_col:
                col_map[col] = 'symbol'
            elif 'dt' in lower_col or 'date' in lower_col:
                col_map[col] = 'timestamp'
            elif 'open' in lower_col:
                col_map[col] = 'open'
            elif 'high' in lower_col:
                col_map[col] = 'high'
            elif 'low' in lower_col:
                col_map[col] = 'low'
            elif 'close' in lower_col:
                col_map[col] = 'close'
            elif 'volume' in lower_col or 'vol' in lower_col:
                col_map[col] = 'volume'

        df.rename(columns=col_map, inplace=True)

        if 'symbol' in df.columns:
            df['symbol'] = df['symbol'].astype(str).str.strip().str.upper()

        if 'timestamp' in df.columns:
            try:
                df['timestamp'] = pd.to_datetime(df['timestamp'], format='%Y%m%d').dt.date
            except Exception:
                try:
                    df['timestamp'] = pd.to_datetime(df['timestamp']).dt.date
                except Exception:
                    pass

        # Sort chronologically by symbol, timestamp ascending (fixing DF-01)
        if 'symbol' in df.columns and 'timestamp' in df.columns:
            temp_dt = pd.to_datetime(df['timestamp'], errors='coerce')
            df['__sort_dt__'] = temp_dt
            df.sort_values(by=['symbol', '__sort_dt__'], ascending=[True, True], inplace=True)
            df.drop(columns=['__sort_dt__'], inplace=True)
            df.reset_index(drop=True, inplace=True)

        # Scale equities by 1,000 to convert CafeF quoted prices to actual VND,
        # while preserving index values (VNINDEX, HNX-INDEX, VN30, etc.) as point values (fixing FIN-01)
        price_cols = [c for c in ['open', 'high', 'low', 'close'] if c in df.columns]
        if price_cols and 'symbol' in df.columns:
            is_index_mask = df['symbol'].apply(cls.is_index_symbol)
            equity_mask = ~is_index_mask
            for col in price_cols:
                numeric_vals = pd.to_numeric(df[col], errors='coerce')
                df[f'__raw_{col}_kvnd__'] = numeric_vals
                scaled_series = df[col].copy()
                valid_equity = equity_mask & numeric_vals.notna()
                scaled_series.loc[valid_equity] = (numeric_vals.loc[valid_equity] * cls.PRICE_SCALE_FACTOR).round(4)
                df[col] = scaled_series
                df[f'__raw_{col}__'] = scaled_series.copy()

            # Evaluate and normalize deterministic CafeF split-adjusted OHLC precision discrepancies
            if len(price_cols) == 4:
                o_num = pd.to_numeric(df['open'], errors='coerce')
                h_num = pd.to_numeric(df['high'], errors='coerce')
                l_num = pd.to_numeric(df['low'], errors='coerce')
                c_num = pd.to_numeric(df['close'], errors='coerce')
                v_num = (
                    pd.to_numeric(df['volume'], errors='coerce')
                    if 'volume' in df.columns
                    else pd.Series(0.0, index=df.index)
                )

                ohlc_violation_mask = (
                    o_num.notna() & h_num.notna() & l_num.notna() & c_num.notna()
                    & (o_num > 0) & (h_num > 0) & (l_num > 0) & (c_num > 0)
                    & (l_num <= h_num)
                    & ((o_num < l_num) | (o_num > h_num) | (c_num < l_num) | (c_num > h_num))
                )
                df['__ohlc_corrected__'] = False
                df['__correction_reason__'] = None

                if ohlc_violation_mask.any():
                    for idx in df.index[ohlc_violation_mask]:
                        sym_val = df.at[idx, 'symbol']
                        o_v = float(o_num.at[idx])
                        h_v = float(h_num.at[idx])
                        l_v = float(l_num.at[idx])
                        c_v = float(c_num.at[idx])
                        v_v = float(v_num.at[idx]) if pd.notna(v_num.at[idx]) else 0.0
                        if v_v < 0:
                            continue
                        rk_o = float(df.at[idx, '__raw_open_kvnd__']) if pd.notna(df.at[idx, '__raw_open_kvnd__']) else o_v / cls.PRICE_SCALE_FACTOR
                        rk_h = float(df.at[idx, '__raw_high_kvnd__']) if pd.notna(df.at[idx, '__raw_high_kvnd__']) else h_v / cls.PRICE_SCALE_FACTOR
                        rk_l = float(df.at[idx, '__raw_low_kvnd__']) if pd.notna(df.at[idx, '__raw_low_kvnd__']) else l_v / cls.PRICE_SCALE_FACTOR
                        rk_c = float(df.at[idx, '__raw_close_kvnd__']) if pd.notna(df.at[idx, '__raw_close_kvnd__']) else c_v / cls.PRICE_SCALE_FACTOR

                        _, norm_h, norm_l, _, is_corr, reason = cls.evaluate_and_normalize_ohlc(
                            sym_val, o_v, h_v, l_v, c_v, v_v, raw_kvnd=(rk_o, rk_h, rk_l, rk_c)
                        )
                        if is_corr:
                            df.at[idx, 'high'] = norm_h
                            df.at[idx, 'low'] = norm_l
                            df.at[idx, '__ohlc_corrected__'] = True
                            df.at[idx, '__correction_reason__'] = reason

        return df

    @classmethod
    def parse_zip(cls, file_content: bytes) -> pd.DataFrame:
        frames = []
        with zipfile.ZipFile(BytesIO(file_content)) as zf:
            for name in zf.namelist():
                if name.lower().endswith(('.csv', '.txt')) and not name.startswith('__MACOSX'):
                    with zf.open(name) as f:
                        content = f.read()
                        df = cls.parse_file(content, name)
                        frames.append(df)
        if not frames:
            raise ValueError("No CSV/TXT files found inside ZIP archive")
        combined_df = pd.concat(frames, ignore_index=True)
        if 'symbol' in combined_df.columns and 'timestamp' in combined_df.columns:
            temp_dt = pd.to_datetime(combined_df['timestamp'], errors='coerce')
            combined_df['__sort_dt__'] = temp_dt
            combined_df.sort_values(by=['symbol', '__sort_dt__'], ascending=[True, True], inplace=True)
            combined_df.drop(columns=['__sort_dt__'], inplace=True)
            combined_df.reset_index(drop=True, inplace=True)
        return combined_df

    @staticmethod
    def _detect_exchange_from_filename(filename: str) -> Optional[str]:
        filename_upper = filename.upper()
        if "HSX" in filename_upper or "HOSE" in filename_upper:
            return "HOSE"
        elif "HNX" in filename_upper:
            return "HNX"
        elif "UPCOM" in filename_upper:
            return "UPCOM"
        elif "INDEX" in filename_upper:
            return "INDEX"
        return None

    @staticmethod
    def preview(
        db: Session,
        file_content: bytes,
        filename: str,
        adjustment_type: str = "unadjusted"
    ):
        from app.services.import_workflow_service import ImportWorkflowService
        return ImportWorkflowService.preview_import(
            db=db,
            file_content=file_content,
            filename=filename,
            source_type="cafef",
            timeframe="1D",
            adjustment_type=adjustment_type
        )

    @staticmethod
    def accept(
        db: Session,
        run_id: str,
        content_sha256: str
    ):
        from app.services.import_workflow_service import ImportWorkflowService
        return ImportWorkflowService.accept_import(
            db=db,
            run_id=run_id,
            content_sha256=content_sha256
        )

    @staticmethod
    def import_data(
        db: Session,
        file_content: bytes,
        filename: str,
        adjustment_type: str = "unadjusted",
        confirm_accept: bool = False,
        run_id: str = None,
        content_sha256: str = None
    ) -> ImportResponse:
        from app.services.import_workflow_service import ImportWorkflowService
        from app.models.import_run import ImportRun, ImportRunItem

        if not confirm_accept or not run_id or not content_sha256:
            preview = ImportWorkflowService.preview_import(
                db=db,
                file_content=file_content,
                filename=filename,
                source_type="cafef",
                timeframe="1D",
                adjustment_type=adjustment_type
            )
            raise RuntimeError(
                f"Tự động chấp nhận bị cấm. Bản xem trước đã tạo run_id='{preview.run_id}' và content_sha256='{preview.content_sha256}'. "
                f"Để chấp nhận chính thức, cần gọi lại với confirm_accept=True, run_id='{preview.run_id}' và content_sha256='{preview.content_sha256}'."
            )

        run = db.query(ImportRun).filter_by(id=run_id).one_or_none()
        if not run or run.content_sha256 != content_sha256:
            raise ValueError("Mã run_id hoặc content_sha256 xác nhận không hợp lệ hoặc không trùng khớp với bản xem trước")

        items = db.query(ImportRunItem).filter_by(run_id=run_id).all()
        symbols = set(item.symbol for item in items if item.symbol)
        parsed_dates = [item.timestamp for item in items if item.classification == "parsed" and item.timestamp]
        start_date = str(min(parsed_dates)) if parsed_dates else None
        end_date = str(max(parsed_dates)) if parsed_dates else None

        warnings = [
            ImportWarning(row_index=item.row_index, message=item.reject_reason or item.classification)
            for item in items
            if item.classification != "parsed" or CafeFImporter.is_normalized_provenance(item.reject_reason)
        ]
        if run.block_reason:
            warnings.insert(0, ImportWarning(row_index=0, message=run.block_reason))

        rejected = sum(1 for it in items if it.classification == "rejected")
        conflicting = sum(1 for it in items if it.classification == "conflicting")
        duplicates = sum(1 for it in items if it.classification == "duplicate")
        corrected = sum(
            1 for it in items
            if it.classification == "parsed" and CafeFImporter.is_normalized_provenance(it.reject_reason)
        )

        if not run.can_accept:
            return ImportResponse(
                imported_rows=0,
                skipped_rows=rejected + conflicting,
                duplicate_rows=duplicates,
                corrected_rows=corrected,
                is_partial=False,
                partial_warning=None,
                symbols_count=len(symbols),
                start_date=start_date,
                end_date=end_date,
                warnings=warnings[:100]
            )

        accept_res = ImportWorkflowService.accept_import(
            db=db,
            run_id=run_id,
            content_sha256=content_sha256
        )

        return ImportResponse(
            imported_rows=accept_res.accepted_count,
            skipped_rows=rejected + conflicting,
            duplicate_rows=duplicates,
            corrected_rows=corrected,
            is_partial=accept_res.is_partial,
            partial_warning=accept_res.partial_warning,
            symbols_count=len(symbols),
            start_date=start_date,
            end_date=end_date,
            warnings=warnings[:100]
        )
