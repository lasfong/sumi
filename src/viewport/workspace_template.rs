//! Zero-dependency, `wasm32-unknown-unknown`-compatible Versioned Workspace Layout Templates
//! (`Milestone R12 / Wave 11`).
//!
//! Replaces legacy `sumi:workspace:${sessionId}` (`IndicatorRepository.ts`) with clean-room,
//! multi-slot layout templates (`schema_version = 1`) supporting both human-editable **RON**
//! (comments, trailing commas) and **JSON** codecs with strict validation and `DEF-001`-safe
//! sanitization (`0` external crates added to `Cargo.toml`).

use std::fmt::{self, Write as _};

use serde::{Deserialize, Serialize};

use crate::data::{DataSourcePreset, Timeframe};
use crate::indicators::{CvdResetMode, IndicatorConfig, ProfileSessionPeriod, VwapAnchorMode};

use super::hotkeys::{
    HotkeyAction, HotkeyChord, HotkeyKey, HotkeyModifiers, HotkeyOverride, HotkeyProfile,
    HotkeyProfileId,
};
use super::multi_chart::{MAX_MULTI_CHART_SLOTS, MultiChartLayoutKind, MultiChartSyncConfig};
use super::price_scale::PriceScaleMode;

/// Current schema version for serialized [`WorkspaceTemplate`] documents.
pub const WORKSPACE_TEMPLATE_SCHEMA_VERSION: u32 = 1;

const MAX_TEMPLATE_NAME_BYTES: usize = 64;
const MAX_TEMPLATE_DESC_BYTES: usize = 200;
const MAX_SYMBOL_BYTES: usize = 24;
const MAX_PARSER_DEPTH: usize = 32;

/// Proportional stretch weights for the main pane and stacked oscillator sub-panes in one chart slot.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspacePaneStretches {
    pub main: f32,
    pub rsi: f32,
    pub macd: f32,
    pub atr: f32,
    pub rs: f32,
    pub cvd: f32,
    pub equity: f32,
}

impl Default for WorkspacePaneStretches {
    fn default() -> Self {
        Self {
            main: 4.0,
            rsi: 1.0,
            macd: 1.0,
            atr: 1.0,
            rs: 1.0,
            cvd: 1.0,
            equity: 1.25,
        }
    }
}

impl WorkspacePaneStretches {
    pub fn sanitize(&mut self) {
        self.main = sanitize_stretch_f32(self.main, 4.0);
        self.rsi = sanitize_stretch_f32(self.rsi, 1.0);
        self.macd = sanitize_stretch_f32(self.macd, 1.0);
        self.atr = sanitize_stretch_f32(self.atr, 1.0);
        self.rs = sanitize_stretch_f32(self.rs, 1.0);
        self.cvd = sanitize_stretch_f32(self.cvd, 1.0);
        self.equity = sanitize_stretch_f32(self.equity, 1.25);
    }
}

#[inline]
fn sanitize_stretch_f32(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.25, 16.0)
    } else {
        fallback
    }
}

/// Complete persistent configuration of one chart slot (`0..4`) inside a [`WorkspaceTemplate`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceSlotTemplate {
    pub slot_index: usize,
    pub data_source: DataSourcePreset,
    pub symbol: String,
    pub timeframe: Timeframe,
    pub scale_mode: PriceScaleMode,
    pub show_volume: bool,
    pub show_grid: bool,
    pub compare_benchmark: Option<String>,
    pub show_compare_overlay: bool,
    pub indicators: IndicatorConfig,
    pub pane_stretches: WorkspacePaneStretches,
}

impl Default for WorkspaceSlotTemplate {
    fn default() -> Self {
        Self {
            slot_index: 0,
            data_source: DataSourcePreset::Vn30Basket,
            symbol: "FPT".to_owned(),
            timeframe: Timeframe::D1,
            scale_mode: PriceScaleMode::Normal,
            show_volume: true,
            show_grid: true,
            compare_benchmark: None,
            show_compare_overlay: false,
            indicators: IndicatorConfig::default(),
            pane_stretches: WorkspacePaneStretches::default(),
        }
    }
}

impl WorkspaceSlotTemplate {
    pub fn sanitize(&mut self, expected_slot_index: usize) {
        self.slot_index = expected_slot_index.min(MAX_MULTI_CHART_SLOTS - 1);
        if self.data_source == DataSourcePreset::CustomFile {
            self.data_source = DataSourcePreset::SyntheticVnindex;
        }
        truncate_utf8(&mut self.symbol, MAX_SYMBOL_BYTES);
        let sym_trim = self.symbol.trim().to_owned();
        self.symbol = if sym_trim.is_empty() {
            "VNINDEX".to_owned()
        } else {
            sym_trim.to_ascii_uppercase()
        };
        if let Some(ref mut bench) = self.compare_benchmark {
            truncate_utf8(bench, MAX_SYMBOL_BYTES);
            let b = bench.trim().to_ascii_uppercase();
            if b.is_empty() {
                self.compare_benchmark = None;
            } else {
                *bench = b;
            }
        }
        self.indicators.sanitize_deserialized();
        self.pane_stretches.sanitize();
    }
}

/// Left Scanner / Breadth dock active tab persisted inside [`WorkspaceDockPreset`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TemplateScannerTab {
    #[default]
    Watchlist,
    Signals,
    Breadth,
}

impl TemplateScannerTab {
    #[inline]
    #[must_use]
    pub const fn as_id(self) -> &'static str {
        match self {
            Self::Watchlist => "Watchlist",
            Self::Signals => "Signals",
            Self::Breadth => "Breadth",
        }
    }
}

/// Right-hand unified execution / alerts / strategy / journal dock active tab persisted inside [`WorkspaceDockPreset`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TemplateRightDockTab {
    #[default]
    Trading,
    Alerts,
    StrategyLab,
    Journal,
}

impl TemplateRightDockTab {
    #[inline]
    #[must_use]
    pub const fn as_id(self) -> &'static str {
        match self {
            Self::Trading => "Trading",
            Self::Alerts => "Alerts",
            Self::StrategyLab => "StrategyLab",
            Self::Journal => "Journal",
        }
    }
}

/// Active sub-view inside the Paper Trading / Execution dock (`OrderTicket` vs `DomLadder`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TemplateTradingViewMode {
    #[default]
    OrderTicket,
    DomLadder,
}

impl TemplateTradingViewMode {
    #[inline]
    #[must_use]
    pub const fn as_id(self) -> &'static str {
        match self {
            Self::OrderTicket => "OrderTicket",
            Self::DomLadder => "DomLadder",
        }
    }

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::OrderTicket => "Ticket",
            Self::DomLadder => "DOM Ladder",
        }
    }
}

/// Serializable workstation side-dock & toolbar layout preset persisted per [`WorkspaceTemplate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceDockPreset {
    pub show_scanner_dock: bool,
    pub scanner_tab: TemplateScannerTab,
    pub show_trading_panel: bool,
    pub show_alerts_panel: bool,
    pub show_strategy_panel: bool,
    pub show_journal_panel: bool,
    pub right_dock_tab: TemplateRightDockTab,
    pub trading_view_mode: TemplateTradingViewMode,
    pub show_replay_bar: bool,
}

/// Versioned multi-chart workspace template persisted/shared as `RON` or `JSON`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceTemplate {
    pub schema_version: u32,
    pub name: String,
    pub description: String,
    pub layout: MultiChartLayoutKind,
    pub sync: MultiChartSyncConfig,
    pub active_slot: usize,
    pub slots: Vec<WorkspaceSlotTemplate>,
    pub dock_layout: WorkspaceDockPreset,
    pub hotkey_profile: HotkeyProfileId,
    #[serde(default)]
    pub hotkey_base_preset: Option<HotkeyProfileId>,
    pub hotkey_overrides: Vec<HotkeyOverride>,
}

impl Default for WorkspaceTemplate {
    fn default() -> Self {
        let mut tpl = Self {
            schema_version: WORKSPACE_TEMPLATE_SCHEMA_VERSION,
            name: "Default Workstation".to_owned(),
            description: "Balanced Sumi chart desk".to_owned(),
            layout: MultiChartLayoutKind::Single1x1,
            sync: MultiChartSyncConfig::default(),
            active_slot: 0,
            slots: default_four_slots(),
            dock_layout: WorkspaceDockPreset::default(),
            hotkey_profile: HotkeyProfileId::default(),
            hotkey_base_preset: None,
            hotkey_overrides: Vec::new(),
        };
        tpl.sanitize();
        tpl
    }
}

impl WorkspaceTemplate {
    /// Normalizes all fields and ensures `slots` holds exactly `4` sanitized slots `0..4`.
    pub fn sanitize(&mut self) {
        if self.schema_version == 0 {
            self.schema_version = WORKSPACE_TEMPLATE_SCHEMA_VERSION;
        }
        truncate_utf8(&mut self.name, MAX_TEMPLATE_NAME_BYTES);
        if self.name.trim().is_empty() {
            self.name = "Untitled Template".to_owned();
        } else {
            self.name = self.name.trim().to_owned();
        }
        truncate_utf8(&mut self.description, MAX_TEMPLATE_DESC_BYTES);
        self.description = self.description.trim().to_owned();
        self.active_slot = self.active_slot.min(self.layout.slot_count() - 1);

        let defaults = default_four_slots();
        let mut normalized = Vec::with_capacity(MAX_MULTI_CHART_SLOTS);
        for (idx, fallback) in defaults.into_iter().enumerate() {
            let mut slot = self.slots.get(idx).cloned().unwrap_or(fallback);
            slot.sanitize(idx);
            normalized.push(slot);
        }
        self.slots = normalized;

        self.hotkey_base_preset = match self.hotkey_base_preset {
            Some(bp)
                if self.hotkey_profile == HotkeyProfileId::Custom
                    && !matches!(
                        bp,
                        HotkeyProfileId::StandardCharting | HotkeyProfileId::Custom
                    ) =>
            {
                Some(bp)
            }
            _ => None,
        };
        let canonical_profile = HotkeyProfile::from_preset_with_base_and_overrides(
            self.hotkey_profile,
            self.hotkey_base_preset,
            &self.hotkey_overrides,
        );
        self.hotkey_overrides = canonical_profile.overrides_from_preset();
    }

    /// Encodes this template into formatted RFC-8259 `JSON`.
    #[must_use]
    pub fn to_json_pretty(&self) -> String {
        let mut copy = self.clone();
        copy.sanitize();
        encode_json_template(&copy)
    }

    /// Encodes this template into human-readable `RON` with an explanatory header comment.
    #[must_use]
    pub fn to_ron_pretty(&self) -> String {
        let mut copy = self.clone();
        copy.sanitize();
        encode_ron_template(&copy)
    }

    /// Parses a template from RFC-8259 `JSON` text and validates `schema_version` & finite numbers.
    pub fn from_json_str(input: &str) -> Result<Self, WorkspaceTemplateError> {
        let val = parse_json_doc(input)?;
        Self::from_doc_value(&val)
    }

    /// Parses a template from `RON` text (supporting comments, trailing commas, and optional struct
    /// name `WorkspaceTemplate(...)`) and validates `schema_version` & finite numbers.
    pub fn from_ron_str(input: &str) -> Result<Self, WorkspaceTemplateError> {
        let val = parse_ron_doc(input)?;
        Self::from_doc_value(&val)
    }

    /// Auto-detects `JSON` vs `RON` from input text and parses the template safely.
    pub fn from_auto_str(input: &str) -> Result<Self, WorkspaceTemplateError> {
        let trimmed = strip_leading_ws_and_comments(input)?;
        if trimmed.starts_with('{') {
            Self::from_json_str(trimmed)
        } else {
            Self::from_ron_str(input)
        }
    }
}

/// Errors emitted while parsing or validating a [`WorkspaceTemplate`] from `RON` or `JSON`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceTemplateError {
    Syntax(String),
    UnsupportedSchemaVersion(u32),
    NonFiniteNumber(String),
    InvalidField { field: String, reason: String },
}

impl fmt::Display for WorkspaceTemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax(msg) => write!(f, "Template syntax error: {msg}"),
            Self::UnsupportedSchemaVersion(v) => write!(
                f,
                "Unsupported schema_version {v} (expected {WORKSPACE_TEMPLATE_SCHEMA_VERSION})"
            ),
            Self::NonFiniteNumber(field) => {
                write!(f, "Non-finite NaN/Inf numeric value in '{field}'")
            }
            Self::InvalidField { field, reason } => {
                write!(f, "Invalid template field '{field}': {reason}")
            }
        }
    }
}

impl std::error::Error for WorkspaceTemplateError {}

#[must_use]
fn default_four_slots() -> Vec<WorkspaceSlotTemplate> {
    vec![
        WorkspaceSlotTemplate {
            slot_index: 0,
            data_source: DataSourcePreset::Vn30Basket,
            symbol: "FPT".to_owned(),
            timeframe: Timeframe::D1,
            indicators: IndicatorConfig {
                show_sma: true,
                show_ema: true,
                show_ichimoku: true,
                show_rsi: false,
                show_cvd: true,
                cvd_reset_mode: CvdResetMode::Cumulative,
                ..IndicatorConfig::default()
            },
            ..WorkspaceSlotTemplate::default()
        },
        WorkspaceSlotTemplate {
            slot_index: 1,
            data_source: DataSourcePreset::CafeFIndexCsv,
            symbol: "VNINDEX".to_owned(),
            timeframe: Timeframe::D1,
            indicators: IndicatorConfig {
                show_sma: true,
                show_ema: false,
                show_ichimoku: true,
                show_rsi: false,
                show_cvd: true,
                cvd_reset_mode: CvdResetMode::Cumulative,
                ..IndicatorConfig::default()
            },
            ..WorkspaceSlotTemplate::default()
        },
        WorkspaceSlotTemplate {
            slot_index: 2,
            data_source: DataSourcePreset::Vn30Basket,
            symbol: "VCB".to_owned(),
            timeframe: Timeframe::D1,
            indicators: IndicatorConfig {
                show_sma: true,
                show_ema: false,
                show_ichimoku: true,
                show_rsi: false,
                show_cvd: true,
                cvd_reset_mode: CvdResetMode::Cumulative,
                ..IndicatorConfig::default()
            },
            ..WorkspaceSlotTemplate::default()
        },
        WorkspaceSlotTemplate {
            slot_index: 3,
            data_source: DataSourcePreset::IntradayVn30F1m,
            symbol: "VN30F1M".to_owned(),
            timeframe: Timeframe::M15,
            indicators: IndicatorConfig {
                show_sma: false,
                show_ema: true,
                show_vwap: true,
                show_ichimoku: true,
                show_rsi: false,
                show_cvd: true,
                cvd_reset_mode: CvdResetMode::Session,
                ..IndicatorConfig::default()
            },
            ..WorkspaceSlotTemplate::default()
        },
    ]
}

/// Returns the `6` built-in institutional workspace layout templates (`1x1`, `1x2`, `2x1`, `2x2`,
/// R13 `"VN30 Session Profile & Flow"`, and R14 `"VN30F DOM Execution & Flow"`).
#[must_use]
pub fn builtin_workspace_templates() -> Vec<WorkspaceTemplate> {
    let ichimoku_daily = IndicatorConfig {
        show_sma: true,
        show_ema: false,
        show_vwap: true,
        show_ichimoku: true,
        show_rsi: false,
        show_macd: false,
        show_atr: false,
        show_cvd: true,
        cvd_reset_mode: CvdResetMode::Session,
        cvd_sma_period: 9,
        ..IndicatorConfig::default()
    };
    let ichimoku_weekly = IndicatorConfig {
        show_sma: true,
        show_ema: false,
        show_vwap: false,
        show_ichimoku: true,
        show_rsi: false,
        show_rs: true,
        show_cvd: true,
        cvd_reset_mode: CvdResetMode::Cumulative,
        cvd_sma_period: 9,
        ..IndicatorConfig::default()
    };

    let mut t1 = WorkspaceTemplate {
        schema_version: WORKSPACE_TEMPLATE_SCHEMA_VERSION,
        name: "Single Focus Ichimoku + CVD".to_owned(),
        description: "Full-width 1x1 FPT Daily with Ichimoku Cloud, Session VWAP & OHLCV_PROXY CVD"
            .to_owned(),
        layout: MultiChartLayoutKind::Single1x1,
        sync: MultiChartSyncConfig::default(),
        active_slot: 0,
        slots: default_four_slots(),
        dock_layout: WorkspaceDockPreset::default(),
        hotkey_profile: HotkeyProfileId::default(),
        hotkey_base_preset: None,
        hotkey_overrides: Vec::new(),
    };
    t1.slots[0].indicators = ichimoku_daily.clone();

    let mut t2 = WorkspaceTemplate {
        schema_version: WORKSPACE_TEMPLATE_SCHEMA_VERSION,
        name: "Intraday MTF Derivative Flow".to_owned(),
        description: "1x2 VN30F1M 15m + 1h synced crosshair & time window with Session CVD & Kumo"
            .to_owned(),
        layout: MultiChartLayoutKind::SplitVertical1x2,
        sync: MultiChartSyncConfig {
            sync_crosshair: true,
            sync_time_range: true,
            sync_symbol: false,
            sync_timeframe: false,
        },
        active_slot: 0,
        slots: default_four_slots(),
        dock_layout: WorkspaceDockPreset::default(),
        hotkey_profile: HotkeyProfileId::default(),
        hotkey_base_preset: None,
        hotkey_overrides: Vec::new(),
    };
    t2.slots[0] = WorkspaceSlotTemplate {
        slot_index: 0,
        data_source: DataSourcePreset::IntradayVn30F1m,
        symbol: "VN30F1M".to_owned(),
        timeframe: Timeframe::M15,
        indicators: IndicatorConfig {
            show_sma: false,
            show_ema: true,
            ema_period: 34,
            show_vwap: true,
            show_ichimoku: true,
            show_rsi: false,
            show_cvd: true,
            cvd_reset_mode: CvdResetMode::Session,
            cvd_sma_period: 9,
            ..IndicatorConfig::default()
        },
        ..WorkspaceSlotTemplate::default()
    };
    t2.slots[1] = WorkspaceSlotTemplate {
        slot_index: 1,
        data_source: DataSourcePreset::IntradayVn30F1m,
        symbol: "VN30F1M".to_owned(),
        timeframe: Timeframe::H1,
        indicators: IndicatorConfig {
            show_sma: false,
            show_ema: true,
            ema_period: 34,
            show_vwap: true,
            show_ichimoku: true,
            show_rsi: false,
            show_cvd: true,
            cvd_reset_mode: CvdResetMode::Session,
            cvd_sma_period: 9,
            ..IndicatorConfig::default()
        },
        ..WorkspaceSlotTemplate::default()
    };

    let mut t3 = WorkspaceTemplate {
        schema_version: WORKSPACE_TEMPLATE_SCHEMA_VERSION,
        name: "Swing Top-Down Daily / Weekly".to_owned(),
        description: "2x1 FPT 1D vs 1W with causal Replay sync, Ichimoku Kumo & RS vs VNINDEX"
            .to_owned(),
        layout: MultiChartLayoutKind::SplitHorizontal2x1,
        sync: MultiChartSyncConfig {
            sync_crosshair: true,
            sync_time_range: true,
            sync_symbol: true,
            sync_timeframe: false,
        },
        active_slot: 0,
        slots: default_four_slots(),
        dock_layout: WorkspaceDockPreset::default(),
        hotkey_profile: HotkeyProfileId::default(),
        hotkey_base_preset: None,
        hotkey_overrides: Vec::new(),
    };
    t3.slots[0].symbol = "FPT".to_owned();
    t3.slots[0].timeframe = Timeframe::D1;
    t3.slots[0].indicators = ichimoku_daily;
    t3.slots[1] = WorkspaceSlotTemplate {
        slot_index: 1,
        data_source: DataSourcePreset::Vn30Basket,
        symbol: "FPT".to_owned(),
        timeframe: Timeframe::W1,
        compare_benchmark: Some("VNINDEX".to_owned()),
        show_compare_overlay: true,
        indicators: ichimoku_weekly,
        ..WorkspaceSlotTemplate::default()
    };

    let mut t4 = WorkspaceTemplate {
        schema_version: WORKSPACE_TEMPLATE_SCHEMA_VERSION,
        name: "Vietnam Institutional Quad Desk".to_owned(),
        description: "2x2 Quad Matrix (FPT 1D, VNINDEX 1D, VCB 1D, VN30F1M 15m) with synced crosshair & Ichimoku + CVD".to_owned(),
        layout: MultiChartLayoutKind::Quad2x2,
        sync: MultiChartSyncConfig {
            sync_crosshair: true,
            sync_time_range: true,
            sync_symbol: false,
            sync_timeframe: false,
        },
        active_slot: 0,
        slots: default_four_slots(),
        dock_layout: WorkspaceDockPreset::default(),
        hotkey_profile: HotkeyProfileId::default(),
        hotkey_base_preset: None,
        hotkey_overrides: Vec::new(),
    };

    let mut t5 = WorkspaceTemplate {
        schema_version: WORKSPACE_TEMPLATE_SCHEMA_VERSION,
        name: "VN30 Session Profile & Flow".to_owned(),
        description: "1x2 VN30F1M 15m TPO + Daily FPT Weekly VWAP & Naked POC with Compact CVD"
            .to_owned(),
        layout: MultiChartLayoutKind::SplitVertical1x2,
        sync: MultiChartSyncConfig {
            sync_crosshair: true,
            sync_time_range: true,
            sync_symbol: false,
            sync_timeframe: false,
        },
        active_slot: 0,
        slots: default_four_slots(),
        dock_layout: WorkspaceDockPreset {
            show_scanner_dock: true,
            scanner_tab: TemplateScannerTab::Breadth,
            ..WorkspaceDockPreset::default()
        },
        hotkey_profile: HotkeyProfileId::default(),
        hotkey_base_preset: None,
        hotkey_overrides: Vec::new(),
    };
    t5.slots[0] = WorkspaceSlotTemplate {
        slot_index: 0,
        data_source: DataSourcePreset::IntradayVn30F1m,
        symbol: "VN30F1M".to_owned(),
        timeframe: Timeframe::M15,
        indicators: IndicatorConfig {
            show_sma: false,
            show_ema: true,
            ema_period: 34,
            show_vwap: true,
            vwap_anchor_mode: VwapAnchorMode::AutoSession,
            show_market_profile: true,
            mp_num_bins: 24,
            mp_ib_bars: 4,
            mp_extend_naked_poc: true,
            show_rsi: false,
            show_cvd: true,
            cvd_reset_mode: CvdResetMode::Session,
            cvd_sma_period: 9,
            ..IndicatorConfig::default()
        },
        ..WorkspaceSlotTemplate::default()
    };
    t5.slots[1] = WorkspaceSlotTemplate {
        slot_index: 1,
        data_source: DataSourcePreset::Vn30Basket,
        symbol: "FPT".to_owned(),
        timeframe: Timeframe::D1,
        indicators: IndicatorConfig {
            show_sma: true,
            show_ema: true,
            show_vwap: true,
            vwap_anchor_mode: VwapAnchorMode::Weekly,
            show_market_profile: true,
            mp_num_bins: 24,
            mp_ib_bars: 2,
            mp_extend_naked_poc: true,
            show_rsi: false,
            show_cvd: true,
            cvd_reset_mode: CvdResetMode::Cumulative,
            cvd_sma_period: 9,
            ..IndicatorConfig::default()
        },
        ..WorkspaceSlotTemplate::default()
    };

    let mut t6 = WorkspaceTemplate {
        schema_version: WORKSPACE_TEMPLATE_SCHEMA_VERSION,
        name: "VN30F DOM Execution & Flow".to_owned(),
        description: "1x2 Execution Desk (VN30F1M 15m + FPT 1D) with OHLCV_SYNTH_DOM_PROXY Ladder & InstitutionalDomFast Hotkeys".to_owned(),
        layout: MultiChartLayoutKind::SplitVertical1x2,
        sync: MultiChartSyncConfig {
            sync_crosshair: true,
            sync_time_range: true,
            sync_symbol: false,
            sync_timeframe: false,
        },
        active_slot: 0,
        slots: default_four_slots(),
        dock_layout: WorkspaceDockPreset {
            show_trading_panel: true,
            right_dock_tab: TemplateRightDockTab::Trading,
            trading_view_mode: TemplateTradingViewMode::DomLadder,
            show_replay_bar: true,
            ..WorkspaceDockPreset::default()
        },
        hotkey_profile: HotkeyProfileId::InstitutionalDomFast,
        hotkey_base_preset: None,
        hotkey_overrides: Vec::new(),
    };
    t6.slots[0] = WorkspaceSlotTemplate {
        slot_index: 0,
        data_source: DataSourcePreset::IntradayVn30F1m,
        symbol: "VN30F1M".to_owned(),
        timeframe: Timeframe::M15,
        indicators: IndicatorConfig {
            show_sma: false,
            show_ema: true,
            ema_period: 21,
            show_vwap: true,
            vwap_anchor_mode: VwapAnchorMode::AutoSession,
            show_market_profile: true,
            mp_num_bins: 24,
            mp_ib_bars: 4,
            mp_extend_naked_poc: true,
            show_rsi: false,
            show_cvd: true,
            cvd_reset_mode: CvdResetMode::Session,
            cvd_sma_period: 9,
            ..IndicatorConfig::default()
        },
        ..WorkspaceSlotTemplate::default()
    };
    t6.slots[1] = WorkspaceSlotTemplate {
        slot_index: 1,
        data_source: DataSourcePreset::Vn30Basket,
        symbol: "FPT".to_owned(),
        timeframe: Timeframe::D1,
        indicators: IndicatorConfig {
            show_sma: true,
            show_ema: true,
            show_vwap: true,
            vwap_anchor_mode: VwapAnchorMode::Weekly,
            show_market_profile: false,
            show_rsi: false,
            show_cvd: true,
            cvd_reset_mode: CvdResetMode::Cumulative,
            cvd_sma_period: 9,
            ..IndicatorConfig::default()
        },
        ..WorkspaceSlotTemplate::default()
    };

    for t in [&mut t1, &mut t2, &mut t3, &mut t4, &mut t5, &mut t6] {
        t.sanitize();
    }
    vec![t1, t2, t3, t4, t5, t6]
}

fn truncate_utf8(s: &mut String, max_bytes: usize) {
    if s.len() <= max_bytes {
        return;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s.truncate(end);
}

// ============================================================================
// Pretty Encoders (`JSON` & `RON`)
// ============================================================================

fn escape_str_into(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn fmt_f64(v: f64) -> String {
    let safe = if v.is_finite() { v } else { 0.0 };
    format!("{safe:?}")
}

fn fmt_f32(v: f32) -> String {
    let safe = if v.is_finite() { v } else { 1.0 };
    format!("{safe:?}")
}

fn encode_json_template(t: &WorkspaceTemplate) -> String {
    let mut out = String::with_capacity(2048);
    out.push_str("{\n");
    let _ = writeln!(out, "  \"schema_version\": {},", t.schema_version);
    out.push_str("  \"name\": ");
    escape_str_into(&mut out, &t.name);
    out.push_str(",\n  \"description\": ");
    escape_str_into(&mut out, &t.description);
    out.push_str(",\n  \"layout\": ");
    escape_str_into(&mut out, t.layout.short_label());
    out.push_str(",\n  \"sync\": {\n");
    let _ = writeln!(out, "    \"sync_crosshair\": {},", t.sync.sync_crosshair);
    let _ = writeln!(out, "    \"sync_time_range\": {},", t.sync.sync_time_range);
    let _ = writeln!(out, "    \"sync_symbol\": {},", t.sync.sync_symbol);
    let _ = writeln!(out, "    \"sync_timeframe\": {}", t.sync.sync_timeframe);
    out.push_str("  },\n");
    let _ = writeln!(out, "  \"active_slot\": {},", t.active_slot);
    out.push_str("  \"slots\": [\n");
    for (idx, s) in t.slots.iter().enumerate() {
        out.push_str("    {\n");
        let _ = writeln!(out, "      \"slot_index\": {},", s.slot_index);
        let _ = writeln!(out, "      \"data_source\": \"{:?}\",", s.data_source);
        out.push_str("      \"symbol\": ");
        escape_str_into(&mut out, &s.symbol);
        let _ = writeln!(out, ",\n      \"timeframe\": \"{:?}\",", s.timeframe);
        let _ = writeln!(out, "      \"scale_mode\": \"{:?}\",", s.scale_mode);
        let _ = writeln!(out, "      \"show_volume\": {},", s.show_volume);
        let _ = writeln!(out, "      \"show_grid\": {},", s.show_grid);
        out.push_str("      \"compare_benchmark\": ");
        match &s.compare_benchmark {
            Some(b) => escape_str_into(&mut out, b),
            None => out.push_str("null"),
        }
        let _ = writeln!(
            out,
            ",\n      \"show_compare_overlay\": {},",
            s.show_compare_overlay
        );
        encode_json_indicators(&mut out, &s.indicators);
        out.push_str(",\n");
        encode_json_stretches(&mut out, &s.pane_stretches);
        out.push('\n');
        if idx + 1 == t.slots.len() {
            out.push_str("    }\n");
        } else {
            out.push_str("    },\n");
        }
    }
    out.push_str("  ],\n");
    encode_json_dock_layout(&mut out, &t.dock_layout);
    out.push_str(",\n  \"hotkey_profile\": ");
    escape_str_into(&mut out, t.hotkey_profile.as_id());
    out.push_str(",\n  \"hotkey_base_preset\": ");
    match t.hotkey_base_preset {
        Some(base) => escape_str_into(&mut out, base.as_id()),
        None => out.push_str("null"),
    }
    out.push_str(",\n");
    encode_json_hotkey_overrides(&mut out, &t.hotkey_overrides);
    out.push_str("\n}");
    out
}

fn encode_json_dock_layout(out: &mut String, dl: &WorkspaceDockPreset) {
    out.push_str("  \"dock_layout\": {\n");
    let _ = writeln!(out, "    \"show_scanner_dock\": {},", dl.show_scanner_dock);
    let _ = writeln!(out, "    \"scanner_tab\": \"{}\",", dl.scanner_tab.as_id());
    let _ = writeln!(
        out,
        "    \"show_trading_panel\": {},",
        dl.show_trading_panel
    );
    let _ = writeln!(out, "    \"show_alerts_panel\": {},", dl.show_alerts_panel);
    let _ = writeln!(
        out,
        "    \"show_strategy_panel\": {},",
        dl.show_strategy_panel
    );
    let _ = writeln!(
        out,
        "    \"show_journal_panel\": {},",
        dl.show_journal_panel
    );
    let _ = writeln!(
        out,
        "    \"right_dock_tab\": \"{}\",",
        dl.right_dock_tab.as_id()
    );
    let _ = writeln!(
        out,
        "    \"trading_view_mode\": \"{}\",",
        dl.trading_view_mode.as_id()
    );
    let _ = writeln!(out, "    \"show_replay_bar\": {}", dl.show_replay_bar);
    out.push_str("  }");
}

fn encode_json_hotkey_overrides(out: &mut String, overrides: &[HotkeyOverride]) {
    if overrides.is_empty() {
        out.push_str("  \"hotkey_overrides\": []");
        return;
    }
    out.push_str("  \"hotkey_overrides\": [\n");
    for (idx, ov) in overrides.iter().enumerate() {
        out.push_str("    { \"action\": ");
        escape_str_into(out, ov.action.as_id());
        out.push_str(", \"chord\": ");
        match ov.chord {
            Some(c) => {
                let _ = write!(
                    out,
                    "{{ \"key\": \"{}\", \"shift\": {}, \"alt\": {} }}",
                    c.key.as_id(),
                    c.modifiers.shift,
                    c.modifiers.alt
                );
            }
            None => out.push_str("null"),
        }
        if idx + 1 == overrides.len() {
            out.push_str(" }\n");
        } else {
            out.push_str(" },\n");
        }
    }
    out.push_str("  ]");
}

fn encode_json_indicators(out: &mut String, ind: &IndicatorConfig) {
    out.push_str("      \"indicators\": {\n");
    let _ = writeln!(out, "        \"show_sma\": {},", ind.show_sma);
    let _ = writeln!(out, "        \"sma_period\": {},", ind.sma_period);
    let _ = writeln!(out, "        \"show_ema\": {},", ind.show_ema);
    let _ = writeln!(out, "        \"ema_period\": {},", ind.ema_period);
    let _ = writeln!(out, "        \"show_bbands\": {},", ind.show_bbands);
    let _ = writeln!(out, "        \"bb_period\": {},", ind.bb_period);
    let _ = writeln!(
        out,
        "        \"bb_std_mult\": {},",
        fmt_f64(ind.bb_std_mult)
    );
    let _ = writeln!(out, "        \"show_vwap\": {},", ind.show_vwap);
    let _ = writeln!(
        out,
        "        \"vwap_anchor_mode\": \"{:?}\",",
        ind.vwap_anchor_mode
    );
    let _ = writeln!(
        out,
        "        \"vwap_band_1\": {},",
        fmt_f64(ind.vwap_band_1)
    );
    let _ = writeln!(
        out,
        "        \"vwap_band_2\": {},",
        fmt_f64(ind.vwap_band_2)
    );
    let _ = writeln!(out, "        \"show_vrvp\": {},", ind.show_vrvp);
    let _ = writeln!(out, "        \"vrvp_rows\": {},", ind.vrvp_rows);
    let _ = writeln!(
        out,
        "        \"vrvp_value_area_pct\": {},",
        fmt_f64(ind.vrvp_value_area_pct)
    );
    let _ = writeln!(
        out,
        "        \"show_market_profile\": {},",
        ind.show_market_profile
    );
    let _ = writeln!(
        out,
        "        \"mp_session_period\": \"{:?}\",",
        ind.mp_session_period
    );
    let _ = writeln!(out, "        \"mp_num_bins\": {},", ind.mp_num_bins);
    let _ = writeln!(out, "        \"mp_ib_bars\": {},", ind.mp_ib_bars);
    let _ = writeln!(
        out,
        "        \"mp_extend_naked_poc\": {},",
        ind.mp_extend_naked_poc
    );
    let _ = writeln!(out, "        \"show_ichimoku\": {},", ind.show_ichimoku);
    let _ = writeln!(out, "        \"ichimoku_tenkan\": {},", ind.ichimoku_tenkan);
    let _ = writeln!(out, "        \"ichimoku_kijun\": {},", ind.ichimoku_kijun);
    let _ = writeln!(
        out,
        "        \"ichimoku_senkou_b\": {},",
        ind.ichimoku_senkou_b
    );
    let _ = writeln!(
        out,
        "        \"ichimoku_displacement\": {},",
        ind.ichimoku_displacement
    );
    let _ = writeln!(
        out,
        "        \"show_ichimoku_chikou\": {},",
        ind.show_ichimoku_chikou
    );
    let _ = writeln!(out, "        \"show_rsi\": {},", ind.show_rsi);
    let _ = writeln!(out, "        \"rsi_period\": {},", ind.rsi_period);
    let _ = writeln!(out, "        \"show_macd\": {},", ind.show_macd);
    let _ = writeln!(out, "        \"macd_fast\": {},", ind.macd_fast);
    let _ = writeln!(out, "        \"macd_slow\": {},", ind.macd_slow);
    let _ = writeln!(out, "        \"macd_signal\": {},", ind.macd_signal);
    let _ = writeln!(out, "        \"show_atr\": {},", ind.show_atr);
    let _ = writeln!(out, "        \"atr_period\": {},", ind.atr_period);
    let _ = writeln!(out, "        \"show_rs\": {},", ind.show_rs);
    let _ = writeln!(out, "        \"rs_sma_period\": {},", ind.rs_sma_period);
    let _ = writeln!(out, "        \"show_cvd\": {},", ind.show_cvd);
    let _ = writeln!(
        out,
        "        \"cvd_reset_mode\": \"{:?}\",",
        ind.cvd_reset_mode
    );
    let _ = writeln!(out, "        \"cvd_sma_period\": {}", ind.cvd_sma_period);
    out.push_str("      }");
}

fn encode_json_stretches(out: &mut String, st: &WorkspacePaneStretches) {
    out.push_str("      \"pane_stretches\": {\n");
    let _ = writeln!(out, "        \"main\": {},", fmt_f32(st.main));
    let _ = writeln!(out, "        \"rsi\": {},", fmt_f32(st.rsi));
    let _ = writeln!(out, "        \"macd\": {},", fmt_f32(st.macd));
    let _ = writeln!(out, "        \"atr\": {},", fmt_f32(st.atr));
    let _ = writeln!(out, "        \"rs\": {},", fmt_f32(st.rs));
    let _ = writeln!(out, "        \"cvd\": {},", fmt_f32(st.cvd));
    let _ = writeln!(out, "        \"equity\": {}", fmt_f32(st.equity));
    out.push_str("      }");
}

fn encode_ron_template(t: &WorkspaceTemplate) -> String {
    let mut out = String::with_capacity(2048);
    let _ = writeln!(
        out,
        "// Sumi Workstation Template (schema_version = {})",
        t.schema_version
    );
    out.push_str("WorkspaceTemplate(\n");
    let _ = writeln!(out, "    schema_version: {},", t.schema_version);
    out.push_str("    name: ");
    escape_str_into(&mut out, &t.name);
    out.push_str(",\n    description: ");
    escape_str_into(&mut out, &t.description);
    let _ = writeln!(out, ",\n    layout: {:?},", t.layout);
    out.push_str("    sync: (\n");
    let _ = writeln!(out, "        sync_crosshair: {},", t.sync.sync_crosshair);
    let _ = writeln!(out, "        sync_time_range: {},", t.sync.sync_time_range);
    let _ = writeln!(out, "        sync_symbol: {},", t.sync.sync_symbol);
    let _ = writeln!(out, "        sync_timeframe: {},", t.sync.sync_timeframe);
    out.push_str("    ),\n");
    let _ = writeln!(out, "    active_slot: {},", t.active_slot);
    out.push_str("    slots: [\n");
    for s in &t.slots {
        out.push_str("        (\n");
        let _ = writeln!(out, "            slot_index: {},", s.slot_index);
        let _ = writeln!(out, "            data_source: {:?},", s.data_source);
        out.push_str("            symbol: ");
        escape_str_into(&mut out, &s.symbol);
        let _ = writeln!(out, ",\n            timeframe: {:?},", s.timeframe);
        let _ = writeln!(out, "            scale_mode: {:?},", s.scale_mode);
        let _ = writeln!(out, "            show_volume: {},", s.show_volume);
        let _ = writeln!(out, "            show_grid: {},", s.show_grid);
        out.push_str("            compare_benchmark: ");
        match &s.compare_benchmark {
            Some(b) => {
                out.push_str("Some(");
                escape_str_into(&mut out, b);
                out.push(')');
            }
            None => out.push_str("None"),
        }
        let _ = writeln!(
            out,
            ",\n            show_compare_overlay: {},",
            s.show_compare_overlay
        );
        encode_ron_indicators(&mut out, &s.indicators);
        encode_ron_stretches(&mut out, &s.pane_stretches);
        out.push_str("        ),\n");
    }
    out.push_str("    ],\n");
    encode_ron_dock_layout(&mut out, &t.dock_layout);
    let _ = writeln!(out, "    hotkey_profile: {},", t.hotkey_profile.as_id());
    match t.hotkey_base_preset {
        Some(base) => {
            let _ = writeln!(out, "    hotkey_base_preset: Some({}),", base.as_id());
        }
        None => {
            let _ = writeln!(out, "    hotkey_base_preset: None,");
        }
    }
    encode_ron_hotkey_overrides(&mut out, &t.hotkey_overrides);
    out.push(')');
    out
}

fn encode_ron_dock_layout(out: &mut String, dl: &WorkspaceDockPreset) {
    out.push_str("    dock_layout: (\n");
    let _ = writeln!(out, "        show_scanner_dock: {},", dl.show_scanner_dock);
    let _ = writeln!(out, "        scanner_tab: {},", dl.scanner_tab.as_id());
    let _ = writeln!(
        out,
        "        show_trading_panel: {},",
        dl.show_trading_panel
    );
    let _ = writeln!(out, "        show_alerts_panel: {},", dl.show_alerts_panel);
    let _ = writeln!(
        out,
        "        show_strategy_panel: {},",
        dl.show_strategy_panel
    );
    let _ = writeln!(
        out,
        "        show_journal_panel: {},",
        dl.show_journal_panel
    );
    let _ = writeln!(
        out,
        "        right_dock_tab: {},",
        dl.right_dock_tab.as_id()
    );
    let _ = writeln!(
        out,
        "        trading_view_mode: {},",
        dl.trading_view_mode.as_id()
    );
    let _ = writeln!(out, "        show_replay_bar: {},", dl.show_replay_bar);
    out.push_str("    ),\n");
}

fn encode_ron_hotkey_overrides(out: &mut String, overrides: &[HotkeyOverride]) {
    if overrides.is_empty() {
        out.push_str("    hotkey_overrides: [],\n");
        return;
    }
    out.push_str("    hotkey_overrides: [\n");
    for ov in overrides {
        let _ = write!(out, "        (action: {}, chord: ", ov.action.as_id());
        match ov.chord {
            Some(c) => {
                let _ = write!(
                    out,
                    "Some((key: {}, shift: {}, alt: {}))",
                    c.key.as_id(),
                    c.modifiers.shift,
                    c.modifiers.alt
                );
            }
            None => out.push_str("None"),
        }
        out.push_str("),\n");
    }
    out.push_str("    ],\n");
}

fn encode_ron_indicators(out: &mut String, ind: &IndicatorConfig) {
    out.push_str("            indicators: (\n");
    let _ = writeln!(out, "                show_sma: {},", ind.show_sma);
    let _ = writeln!(out, "                sma_period: {},", ind.sma_period);
    let _ = writeln!(out, "                show_ema: {},", ind.show_ema);
    let _ = writeln!(out, "                ema_period: {},", ind.ema_period);
    let _ = writeln!(out, "                show_bbands: {},", ind.show_bbands);
    let _ = writeln!(out, "                bb_period: {},", ind.bb_period);
    let _ = writeln!(
        out,
        "                bb_std_mult: {},",
        fmt_f64(ind.bb_std_mult)
    );
    let _ = writeln!(out, "                show_vwap: {},", ind.show_vwap);
    let _ = writeln!(
        out,
        "                vwap_anchor_mode: {:?},",
        ind.vwap_anchor_mode
    );
    let _ = writeln!(
        out,
        "                vwap_band_1: {},",
        fmt_f64(ind.vwap_band_1)
    );
    let _ = writeln!(
        out,
        "                vwap_band_2: {},",
        fmt_f64(ind.vwap_band_2)
    );
    let _ = writeln!(out, "                show_vrvp: {},", ind.show_vrvp);
    let _ = writeln!(out, "                vrvp_rows: {},", ind.vrvp_rows);
    let _ = writeln!(
        out,
        "                vrvp_value_area_pct: {},",
        fmt_f64(ind.vrvp_value_area_pct)
    );
    let _ = writeln!(
        out,
        "                show_market_profile: {},",
        ind.show_market_profile
    );
    let _ = writeln!(
        out,
        "                mp_session_period: {:?},",
        ind.mp_session_period
    );
    let _ = writeln!(out, "                mp_num_bins: {},", ind.mp_num_bins);
    let _ = writeln!(out, "                mp_ib_bars: {},", ind.mp_ib_bars);
    let _ = writeln!(
        out,
        "                mp_extend_naked_poc: {},",
        ind.mp_extend_naked_poc
    );
    let _ = writeln!(out, "                show_ichimoku: {},", ind.show_ichimoku);
    let _ = writeln!(
        out,
        "                ichimoku_tenkan: {},",
        ind.ichimoku_tenkan
    );
    let _ = writeln!(
        out,
        "                ichimoku_kijun: {},",
        ind.ichimoku_kijun
    );
    let _ = writeln!(
        out,
        "                ichimoku_senkou_b: {},",
        ind.ichimoku_senkou_b
    );
    let _ = writeln!(
        out,
        "                ichimoku_displacement: {},",
        ind.ichimoku_displacement
    );
    let _ = writeln!(
        out,
        "                show_ichimoku_chikou: {},",
        ind.show_ichimoku_chikou
    );
    let _ = writeln!(out, "                show_rsi: {},", ind.show_rsi);
    let _ = writeln!(out, "                rsi_period: {},", ind.rsi_period);
    let _ = writeln!(out, "                show_macd: {},", ind.show_macd);
    let _ = writeln!(out, "                macd_fast: {},", ind.macd_fast);
    let _ = writeln!(out, "                macd_slow: {},", ind.macd_slow);
    let _ = writeln!(out, "                macd_signal: {},", ind.macd_signal);
    let _ = writeln!(out, "                show_atr: {},", ind.show_atr);
    let _ = writeln!(out, "                atr_period: {},", ind.atr_period);
    let _ = writeln!(out, "                show_rs: {},", ind.show_rs);
    let _ = writeln!(out, "                rs_sma_period: {},", ind.rs_sma_period);
    let _ = writeln!(out, "                show_cvd: {},", ind.show_cvd);
    let _ = writeln!(
        out,
        "                cvd_reset_mode: {:?},",
        ind.cvd_reset_mode
    );
    let _ = writeln!(
        out,
        "                cvd_sma_period: {},",
        ind.cvd_sma_period
    );
    out.push_str("            ),\n");
}

fn encode_ron_stretches(out: &mut String, st: &WorkspacePaneStretches) {
    out.push_str("            pane_stretches: (\n");
    let _ = writeln!(out, "                main: {},", fmt_f32(st.main));
    let _ = writeln!(out, "                rsi: {},", fmt_f32(st.rsi));
    let _ = writeln!(out, "                macd: {},", fmt_f32(st.macd));
    let _ = writeln!(out, "                atr: {},", fmt_f32(st.atr));
    let _ = writeln!(out, "                rs: {},", fmt_f32(st.rs));
    let _ = writeln!(out, "                cvd: {},", fmt_f32(st.cvd));
    let _ = writeln!(out, "                equity: {},", fmt_f32(st.equity));
    out.push_str("            ),\n");
}

// ============================================================================
// Unified Recursive-Descent JSON / RON Parser & Typed AST Validator
// ============================================================================

#[derive(Debug, Clone)]
enum DocValue {
    Struct(Vec<(String, DocValue)>),
    Array(Vec<DocValue>),
    String(String),
    Number(f64),
    Bool(bool),
    None,
    Some(Box<DocValue>),
    Ident(String),
}

struct Scanner<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
    allow_comments: bool,
}

impl<'a> Scanner<'a> {
    fn new(src: &'a str, allow_comments: bool) -> Self {
        Self {
            src,
            bytes: src.as_bytes(),
            pos: 0,
            allow_comments,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws(&mut self) -> Result<(), WorkspaceTemplateError> {
        loop {
            while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
                self.pos += 1;
            }
            if self.allow_comments && self.bytes[self.pos..].starts_with(b"//") {
                self.pos += 2;
                while let Some(b) = self.peek() {
                    self.pos += 1;
                    if b == b'\n' {
                        break;
                    }
                }
                continue;
            }
            if self.allow_comments && self.bytes[self.pos..].starts_with(b"/*") {
                self.pos += 2;
                let mut depth = 1_usize;
                while depth > 0 {
                    if self.pos + 1 >= self.bytes.len() {
                        return Err(WorkspaceTemplateError::Syntax(
                            "Unterminated block comment".to_owned(),
                        ));
                    }
                    if self.bytes[self.pos..].starts_with(b"/*") {
                        depth += 1;
                        self.pos += 2;
                    } else if self.bytes[self.pos..].starts_with(b"*/") {
                        depth -= 1;
                        self.pos += 2;
                    } else {
                        self.pos += 1;
                    }
                }
                continue;
            }
            break;
        }
        Ok(())
    }

    fn expect_byte(&mut self, expected: u8) -> Result<(), WorkspaceTemplateError> {
        self.skip_ws()?;
        match self.peek() {
            Some(b) if b == expected => {
                self.pos += 1;
                Ok(())
            }
            Some(other) => Err(WorkspaceTemplateError::Syntax(format!(
                "Expected '{}', found '{}' at byte {}",
                expected as char, other as char, self.pos
            ))),
            None => Err(WorkspaceTemplateError::Syntax(format!(
                "Expected '{}', reached EOF",
                expected as char
            ))),
        }
    }

    fn parse_string(&mut self) -> Result<String, WorkspaceTemplateError> {
        self.expect_byte(b'"')?;
        let mut out = String::new();
        while let Some(b) = self.peek() {
            self.pos += 1;
            if b == b'"' {
                return Ok(out);
            }
            if b == b'\\' {
                let esc = self.peek().ok_or_else(|| {
                    WorkspaceTemplateError::Syntax("Unterminated escape in string".to_owned())
                })?;
                self.pos += 1;
                match esc {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{0008}'),
                    b'f' => out.push('\u{000C}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let cp = self.parse_unicode_escape()?;
                        if (0xD800..=0xDBFF).contains(&cp) {
                            if !self.bytes[self.pos..].starts_with(b"\\u") {
                                return Err(WorkspaceTemplateError::Syntax(
                                    "Unpaired UTF-16 high surrogate".to_owned(),
                                ));
                            }
                            self.pos += 2;
                            let low = self.parse_unicode_escape()?;
                            if !(0xDC00..=0xDFFF).contains(&low) {
                                return Err(WorkspaceTemplateError::Syntax(
                                    "Invalid UTF-16 low surrogate".to_owned(),
                                ));
                            }
                            let full = 0x1_0000 + (((cp - 0xD800) << 10) | (low - 0xDC00));
                            let ch = char::from_u32(full).ok_or_else(|| {
                                WorkspaceTemplateError::Syntax("Invalid codepoint".to_owned())
                            })?;
                            out.push(ch);
                        } else {
                            let ch = char::from_u32(cp).ok_or_else(|| {
                                WorkspaceTemplateError::Syntax("Invalid unicode escape".to_owned())
                            })?;
                            out.push(ch);
                        }
                    }
                    other => {
                        return Err(WorkspaceTemplateError::Syntax(format!(
                            "Unknown escape '\\{}'",
                            other as char
                        )));
                    }
                }
            } else if !self.allow_comments && b < 0x20 {
                return Err(WorkspaceTemplateError::Syntax(format!(
                    "Unescaped control byte 0x{b:02x} in JSON string"
                )));
            } else if b < 0x80 {
                out.push(b as char);
            } else {
                // Consume full multibyte UTF-8 character in O(1) slice operations
                let start = self.pos - 1;
                let ch = self.src[start..].chars().next().ok_or_else(|| {
                    WorkspaceTemplateError::Syntax("Invalid UTF-8 char".to_owned())
                })?;
                self.pos = start + ch.len_utf8();
                out.push(ch);
            }
        }
        Err(WorkspaceTemplateError::Syntax(
            "Unterminated string literal".to_owned(),
        ))
    }

    fn parse_unicode_escape(&mut self) -> Result<u32, WorkspaceTemplateError> {
        if !self.allow_comments || self.peek() != Some(b'{') {
            if self.pos + 4 > self.bytes.len() {
                return Err(WorkspaceTemplateError::Syntax(
                    "Truncated \\uXXXX escape".to_owned(),
                ));
            }
            let hex = std::str::from_utf8(&self.bytes[self.pos..self.pos + 4]).map_err(|_| {
                WorkspaceTemplateError::Syntax("Invalid unicode escape bytes".to_owned())
            })?;
            let val = u32::from_str_radix(hex, 16)
                .map_err(|_| WorkspaceTemplateError::Syntax(format!("Invalid hex in \\u{hex}")))?;
            self.pos += 4;
            return Ok(val);
        }
        self.pos += 1; // skip '{'
        let start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F')) {
            self.pos += 1;
        }
        let hex_len = self.pos - start;
        if hex_len == 0 || hex_len > 6 {
            return Err(WorkspaceTemplateError::Syntax(
                "Invalid RON \\u{...} hex length".to_owned(),
            ));
        }
        if self.peek() != Some(b'}') {
            return Err(WorkspaceTemplateError::Syntax(
                "Expected '}' closing RON unicode escape".to_owned(),
            ));
        }
        let hex = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| WorkspaceTemplateError::Syntax("Invalid RON unicode escape".to_owned()))?;
        self.pos += 1; // skip '}'
        u32::from_str_radix(hex, 16)
            .map_err(|_| WorkspaceTemplateError::Syntax(format!("Invalid RON \\u{{{hex}}}")))
    }

    fn parse_ident(&mut self) -> Result<String, WorkspaceTemplateError> {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b.is_ascii_alphanumeric() || b == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start {
            return Err(WorkspaceTemplateError::Syntax(format!(
                "Expected identifier at byte {}",
                self.pos
            )));
        }
        Ok(String::from_utf8_lossy(&self.bytes[start..self.pos]).into_owned())
    }

    fn parse_number(&mut self) -> Result<f64, WorkspaceTemplateError> {
        let start = self.pos;
        if self.peek() == Some(b'-') || (self.allow_comments && self.peek() == Some(b'+')) {
            self.pos += 1;
        }
        if self.allow_comments
            && let Some(b'n' | b'N' | b'i' | b'I') = self.peek()
        {
            while let Some(b) = self.peek() {
                if b.is_ascii_alphanumeric() || b == b'_' {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            let raw = std::str::from_utf8(&self.bytes[start..self.pos]).unwrap_or("");
            return Err(WorkspaceTemplateError::NonFiniteNumber(raw.to_owned()));
        }
        while let Some(b) = self.peek() {
            if b.is_ascii_digit() || matches!(b, b'.' | b'e' | b'E' | b'+' | b'-' | b'_') {
                if self.allow_comments && b == b'_' {
                    self.pos += 1;
                    continue;
                }
                self.pos += 1;
            } else {
                break;
            }
        }
        let raw_slice = &self.bytes[start..self.pos];
        let raw = if self.allow_comments {
            String::from_utf8_lossy(raw_slice).replace('_', "")
        } else {
            String::from_utf8_lossy(raw_slice).into_owned()
        };
        let val = raw.parse::<f64>().map_err(|_| {
            WorkspaceTemplateError::Syntax(format!("Invalid number literal '{raw}'"))
        })?;
        if !val.is_finite() {
            return Err(WorkspaceTemplateError::NonFiniteNumber(raw));
        }
        Ok(val)
    }

    fn parse_json_value(&mut self, depth: usize) -> Result<DocValue, WorkspaceTemplateError> {
        if depth > MAX_PARSER_DEPTH {
            return Err(WorkspaceTemplateError::Syntax(
                "Max nesting depth exceeded".to_owned(),
            ));
        }
        self.skip_ws()?;
        match self.peek() {
            Some(b'{') => {
                self.pos += 1;
                let mut fields = Vec::new();
                self.skip_ws()?;
                if self.peek() == Some(b'}') {
                    self.pos += 1;
                    return Ok(DocValue::Struct(fields));
                }
                loop {
                    let key = self.parse_string()?;
                    self.expect_byte(b':')?;
                    let val = self.parse_json_value(depth + 1)?;
                    fields.push((key, val));
                    self.skip_ws()?;
                    match self.peek() {
                        Some(b',') => self.pos += 1,
                        Some(b'}') => {
                            self.pos += 1;
                            break;
                        }
                        _ => {
                            return Err(WorkspaceTemplateError::Syntax(format!(
                                "Expected ',' or '}}' in JSON object at byte {}",
                                self.pos
                            )));
                        }
                    }
                }
                Ok(DocValue::Struct(fields))
            }
            Some(b'[') => {
                self.pos += 1;
                let mut items = Vec::new();
                self.skip_ws()?;
                if self.peek() == Some(b']') {
                    self.pos += 1;
                    return Ok(DocValue::Array(items));
                }
                loop {
                    items.push(self.parse_json_value(depth + 1)?);
                    self.skip_ws()?;
                    match self.peek() {
                        Some(b',') => self.pos += 1,
                        Some(b']') => {
                            self.pos += 1;
                            break;
                        }
                        _ => {
                            return Err(WorkspaceTemplateError::Syntax(format!(
                                "Expected ',' or ']' in JSON array at byte {}",
                                self.pos
                            )));
                        }
                    }
                }
                Ok(DocValue::Array(items))
            }
            Some(b'"') => Ok(DocValue::String(self.parse_string()?)),
            Some(b't' | b'f' | b'n') => {
                let id = self.parse_ident()?;
                match id.as_str() {
                    "true" => Ok(DocValue::Bool(true)),
                    "false" => Ok(DocValue::Bool(false)),
                    "null" => Ok(DocValue::None),
                    other => Err(WorkspaceTemplateError::Syntax(format!(
                        "Unexpected JSON keyword '{other}'"
                    ))),
                }
            }
            Some(b'-' | b'0'..=b'9') => Ok(DocValue::Number(self.parse_number()?)),
            Some(other) => Err(WorkspaceTemplateError::Syntax(format!(
                "Unexpected character '{}' at byte {}",
                other as char, self.pos
            ))),
            None => Err(WorkspaceTemplateError::Syntax(
                "Unexpected EOF in JSON".to_owned(),
            )),
        }
    }

    fn parse_ron_value(&mut self, depth: usize) -> Result<DocValue, WorkspaceTemplateError> {
        if depth > MAX_PARSER_DEPTH {
            return Err(WorkspaceTemplateError::Syntax(
                "Max nesting depth exceeded".to_owned(),
            ));
        }
        self.skip_ws()?;
        match self.peek() {
            Some(b'(') => self.parse_ron_struct_body(depth),
            Some(b'[') => {
                self.pos += 1;
                let mut items = Vec::new();
                loop {
                    self.skip_ws()?;
                    if self.peek() == Some(b']') {
                        self.pos += 1;
                        break;
                    }
                    items.push(self.parse_ron_value(depth + 1)?);
                    self.skip_ws()?;
                    if self.peek() == Some(b',') {
                        self.pos += 1;
                    } else if self.peek() == Some(b']') {
                        self.pos += 1;
                        break;
                    } else {
                        return Err(WorkspaceTemplateError::Syntax(format!(
                            "Expected ',' or ']' in RON array at byte {}",
                            self.pos
                        )));
                    }
                }
                Ok(DocValue::Array(items))
            }
            Some(b'"') => Ok(DocValue::String(self.parse_string()?)),
            Some(b'-' | b'+' | b'0'..=b'9') => Ok(DocValue::Number(self.parse_number()?)),
            Some(b) if b.is_ascii_alphabetic() || b == b'_' => {
                let ident = self.parse_ident()?;
                if matches!(
                    ident.to_ascii_lowercase().as_str(),
                    "nan" | "inf" | "infinity"
                ) {
                    return Err(WorkspaceTemplateError::NonFiniteNumber(ident));
                }
                if ident == "true" {
                    return Ok(DocValue::Bool(true));
                }
                if ident == "false" {
                    return Ok(DocValue::Bool(false));
                }
                if ident == "None" {
                    return Ok(DocValue::None);
                }
                self.skip_ws()?;
                if ident == "Some" && self.peek() == Some(b'(') {
                    self.pos += 1;
                    let inner = self.parse_ron_value(depth + 1)?;
                    self.skip_ws()?;
                    if self.peek() == Some(b',') {
                        self.pos += 1;
                    }
                    self.expect_byte(b')')?;
                    return Ok(DocValue::Some(Box::new(inner)));
                }
                if self.peek() == Some(b'(') {
                    return self.parse_ron_struct_body(depth);
                }
                Ok(DocValue::Ident(ident))
            }
            Some(other) => Err(WorkspaceTemplateError::Syntax(format!(
                "Unexpected character '{}' at byte {}",
                other as char, self.pos
            ))),
            None => Err(WorkspaceTemplateError::Syntax(
                "Unexpected EOF in RON".to_owned(),
            )),
        }
    }

    fn parse_ron_struct_body(&mut self, depth: usize) -> Result<DocValue, WorkspaceTemplateError> {
        self.expect_byte(b'(')?;
        let mut fields = Vec::new();
        loop {
            self.skip_ws()?;
            if self.peek() == Some(b')') {
                self.pos += 1;
                break;
            }
            let key = if self.peek() == Some(b'"') {
                self.parse_string()?
            } else {
                self.parse_ident()?
            };
            self.expect_byte(b':')?;
            let val = self.parse_ron_value(depth + 1)?;
            fields.push((key, val));
            self.skip_ws()?;
            if self.peek() == Some(b',') {
                self.pos += 1;
            } else if self.peek() == Some(b')') {
                self.pos += 1;
                break;
            } else {
                return Err(WorkspaceTemplateError::Syntax(format!(
                    "Expected ',' or ')' in RON struct at byte {}",
                    self.pos
                )));
            }
        }
        Ok(DocValue::Struct(fields))
    }
}

fn strip_leading_ws_and_comments(mut src: &str) -> Result<&str, WorkspaceTemplateError> {
    loop {
        src = src.trim_start();
        if let Some(rest) = src.strip_prefix("//") {
            src = rest.split_once('\n').map_or("", |(_, after)| after);
            continue;
        }
        if let Some(rest) = src.strip_prefix("/*") {
            let bytes = rest.as_bytes();
            let mut i = 0_usize;
            let mut depth = 1_usize;
            while depth > 0 {
                if i + 1 >= bytes.len() {
                    return Err(WorkspaceTemplateError::Syntax(
                        "Unterminated block comment".to_owned(),
                    ));
                }
                if bytes[i..].starts_with(b"/*") {
                    depth += 1;
                    i += 2;
                } else if bytes[i..].starts_with(b"*/") {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            src = &rest[i..];
            continue;
        }
        break;
    }
    Ok(src)
}

fn parse_json_doc(input: &str) -> Result<DocValue, WorkspaceTemplateError> {
    let mut sc = Scanner::new(input, false);
    let doc = sc.parse_json_value(0)?;
    sc.skip_ws()?;
    if sc.pos < sc.bytes.len() {
        return Err(WorkspaceTemplateError::Syntax(format!(
            "Trailing tokens at byte {}",
            sc.pos
        )));
    }
    Ok(doc)
}

fn parse_ron_doc(input: &str) -> Result<DocValue, WorkspaceTemplateError> {
    let mut sc = Scanner::new(input, true);
    let doc = sc.parse_ron_value(0)?;
    sc.skip_ws()?;
    if sc.pos < sc.bytes.len() {
        return Err(WorkspaceTemplateError::Syntax(format!(
            "Trailing RON tokens at byte {}",
            sc.pos
        )));
    }
    Ok(doc)
}

// ============================================================================
// Typed Extraction From `DocValue`
// ============================================================================

impl WorkspaceTemplate {
    fn from_doc_value(val: &DocValue) -> Result<Self, WorkspaceTemplateError> {
        let fields = expect_struct(val, "WorkspaceTemplate")?;
        validate_struct_fields(
            fields,
            "WorkspaceTemplate",
            &[
                "schema_version",
                "name",
                "description",
                "layout",
                "sync",
                "active_slot",
                "slots",
                "dock_layout",
                "hotkey_profile",
                "hotkey_base_preset",
                "hotkey_overrides",
            ],
        )?;
        let mut tpl = Self::default();
        let ver_val = find_field(fields, "schema_version").ok_or_else(|| {
            WorkspaceTemplateError::InvalidField {
                field: "schema_version".to_owned(),
                reason: "missing required field 'schema_version'".to_owned(),
            }
        })?;
        let ver_f = expect_f64(ver_val, "schema_version")?;
        if ver_f < 0.0 || ver_f.fract() != 0.0 || ver_f > f64::from(u32::MAX) {
            return Err(WorkspaceTemplateError::InvalidField {
                field: "schema_version".to_owned(),
                reason: format!("expected non-negative integer schema_version, got {ver_f}"),
            });
        }
        let ver = ver_f as u32;
        if ver != WORKSPACE_TEMPLATE_SCHEMA_VERSION {
            return Err(WorkspaceTemplateError::UnsupportedSchemaVersion(ver));
        }
        tpl.schema_version = ver;
        if let Some(v) = find_field(fields, "name") {
            tpl.name = expect_string(v, "name")?;
        }
        if let Some(v) = find_field(fields, "description") {
            tpl.description = expect_string(v, "description")?;
        }
        if let Some(v) = find_field(fields, "layout") {
            let raw = expect_label(v, "layout")?;
            tpl.layout = MultiChartLayoutKind::parse(&raw).ok_or_else(|| {
                WorkspaceTemplateError::InvalidField {
                    field: "layout".to_owned(),
                    reason: format!("unknown layout '{raw}'"),
                }
            })?;
        }
        if let Some(v) = find_field(fields, "sync") {
            tpl.sync = decode_sync(v)?;
        }
        if let Some(v) = find_field(fields, "active_slot") {
            tpl.active_slot = expect_usize(v, "active_slot")?;
        }
        if tpl.active_slot >= tpl.layout.slot_count() {
            return Err(WorkspaceTemplateError::InvalidField {
                field: "active_slot".to_owned(),
                reason: format!(
                    "active_slot {} out of range for layout {:?} ({} slots)",
                    tpl.active_slot,
                    tpl.layout,
                    tpl.layout.slot_count()
                ),
            });
        }
        let slots_val =
            find_field(fields, "slots").ok_or_else(|| WorkspaceTemplateError::InvalidField {
                field: "slots".to_owned(),
                reason: "missing required field 'slots'".to_owned(),
            })?;
        let arr = match slots_val {
            DocValue::Array(items) => items,
            _ => {
                return Err(WorkspaceTemplateError::InvalidField {
                    field: "slots".to_owned(),
                    reason: "expected array of slot templates".to_owned(),
                });
            }
        };
        let mut decoded = Vec::with_capacity(arr.len().min(MAX_MULTI_CHART_SLOTS));
        for (i, item) in arr.iter().take(MAX_MULTI_CHART_SLOTS).enumerate() {
            decoded.push(decode_slot(item, i)?);
        }
        if arr.len() < tpl.layout.slot_count() || arr.len() > MAX_MULTI_CHART_SLOTS {
            return Err(WorkspaceTemplateError::InvalidField {
                field: "slots".to_owned(),
                reason: format!(
                    "expected between {} and {} slots for layout {:?}, got {}",
                    tpl.layout.slot_count(),
                    MAX_MULTI_CHART_SLOTS,
                    tpl.layout,
                    arr.len()
                ),
            });
        }
        tpl.slots = decoded;
        if let Some(v) = find_field(fields, "dock_layout") {
            tpl.dock_layout = decode_dock_layout(v)?;
        }
        if let Some(v) = find_field(fields, "hotkey_profile") {
            tpl.hotkey_profile = decode_hotkey_profile_id(v, "hotkey_profile")?;
        }
        if let Some(v) = find_field(fields, "hotkey_base_preset") {
            tpl.hotkey_base_preset = match v {
                DocValue::None => None,
                DocValue::Some(inner) => {
                    Some(decode_hotkey_profile_id(inner, "hotkey_base_preset")?)
                }
                other => Some(decode_hotkey_profile_id(other, "hotkey_base_preset")?),
            };
        }
        if let Some(v) = find_field(fields, "hotkey_overrides") {
            tpl.hotkey_overrides = decode_hotkey_overrides(v)?;
        }
        tpl.sanitize();
        Ok(tpl)
    }
}

fn validate_struct_fields(
    fields: &[(String, DocValue)],
    parent: &str,
    allowed: &[&str],
) -> Result<(), WorkspaceTemplateError> {
    let mut seen = std::collections::HashSet::with_capacity(fields.len());
    for (k, _) in fields {
        if !seen.insert(k.as_str()) {
            return Err(WorkspaceTemplateError::InvalidField {
                field: format!("{parent}.{k}"),
                reason: "duplicate field".to_owned(),
            });
        }
        if !allowed.contains(&k.as_str()) {
            return Err(WorkspaceTemplateError::InvalidField {
                field: format!("{parent}.{k}"),
                reason: "unknown field".to_owned(),
            });
        }
    }
    Ok(())
}

fn find_field<'a>(fields: &'a [(String, DocValue)], key: &str) -> Option<&'a DocValue> {
    fields.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn expect_struct<'a>(
    val: &'a DocValue,
    field: &str,
) -> Result<&'a [(String, DocValue)], WorkspaceTemplateError> {
    match val {
        DocValue::Struct(fields) => Ok(fields.as_slice()),
        _ => Err(WorkspaceTemplateError::InvalidField {
            field: field.to_owned(),
            reason: "expected object/struct".to_owned(),
        }),
    }
}

fn expect_f64(val: &DocValue, field: &str) -> Result<f64, WorkspaceTemplateError> {
    match val {
        DocValue::Number(v) if v.is_finite() => Ok(*v),
        DocValue::Number(_) => Err(WorkspaceTemplateError::NonFiniteNumber(field.to_owned())),
        _ => Err(WorkspaceTemplateError::InvalidField {
            field: field.to_owned(),
            reason: "expected finite number".to_owned(),
        }),
    }
}

fn expect_f32(val: &DocValue, field: &str) -> Result<f32, WorkspaceTemplateError> {
    let v = expect_f64(val, field)? as f32;
    if v.is_finite() {
        Ok(v)
    } else {
        Err(WorkspaceTemplateError::NonFiniteNumber(field.to_owned()))
    }
}

fn expect_usize(val: &DocValue, field: &str) -> Result<usize, WorkspaceTemplateError> {
    let n = expect_f64(val, field)?;
    let max_exact = ((1_u64 << 53) - 1) as f64;
    if n < 0.0 || n.fract() != 0.0 || n > max_exact || (n as u128) > (usize::MAX as u128) {
        return Err(WorkspaceTemplateError::InvalidField {
            field: field.to_owned(),
            reason: format!("expected non-negative exact integer, got {n}"),
        });
    }
    Ok(n as usize)
}

fn expect_bool(val: &DocValue, field: &str) -> Result<bool, WorkspaceTemplateError> {
    match val {
        DocValue::Bool(b) => Ok(*b),
        _ => Err(WorkspaceTemplateError::InvalidField {
            field: field.to_owned(),
            reason: "expected boolean".to_owned(),
        }),
    }
}

fn expect_string(val: &DocValue, field: &str) -> Result<String, WorkspaceTemplateError> {
    match val {
        DocValue::String(s) => Ok(s.clone()),
        _ => Err(WorkspaceTemplateError::InvalidField {
            field: field.to_owned(),
            reason: "expected string".to_owned(),
        }),
    }
}

fn expect_label(val: &DocValue, field: &str) -> Result<String, WorkspaceTemplateError> {
    match val {
        DocValue::String(s) | DocValue::Ident(s) => Ok(s.clone()),
        _ => Err(WorkspaceTemplateError::InvalidField {
            field: field.to_owned(),
            reason: "expected enum variant or string".to_owned(),
        }),
    }
}

fn expect_opt_string(
    val: &DocValue,
    field: &str,
) -> Result<Option<String>, WorkspaceTemplateError> {
    match val {
        DocValue::None => Ok(None),
        DocValue::Some(inner) => expect_string(inner, field).map(Some),
        DocValue::String(s) => Ok(Some(s.clone())),
        _ => Err(WorkspaceTemplateError::InvalidField {
            field: field.to_owned(),
            reason: "expected null/None or string".to_owned(),
        }),
    }
}

fn decode_sync(val: &DocValue) -> Result<MultiChartSyncConfig, WorkspaceTemplateError> {
    let f = expect_struct(val, "sync")?;
    validate_struct_fields(
        f,
        "sync",
        &[
            "sync_crosshair",
            "sync_time_range",
            "sync_symbol",
            "sync_timeframe",
        ],
    )?;
    let mut cfg = MultiChartSyncConfig::default();
    if let Some(v) = find_field(f, "sync_crosshair") {
        cfg.sync_crosshair = expect_bool(v, "sync.sync_crosshair")?;
    }
    if let Some(v) = find_field(f, "sync_time_range") {
        cfg.sync_time_range = expect_bool(v, "sync.sync_time_range")?;
    }
    if let Some(v) = find_field(f, "sync_symbol") {
        cfg.sync_symbol = expect_bool(v, "sync.sync_symbol")?;
    }
    if let Some(v) = find_field(f, "sync_timeframe") {
        cfg.sync_timeframe = expect_bool(v, "sync.sync_timeframe")?;
    }
    Ok(cfg)
}

fn decode_slot(
    val: &DocValue,
    default_idx: usize,
) -> Result<WorkspaceSlotTemplate, WorkspaceTemplateError> {
    let f = expect_struct(val, "slot")?;
    validate_struct_fields(
        f,
        "slot",
        &[
            "slot_index",
            "data_source",
            "symbol",
            "timeframe",
            "scale_mode",
            "show_volume",
            "show_grid",
            "compare_benchmark",
            "show_compare_overlay",
            "indicators",
            "pane_stretches",
        ],
    )?;
    let mut s = WorkspaceSlotTemplate {
        slot_index: default_idx,
        ..WorkspaceSlotTemplate::default()
    };
    if let Some(v) = find_field(f, "slot_index") {
        s.slot_index = expect_usize(v, "slot.slot_index")?;
    }
    if let Some(v) = find_field(f, "data_source") {
        let raw = expect_label(v, "slot.data_source")?;
        s.data_source =
            parse_data_source(&raw).ok_or_else(|| WorkspaceTemplateError::InvalidField {
                field: "slot.data_source".to_owned(),
                reason: format!("unknown data_source '{raw}'"),
            })?;
    }
    if let Some(v) = find_field(f, "symbol") {
        s.symbol = expect_string(v, "slot.symbol")?;
    }
    if let Some(v) = find_field(f, "timeframe") {
        let raw = expect_label(v, "slot.timeframe")?;
        s.timeframe =
            parse_timeframe(&raw).ok_or_else(|| WorkspaceTemplateError::InvalidField {
                field: "slot.timeframe".to_owned(),
                reason: format!("unknown timeframe '{raw}'"),
            })?;
    }
    if let Some(v) = find_field(f, "scale_mode") {
        let raw = expect_label(v, "slot.scale_mode")?;
        s.scale_mode =
            parse_scale_mode(&raw).ok_or_else(|| WorkspaceTemplateError::InvalidField {
                field: "slot.scale_mode".to_owned(),
                reason: format!("unknown scale_mode '{raw}'"),
            })?;
    }
    if let Some(v) = find_field(f, "show_volume") {
        s.show_volume = expect_bool(v, "slot.show_volume")?;
    }
    if let Some(v) = find_field(f, "show_grid") {
        s.show_grid = expect_bool(v, "slot.show_grid")?;
    }
    if let Some(v) = find_field(f, "compare_benchmark") {
        s.compare_benchmark = expect_opt_string(v, "slot.compare_benchmark")?;
    }
    if let Some(v) = find_field(f, "show_compare_overlay") {
        s.show_compare_overlay = expect_bool(v, "slot.show_compare_overlay")?;
    }
    if let Some(v) = find_field(f, "indicators") {
        s.indicators = decode_indicators(v)?;
    }
    if let Some(v) = find_field(f, "pane_stretches") {
        s.pane_stretches = decode_stretches(v)?;
    }
    Ok(s)
}

fn decode_indicators(val: &DocValue) -> Result<IndicatorConfig, WorkspaceTemplateError> {
    let f = expect_struct(val, "indicators")?;
    validate_struct_fields(
        f,
        "indicators",
        &[
            "show_sma",
            "sma_period",
            "show_ema",
            "ema_period",
            "show_bbands",
            "bb_period",
            "bb_std_mult",
            "show_vwap",
            "vwap_anchor_mode",
            "vwap_band_1",
            "vwap_band_2",
            "show_vrvp",
            "vrvp_rows",
            "vrvp_value_area_pct",
            "show_market_profile",
            "mp_session_period",
            "mp_num_bins",
            "mp_ib_bars",
            "mp_extend_naked_poc",
            "show_ichimoku",
            "ichimoku_tenkan",
            "ichimoku_kijun",
            "ichimoku_senkou_b",
            "ichimoku_displacement",
            "show_ichimoku_chikou",
            "show_rsi",
            "rsi_period",
            "show_macd",
            "macd_fast",
            "macd_slow",
            "macd_signal",
            "show_atr",
            "atr_period",
            "show_rs",
            "rs_sma_period",
            "show_cvd",
            "cvd_reset_mode",
            "cvd_sma_period",
        ],
    )?;
    let mut c = IndicatorConfig::default();
    if let Some(v) = find_field(f, "show_sma") {
        c.show_sma = expect_bool(v, "indicators.show_sma")?;
    }
    if let Some(v) = find_field(f, "sma_period") {
        c.sma_period = expect_usize(v, "indicators.sma_period")?;
    }
    if let Some(v) = find_field(f, "show_ema") {
        c.show_ema = expect_bool(v, "indicators.show_ema")?;
    }
    if let Some(v) = find_field(f, "ema_period") {
        c.ema_period = expect_usize(v, "indicators.ema_period")?;
    }
    if let Some(v) = find_field(f, "show_bbands") {
        c.show_bbands = expect_bool(v, "indicators.show_bbands")?;
    }
    if let Some(v) = find_field(f, "bb_period") {
        c.bb_period = expect_usize(v, "indicators.bb_period")?;
    }
    if let Some(v) = find_field(f, "bb_std_mult") {
        c.bb_std_mult = expect_f64(v, "indicators.bb_std_mult")?;
    }
    if let Some(v) = find_field(f, "show_vwap") {
        c.show_vwap = expect_bool(v, "indicators.show_vwap")?;
    }
    if let Some(v) = find_field(f, "vwap_anchor_mode") {
        let raw = expect_label(v, "indicators.vwap_anchor_mode")?;
        c.vwap_anchor_mode =
            parse_vwap_anchor_mode(&raw).ok_or_else(|| WorkspaceTemplateError::InvalidField {
                field: "indicators.vwap_anchor_mode".to_owned(),
                reason: format!("unknown VwapAnchorMode '{raw}'"),
            })?;
    }
    if let Some(v) = find_field(f, "vwap_band_1") {
        c.vwap_band_1 = expect_f64(v, "indicators.vwap_band_1")?;
    }
    if let Some(v) = find_field(f, "vwap_band_2") {
        c.vwap_band_2 = expect_f64(v, "indicators.vwap_band_2")?;
    }
    if let Some(v) = find_field(f, "show_vrvp") {
        c.show_vrvp = expect_bool(v, "indicators.show_vrvp")?;
    }
    if let Some(v) = find_field(f, "vrvp_rows") {
        c.vrvp_rows = expect_usize(v, "indicators.vrvp_rows")?;
    }
    if let Some(v) = find_field(f, "vrvp_value_area_pct") {
        c.vrvp_value_area_pct = expect_f64(v, "indicators.vrvp_value_area_pct")?;
    }
    if let Some(v) = find_field(f, "show_market_profile") {
        c.show_market_profile = expect_bool(v, "indicators.show_market_profile")?;
    }
    if let Some(v) = find_field(f, "mp_session_period") {
        let raw = expect_label(v, "indicators.mp_session_period")?;
        c.mp_session_period = parse_profile_session_period(&raw).ok_or_else(|| {
            WorkspaceTemplateError::InvalidField {
                field: "indicators.mp_session_period".to_owned(),
                reason: format!("unknown ProfileSessionPeriod '{raw}'"),
            }
        })?;
    }
    if let Some(v) = find_field(f, "mp_num_bins") {
        c.mp_num_bins = expect_usize(v, "indicators.mp_num_bins")?;
    }
    if let Some(v) = find_field(f, "mp_ib_bars") {
        c.mp_ib_bars = expect_usize(v, "indicators.mp_ib_bars")?;
    }
    if let Some(v) = find_field(f, "mp_extend_naked_poc") {
        c.mp_extend_naked_poc = expect_bool(v, "indicators.mp_extend_naked_poc")?;
    }
    if let Some(v) = find_field(f, "show_ichimoku") {
        c.show_ichimoku = expect_bool(v, "indicators.show_ichimoku")?;
    }
    if let Some(v) = find_field(f, "ichimoku_tenkan") {
        c.ichimoku_tenkan = expect_usize(v, "indicators.ichimoku_tenkan")?;
    }
    if let Some(v) = find_field(f, "ichimoku_kijun") {
        c.ichimoku_kijun = expect_usize(v, "indicators.ichimoku_kijun")?;
    }
    if let Some(v) = find_field(f, "ichimoku_senkou_b") {
        c.ichimoku_senkou_b = expect_usize(v, "indicators.ichimoku_senkou_b")?;
    }
    if let Some(v) = find_field(f, "ichimoku_displacement") {
        c.ichimoku_displacement = expect_usize(v, "indicators.ichimoku_displacement")?;
    }
    if let Some(v) = find_field(f, "show_ichimoku_chikou") {
        c.show_ichimoku_chikou = expect_bool(v, "indicators.show_ichimoku_chikou")?;
    }
    if let Some(v) = find_field(f, "show_rsi") {
        c.show_rsi = expect_bool(v, "indicators.show_rsi")?;
    }
    if let Some(v) = find_field(f, "rsi_period") {
        c.rsi_period = expect_usize(v, "indicators.rsi_period")?;
    }
    if let Some(v) = find_field(f, "show_macd") {
        c.show_macd = expect_bool(v, "indicators.show_macd")?;
    }
    if let Some(v) = find_field(f, "macd_fast") {
        c.macd_fast = expect_usize(v, "indicators.macd_fast")?;
    }
    if let Some(v) = find_field(f, "macd_slow") {
        c.macd_slow = expect_usize(v, "indicators.macd_slow")?;
    }
    if let Some(v) = find_field(f, "macd_signal") {
        c.macd_signal = expect_usize(v, "indicators.macd_signal")?;
    }
    if let Some(v) = find_field(f, "show_atr") {
        c.show_atr = expect_bool(v, "indicators.show_atr")?;
    }
    if let Some(v) = find_field(f, "atr_period") {
        c.atr_period = expect_usize(v, "indicators.atr_period")?;
    }
    if let Some(v) = find_field(f, "show_rs") {
        c.show_rs = expect_bool(v, "indicators.show_rs")?;
    }
    if let Some(v) = find_field(f, "rs_sma_period") {
        c.rs_sma_period = expect_usize(v, "indicators.rs_sma_period")?;
    }
    if let Some(v) = find_field(f, "show_cvd") {
        c.show_cvd = expect_bool(v, "indicators.show_cvd")?;
    }
    if let Some(v) = find_field(f, "cvd_reset_mode") {
        let raw = expect_label(v, "indicators.cvd_reset_mode")?;
        c.cvd_reset_mode =
            parse_cvd_reset_mode(&raw).ok_or_else(|| WorkspaceTemplateError::InvalidField {
                field: "indicators.cvd_reset_mode".to_owned(),
                reason: format!("unknown CvdResetMode '{raw}'"),
            })?;
    }
    if let Some(v) = find_field(f, "cvd_sma_period") {
        c.cvd_sma_period = expect_usize(v, "indicators.cvd_sma_period")?;
    }
    Ok(c)
}

fn decode_stretches(val: &DocValue) -> Result<WorkspacePaneStretches, WorkspaceTemplateError> {
    let f = expect_struct(val, "pane_stretches")?;
    validate_struct_fields(
        f,
        "pane_stretches",
        &["main", "rsi", "macd", "atr", "rs", "cvd", "equity"],
    )?;
    let mut st = WorkspacePaneStretches::default();
    if let Some(v) = find_field(f, "main") {
        st.main = expect_f32(v, "pane_stretches.main")?;
    }
    if let Some(v) = find_field(f, "rsi") {
        st.rsi = expect_f32(v, "pane_stretches.rsi")?;
    }
    if let Some(v) = find_field(f, "macd") {
        st.macd = expect_f32(v, "pane_stretches.macd")?;
    }
    if let Some(v) = find_field(f, "atr") {
        st.atr = expect_f32(v, "pane_stretches.atr")?;
    }
    if let Some(v) = find_field(f, "rs") {
        st.rs = expect_f32(v, "pane_stretches.rs")?;
    }
    if let Some(v) = find_field(f, "cvd") {
        st.cvd = expect_f32(v, "pane_stretches.cvd")?;
    }
    if let Some(v) = find_field(f, "equity") {
        st.equity = expect_f32(v, "pane_stretches.equity")?;
    }
    st.sanitize();
    Ok(st)
}

fn parse_data_source(raw: &str) -> Option<DataSourcePreset> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "syntheticvnindex" | "vnindex synth" => Some(DataSourcePreset::SyntheticVnindex),
        "intradayvn30f1m" | "vn30f1m 1m" => Some(DataSourcePreset::IntradayVn30F1m),
        "cafefindexcsv" | "cafef csv" => Some(DataSourcePreset::CafeFIndexCsv),
        "vn30basket" | "vn30 basket" => Some(DataSourcePreset::Vn30Basket),
        "spydailycsv" | "spy csv" => Some(DataSourcePreset::SpyDailyCsv),
        "btcusdt15mjson" | "btc json" => Some(DataSourcePreset::BtcUsdt15mJson),
        "customfile" | "file" => Some(DataSourcePreset::CustomFile),
        _ => None,
    }
}

fn parse_timeframe(raw: &str) -> Option<Timeframe> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "m1" | "1m" => Some(Timeframe::M1),
        "m5" | "5m" => Some(Timeframe::M5),
        "m15" | "15m" => Some(Timeframe::M15),
        "h1" | "1h" => Some(Timeframe::H1),
        "h4" | "4h" => Some(Timeframe::H4),
        "d1" | "1d" => Some(Timeframe::D1),
        "w1" | "1w" => Some(Timeframe::W1),
        _ => None,
    }
}

fn parse_scale_mode(raw: &str) -> Option<PriceScaleMode> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "normal" | "linear" => Some(PriceScaleMode::Normal),
        "logarithmic" | "log" => Some(PriceScaleMode::Logarithmic),
        "percentage" | "%" => Some(PriceScaleMode::Percentage),
        "indexedto100" | "idx100" => Some(PriceScaleMode::IndexedTo100),
        _ => None,
    }
}

fn parse_cvd_reset_mode(raw: &str) -> Option<CvdResetMode> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "session" => Some(CvdResetMode::Session),
        "cumulative" | "cumul" => Some(CvdResetMode::Cumulative),
        _ => None,
    }
}

fn parse_profile_session_period(raw: &str) -> Option<ProfileSessionPeriod> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "auto" => Some(ProfileSessionPeriod::Auto),
        "daily" | "day" | "1d" => Some(ProfileSessionPeriod::Daily),
        "weekly" | "week" | "1w" => Some(ProfileSessionPeriod::Weekly),
        _ => None,
    }
}

fn parse_vwap_anchor_mode(raw: &str) -> Option<VwapAnchorMode> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "autosession" | "session" | "auto" => Some(VwapAnchorMode::AutoSession),
        "daily" | "day" | "1d" => Some(VwapAnchorMode::Daily),
        "weekly" | "week" | "1w" => Some(VwapAnchorMode::Weekly),
        "monthly" | "month" | "1mo" => Some(VwapAnchorMode::Monthly),
        "yearly" | "year" | "1y" => Some(VwapAnchorMode::Yearly),
        _ => None,
    }
}

fn parse_template_scanner_tab(raw: &str) -> Option<TemplateScannerTab> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "watchlist" | "scanner" => Some(TemplateScannerTab::Watchlist),
        "signals" => Some(TemplateScannerTab::Signals),
        "breadth" => Some(TemplateScannerTab::Breadth),
        _ => None,
    }
}

fn parse_template_right_dock_tab(raw: &str) -> Option<TemplateRightDockTab> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "trading" | "dom" => Some(TemplateRightDockTab::Trading),
        "alerts" => Some(TemplateRightDockTab::Alerts),
        "strategylab" | "strategy" => Some(TemplateRightDockTab::StrategyLab),
        "journal" | "risk" => Some(TemplateRightDockTab::Journal),
        _ => None,
    }
}

fn parse_template_trading_view_mode(raw: &str) -> Option<TemplateTradingViewMode> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "orderticket" | "ticket" => Some(TemplateTradingViewMode::OrderTicket),
        "domladder" | "dom" | "ladder" => Some(TemplateTradingViewMode::DomLadder),
        _ => None,
    }
}

fn decode_dock_layout(val: &DocValue) -> Result<WorkspaceDockPreset, WorkspaceTemplateError> {
    let f = expect_struct(val, "dock_layout")?;
    validate_struct_fields(
        f,
        "dock_layout",
        &[
            "show_scanner_dock",
            "scanner_tab",
            "show_trading_panel",
            "show_alerts_panel",
            "show_strategy_panel",
            "show_journal_panel",
            "right_dock_tab",
            "trading_view_mode",
            "show_replay_bar",
        ],
    )?;
    let mut dl = WorkspaceDockPreset::default();
    if let Some(v) = find_field(f, "show_scanner_dock") {
        dl.show_scanner_dock = expect_bool(v, "dock_layout.show_scanner_dock")?;
    }
    if let Some(v) = find_field(f, "scanner_tab") {
        let raw = expect_label(v, "dock_layout.scanner_tab")?;
        dl.scanner_tab = parse_template_scanner_tab(&raw).ok_or_else(|| {
            WorkspaceTemplateError::InvalidField {
                field: "dock_layout.scanner_tab".to_owned(),
                reason: format!("unknown TemplateScannerTab '{raw}'"),
            }
        })?;
    }
    if let Some(v) = find_field(f, "show_trading_panel") {
        dl.show_trading_panel = expect_bool(v, "dock_layout.show_trading_panel")?;
    }
    if let Some(v) = find_field(f, "show_alerts_panel") {
        dl.show_alerts_panel = expect_bool(v, "dock_layout.show_alerts_panel")?;
    }
    if let Some(v) = find_field(f, "show_strategy_panel") {
        dl.show_strategy_panel = expect_bool(v, "dock_layout.show_strategy_panel")?;
    }
    if let Some(v) = find_field(f, "show_journal_panel") {
        dl.show_journal_panel = expect_bool(v, "dock_layout.show_journal_panel")?;
    }
    if let Some(v) = find_field(f, "right_dock_tab") {
        let raw = expect_label(v, "dock_layout.right_dock_tab")?;
        dl.right_dock_tab = parse_template_right_dock_tab(&raw).ok_or_else(|| {
            WorkspaceTemplateError::InvalidField {
                field: "dock_layout.right_dock_tab".to_owned(),
                reason: format!("unknown TemplateRightDockTab '{raw}'"),
            }
        })?;
    }
    if let Some(v) = find_field(f, "trading_view_mode") {
        let raw = expect_label(v, "dock_layout.trading_view_mode")?;
        dl.trading_view_mode = parse_template_trading_view_mode(&raw).ok_or_else(|| {
            WorkspaceTemplateError::InvalidField {
                field: "dock_layout.trading_view_mode".to_owned(),
                reason: format!("unknown TemplateTradingViewMode '{raw}'"),
            }
        })?;
    }
    if let Some(v) = find_field(f, "show_replay_bar") {
        dl.show_replay_bar = expect_bool(v, "dock_layout.show_replay_bar")?;
    }
    Ok(dl)
}

fn decode_hotkey_overrides(val: &DocValue) -> Result<Vec<HotkeyOverride>, WorkspaceTemplateError> {
    let items = match val {
        DocValue::Array(items) => items,
        _ => {
            return Err(WorkspaceTemplateError::InvalidField {
                field: "hotkey_overrides".to_owned(),
                reason: "expected array of hotkey overrides".to_owned(),
            });
        }
    };
    if items.len() > 64 {
        return Err(WorkspaceTemplateError::InvalidField {
            field: "hotkey_overrides".to_owned(),
            reason: format!("too many hotkey overrides ({}, max 64)", items.len()),
        });
    }
    let mut out = Vec::with_capacity(items.len());
    for (idx, item) in items.iter().enumerate() {
        let ctx = format!("hotkey_overrides[{idx}]");
        let f = expect_struct(item, &ctx)?;
        validate_struct_fields(f, &ctx, &["action", "chord"])?;
        let action_val =
            find_field(f, "action").ok_or_else(|| WorkspaceTemplateError::InvalidField {
                field: format!("{ctx}.action"),
                reason: "missing required field 'action'".to_owned(),
            })?;
        let raw_action = expect_label(action_val, &format!("{ctx}.action"))?;
        let action = HotkeyAction::from_id(&raw_action).ok_or_else(|| {
            WorkspaceTemplateError::InvalidField {
                field: format!("{ctx}.action"),
                reason: format!("unknown HotkeyAction '{raw_action}'"),
            }
        })?;
        let chord = match find_field(f, "chord") {
            None | Some(DocValue::None) => None,
            Some(DocValue::Some(inner)) => {
                Some(decode_hotkey_chord(inner, &format!("{ctx}.chord"))?)
            }
            Some(v) => Some(decode_hotkey_chord(v, &format!("{ctx}.chord"))?),
        };
        out.push(HotkeyOverride { action, chord });
    }
    Ok(out)
}

fn decode_hotkey_chord(val: &DocValue, ctx: &str) -> Result<HotkeyChord, WorkspaceTemplateError> {
    let f = expect_struct(val, ctx)?;
    validate_struct_fields(f, ctx, &["key", "shift", "alt"])?;
    let key_val = find_field(f, "key").ok_or_else(|| WorkspaceTemplateError::InvalidField {
        field: format!("{ctx}.key"),
        reason: "missing required field 'key'".to_owned(),
    })?;
    let raw_key = expect_label(key_val, &format!("{ctx}.key"))?;
    let key = HotkeyKey::from_id(&raw_key).ok_or_else(|| WorkspaceTemplateError::InvalidField {
        field: format!("{ctx}.key"),
        reason: format!("unknown or reserved HotkeyKey '{raw_key}'"),
    })?;
    let shift = match find_field(f, "shift") {
        Some(v) => expect_bool(v, &format!("{ctx}.shift"))?,
        None => false,
    };
    let alt = match find_field(f, "alt") {
        Some(v) => expect_bool(v, &format!("{ctx}.alt"))?,
        None => false,
    };
    Ok(HotkeyChord::new(key, HotkeyModifiers { shift, alt }))
}

fn decode_hotkey_profile_id(
    val: &DocValue,
    field: &str,
) -> Result<HotkeyProfileId, WorkspaceTemplateError> {
    let raw = expect_label(val, field)?;
    HotkeyProfileId::from_id(&raw).ok_or_else(|| WorkspaceTemplateError::InvalidField {
        field: field.to_owned(),
        reason: format!("unknown HotkeyProfileId '{raw}'"),
    })
}
