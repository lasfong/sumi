import pandas as pd
import zipfile
from typing import List, Tuple
from io import BytesIO
from sqlalchemy.orm import Session

from app.models.candle import Candle
from app.models.symbol import Symbol
from app.schemas.import_schema import ImportResponse, ImportWarning

class CafeFImporter:
    @staticmethod
    def parse_file(file_content: bytes, filename: str) -> pd.DataFrame:
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

        if 'timestamp' in df.columns:
            try:
                df['timestamp'] = pd.to_datetime(df['timestamp'], format='%Y%m%d').dt.date
            except Exception:
                try:
                    df['timestamp'] = pd.to_datetime(df['timestamp']).dt.date
                except Exception:
                    pass

        return df

    @staticmethod
    def parse_zip(file_content: bytes) -> pd.DataFrame:
        frames = []
        with zipfile.ZipFile(BytesIO(file_content)) as zf:
            for name in zf.namelist():
                if name.lower().endswith(('.csv', '.txt')) and not name.startswith('__MACOSX'):
                    with zf.open(name) as f:
                        content = f.read()
                        df = CafeFImporter.parse_file(content, name)
                        frames.append(df)
        if not frames:
            raise ValueError("No CSV/TXT files found inside ZIP archive")
        return pd.concat(frames, ignore_index=True)

    @staticmethod
    def _detect_exchange_from_filename(filename: str) -> str:
        filename_upper = filename.upper()
        if "HSX" in filename_upper or "HOSE" in filename_upper:
            return "HOSE"
        elif "HNX" in filename_upper:
            return "HNX"
        elif "UPCOM" in filename_upper:
            return "UPCOM"
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
            for item in items if item.classification != "parsed"
        ]
        if run.block_reason:
            warnings.insert(0, ImportWarning(row_index=0, message=run.block_reason))

        rejected = sum(1 for it in items if it.classification == "rejected")
        conflicting = sum(1 for it in items if it.classification == "conflicting")
        duplicates = sum(1 for it in items if it.classification == "duplicate")

        if not run.can_accept:
            return ImportResponse(
                imported_rows=0,
                skipped_rows=rejected + conflicting,
                duplicate_rows=duplicates,
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
            symbols_count=len(symbols),
            start_date=start_date,
            warnings=warnings[:100]
        )
