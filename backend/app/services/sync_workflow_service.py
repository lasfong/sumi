import os
import time
import uuid
from datetime import datetime, timezone, date
from typing import List, Dict, Tuple, Optional, Set, Any
import pandas as pd
from fastapi import HTTPException
from sqlalchemy.orm import Session
from sqlalchemy import func

from app.models.candle import Candle
from app.models.symbol import Symbol
from app.models.sync_run import SyncRun, SyncRunItem, SyncRunMutation
from app.models.import_run import ImportRun, ImportRunMutation
from app.schemas.sync_schema import (
    ProviderMetadataSchema,
    TestConnectionResponse,
    SyncPreviewRequest,
    SyncPreviewResponse,
    SyncRunItemSchema,
    SyncExecuteResponse,
    SyncRollbackResponse,
    SyncManifestSchema,
)
from app.services.data_providers.provider_registry import ProviderRegistry
from app.services.data_providers.base_provider import (
    ProviderAuthError,
    ProviderRateLimitError,
    ProviderNetworkError,
    ProviderDataError,
    ProviderException,
)
from app.services.import_classifier import ImportClassifier
from app.services.weekly_aggregator import WeeklyAggregator

class SyncWorkflowService:
    @staticmethod
    def list_providers() -> List[ProviderMetadataSchema]:
        """Lists registered providers and indicates if environment credentials exist."""
        registry = ProviderRegistry.get_instance()
        metadata_list = registry.list_providers()
        
        result: List[ProviderMetadataSchema] = []
        for meta in metadata_list:
            is_configured = False
            if meta.provider_id == "ssi":
                is_configured = bool(os.getenv("SUMI_DATA_PROVIDER_KEY") or os.getenv("SUMI_SSI_CONSUMER_ID"))
            elif not meta.requires_auth:
                is_configured = True
            
            result.append(ProviderMetadataSchema(
                provider_id=meta.provider_id,
                display_name=meta.display_name,
                is_official=meta.is_official,
                requires_auth=meta.requires_auth,
                supported_timeframes=meta.supported_timeframes,
                supported_adjustments=meta.supported_adjustments,
                rate_limit_rps=meta.rate_limit_rps,
                description=meta.description,
                auth_fields=meta.auth_fields,
                is_configured=is_configured
            ))
        return result

    @staticmethod
    def test_connection(provider_id: str, credentials: Optional[Dict[str, Any]] = None) -> TestConnectionResponse:
        """Validates provider connectivity without altering local database data."""
        registry = ProviderRegistry.get_instance()
        adapter = registry.get_provider(provider_id)
        if not adapter:
            raise HTTPException(status_code=404, detail=f"Không tìm thấy nhà cung cấp dữ liệu '{provider_id}'")

        t_start = time.perf_counter()
        try:
            adapter.test_connection(credentials)
            latency = (time.perf_counter() - t_start) * 1000.0
            return TestConnectionResponse(
                success=True,
                provider_id=provider_id,
                message=f"Kết nối thành công đến {adapter.get_metadata().display_name} (Độ trễ: {latency:.1f}ms)",
                latency_ms=round(latency, 2)
            )
        except ProviderAuthError as e:
            return TestConnectionResponse(
                success=False,
                provider_id=provider_id,
                message=f"Lỗi xác thực: {str(e)}"
            )
        except ProviderRateLimitError as e:
            return TestConnectionResponse(
                success=False,
                provider_id=provider_id,
                message=f"Vượt quá giới hạn tần suất: {str(e)}"
            )
        except ProviderNetworkError as e:
            return TestConnectionResponse(
                success=False,
                provider_id=provider_id,
                message=f"Lỗi mạng / Không thể kết nối: {str(e)}"
            )
        except Exception as e:
            return TestConnectionResponse(
                success=False,
                provider_id=provider_id,
                message=f"Lỗi kiểm tra kết nối: {str(e)}"
            )

    @staticmethod
    def generate_sync_preview(db: Session, request: SyncPreviewRequest) -> SyncPreviewResponse:
        """
        Fetches daily candles from provider and produces a dry-run preview with conflict classification.
        """
        # Validate date range
        try:
            start_dt = datetime.strptime(request.start_date.strip(), "%Y-%m-%d").date()
            end_dt = datetime.strptime(request.end_date.strip(), "%Y-%m-%d").date()
        except ValueError:
            raise HTTPException(status_code=400, detail="Định dạng ngày không hợp lệ. Vui lòng sử dụng định dạng YYYY-MM-DD")

        if start_dt > end_dt:
            raise HTTPException(status_code=400, detail=f"Ngày bắt đầu ({request.start_date}) không thể sau ngày kết thúc ({request.end_date})")

        adj_type = request.adjustment_type.strip().lower()
        if adj_type not in ["unadjusted", "adjusted"]:
            raise HTTPException(status_code=400, detail=f"Loại điều chỉnh giá không hợp lệ: '{request.adjustment_type}'. Phải là 'unadjusted' hoặc 'adjusted'")

        sym = request.symbol.strip().upper()
        if not sym:
            raise HTTPException(status_code=400, detail="Mã chứng khoán không được để trống")

        registry = ProviderRegistry.get_instance()
        adapter = registry.get_provider(request.provider_id)
        if not adapter:
            raise HTTPException(status_code=404, detail=f"Không tìm thấy nhà cung cấp dữ liệu '{request.provider_id}'")

        # Fetch candles through adapter boundary
        try:
            candle_dtos = adapter.fetch_daily_candles(
                symbol=sym,
                start_date=start_dt,
                end_date=end_dt,
                adjustment_type=adj_type,
                credentials=request.credentials
            )
        except ProviderAuthError as e:
            raise HTTPException(status_code=401, detail=f"Lỗi xác thực nhà cung cấp: {str(e)}")
        except ProviderRateLimitError as e:
            raise HTTPException(status_code=429, detail=f"Giới hạn tần suất yêu cầu: {str(e)}")
        except ProviderNetworkError as e:
            raise HTTPException(status_code=503, detail=f"Lỗi mạng khi kết nối nhà cung cấp: {str(e)}")
        except ProviderDataError as e:
            raise HTTPException(status_code=400, detail=f"Dữ liệu không hợp lệ từ nhà cung cấp: {str(e)}")
        except Exception as e:
            raise HTTPException(status_code=500, detail=f"Lỗi khi lấy dữ liệu: {str(e)}")

        # Convert DTOs to DataFrame
        raw_rows = []
        for dto in candle_dtos:
            raw_rows.append({
                "symbol": dto.symbol,
                "timestamp": dto.timestamp.strftime("%Y-%m-%d") if isinstance(dto.timestamp, (date, datetime)) else str(dto.timestamp),
                "open": dto.open,
                "high": dto.high,
                "low": dto.low,
                "close": dto.close,
                "volume": dto.volume
            })
        raw_df = pd.DataFrame(raw_rows)

        # Build existing candles map for classification
        existing_candles_map: Dict[Tuple[str, str, date, str], Tuple[float, float, float, float, float]] = {}
        db_candles = (
            db.query(Candle)
            .filter(
                Candle.symbol == sym,
                Candle.timeframe == "1D",
                Candle.adjustment_type == adj_type
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
            raw_df, existing_candles_map, timeframe="1D", adjustment_type=adj_type
        )

        content_sha256 = ImportClassifier.compute_semantic_checksum(classified_items)
        sync_id = str(uuid.uuid4())
        status_str = "previewed" if can_accept else "blocked"

        sync_run = SyncRun(
            id=sync_id,
            provider_id=request.provider_id,
            symbol=sym,
            start_date=request.start_date,
            end_date=request.end_date,
            timeframe="1D",
            adjustment_type=adj_type,
            status=status_str,
            can_accept=can_accept,
            block_reason=block_reason,
            parsed_count=counts["parsed"],
            rejected_count=counts["rejected"],
            duplicate_count=counts["duplicate"],
            conflicting_count=counts["conflicting"],
            missing_count=counts["missing"],
            out_of_order_count=counts["out_of_order"],
            accepted_count=0,
            content_sha256=content_sha256,
            duration_ms=0.0
        )
        db.add(sync_run)

        item_schemas: List[SyncRunItemSchema] = []
        for it in classified_items:
            db_item = SyncRunItem(
                sync_id=sync_id,
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
            item_schemas.append(SyncRunItemSchema(
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

        return SyncPreviewResponse(
            sync_id=sync_run.id,
            provider_id=request.provider_id,
            symbol=sym,
            start_date=request.start_date,
            end_date=request.end_date,
            timeframe="1D",
            adjustment_type=adj_type,
            status=status_str,
            parsed_count=counts["parsed"],
            rejected_count=counts["rejected"],
            duplicate_count=counts["duplicate"],
            conflicting_count=counts["conflicting"],
            missing_count=counts["missing"],
            out_of_order_count=counts["out_of_order"],
            can_accept=can_accept,
            block_reason=block_reason,
            content_sha256=content_sha256,
            items=item_schemas
        )

    @staticmethod
    def execute_sync(db: Session, sync_id: str, content_sha256: str) -> SyncExecuteResponse:
        """
        Atomically commits the previewed sync batch, mutates Candle store, triggers WeeklyAggregator,
        and generates an immutable audit manifest.
        """
        t_start = time.perf_counter()

        run = db.query(SyncRun).filter(SyncRun.id == sync_id).first()
        if not run:
            raise HTTPException(status_code=404, detail=f"Không tìm thấy lượt đồng bộ dữ liệu {sync_id}")

        # Idempotent repeat check
        if run.status == "accepted":
            if run.content_sha256 == content_sha256:
                return SyncExecuteResponse(
                    sync_id=sync_id,
                    status="noop",
                    accepted_count=0,
                    symbol=run.symbol,
                    timeframe=run.timeframe,
                    adjustment_type=run.adjustment_type,
                    duration_ms=run.duration_ms,
                    message="Lượt đồng bộ này đã được chấp nhận trước đó (Idempotent no-op)",
                    manifest=run.manifest_json
                )
            else:
                raise HTTPException(status_code=400, detail="Mã checksum không trùng khớp với lượt đồng bộ đã chấp nhận")

        if not run.can_accept:
            raise HTTPException(status_code=400, detail=f"Không thể chấp nhận lượt đồng bộ này: {run.block_reason}")

        if run.status != "previewed":
            raise HTTPException(status_code=400, detail=f"Trạng thái lượt đồng bộ không hợp lệ: {run.status}")

        if run.content_sha256 != content_sha256:
            raise HTTPException(status_code=400, detail="Mã kiểm tra bản xem trước không khớp. Vui lòng tạo lại bản xem trước")

        parsed_items = (
            db.query(SyncRunItem)
            .filter(SyncRunItem.sync_id == sync_id, SyncRunItem.classification == "parsed")
            .all()
        )

        if not parsed_items and run.duplicate_count > 0:
            run.status = "noop"
            db.commit()
            return SyncExecuteResponse(
                sync_id=sync_id,
                status="noop",
                accepted_count=0,
                symbol=run.symbol,
                timeframe=run.timeframe,
                adjustment_type=run.adjustment_type,
                duration_ms=0.0,
                message="Tất cả dòng dữ liệu đã tồn tại trong hệ thống (Idempotent no-op)",
                manifest=None
            )

        # Fail-closed check against stale preview
        if parsed_items:
            existing_candles = (
                db.query(Candle)
                .filter(
                    Candle.symbol == run.symbol,
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

        # Ensure Symbol exists
        sym_rec = db.query(Symbol).filter(Symbol.symbol == run.symbol).first()
        if not sym_rec:
            sym_rec = Symbol(symbol=run.symbol, asset_type="stock", is_active=True)
            db.add(sym_rec)
            db.flush()

        for item in parsed_items:
            item_ts = item.timestamp
            mutation = SyncRunMutation(
                sync_id=sync_id,
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
                source=f"sync:{sync_id}",
                adjustment_type=item.adjustment_type
            )
            db.add(new_candle)
            db.add(mutation)

        db.flush()

        # Trigger Weekly Candle derivation
        WeeklyAggregator.derive_weekly_candles(db, {run.symbol}, {run.adjustment_type})

        duration_ms = (time.perf_counter() - t_start) * 1000.0

        run.status = "accepted"
        run.accepted_at = datetime.now(timezone.utc)
        run.accepted_count = len(parsed_items)
        run.duration_ms = round(duration_ms, 2)

        # Build immutable audit manifest
        manifest = {
            "sync_id": run.id,
            "audit_version": "SUMI_SYNC_MANIFEST_V1",
            "provider_id": run.provider_id,
            "symbol": run.symbol,
            "start_date": run.start_date,
            "end_date": run.end_date,
            "timeframe": run.timeframe,
            "adjustment_type": run.adjustment_type,
            "status": "accepted",
            "counts": {
                "parsed": run.parsed_count,
                "duplicate": run.duplicate_count,
                "conflicting": run.conflicting_count,
                "rejected": run.rejected_count,
                "accepted": run.accepted_count,
            },
            "content_sha256": run.content_sha256,
            "created_at": run.created_at.isoformat(),
            "accepted_at": run.accepted_at.isoformat(),
            "duration_ms": run.duration_ms,
        }
        run.manifest_json = manifest

        db.commit()

        return SyncExecuteResponse(
            sync_id=sync_id,
            status="accepted",
            accepted_count=len(parsed_items),
            symbol=run.symbol,
            timeframe=run.timeframe,
            adjustment_type=run.adjustment_type,
            duration_ms=run.duration_ms,
            message=f"Đã đồng bộ và lưu thành công {len(parsed_items)} nến cho {run.symbol}",
            manifest=manifest
        )

    @staticmethod
    def rollback_sync(db: Session, sync_id: str) -> SyncRollbackResponse:
        """
        Reverts candles inserted/mutated by a sync run and re-derives weekly series.
        """
        run = db.query(SyncRun).filter(SyncRun.id == sync_id).first()
        if not run:
            raise HTTPException(status_code=404, detail=f"Không tìm thấy lượt đồng bộ dữ liệu {sync_id}")

        if run.status != "accepted":
            raise HTTPException(status_code=400, detail="Chỉ có thể hoàn tác các lượt đồng bộ đã chấp nhận")

        mutations = db.query(SyncRunMutation).filter(SyncRunMutation.sync_id == sync_id).all()
        if not mutations:
            run.status = "rolled_back"
            run.rolled_back_at = datetime.now(timezone.utc)
            db.commit()
            return SyncRollbackResponse(
                sync_id=sync_id,
                status="rolled_back",
                restored_mutations_count=0,
                message="Không có sự thay đổi dữ liệu nào cần hoàn tác"
            )

        # Safety check: ensure no subsequent accepted sync run modified this symbol
        subsequent_syncs = (
            db.query(SyncRun)
            .filter(
                SyncRun.status == "accepted",
                SyncRun.symbol == run.symbol,
                SyncRun.accepted_at > run.accepted_at
            )
            .all()
        )
        if subsequent_syncs:
            raise HTTPException(
                status_code=400,
                detail=f"Hoàn tác bị từ chối: Đã có lượt đồng bộ mới hơn ({subsequent_syncs[0].id}) thay đổi mã {run.symbol}"
            )

        # Safety check: ensure no subsequent accepted import run modified this symbol
        subsequent_imports = (
            db.query(ImportRun)
            .join(ImportRunMutation, ImportRunMutation.run_id == ImportRun.id)
            .filter(
                ImportRun.status == "accepted",
                ImportRunMutation.symbol == run.symbol,
                ImportRun.accepted_at > run.accepted_at
            )
            .all()
        )
        if subsequent_imports:
            raise HTTPException(
                status_code=400,
                detail=f"Hoàn tác bị từ chối: Đã có lượt nhập tập tin mới hơn ({subsequent_imports[0].file_name}) thay đổi mã {run.symbol}"
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
        WeeklyAggregator.derive_weekly_candles(db, {run.symbol}, {run.adjustment_type})

        db.commit()

        return SyncRollbackResponse(
            sync_id=sync_id,
            status="rolled_back",
            restored_mutations_count=restored_count,
            message=f"Hoàn tác thành công lượt đồng bộ {run.symbol} ({run.start_date} -> {run.end_date}), khôi phục {restored_count} điểm dữ liệu"
        )

    @staticmethod
    def get_sync_history(db: Session, limit: int = 50) -> List[SyncManifestSchema]:
        """Returns history of sync runs with manifest details."""
        runs = (
            db.query(SyncRun)
            .order_by(SyncRun.created_at.desc())
            .limit(limit)
            .all()
        )
        history: List[SyncManifestSchema] = []
        for r in runs:
            history.append(SyncManifestSchema(
                sync_id=r.id,
                created_at=r.created_at.isoformat(),
                provider_id=r.provider_id,
                symbol=r.symbol,
                start_date=r.start_date,
                end_date=r.end_date,
                timeframe=r.timeframe,
                adjustment_type=r.adjustment_type,
                status=r.status,
                parsed_count=r.parsed_count,
                duplicate_count=r.duplicate_count,
                conflicting_count=r.conflicting_count,
                accepted_count=r.accepted_count,
                duration_ms=r.duration_ms,
                accepted_at=r.accepted_at.isoformat() if r.accepted_at else None,
                rolled_back_at=r.rolled_back_at.isoformat() if r.rolled_back_at else None,
                manifest=r.manifest_json
            ))
        return history

    @staticmethod
    def get_sync_run(db: Session, sync_id: str) -> Optional[SyncRun]:
        return db.query(SyncRun).filter(SyncRun.id == sync_id).first()
