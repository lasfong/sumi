import uuid
from datetime import datetime, timezone, date
from typing import List, Dict, Tuple, Optional, Set
from fastapi import HTTPException
from sqlalchemy.orm import Session
from sqlalchemy import func

from app.models.candle import Candle
from app.models.symbol import Symbol
from app.models.import_run import ImportRun, ImportRunItem, ImportRunMutation
from app.schemas.import_schema import (
    ImportPreviewResponse,
    ImportRunItemSchema,
    ImportAcceptResponse,
    ImportRollbackResponse,
    CatalogItemSchema
)
from app.services.cafef_importer import CafeFImporter
from app.services.import_classifier import ImportClassifier, ClassifiedRow
from app.services.weekly_aggregator import WeeklyAggregator

class ImportWorkflowService:
    @staticmethod
    def preview_import(
        db: Session,
        file_content: bytes,
        filename: str,
        source_type: str = "cafef",
        timeframe: str = "1D",
        adjustment_type: str = "unadjusted",
        timezone_str: str = "Asia/Ho_Chi_Minh"
    ) -> ImportPreviewResponse:
        file_sha256 = ImportClassifier.compute_sha256(file_content)

        # 1. Metadata validation fail-closed check
        metadata_err = ImportClassifier.validate_metadata(source_type, timeframe, adjustment_type, timezone_str)
        if metadata_err:
            run = ImportRun(
                id=str(uuid.uuid4()),
                file_name=filename,
                file_sha256=file_sha256,
                content_sha256=file_sha256,
                parser_version="cafef_v1",
                source_type=source_type,
                timeframe=timeframe,
                adjustment_type=adjustment_type,
                timezone=timezone_str,
                status="blocked",
                can_accept=False,
                block_reason=metadata_err
            )
            db.add(run)
            db.commit()
            return ImportPreviewResponse(
                run_id=run.id,
                file_name=filename,
                file_sha256=file_sha256,
                content_sha256=file_sha256,
                parser_version="cafef_v1",
                source_type=source_type,
                timeframe=timeframe,
                adjustment_type=adjustment_type,
                timezone=timezone_str,
                status="blocked",
                parsed_count=0,
                rejected_count=0,
                duplicate_count=0,
                conflicting_count=0,
                missing_count=0,
                out_of_order_count=0,
                accepted_count=0,
                can_accept=False,
                block_reason=metadata_err,
                items=[]
            )

        # Parse file into DataFrame
        try:
            if filename.lower().endswith('.zip'):
                raw_df = CafeFImporter.parse_zip(file_content)
            else:
                raw_df = CafeFImporter.parse_file(file_content, filename)
        except Exception as e:

            # Create a blocked run record for unparseable file
            run = ImportRun(
                id=str(uuid.uuid4()),
                file_name=filename,
                file_sha256=file_sha256,
                content_sha256=file_sha256,
                parser_version="cafef_v1",
                source_type=source_type,
                timeframe=timeframe,
                adjustment_type=adjustment_type,
                timezone=timezone_str,
                status="blocked",
                can_accept=False,
                block_reason=f"Không thể đọc hoặc giải mã tập tin: {str(e)}"
            )
            db.add(run)
            db.commit()
            return ImportPreviewResponse(
                run_id=run.id,
                file_name=filename,
                file_sha256=file_sha256,
                content_sha256=file_sha256,
                parser_version="cafef_v1",
                source_type=source_type,
                timeframe=timeframe,
                adjustment_type=adjustment_type,
                timezone=timezone_str,
                status="blocked",
                parsed_count=0,
                rejected_count=1,
                duplicate_count=0,
                conflicting_count=0,
                missing_count=0,
                out_of_order_count=0,
                accepted_count=0,
                can_accept=False,
                block_reason=run.block_reason,
                items=[]
            )

        # Build existing candles map for conflict/duplicate detection
        symbols_in_file = set()
        if 'symbol' in raw_df.columns:
            symbols_in_file = set(raw_df['symbol'].dropna().astype(str).str.strip().str.upper().unique())

        existing_candles_map: Dict[Tuple[str, str, date, str], Tuple[float, float, float, float, float]] = {}
        if symbols_in_file:
            db_candles = (
                db.query(Candle)
                .filter(
                    Candle.symbol.in_(list(symbols_in_file)),
                    Candle.timeframe == timeframe,
                    Candle.adjustment_type == adjustment_type
                )
                .all()
            )
            for c in db_candles:
                c_date = c.timestamp.date() if isinstance(c.timestamp, datetime) else c.timestamp
                existing_candles_map[(c.symbol, c.timeframe, c_date, c.adjustment_type)] = (
                    float(c.open), float(c.high), float(c.low), float(c.close), float(c.volume)
                )

        # Run classification
        classified_items, counts, can_accept, block_reason = ImportClassifier.classify_records(
            raw_df, existing_candles_map, timeframe=timeframe, adjustment_type=adjustment_type
        )

        content_sha256 = ImportClassifier.compute_semantic_checksum(classified_items)

        run_id = str(uuid.uuid4())
        status_str = "previewed" if can_accept else "blocked"

        run = ImportRun(
            id=run_id,
            file_name=filename,
            file_sha256=file_sha256,
            content_sha256=content_sha256,
            parser_version="cafef_v1",
            source_type=source_type,
            timeframe=timeframe,
            adjustment_type=adjustment_type,
            timezone=timezone_str,
            status=status_str,
            parsed_count=counts["parsed"],
            rejected_count=counts["rejected"],
            duplicate_count=counts["duplicate"],
            conflicting_count=counts["conflicting"],
            missing_count=counts["missing"],
            out_of_order_count=counts["out_of_order"],
            accepted_count=0,
            can_accept=can_accept,
            block_reason=block_reason
        )
        db.add(run)

        # Add preview items
        item_schemas: List[ImportRunItemSchema] = []
        for it in classified_items:
            db_item = ImportRunItem(
                run_id=run_id,
                row_index=it.row_index,
                symbol=it.symbol,
                timeframe=it.timeframe,
                timestamp=datetime.combine(it.timestamp, datetime.min.time()) if isinstance(it.timestamp, date) else it.timestamp,
                adjustment_type=it.adjustment_type,
                open=it.open,
                high=it.high,
                low=it.low,
                close=it.close,
                volume=it.volume,
                classification=it.classification,
                reject_reason=it.reject_reason
            )
            db.add(db_item)
            item_schemas.append(ImportRunItemSchema(
                row_index=it.row_index,
                symbol=it.symbol,
                timeframe=it.timeframe,
                timestamp=it.timestamp.strftime("%Y-%m-%d") if isinstance(it.timestamp, (date, datetime)) else str(it.timestamp),
                adjustment_type=it.adjustment_type,
                open=it.open,
                high=it.high,
                low=it.low,
                close=it.close,
                volume=it.volume,
                classification=it.classification,
                reject_reason=it.reject_reason
            ))

        db.commit()

        return ImportPreviewResponse(
            run_id=run.id,
            file_name=filename,
            file_sha256=file_sha256,
            content_sha256=content_sha256,
            parser_version="cafef_v1",
            source_type=source_type,
            timeframe=timeframe,
            adjustment_type=adjustment_type,
            timezone=timezone_str,
            status=status_str,
            parsed_count=counts["parsed"],
            rejected_count=counts["rejected"],
            duplicate_count=counts["duplicate"],
            conflicting_count=counts["conflicting"],
            missing_count=counts["missing"],
            out_of_order_count=counts["out_of_order"],
            accepted_count=0,
            can_accept=can_accept,
            block_reason=block_reason,
            items=item_schemas
        )

    @staticmethod
    def accept_import(
        db: Session,
        run_id: str,
        content_sha256: str
    ) -> ImportAcceptResponse:
        run = db.query(ImportRun).filter(ImportRun.id == run_id).first()
        if not run:
            raise HTTPException(status_code=404, detail=f"Không tìm thấy lượt nhập dữ liệu {run_id}")

        # Idempotent repeat check
        if run.status == "accepted":
            if run.content_sha256 == content_sha256:
                return ImportAcceptResponse(
                    run_id=run_id,
                    status="noop",
                    accepted_count=0,
                    message="Tập tin dữ liệu này đã được chấp nhận trước đó (Idempotent no-op)"
                )
            else:
                raise HTTPException(status_code=400, detail="Mã kiểm tra nội dung (checksum) không trùng khớp với lượt nhập đã chấp nhận")

        if not run.can_accept:
            raise HTTPException(status_code=400, detail=f"Không thể chấp nhận lượt nhập này: {run.block_reason}")

        if run.status != "previewed":
            raise HTTPException(status_code=400, detail=f"Trạng thái lượt nhập không hợp lệ để chấp nhận: {run.status}")

        if run.content_sha256 != content_sha256:
            raise HTTPException(status_code=400, detail="Mã kiểm tra bản xem trước không khớp. Vui lòng tạo lại bản xem trước")

        # Fetch parsed items to accept
        parsed_items = (
            db.query(ImportRunItem)
            .filter(ImportRunItem.run_id == run_id, ImportRunItem.classification == "parsed")
            .all()
        )

        if not parsed_items and run.duplicate_count > 0:
            # All items were duplicates -> noop
            run.status = "noop"
            db.commit()
            return ImportAcceptResponse(
                run_id=run_id,
                status="noop",
                accepted_count=0,
                message="Tất cả dòng dữ liệu đã tồn tại trong hệ thống (Idempotent no-op)"
            )

        # Revalidate every staged 'parsed' candidate against current Candle rows.
        # A candidate that was absent at preview but now exists is stale, even if its values happen to match.
        if parsed_items:
            symbols_to_check = set(item.symbol for item in parsed_items)
            existing_candles = (
                db.query(Candle)
                .filter(
                    Candle.symbol.in_(list(symbols_to_check)),
                    Candle.timeframe == run.timeframe,
                    Candle.adjustment_type == run.adjustment_type
                )
                .all()
            )
            existing_keys = {
                (
                    c.symbol,
                    c.timeframe,
                    c.timestamp.date() if isinstance(c.timestamp, datetime) else c.timestamp,
                    c.adjustment_type
                )
                for c in existing_candles
            }

            stale_items = []
            for item in parsed_items:
                item_date = item.timestamp.date() if isinstance(item.timestamp, datetime) else item.timestamp
                key = (item.symbol, item.timeframe, item_date, item.adjustment_type)
                if key in existing_keys:
                    stale_items.append((item, item_date))

            if stale_items:
                first_item, dt = stale_items[0]
                dt_str = dt.strftime("%Y-%m-%d") if isinstance(dt, (date, datetime)) else str(dt)
                block_reason = (
                    f"Bản xem trước đã hết hạn: Dữ liệu nến cho mã {first_item.symbol} ngày {dt_str} "
                    f"đã được tạo hoặc thay đổi sau khi xem trước. Vui lòng tạo lại bản xem trước."
                )
                run.status = "blocked"
                run.can_accept = False
                run.block_reason = block_reason
                db.commit()
                raise HTTPException(status_code=400, detail=block_reason)

        affected_symbols: Set[str] = set()

        for item in parsed_items:
            affected_symbols.add(item.symbol)

            # Ensure Symbol exists
            sym_rec = db.query(Symbol).filter(Symbol.symbol == item.symbol).first()
            if not sym_rec:
                sym_rec = Symbol(symbol=item.symbol, asset_type="stock", is_active=True)
                db.add(sym_rec)
                db.flush()

            item_ts = item.timestamp
            mutation = ImportRunMutation(
                run_id=run_id,
                action="INSERT",
                symbol=item.symbol,
                timeframe=item.timeframe,
                timestamp=item_ts,
                adjustment_type=item.adjustment_type,
                before_open=None, before_high=None, before_low=None, before_close=None, before_volume=None,
                after_open=item.open, after_high=item.high, after_low=item.low, after_close=item.close, after_volume=item.volume
            )
            new_candle = Candle(
                symbol=item.symbol,
                timeframe=item.timeframe,
                timestamp=item_ts,
                open=item.open,
                high=item.high,
                low=item.low,
                close=item.close,
                volume=item.volume,
                source=run_id,
                adjustment_type=item.adjustment_type
            )
            db.add(new_candle)
            db.add(mutation)

        run.status = "accepted"
        run.accepted_at = datetime.now(timezone.utc)
        run.accepted_count = len(parsed_items)

        db.flush()

        # Trigger Weekly Candle derivation
        WeeklyAggregator.derive_weekly_candles(db, affected_symbols, {run.adjustment_type})

        db.commit()


        return ImportAcceptResponse(
            run_id=run_id,
            status="accepted",
            accepted_count=len(parsed_items),
            message=f"Đã chấp nhận và lưu {len(parsed_items)} dòng dữ liệu cho {len(affected_symbols)} mã"
        )

    @staticmethod
    def rollback_import(
        db: Session,
        run_id: str
    ) -> ImportRollbackResponse:
        run = db.query(ImportRun).filter(ImportRun.id == run_id).first()
        if not run:
            raise HTTPException(status_code=404, detail=f"Không tìm thấy lượt nhập dữ liệu {run_id}")

        if run.status != "accepted":
            raise HTTPException(status_code=400, detail="Chỉ có thể hoàn tác các lượt nhập dữ liệu đã chấp nhận")

        mutations = db.query(ImportRunMutation).filter(ImportRunMutation.run_id == run_id).all()
        if not mutations:
            run.status = "rolled_back"
            run.rolled_back_at = datetime.now(timezone.utc)
            db.commit()
            return ImportRollbackResponse(
                run_id=run_id,
                status="rolled_back",
                restored_mutations_count=0,
                message="Không có sự thay đổi dữ liệu nào cần hoàn tác"
            )

        affected_symbols = set([m.symbol for m in mutations])

        # Safety check: ensure no later accepted run modified these same symbols/candles
        subsequent_runs = (
            db.query(ImportRun)
            .filter(
                ImportRun.status == "accepted",
                ImportRun.accepted_at > run.accepted_at
            )
            .all()
        )

        for sub_run in subsequent_runs:
            sub_mutations = db.query(ImportRunMutation).filter(ImportRunMutation.run_id == sub_run.id).all()
            sub_symbols = set([m.symbol for m in sub_mutations])
            overlap = affected_symbols.intersection(sub_symbols)
            if overlap:
                raise HTTPException(
                    status_code=400,
                    detail=f"Hoàn tác bị từ chối: Đã có lượt nhập dữ liệu mới hơn ({sub_run.file_name}) thay đổi các mã {', '.join(overlap)}"
                )

        restored_count = 0
        for m in mutations:
            candle = (
                db.query(Candle)
                .filter(
                    Candle.symbol == m.symbol,
                    Candle.timeframe == m.timeframe,
                    Candle.timestamp == m.timestamp,
                    Candle.adjustment_type == m.adjustment_type
                )
                .first()
            )

            if m.action == "INSERT":
                if candle:
                    db.delete(candle)
                    restored_count += 1
            elif m.action == "UPDATE":
                if candle:
                    candle.open = m.before_open
                    candle.high = m.before_high
                    candle.low = m.before_low
                    candle.close = m.before_close
                    candle.volume = m.before_volume
                    restored_count += 1

        run.status = "rolled_back"
        run.rolled_back_at = datetime.now(timezone.utc)

        db.flush()

        # Re-derive Weekly candles
        WeeklyAggregator.derive_weekly_candles(db, affected_symbols, {run.adjustment_type})

        db.commit()


        return ImportRollbackResponse(
            run_id=run_id,
            status="rolled_back",
            restored_mutations_count=restored_count,
            message=f"Hoàn tác thành công lượt nhập {run.file_name}, khôi phục {restored_count} điểm dữ liệu"
        )

    @staticmethod
    def get_catalog(db: Session) -> List[CatalogItemSchema]:
        """
        Returns data catalog reporting source, symbols, timeframe, adjustment,
        first/last timestamp, row count, and last accepted update provenance.
        """
        # Query distinct symbol, timeframe, adjustment_type from Candle
        results = (
            db.query(
                Candle.symbol,
                Candle.timeframe,
                Candle.adjustment_type,
                func.min(Candle.timestamp).label("min_ts"),
                func.max(Candle.timestamp).label("max_ts"),
                func.count(Candle.id).label("row_count")
            )
            .group_by(Candle.symbol, Candle.timeframe, Candle.adjustment_type)
            .order_by(Candle.symbol.asc(), Candle.timeframe.asc(), Candle.adjustment_type.asc())
            .all()
        )

        catalog_items: List[CatalogItemSchema] = []

        # Build symbol exchange map
        symbols_recs = db.query(Symbol).all()
        exchange_map = {s.symbol: s.exchange for s in symbols_recs}

        for row in results:
            sym = row.symbol
            tf = row.timeframe
            adj = row.adjustment_type
            min_ts = row.min_ts
            max_ts = row.max_ts
            row_count = row.row_count

            # Check if there is any import run associated
            latest_run = (
                db.query(ImportRun)
                .join(ImportRunMutation, ImportRunMutation.run_id == ImportRun.id)
                .filter(
                    ImportRunMutation.symbol == sym,
                    ImportRunMutation.timeframe == tf,
                    ImportRunMutation.adjustment_type == adj,
                    ImportRun.status == "accepted"
                )
                .order_by(ImportRun.accepted_at.desc())
                .first()
            )

            prov_state = "import_run" if latest_run else "local_legacy"
            last_accepted = latest_run.accepted_at.isoformat() if (latest_run and latest_run.accepted_at) else None

            start_str = min_ts.strftime("%Y-%m-%d") if isinstance(min_ts, (date, datetime)) else str(min_ts)
            end_str = max_ts.strftime("%Y-%m-%d") if isinstance(max_ts, (date, datetime)) else str(max_ts)

            catalog_items.append(CatalogItemSchema(
                symbol=sym,
                exchange=exchange_map.get(sym),
                timeframe=tf,
                adjustment_type=adj,
                start_date=start_str,
                end_date=end_str,
                row_count=row_count,
                last_accepted_at=last_accepted,
                provenance_state=prov_state
            ))

        return catalog_items

    @staticmethod
    def get_import_run(db: Session, run_id: str) -> Optional[ImportRun]:
        return db.query(ImportRun).filter(ImportRun.id == run_id).first()

    @staticmethod
    def list_import_runs(db: Session) -> List[ImportRun]:
        return db.query(ImportRun).order_by(ImportRun.created_at.desc()).all()
