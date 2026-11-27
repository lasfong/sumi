#![warn(clippy::all, rust_2018_idioms)]

pub mod contract;
pub mod evidence;
pub mod reader;

pub use contract::{
    BacktestRequestV1, BacktestSummaryV1, Basis, BridgeError, BridgeResult, CandleBarV1,
    CandleSeriesV1, HistoricalMode, IndicatorColumnV1, IndicatorFrameV1, KT01GateVerdictV1,
    Kt01AfcSummary, Kt01ExecutionLevels, Kt01GateVerdict, MtfAlignmentSummary, QuantityUnit,
    SUMI_CONTRACT_VERSION_V1, SUMI_KT01_VERDICT_VERSION_V1, Scale, TechnicalEvidenceBundleV1,
    TimeframeV1, candle_series_v1_to_sumi_bars, canonical_sha256_value,
    is_index_or_futures_symbol, normalize_price_to_sumi_vnd, normalize_sumi_vnd_to_kvnd,
    parse_provider_price_unit, scale_multiplier_to_sumi_chart, strip_exchange_prefix,
};
pub use evidence::{
    ExportedVerdictListItem, ExportedVerdictReceipt, evaluate_kt01_gate,
    export_kt01_verdict_to_disk, list_exported_verdicts, render_kt01_markdown,
};
pub use reader::{
    Dm01HoldingDto, Dm01WatchlistEntryDto, DnseLoadedSeries, EpisodicThesisDto,
    MacroObservationDto, QmvActiveMoneyPointDto, QmvForeignFlowPointDto, QmvRealChartSeriesDto,
    QmvScreenerSnapshotDto, QmvStockBbPointDto, QmvStockBlackboxSnapshotDto,
    ResearchExportVerificationDto, SectorBreadthSnapshotDto, TopDownSymbolContextDto,
    ValuationQuarterDto, Vm01MarketContextDto, YumekoBridgeStatus, YumekoReadOnlyBridge,
};
