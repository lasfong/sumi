//! Pure, Zero-Dependency Point-in-Time (`PIT`) vs `RetrospectiveFixed` Universe Resolver
//! (`Milestone R18 / Wave 17` — `B4`).
//!
//! Aligned with `ARCHIVE/backend/app/domain/universe/{models,resolver}.py` and compatible with
//! `pit_test_universe.json` and `sumi420_v1_candidate.json` (`0` external crates added to `Cargo.toml`).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::viewport::workspace_template::{
    DocValue, expect_f64, expect_label, expect_opt_string, expect_string, expect_struct,
    find_field, parse_json_doc, parse_ron_doc, splitmix64_hash_bytes,
    strip_leading_ws_and_comments,
};

/// Compact, `wasm32`-deterministic UTC civil calendar date (`YYYY-MM-DD`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SessionDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl SessionDate {
    pub const MIN: Self = Self {
        year: -9999,
        month: 1,
        day: 1,
    };
    pub const MAX: Self = Self {
        year: 9999,
        month: 12,
        day: 31,
    };

    #[must_use]
    pub const fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        if year < -9999 || year > 9999 || month < 1 || month > 12 || day < 1 {
            return None;
        }
        let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let max_day = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if is_leap {
                    29
                } else {
                    28
                }
            }
            _ => return None,
        };
        if day > max_day {
            return None;
        }
        Some(Self { year, month, day })
    }

    /// Parses `"YYYY-MM-DD"` (or ISO timestamp prefix `"YYYY-MM-DDTHH:MM:SS..."`).
    #[must_use]
    pub fn parse_iso(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        let date_part = trimmed.get(..10)?;
        let bytes = date_part.as_bytes();
        if bytes.get(4) != Some(&b'-') || bytes.get(7) != Some(&b'-') {
            return None;
        }
        let year = date_part.get(0..4)?.parse::<i32>().ok()?;
        let month = date_part.get(5..7)?.parse::<u8>().ok()?;
        let day = date_part.get(8..10)?.parse::<u8>().ok()?;
        Self::new(year, month, day)
    }

    /// Converts a Unix UTC epoch timestamp (seconds) into its civil UTC [`SessionDate`]
    /// using Howard Hinnant's exact `civil_from_days` algorithm (`0` OS clock/tz calls).
    #[must_use]
    pub const fn from_utc_timestamp(ts: i64) -> Self {
        let z = ts.div_euclid(86_400) + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = (z - era * 146_097) as u64; // [0, 146096]
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
        let y = (yoe as i64) + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
        let mp = (5 * doy + 2) / 153; // [0, 11]
        let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
        let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
        let raw_y = if m <= 2 { y + 1 } else { y };
        let year = (if raw_y < -9999 {
            -9999
        } else if raw_y > 9999 {
            9999
        } else {
            raw_y
        }) as i32;
        Self {
            year,
            month: m as u8,
            day: d as u8,
        }
    }

    /// Formats as canonical `"YYYY-MM-DD"`.
    #[must_use]
    pub fn to_iso_string(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl fmt::Display for SessionDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// Evaluation semantics for constituent eligibility across history (`AT11`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum UniverseMode {
    /// Strictly enforces universe `effective_from` and each member's `[effective_from, effective_to]`
    /// active interval (`is_point_in_time = true`, zero survivorship bias).
    #[default]
    PointInTime,
    /// Evaluates a fixed snapshot of `ACTIVE` members across all dates (`is_point_in_time = false`,
    /// emits explicit `survivor_bias_caveat`).
    RetrospectiveFixed,
}

impl UniverseMode {
    #[inline]
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::PointInTime => "POINT_IN_TIME",
            Self::RetrospectiveFixed => "RETROSPECTIVE_FIXED",
        }
    }

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::PointInTime => "PIT (No Survivorship Bias)",
            Self::RetrospectiveFixed => "Retrospective Fixed",
        }
    }

    #[must_use]
    pub fn parse_str(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_uppercase().as_str() {
            "POINT_IN_TIME" | "POINTINTIME" | "PIT" => Some(Self::PointInTime),
            "RETROSPECTIVE_FIXED" | "RETROSPECTIVEFIXED" | "FIXED" => {
                Some(Self::RetrospectiveFixed)
            }
            _ => None,
        }
    }
}

/// Governance approval status of a [`UniverseDefinition`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum UniverseStatus {
    #[default]
    Approved,
    Candidate,
    Experimental,
    Deprecated,
}

impl UniverseStatus {
    #[inline]
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Approved => "APPROVED",
            Self::Candidate => "CANDIDATE",
            Self::Experimental => "EXPERIMENTAL",
            Self::Deprecated => "DEPRECATED",
        }
    }

    #[must_use]
    pub fn parse_str(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_uppercase().as_str() {
            "APPROVED" => Some(Self::Approved),
            "CANDIDATE" => Some(Self::Candidate),
            "EXPERIMENTAL" => Some(Self::Experimental),
            "DEPRECATED" => Some(Self::Deprecated),
            _ => None,
        }
    }
}

/// Constituent membership lifecycle state inside a [`UniverseDefinition`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum MemberStatus {
    #[default]
    Active,
    Suspended,
    Reserve,
    Delisted,
    Inactive,
}

impl MemberStatus {
    #[inline]
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Active => "ACTIVE",
            Self::Suspended => "SUSPENDED",
            Self::Reserve => "RESERVE",
            Self::Delisted => "DELISTED",
            Self::Inactive => "INACTIVE",
        }
    }

    #[must_use]
    pub fn parse_str(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_uppercase().as_str() {
            "ACTIVE" => Some(Self::Active),
            "SUSPENDED" => Some(Self::Suspended),
            "RESERVE" => Some(Self::Reserve),
            "DELISTED" => Some(Self::Delisted),
            "INACTIVE" => Some(Self::Inactive),
            _ => None,
        }
    }
}

/// Coverage gate tier for available market data against resolved target constituents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CoverageStatus {
    /// `coverage_ratio >= threshold` (default `80%`).
    Satisfied,
    /// `0.50 <= coverage_ratio < threshold`.
    Degraded,
    /// `coverage_ratio < 0.50` or `target_count == 0`.
    Failed,
}

impl CoverageStatus {
    #[inline]
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Satisfied => "SATISFIED",
            Self::Degraded => "DEGRADED",
            Self::Failed => "FAILED",
        }
    }
}

/// Single constituent membership episode within a [`UniverseDefinition`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniverseMember {
    pub symbol: String,
    pub exchange: String,
    pub company_name: Option<String>,
    pub sector: Option<String>,
    pub effective_from: Option<SessionDate>,
    pub effective_to: Option<SessionDate>,
    pub status: MemberStatus,
    pub inclusion_reason: String,
    pub weight: f64,
}

impl UniverseMember {
    /// Returns `true` iff this member episode is `Active` and covers `as_of` inclusively.
    #[inline]
    #[must_use]
    pub fn is_active_on(&self, as_of: SessionDate) -> bool {
        if self.status != MemberStatus::Active {
            return false;
        }
        if let Some(from) = self.effective_from
            && as_of < from
        {
            return false;
        }
        if let Some(to) = self.effective_to
            && as_of > to
        {
            return false;
        }
        true
    }
}

/// Versioned constituent universe definition supporting both `PointInTime` and `RetrospectiveFixed` modes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniverseDefinition {
    pub universe_id: String,
    pub version: String,
    pub name: String,
    pub status: UniverseStatus,
    pub default_mode: UniverseMode,
    pub effective_from: Option<SessionDate>,
    pub effective_to: Option<SessionDate>,
    pub selection_purpose: String,
    pub source_evidence: String,
    pub members: Vec<UniverseMember>,
}

/// Validation or parsing error emitted by [`UniverseResolver`] / [`UniverseDefinition`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UniverseError {
    EmptyUniverseId,
    EmptyVersion,
    EmptyMemberSymbol,
    InvertedUniverseInterval {
        effective_from: SessionDate,
        effective_to: SessionDate,
    },
    InvertedMemberInterval {
        symbol: String,
        effective_from: SessionDate,
        effective_to: SessionDate,
    },
    OverlappingMemberEpisodes {
        symbol: String,
        first: (SessionDate, SessionDate),
        second: (SessionDate, SessionDate),
    },
    InvalidMemberWeight {
        symbol: String,
    },
    InvalidCoverageThreshold,
    Parse(String),
}

impl fmt::Display for UniverseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyUniverseId => write!(f, "universe_id cannot be empty"),
            Self::EmptyVersion => write!(f, "universe version cannot be empty"),
            Self::EmptyMemberSymbol => write!(f, "member symbol cannot be empty"),
            Self::InvertedUniverseInterval {
                effective_from,
                effective_to,
            } => write!(
                f,
                "universe effective_to ({effective_to}) < effective_from ({effective_from})"
            ),
            Self::InvertedMemberInterval {
                symbol,
                effective_from,
                effective_to,
            } => write!(
                f,
                "member '{symbol}' has effective_to ({effective_to}) < effective_from ({effective_from})"
            ),
            Self::OverlappingMemberEpisodes {
                symbol,
                first,
                second,
            } => write!(
                f,
                "overlapping episodes for symbol '{symbol}': [{}..={}] overlaps [{}..={}]",
                first.0, first.1, second.0, second.1
            ),
            Self::InvalidMemberWeight { symbol } => {
                write!(
                    f,
                    "member '{symbol}' weight must be finite and in (0.0, 1e6]"
                )
            }
            Self::InvalidCoverageThreshold => {
                write!(f, "coverage threshold must lie in [0.0, 1.0]")
            }
            Self::Parse(msg) => write!(f, "universe parse error: {msg}"),
        }
    }
}

impl std::error::Error for UniverseError {}

impl UniverseDefinition {
    /// Deterministic `SplitMix64` content hash (`16`-char lowercase hex) over canonical universe members & dates.
    #[must_use]
    pub fn content_hash_hex(&self) -> String {
        let mut buf = String::with_capacity(512 + self.members.len() * 64);
        buf.push_str(self.universe_id.trim());
        buf.push('|');
        buf.push_str(self.version.trim());
        buf.push('|');
        buf.push_str(self.status.code());
        buf.push('|');
        buf.push_str(self.default_mode.code());
        buf.push('|');
        if let Some(f) = self.effective_from {
            buf.push_str(&f.to_iso_string());
        }
        buf.push('|');
        if let Some(t) = self.effective_to {
            buf.push_str(&t.to_iso_string());
        }
        let mut sorted_members: Vec<&UniverseMember> = self.members.iter().collect();
        sorted_members.sort_by(|a, b| {
            a.symbol
                .cmp(&b.symbol)
                .then_with(|| a.effective_from.cmp(&b.effective_from))
                .then_with(|| a.effective_to.cmp(&b.effective_to))
        });
        for m in sorted_members {
            buf.push('\n');
            buf.push_str(m.symbol.trim());
            buf.push(':');
            buf.push_str(m.status.code());
            buf.push(':');
            if let Some(f) = m.effective_from {
                buf.push_str(&f.to_iso_string());
            }
            buf.push_str("..");
            if let Some(t) = m.effective_to {
                buf.push_str(&t.to_iso_string());
            }
            buf.push(':');
            buf.push_str(&format!(
                "{:.6}",
                if m.weight.is_finite() { m.weight } else { 1.0 }
            ));
        }
        format!("{:016x}", splitmix64_hash_bytes(buf.as_bytes()))
    }

    /// Bundled institutional `VN30_PIT_V1` point-in-time constituent definition covering Sumi's
    /// HOSE blue-chip basket with realistic rotation intervals (so `PIT` vs `RetrospectiveFixed`
    /// survivorship-bias behavior is directly inspectable offline and in `wasm32`).
    #[must_use]
    pub fn bundled_vn30_pit() -> Self {
        let d = |y, m, day| SessionDate::new(y, m, day).expect("valid static SessionDate");
        Self {
            universe_id: "VN30-PIT".to_owned(),
            version: "vn30_pit_v1".to_owned(),
            name: "HOSE VN30 Point-in-Time Institutional Basket".to_owned(),
            status: UniverseStatus::Approved,
            default_mode: UniverseMode::PointInTime,
            effective_from: Some(d(2020, 1, 1)),
            effective_to: None,
            selection_purpose:
                "Point-in-time HOSE VN30 constituent basket eliminating survivorship & late-addition bias"
                    .to_owned(),
            source_evidence: "Sumi bundled VN30 constituent schedule".to_owned(),
            members: vec![
                UniverseMember {
                    symbol: "FPT".to_owned(),
                    exchange: "HOSE".to_owned(),
                    company_name: Some("FPT Corp".to_owned()),
                    sector: Some("Technology".to_owned()),
                    effective_from: Some(d(2020, 1, 1)),
                    effective_to: None,
                    status: MemberStatus::Active,
                    inclusion_reason: "Core VN30 technology leader".to_owned(),
                    weight: 1.0,
                },
                UniverseMember {
                    symbol: "VCB".to_owned(),
                    exchange: "HOSE".to_owned(),
                    company_name: Some("Vietcombank".to_owned()),
                    sector: Some("Financials".to_owned()),
                    effective_from: Some(d(2020, 1, 1)),
                    effective_to: None,
                    status: MemberStatus::Active,
                    inclusion_reason: "Core VN30 banking anchor".to_owned(),
                    weight: 1.0,
                },
                UniverseMember {
                    symbol: "HPG".to_owned(),
                    exchange: "HOSE".to_owned(),
                    company_name: Some("Hoa Phat Group".to_owned()),
                    sector: Some("Materials".to_owned()),
                    effective_from: Some(d(2020, 1, 1)),
                    effective_to: None,
                    status: MemberStatus::Active,
                    inclusion_reason: "Core VN30 industrials/steel".to_owned(),
                    weight: 1.0,
                },
                UniverseMember {
                    symbol: "ACB".to_owned(),
                    exchange: "HOSE".to_owned(),
                    company_name: Some("Asia Commercial Bank".to_owned()),
                    sector: Some("Financials".to_owned()),
                    effective_from: Some(d(2020, 1, 1)),
                    effective_to: None,
                    status: MemberStatus::Active,
                    inclusion_reason: "Core VN30 commercial bank".to_owned(),
                    weight: 1.0,
                },
                UniverseMember {
                    symbol: "TCB".to_owned(),
                    exchange: "HOSE".to_owned(),
                    company_name: Some("Techcombank".to_owned()),
                    sector: Some("Financials".to_owned()),
                    effective_from: Some(d(2020, 1, 1)),
                    effective_to: None,
                    status: MemberStatus::Active,
                    inclusion_reason: "Core VN30 private bank".to_owned(),
                    weight: 1.0,
                },
                UniverseMember {
                    symbol: "MWG".to_owned(),
                    exchange: "HOSE".to_owned(),
                    company_name: Some("Mobile World".to_owned()),
                    sector: Some("Consumer Discretionary".to_owned()),
                    effective_from: Some(d(2020, 1, 1)),
                    effective_to: None,
                    status: MemberStatus::Active,
                    inclusion_reason: "Core VN30 retail constituent".to_owned(),
                    weight: 1.0,
                },
                UniverseMember {
                    symbol: "VIC".to_owned(),
                    exchange: "HOSE".to_owned(),
                    company_name: Some("Vingroup".to_owned()),
                    sector: Some("Real Estate".to_owned()),
                    effective_from: Some(d(2020, 1, 1)),
                    // Semi-annual rebalance removal window demonstrating PIT constituent exit:
                    effective_to: Some(d(2026, 4, 30)),
                    status: MemberStatus::Active,
                    inclusion_reason: "VN30 constituent removed after 2026-04-30 rebalance".to_owned(),
                    weight: 1.0,
                },
                UniverseMember {
                    symbol: "SSI".to_owned(),
                    exchange: "HOSE".to_owned(),
                    company_name: Some("SSI Securities".to_owned()),
                    sector: Some("Financials".to_owned()),
                    // Semi-annual index re-addition window demonstrating genuine PIT inclusion gating:
                    effective_from: Some(d(2026, 2, 2)),
                    effective_to: None,
                    status: MemberStatus::Active,
                    inclusion_reason: "VN30 Q1 rebalance addition on 2026-02-02".to_owned(),
                    weight: 1.0,
                },
            ],
        }
    }

    /// Parses a [`UniverseDefinition`] from RFC-8259 `JSON` or `RON` (`pit_test_universe.json`,
    /// `sumi420_v1_candidate.json`, or custom user universe files) and validates interval integrity.
    pub fn from_auto_str(input: &str) -> Result<Self, UniverseError> {
        let trimmed = strip_leading_ws_and_comments(input)
            .map_err(|e| UniverseError::Parse(e.to_string()))?;
        let doc = if trimmed.starts_with('{') {
            parse_json_doc(trimmed).map_err(|e| UniverseError::Parse(e.to_string()))?
        } else {
            parse_ron_doc(input).map_err(|e| UniverseError::Parse(e.to_string()))?
        };
        let def = Self::from_doc_value(&doc)?;
        UniverseResolver::validate_definition(&def)?;
        Ok(def)
    }

    fn from_doc_value(val: &DocValue) -> Result<Self, UniverseError> {
        let fields = expect_struct(val, "UniverseDefinition")
            .map_err(|e| UniverseError::Parse(e.to_string()))?;

        let universe_id = find_field(fields, "universe_id")
            .ok_or(UniverseError::EmptyUniverseId)
            .and_then(|v| {
                expect_string(v, "universe_id").map_err(|e| UniverseError::Parse(e.to_string()))
            })?
            .trim()
            .to_owned();
        if universe_id.is_empty() {
            return Err(UniverseError::EmptyUniverseId);
        }

        let version = find_field(fields, "version")
            .ok_or(UniverseError::EmptyVersion)
            .and_then(|v| {
                expect_string(v, "version").map_err(|e| UniverseError::Parse(e.to_string()))
            })?
            .trim()
            .to_owned();
        if version.is_empty() {
            return Err(UniverseError::EmptyVersion);
        }

        let name = match find_field(fields, "name") {
            Some(v) => expect_string(v, "name").map_err(|e| UniverseError::Parse(e.to_string()))?,
            None => universe_id.clone(),
        };
        let status = match find_field(fields, "status") {
            Some(v) => {
                let raw =
                    expect_label(v, "status").map_err(|e| UniverseError::Parse(e.to_string()))?;
                UniverseStatus::parse_str(&raw).ok_or_else(|| {
                    UniverseError::Parse(format!("unknown UniverseStatus '{raw}'"))
                })?
            }
            None => UniverseStatus::Approved,
        };
        let default_mode = match find_field(fields, "default_mode") {
            Some(v) => {
                let raw = expect_label(v, "default_mode")
                    .map_err(|e| UniverseError::Parse(e.to_string()))?;
                UniverseMode::parse_str(&raw)
                    .ok_or_else(|| UniverseError::Parse(format!("unknown UniverseMode '{raw}'")))?
            }
            None => UniverseMode::PointInTime,
        };
        let effective_from = parse_opt_date_field(fields, "effective_from")?;
        let effective_to = parse_opt_date_field(fields, "effective_to")?;
        let selection_purpose = match find_field(fields, "selection_purpose") {
            Some(v) => expect_opt_string(v, "selection_purpose")
                .map_err(|e| UniverseError::Parse(e.to_string()))?
                .unwrap_or_default(),
            None => String::new(),
        };
        let source_evidence = match find_field(fields, "source_evidence") {
            Some(v) => expect_opt_string(v, "source_evidence")
                .map_err(|e| UniverseError::Parse(e.to_string()))?
                .unwrap_or_default(),
            None => String::new(),
        };

        let members_val = find_field(fields, "members")
            .ok_or_else(|| UniverseError::Parse("missing required field 'members'".to_owned()))?;
        let arr = match members_val {
            DocValue::Array(items) => items,
            _ => {
                return Err(UniverseError::Parse(
                    "expected array for 'members'".to_owned(),
                ));
            }
        };

        let mut members = Vec::with_capacity(arr.len());
        for (idx, item) in arr.iter().enumerate() {
            let ctx = format!("members[{idx}]");
            let mf = expect_struct(item, &ctx).map_err(|e| UniverseError::Parse(e.to_string()))?;
            let symbol = find_field(mf, "symbol")
                .ok_or(UniverseError::EmptyMemberSymbol)
                .and_then(|v| {
                    expect_string(v, "symbol").map_err(|e| UniverseError::Parse(e.to_string()))
                })?
                .trim()
                .to_ascii_uppercase();
            if symbol.is_empty() {
                return Err(UniverseError::EmptyMemberSymbol);
            }
            let exchange = match find_field(mf, "exchange") {
                Some(v) => expect_opt_string(v, "exchange")
                    .map_err(|e| UniverseError::Parse(e.to_string()))?
                    .unwrap_or_else(|| "HOSE".to_owned()),
                None => "HOSE".to_owned(),
            };
            let company_name = match find_field(mf, "company_name") {
                Some(v) => expect_opt_string(v, "company_name")
                    .map_err(|e| UniverseError::Parse(e.to_string()))?,
                None => None,
            };
            let sector = match find_field(mf, "sector") {
                Some(v) => expect_opt_string(v, "sector")
                    .map_err(|e| UniverseError::Parse(e.to_string()))?,
                None => None,
            };
            let m_from = parse_opt_date_field(mf, "effective_from")?;
            let m_to = parse_opt_date_field(mf, "effective_to")?;
            let m_status = match find_field(mf, "status") {
                Some(v) => {
                    let raw = expect_label(v, "member.status")
                        .map_err(|e| UniverseError::Parse(e.to_string()))?;
                    MemberStatus::parse_str(&raw).ok_or_else(|| {
                        UniverseError::Parse(format!("unknown MemberStatus '{raw}'"))
                    })?
                }
                None => MemberStatus::Active,
            };
            let inclusion_reason = match find_field(mf, "inclusion_reason") {
                Some(v) => expect_opt_string(v, "inclusion_reason")
                    .map_err(|e| UniverseError::Parse(e.to_string()))?
                    .unwrap_or_default(),
                None => String::new(),
            };
            let weight = match find_field(mf, "weight") {
                Some(DocValue::None) | None => 1.0,
                Some(v) => {
                    let w = expect_f64(v, "member.weight")
                        .map_err(|e| UniverseError::Parse(e.to_string()))?;
                    if w <= 0.0 {
                        return Err(UniverseError::Parse(format!(
                            "member '{symbol}' weight must be > 0, got {w}"
                        )));
                    }
                    w
                }
            };
            members.push(UniverseMember {
                symbol,
                exchange,
                company_name,
                sector,
                effective_from: m_from,
                effective_to: m_to,
                status: m_status,
                inclusion_reason,
                weight,
            });
        }

        Ok(Self {
            universe_id,
            version,
            name,
            status,
            default_mode,
            effective_from,
            effective_to,
            selection_purpose,
            source_evidence,
            members,
        })
    }
}

fn parse_opt_date_field(
    fields: &[(String, DocValue)],
    key: &str,
) -> Result<Option<SessionDate>, UniverseError> {
    let Some(val) = find_field(fields, key) else {
        return Ok(None);
    };
    let Some(raw) = expect_opt_string(val, key).map_err(|e| UniverseError::Parse(e.to_string()))?
    else {
        return Ok(None);
    };
    if raw.trim().is_empty() {
        return Ok(None);
    }
    SessionDate::parse_iso(&raw)
        .map(Some)
        .ok_or_else(|| UniverseError::Parse(format!("invalid ISO date '{raw}' in '{key}'")))
}

/// Resolved constituent snapshot as of `as_of` under `mode`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UniverseResolution {
    pub universe_id: String,
    pub version: String,
    pub as_of: SessionDate,
    pub mode: UniverseMode,
    pub is_point_in_time: bool,
    pub active_symbols: Vec<String>,
    pub excluded_symbols: Vec<String>,
    pub active_count: usize,
    pub total_target_count: usize,
    pub content_hash: String,
    pub survivor_bias_caveat: Option<String>,
}

/// Coverage audit comparing available market symbols against a [`UniverseResolution`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniverseCoverageReport {
    pub universe_id: String,
    pub version: String,
    pub as_of: SessionDate,
    pub target_count: usize,
    pub available_count: usize,
    pub missing_count: usize,
    pub coverage_ratio: f64,
    pub missing_symbols: Vec<String>,
    pub status: CoverageStatus,
    pub threshold: f64,
}

/// Pure domain resolver for Point-in-Time (`PIT`) and `RetrospectiveFixed` universes.
pub struct UniverseResolver;

impl UniverseResolver {
    /// Validates non-empty IDs/symbols, non-inverted member intervals, and non-overlapping
    /// episodes per symbol (`ARCHIVE/backend/app/domain/universe/resolver.py:22-49`).
    pub fn validate_definition(universe: &UniverseDefinition) -> Result<(), UniverseError> {
        if universe.universe_id.trim().is_empty() {
            return Err(UniverseError::EmptyUniverseId);
        }
        if universe.version.trim().is_empty() {
            return Err(UniverseError::EmptyVersion);
        }
        if let (Some(u_from), Some(u_to)) = (universe.effective_from, universe.effective_to)
            && u_to < u_from
        {
            return Err(UniverseError::InvertedUniverseInterval {
                effective_from: u_from,
                effective_to: u_to,
            });
        }

        let mut episodes_by_symbol: BTreeMap<String, Vec<(SessionDate, SessionDate)>> =
            BTreeMap::new();
        for m in &universe.members {
            let sym = m.symbol.trim().to_ascii_uppercase();
            if sym.is_empty() {
                return Err(UniverseError::EmptyMemberSymbol);
            }
            if !m.weight.is_finite() || m.weight <= 0.0 || m.weight > 1.0e6 {
                return Err(UniverseError::InvalidMemberWeight { symbol: sym });
            }
            if let (Some(from), Some(to)) = (m.effective_from, m.effective_to)
                && to < from
            {
                return Err(UniverseError::InvertedMemberInterval {
                    symbol: sym,
                    effective_from: from,
                    effective_to: to,
                });
            }
            let start = m.effective_from.unwrap_or(SessionDate::MIN);
            let end = m.effective_to.unwrap_or(SessionDate::MAX);
            let list = episodes_by_symbol.entry(sym.clone()).or_default();
            for &(prev_s, prev_e) in list.iter() {
                if !(end < prev_s || start > prev_e) {
                    return Err(UniverseError::OverlappingMemberEpisodes {
                        symbol: sym,
                        first: (prev_s, prev_e),
                        second: (start, end),
                    });
                }
            }
            list.push((start, end));
        }
        Ok(())
    }

    /// Fast predicate checking whether `symbol` is an active eligible constituent on `as_of`
    /// under `mode` (`0` String/BTreeSet allocations — safe inside inner simulation loops!).
    #[must_use]
    pub fn is_symbol_active_on(
        universe: &UniverseDefinition,
        symbol: &str,
        as_of: SessionDate,
        mode: Option<UniverseMode>,
    ) -> bool {
        let eval_mode = mode.unwrap_or(universe.default_mode);
        let sym_trim = symbol.trim();
        match eval_mode {
            UniverseMode::PointInTime => {
                if let Some(u_from) = universe.effective_from
                    && as_of < u_from
                {
                    return false;
                }
                if let Some(u_to) = universe.effective_to
                    && as_of > u_to
                {
                    return false;
                }
                universe.members.iter().any(|m| {
                    m.symbol.trim().eq_ignore_ascii_case(sym_trim) && m.is_active_on(as_of)
                })
            }
            UniverseMode::RetrospectiveFixed => universe.members.iter().any(|m| {
                m.symbol.trim().eq_ignore_ascii_case(sym_trim) && m.status == MemberStatus::Active
            }),
        }
    }

    /// Resolves active & excluded constituent sets as of `as_of`.
    pub fn resolve(
        universe: &UniverseDefinition,
        as_of: SessionDate,
        mode: Option<UniverseMode>,
    ) -> Result<UniverseResolution, UniverseError> {
        Self::validate_definition(universe)?;
        let eval_mode = mode.unwrap_or(universe.default_mode);

        let (is_pit, caveat, active_symbols, excluded_symbols) = match eval_mode {
            UniverseMode::PointInTime => {
                let out_of_universe_window =
                    universe.effective_from.is_some_and(|u_from| as_of < u_from)
                        || universe.effective_to.is_some_and(|u_to| as_of > u_to);
                if out_of_universe_window {
                    let all_syms: BTreeSet<String> = universe
                        .members
                        .iter()
                        .map(|m| m.symbol.trim().to_ascii_uppercase())
                        .collect();
                    (true, None, Vec::new(), all_syms.into_iter().collect())
                } else {
                    let mut active_set = BTreeSet::new();
                    let mut excluded_set = BTreeSet::new();
                    for m in &universe.members {
                        let sym = m.symbol.trim().to_ascii_uppercase();
                        if m.is_active_on(as_of) {
                            active_set.insert(sym);
                        } else {
                            excluded_set.insert(sym);
                        }
                    }
                    for s in &active_set {
                        excluded_set.remove(s);
                    }
                    (
                        true,
                        None,
                        active_set.into_iter().collect(),
                        excluded_set.into_iter().collect(),
                    )
                }
            }
            UniverseMode::RetrospectiveFixed => {
                let eff_lbl = universe
                    .effective_from
                    .map_or_else(|| "curated".to_owned(), |d| d.to_iso_string());
                let caveat = Some(format!(
                    "RETROSPECTIVE_FIXED MODE: Constituents reflect fixed snapshot (effective {eff_lbl}). \
                     Evaluation as of {as_of} carries survivorship bias and must not be used as point-in-time truth."
                ));
                let mut active_set = BTreeSet::new();
                let mut excluded_set = BTreeSet::new();
                for m in &universe.members {
                    let sym = m.symbol.trim().to_ascii_uppercase();
                    if m.status == MemberStatus::Active {
                        active_set.insert(sym);
                    } else {
                        excluded_set.insert(sym);
                    }
                }
                for s in &active_set {
                    excluded_set.remove(s);
                }
                (
                    false,
                    caveat,
                    active_set.into_iter().collect(),
                    excluded_set.into_iter().collect(),
                )
            }
        };

        let active_count = active_symbols.len();
        Ok(UniverseResolution {
            universe_id: universe.universe_id.clone(),
            version: universe.version.clone(),
            as_of,
            mode: eval_mode,
            is_point_in_time: is_pit,
            active_symbols,
            excluded_symbols,
            active_count,
            total_target_count: active_count,
            content_hash: universe.content_hash_hex(),
            survivor_bias_caveat: caveat,
        })
    }

    /// Evaluates constituent data availability against `resolution` (`threshold` default `0.80`).
    pub fn check_coverage<S: AsRef<str>>(
        resolution: &UniverseResolution,
        available_symbols: &[S],
        threshold: f64,
    ) -> Result<UniverseCoverageReport, UniverseError> {
        if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) {
            return Err(UniverseError::InvalidCoverageThreshold);
        }
        let avail_set: BTreeSet<String> = available_symbols
            .iter()
            .map(|s| s.as_ref().trim().to_ascii_uppercase())
            .collect();
        let target_set: BTreeSet<String> = resolution.active_symbols.iter().cloned().collect();

        let mut available_count = 0_usize;
        let mut missing_symbols = Vec::new();
        for t in &target_set {
            if avail_set.contains(t) {
                available_count += 1;
            } else {
                missing_symbols.push(t.clone());
            }
        }

        let target_count = target_set.len();
        let missing_count = missing_symbols.len();
        let raw_ratio = if target_count > 0 {
            (available_count as f64) / (target_count as f64)
        } else {
            1.0
        };
        let status = if target_count == 0 {
            CoverageStatus::Failed
        } else if raw_ratio >= threshold {
            CoverageStatus::Satisfied
        } else if raw_ratio >= 0.50 {
            CoverageStatus::Degraded
        } else {
            CoverageStatus::Failed
        };

        Ok(UniverseCoverageReport {
            universe_id: resolution.universe_id.clone(),
            version: resolution.version.clone(),
            as_of: resolution.as_of,
            target_count,
            available_count,
            missing_count,
            coverage_ratio: (raw_ratio * 10_000.0).round() / 10_000.0,
            missing_symbols,
            status,
            threshold,
        })
    }
}
