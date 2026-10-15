import uuid
from collections import defaultdict
from datetime import datetime, timezone, date
from typing import List, Dict, Tuple, Optional, Set
from fastapi import HTTPException
from sqlalchemy.orm import Session
from sqlalchemy import func

from app.models.candle import Candle
from app.models.symbol import Symbol
from app.models.import_run import ImportRun, ImportRunItem, ImportRunMutation
from app.models.sync_run import SyncRun
from app.schemas.import_schema import (
    ImportPreviewResponse as BaseImportPreviewResponse,
    ImportRunItemSchema,
    ImportAcceptResponse,
    ImportRollbackResponse,
    CatalogItemSchema
)

class ImportPreviewResponse(BaseImportPreviewResponse):
    staged: bool = False
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
        timezone_str: str = "Asia/Ho_Chi_Minh",
        confirm: bool = True
    ) -> ImportPreviewResponse:
        file_sha256 = ImportClassifier.compute_sha256(file_content)

        # 1. Metadata validation fail-closed check
        metadata_err = ImportClassifier.validate_metadata(source_type, timeframe, adjustment_type, timezone_str)
        if metadata_err:
            run_id = str(uuid.uuid4())
            if confirm:
                run = ImportRun(
                    id=run_id,
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
                try:
                    db.add(run)
                    db.commit()
                except Exception:
                    db.rollback()
                    raise
            return ImportPreviewResponse(
                run_id=run_id,
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
                items=[],
                staged=bool(confirm)
            )

        # Parse file into DataFrame
        try:
            if filename.lower().endswith('.zip'):
                raw_df = CafeFImporter.parse_zip(file_content)
            else:
                raw_df = CafeFImporter.parse_file(file_content, filename)
        except Exception as e:
            run_id = str(uuid.uuid4())
            block_msg = f"Không thể đọc hoặc giải mã tập tin: {str(e)}"
            if confirm:
                run = ImportRun(
                    id=run_id,
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
                    block_reason=block_msg
                )
                try:
                    db.add(run)
                    db.commit()
                except Exception:
                    db.rollback()
                    raise
            return ImportPreviewResponse(
                run_id=run_id,
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
                block_reason=block_msg,
                items=[],
                staged=bool(confirm)
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

        item_schemas: List[ImportRunItemSchema] = []
        for it in classified_items:
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

        if confirm:
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
            try:
                db.add(run)

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

                db.commit()
            except Exception:
                db.rollback()
                raise

        return ImportPreviewResponse(
            run_id=run_id,
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
            items=item_schemas,
            staged=bool(confirm)
        )

    @staticmethod
    def accept_import(
        db: Session,
        run_id: str,
        content_sha256: str
    ) -> ImportAcceptResponse:
        run = db.query(ImportRun).filter(ImportRun.id == run_id).first()
        if not run:
            raise HTTPException(
                status_code=400,
                detail="Cannot accept unstaged preview. Staged run required before acceptance."
            )

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
            try:
                run.status = "noop"
                db.commit()
            except Exception:
                db.rollback()
                raise
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
                try:
                    run.status = "blocked"
                    run.can_accept = False
                    run.block_reason = block_reason
                    db.commit()
                except Exception:
                    db.rollback()
                    raise
                raise HTTPException(status_code=400, detail=block_reason)

        try:
            affected_symbols: Set[str] = set(item.symbol for item in parsed_items)

            # Query existing quarantine candles for affected symbols to allow atomic takeover
            existing_quarantine_candles = (
                db.query(Candle)
                .execution_options(include_quarantine=True)
                .filter(
                    Candle.symbol.in_(list(affected_symbols)),
                    Candle.timeframe == run.timeframe,
                    Candle.adjustment_type == run.adjustment_type
                )
                .all()
            )
            # Group existing quarantine candles by date key to detect ambiguous collisions (Finding R2)
            quarantine_groups = defaultdict(list)
            for c in existing_quarantine_candles:
                if c.source and c.source.startswith("quarantine:"):
                    c_date = c.timestamp.date() if isinstance(c.timestamp, datetime) else c.timestamp
                    key = (c.symbol, c.timeframe, c_date, c.adjustment_type)
                    quarantine_groups[key].append(c)

            # Pre-validate all parsed candidates against quarantine ambiguity (Finding R2)
            ambiguous_items = []
            for item in parsed_items:
                item_date = item.timestamp.date() if isinstance(item.timestamp, datetime) else item.timestamp
                item_key = (item.symbol, item.timeframe, item_date, item.adjustment_type)
                candidates = quarantine_groups.get(item_key, [])
                if len(candidates) > 1:
                    ambiguous_items.append((item.symbol, item_date, len(candidates)))

            if ambiguous_items:
                sym, dt, count = ambiguous_items[0]
                dt_str = dt.strftime("%Y-%m-%d") if isinstance(dt, (date, datetime)) else str(dt)
                detail_msg = (
                    f"Ambiguous quarantine candidates detected: {count} nến cách ly (quarantine) trùng ngày {dt_str} cho mã {sym}. "
                    f"Không thể thay thế tự động do dữ liệu không rõ ràng."
                )
                db.rollback()
                raise HTTPException(status_code=400, detail=detail_msg)

            # Unambiguous candidates map (single quarantine candle per key)
            quarantine_map = {
                k: v[0] for k, v in quarantine_groups.items() if len(v) == 1
            }

            for item in parsed_items:
                # Ensure Symbol exists
                sym_rec = db.query(Symbol).filter(Symbol.symbol == item.symbol).first()
                if not sym_rec:
                    asset_type = "index" if CafeFImporter.is_index_symbol(item.symbol) else "stock"
                    sym_rec = Symbol(symbol=item.symbol, asset_type=asset_type, is_active=True)
                    db.add(sym_rec)
                    db.flush()

                item_ts = item.timestamp
                item_date = item_ts.date() if isinstance(item_ts, datetime) else item_ts
                item_key = (item.symbol, item.timeframe, item_date, item.adjustment_type)

                quar_candle = quarantine_map.get(item_key)
                if quar_candle:
                    # Canonical data supersedes quarantine placeholder row atomically (Option A)
                    quar_ts_str = quar_candle.timestamp.isoformat() if hasattr(quar_candle.timestamp, 'isoformat') else str(quar_candle.timestamp)
                    mutation = ImportRunMutation(
                        run_id=run_id,
                        action=f"UPDATE:{quar_candle.source}|{quar_ts_str}",
                        symbol=item.symbol,
                        timeframe=item.timeframe,
                        timestamp=item_ts,
                        adjustment_type=item.adjustment_type,
                        before_open=quar_candle.open,
                        before_high=quar_candle.high,
                        before_low=quar_candle.low,
                        before_close=quar_candle.close,
                        before_volume=quar_candle.volume,
                        after_open=item.open,
                        after_high=item.high,
                        after_low=item.low,
                        after_close=item.close,
                        after_volume=item.volume
                    )
                    quar_candle.open = item.open
                    quar_candle.high = item.high
                    quar_candle.low = item.low
                    quar_candle.close = item.close
                    quar_candle.volume = item.volume
                    quar_candle.timestamp = item_ts
                    quar_candle.source = run_id
                    db.add(mutation)
                else:
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
        except Exception:
            db.rollback()
            raise


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
            try:
                run.status = "rolled_back"
                run.rolled_back_at = datetime.now(timezone.utc)
                db.commit()
            except Exception:
                db.rollback()
                raise
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

        # Safety check: ensure no subsequent accepted sync run modified these symbols
        subsequent_syncs = (
            db.query(SyncRun)
            .filter(
                SyncRun.status == "accepted",
                SyncRun.symbol.in_(list(affected_symbols)),
                SyncRun.accepted_at > run.accepted_at
            )
            .all()
        )
        if subsequent_syncs:
            overlap = affected_symbols.intersection(set(s.symbol for s in subsequent_syncs))
            raise HTTPException(
                status_code=400,
                detail=f"Hoàn tác bị từ chối: Đã có lượt đồng bộ mới hơn ({subsequent_syncs[0].id}) thay đổi các mã {', '.join(overlap)}"
            )

        try:
            restored_count = 0
            for m in mutations:
                candle = (
                    db.query(Candle)
                    .execution_options(include_quarantine=True)
                    .filter(
                        Candle.symbol == m.symbol,
                        Candle.timeframe == m.timeframe,
                        Candle.timestamp == m.timestamp,
                        Candle.adjustment_type == m.adjustment_type
                    )
                    .first()
                )
                if not candle and m.timeframe in ("1D", "D"):
                    m_date = m.timestamp.date() if isinstance(m.timestamp, datetime) else m.timestamp
                    candidates = (
                        db.query(Candle)
                        .execution_options(include_quarantine=True)
                        .filter(
                            Candle.symbol == m.symbol,
                            Candle.timeframe == m.timeframe,
                            Candle.adjustment_type == m.adjustment_type
                        )
                        .all()
                    )
                    matching = [
                        c for c in candidates
                        if (c.timestamp.date() if isinstance(c.timestamp, datetime) else c.timestamp) == m_date
                    ]
                    if len(matching) > 1:
                        raise HTTPException(
                            status_code=400,
                            detail=f"Hoàn tác bị từ chối: Tìm thấy nhiều cây nến trùng ngày ({len(matching)}) cho mã {m.symbol} ngày {m_date} (Ambiguous rollback target)"
                        )
                    if len(matching) == 1:
                        candle = matching[0]

                if not candle:
                    raise HTTPException(
                        status_code=400,
                        detail=f"Hoàn tác bị từ chối: Không tìm thấy cây nến cần khôi phục cho mã {m.symbol} lúc {m.timestamp} (Missing rollback target)"
                    )

                if m.action == "INSERT":
                    db.delete(candle)
                    restored_count += 1
                elif m.action.startswith("UPDATE"):
                    candle.open = m.before_open
                    candle.high = m.before_high
                    candle.low = m.before_low
                    candle.close = m.before_close
                    candle.volume = m.before_volume
                    if ":" in m.action:
                        parts = m.action.split(":", 1)[1]
                        if "|" in parts:
                            src, prior_ts_str = parts.rsplit("|", 1)
                            prior_ts_str = prior_ts_str.strip()
                            if not prior_ts_str:
                                raise HTTPException(
                                    status_code=400,
                                    detail=f"Hoàn tác bị từ chối: Dữ liệu nguồn gốc thời gian bị trống cho mã {m.symbol} (Empty provenance timestamp)"
                                )
                            try:
                                prior_ts = datetime.fromisoformat(prior_ts_str)
                            except (ValueError, TypeError) as exc:
                                raise HTTPException(
                                    status_code=400,
                                    detail=f"Hoàn tác bị từ chối: Dữ liệu nguồn gốc thời gian không hợp lệ '{prior_ts_str}' cho mã {m.symbol} (Malformed provenance timestamp)"
                                ) from exc
                            candle.source = src
                            candle.timestamp = prior_ts
                        else:
                            candle.source = parts
                    restored_count += 1
                else:
                    raise HTTPException(
                        status_code=400,
                        detail=f"Hoàn tác bị từ chối: Thao tác mutation không xác định '{m.action}' cho mã {m.symbol}"
                    )

            if restored_count != len(mutations):
                raise HTTPException(
                    status_code=400,
                    detail=f"Hoàn tác chưa hoàn tất: chỉ khôi phục {restored_count}/{len(mutations)} điểm dữ liệu"
                )

            run.status = "rolled_back"
            run.rolled_back_at = datetime.now(timezone.utc)

            db.flush()

            # Re-derive Weekly candles
            WeeklyAggregator.derive_weekly_candles(db, affected_symbols, {run.adjustment_type})

            db.commit()
        except Exception:
            db.rollback()
            raise


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
