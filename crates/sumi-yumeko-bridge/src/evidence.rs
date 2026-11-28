//! KT01 Technical Confirmation Gate Evaluator & Bidirectional Evidence Exporter
//! (`Sumi -> Yumeko Step 6: stock_confirmation_gate` & `agy acp`).
//!
//! Computes deterministic MTF (`1W / 1D / 1H`) technical structure, QMV Blackbox / Proxy
//! confirmation, AFC probability cone alignment, VND/kVND normalized `Entry / SL / TP1 / TP2`,
//! builds a SHA-256 pinned `KT01GateVerdictV1` + `TechnicalEvidenceBundleV1` (`advisory_only = true`),
//! and exports `.json` + `.md` artifacts safely to `~/.local/share/sumi/kt01_verdicts/<as_of>/`
//! (plus optional read-only dropzone copy to `<YUMEKO_DATA_DIR>/sumi_evidence/`).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use chrono::{FixedOffset, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sumi_core::data::{Timeframe, aggregate_ohlcv};
use sumi_core::model::Candle;
use sumi_core::scanner::{
    ConfluenceWeightProfile, MarketRegime, PrecomputedCatalogMatrix, SIGNAL_CATALOG,
    SignalOutputType, evaluate_symbol_scan, is_qmv_screener_signal,
};
use sumi_data::{OhlcvBar, sumi_data_dir, yumeko_data_dir};

use crate::contract::{
    Basis, BridgeError, BridgeResult, CandleBarV1, CandleSeriesV1, HistoricalMode,
    IndicatorColumnV1, IndicatorFrameV1, KT01GateVerdictV1, Kt01AfcSummary, Kt01ExecutionLevels,
    Kt01GateVerdict, MtfAlignmentSummary, QuantityUnit, SUMI_CONTRACT_VERSION_V1,
    SUMI_KT01_VERDICT_VERSION_V1, Scale, TechnicalEvidenceBundleV1, TimeframeV1,
    canonical_sha256_value, is_index_or_futures_symbol, normalize_sumi_vnd_to_kvnd,
    strip_exchange_prefix,
};
use crate::reader::TopDownSymbolContextDto;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedVerdictReceipt {
    pub verdict: KT01GateVerdictV1,
    pub json_path: String,
    pub markdown_path: String,
    pub yumeko_dropzone_path: Option<String>,
    pub markdown_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedVerdictListItem {
    pub verdict_id: String,
    pub symbol: String,
    pub as_of: String,
    pub verdict: Kt01GateVerdict,
    pub confidence_score: f64,
    pub flow_provenance: String,
    pub json_path: String,
    pub markdown_path: String,
    pub generated_at_utc: String,
}

/// Converts a `YYYY-MM-DD` `as_of` date to Vietnam (`UTC+7`) end-of-day cutoff in epoch milliseconds
/// (`23:59:59.999+07:00` = `16:59:59.999Z`), cleanly including all `1D` (`00:00Z`) and `1H` VN intraday
/// sessions on `as_of` while excluding every bar of `as_of + 1` (`W6-04`).
#[must_use]
pub fn vn_as_of_end_of_day_ms(as_of: &str) -> Option<i64> {
    let d = NaiveDate::parse_from_str(as_of.trim(), "%Y-%m-%d").ok()?;
    let vn_tz = FixedOffset::east_opt(7 * 3_600)?;
    let local_end = d.and_hms_milli_opt(23, 59, 59, 999)?;
    vn_tz
        .from_local_datetime(&local_end)
        .single()
        .map(|dt| dt.timestamp_millis())
}

/// Evaluates the deterministic `KT01` confirmation gate from `1D` OHLCV bars (plus optional `1H`
/// bars, optional AFC summary, and upstream Yumeko Top-Down context).
pub fn evaluate_kt01_gate(
    ticker: &str,
    daily_bars: &[OhlcvBar],
    hourly_bars: Option<&[OhlcvBar]>,
    afc: Option<Kt01AfcSummary>,
    topdown: Option<&TopDownSymbolContextDto>,
    as_of_override: Option<&str>,
) -> BridgeResult<KT01GateVerdictV1> {
    if daily_bars.len() < 20 {
        return Err(BridgeError::Contract(format!(
            "Không đủ nến 1D để đánh giá KT01 cho {ticker} (có {}, cần >= 20)",
            daily_bars.len()
        )));
    }

    let sym = strip_exchange_prefix(ticker);
    let exchange = ticker
        .split_once(':')
        .map_or("HOSE", |(ex, _)| ex)
        .to_ascii_uppercase();

    // Filter daily & hourly bars strictly <= end-of-day Vietnam (UTC+7) `as_of` (Point-in-Time guard, `W6-04`)!
    let cutoff_ms = as_of_override.and_then(vn_as_of_end_of_day_ms);
    let pit_daily: Vec<OhlcvBar> = match cutoff_ms {
        Some(cut) => daily_bars
            .iter()
            .copied()
            .filter(|b| b.timestamp <= cut)
            .collect(),
        None => daily_bars.to_vec(),
    };
    if pit_daily.len() < 20 {
        return Err(BridgeError::Contract(format!(
            "Không đủ nến 1D trước ngày chốt as_of cho {ticker} (còn {} nến)",
            pit_daily.len()
        )));
    }

    let candles_d1: Vec<Candle> = pit_daily
        .iter()
        .map(|b| b.to_core_candle(sumi_data::Interval::D1))
        .collect();
    let last_idx = candles_d1.len() - 1;
    let last_bar = &pit_daily[last_idx];
    let last_date = Utc
        .timestamp_millis_opt(last_bar.timestamp)
        .single()
        .map(|dt| dt.date_naive())
        .unwrap_or_else(|| Utc::now().date_naive());
    let as_of_str = as_of_override
        .filter(|s| NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").is_ok())
        .map_or_else(|| last_date.format("%Y-%m-%d").to_string(), str::to_owned);

    // 1. Daily Structure & 123-Signal Matrix
    let scan_d1 = evaluate_symbol_scan(&ticker.to_ascii_uppercase(), &sym, &exchange, Timeframe::D1, &candles_d1, None, 5);
    let matrix_d1 = PrecomputedCatalogMatrix::compute(&candles_d1);
    let profile = ConfluenceWeightProfile::Balanced;
    let health_d1 = matrix_d1
        .health_score_at(last_idx, profile)
        .ok()
        .filter(|v| v.is_finite());
    let daily_regime = scan_d1
        .as_ref()
        .map_or(MarketRegime::Neutral, |r| r.regime);

    let mut active_signals = Vec::new();
    for (idx, entry) in SIGNAL_CATALOG.iter().enumerate().rev() {
        if !is_qmv_screener_signal(entry.id) || entry.output_type != SignalOutputType::Bool {
            continue;
        }
        if let Some(pt) = matrix_d1.point_at_with_profile(idx, last_idx, profile)
            && pt.quality.is_valid()
            && pt.bool_value == Some(true)
        {
            active_signals.push(entry.label_vi.to_owned());
            if active_signals.len() >= 8 {
                break;
            }
        }
    }

    // 2. Weekly MTF Structure (causally rolled up from 1D)
    let weekly_trend = aggregate_ohlcv(&candles_d1, Timeframe::D1, Timeframe::W1)
        .ok()
        .and_then(|w_bars| {
            if w_bars.len() < 12 {
                return None;
            }
            let last_w = w_bars.last()?.close;
            let sma10: f64 = w_bars[w_bars.len() - 10..]
                .iter()
                .map(|c| c.close)
                .sum::<f64>()
                / 10.0;
            let sma20: Option<f64> = (w_bars.len() >= 20).then(|| {
                w_bars[w_bars.len() - 20..]
                    .iter()
                    .map(|c| c.close)
                    .sum::<f64>()
                    / 20.0
            });
            Some(match sma20 {
                Some(m20) if last_w > sma10 && sma10 >= m20 => "Tăng (1W > MA10W >= MA20W)".to_owned(),
                Some(m20) if last_w < sma10 && sma10 < m20 => "Giảm (1W < MA10W < MA20W)".to_owned(),
                _ if last_w >= sma10 => "Tích lũy / Hồi phục (1W >= MA10W)".to_owned(),
                _ => "Điều chỉnh (1W < MA10W)".to_owned(),
            })
        })
        .unwrap_or_else(|| "Chưa đủ dữ liệu 1W".to_owned());

    // 3. Hourly Timing (optional 1H bars — strictly filtered <= VN `as_of` cutoff, `W6-04`)
    let hourly_timing = hourly_bars.and_then(|raw_h| {
        let h_bars: Vec<OhlcvBar> = match cutoff_ms {
            Some(cut) => raw_h
                .iter()
                .copied()
                .filter(|b| b.timestamp <= cut)
                .collect(),
            None => raw_h.to_vec(),
        };
        if h_bars.len() < 20 {
            return None;
        }
        let last_h = h_bars.last()?.close;
        let ema20: f64 = h_bars[h_bars.len() - 20..]
            .iter()
            .map(|b| b.close)
            .sum::<f64>()
            / 20.0;
        Some(if last_h >= ema20 {
            "1H tích cực (Giá >= MA20H)".to_owned()
        } else {
            "1H đang điều chỉnh (< MA20H)".to_owned()
        })
    });

    // Moving Averages & ATR(14) for execution levels
    let ma20 = sma_tail(&candles_d1, 20);
    let ma50 = sma_tail(&candles_d1, 50);
    let ma200 = sma_tail(&candles_d1, 200);
    let atr14 = compute_atr14(&candles_d1).unwrap_or(last_bar.close * 0.025);
    let rsi_14 = scan_d1.as_ref().and_then(|r| r.rsi_14);
    let rvol_20 = scan_d1.as_ref().and_then(|r| r.rvol_20);

    // 4. Execution Levels (`SL / TP1 / TP2`) — prefer QMV real levels when available, fallback to ATR/Swing structure
    let qmv_bb = topdown.and_then(|t| t.hp01_blackbox.as_ref());
    let swing_low_20 = candles_d1[candles_d1.len().saturating_sub(20)..]
        .iter()
        .map(|c| c.low)
        .fold(f64::INFINITY, f64::min);
    let last_close = last_bar.close;

    let (sl_vnd, tp1_vnd, tp2_vnd, levels_source) = if let Some(bb) = qmv_bb
        && let Some(q_sl) = bb.stop_loss_vnd.filter(|v| *v > 0.0 && *v < last_close)
    {
        let q_tp1 = bb
            .target_price_1_vnd
            .filter(|v| *v > last_close)
            .unwrap_or(last_close + (last_close - q_sl) * 2.0);
        let q_tp2 = bb
            .target_price_2_vnd
            .filter(|v| *v > q_tp1)
            .unwrap_or(last_close + (last_close - q_sl) * 3.0);
        (q_sl, q_tp1, q_tp2, "qmv_real_blackbox".to_owned())
    } else {
        let structural_sl = (swing_low_20 * 0.992)
            .max(last_close - 2.2 * atr14)
            .min(last_close * 0.95);
        let risk = (last_close - structural_sl).max(last_close * 0.02);
        (
            structural_sl,
            last_close + risk * 2.0,
            last_close + risk * 3.2,
            "sumi_atr_structure".to_owned(),
        )
    };

    let risk_pct = ((last_close - sl_vnd) / last_close.max(1e-9)) * 100.0;
    let reward_tp1_pct = ((tp1_vnd - last_close) / last_close.max(1e-9)) * 100.0;
    let rr_ratio_tp1 = if risk_pct > 1e-6 {
        reward_tp1_pct / risk_pct
    } else {
        0.0
    };

    let entry_zone_low = (last_close - 0.5 * atr14).max(sl_vnd * 1.01);
    let entry_zone_high = last_close + 0.25 * atr14;

    // 5. Deterministic KT01 Gate Verdict & Confidence Scoring
    let mut score: f64 = 50.0;
    match daily_regime {
        MarketRegime::Uptrend => score += 16.0,
        MarketRegime::Recovery | MarketRegime::Pullback => score += 10.0,
        MarketRegime::SidewaysSqueeze => score += 4.0,
        MarketRegime::Downtrend => score -= 22.0,
        MarketRegime::Neutral | MarketRegime::InsufficientHistory => {}
    }
    if weekly_trend.starts_with("Tăng") {
        score += 10.0;
    } else if weekly_trend.starts_with("Giảm") {
        score -= 12.0;
    }
    if let Some(h) = health_d1 {
        score += (h * 0.25).clamp(-15.0, 18.0);
    }
    if let Some(bb) = qmv_bb {
        if bb.money_in_today || bb.money_in_2_sessions || bb.reversal_up_t_plus {
            score += 12.0;
        }
        if bb.dspi.is_some_and(|d| d >= 75.0) {
            score -= 8.0; // Overheated DSPI saturation caution
        }
    }
    if let Some(ref afc_s) = afc {
        if afc_s.win_rate_pct >= 55.0 && afc_s.expected_p50_pct > 0.0 {
            score += 8.0;
        } else if afc_s.win_rate_pct < 42.0 {
            score -= 8.0;
        }
    }
    let confidence_score = score.clamp(5.0, 98.0);

    let overbought = rsi_14.is_some_and(|r| r >= 72.0)
        || qmv_bb.and_then(|b| b.dspi).is_some_and(|d| d >= 78.0);
    let verdict = if daily_regime == MarketRegime::Downtrend || confidence_score < 42.0 {
        Kt01GateVerdict::Reject
    } else if overbought && confidence_score >= 55.0 {
        Kt01GateVerdict::WaitPullback
    } else if confidence_score >= 66.0 && rr_ratio_tp1 >= 1.5 {
        Kt01GateVerdict::Pass
    } else {
        Kt01GateVerdict::Watch
    };

    let flow_provenance = topdown
        .map_or_else(|| "local_proxy".to_owned(), |t| t.flow_provenance.clone());
    let mut caveats = topdown.map_or_else(Vec::new, |t| t.gaps.clone());
    caveats.push("advisory_only: Kết quả KT01 chỉ mang tính bằng chứng nghiên cứu định lượng, không phải lệnh đặt tự động.".to_owned());

    let mut invalidation_triggers = vec![format!(
        "Đóng cửa ngày thủng mốc cắt lỗ {:.0} VND ({:.2} kVND)",
        sl_vnd,
        normalize_sumi_vnd_to_kvnd(sl_vnd, &sym)
    )];
    if let Some(m20) = ma20
        && last_close >= m20
    {
        invalidation_triggers.push(format!(
            "Đóng cửa mất MA20 ({:.0} VND) kèm khối lượng lớn (>= 1.3x MA20)",
            m20
        ));
    }

    // Build wire-identical `TechnicalEvidenceBundleV1` (`yumeko_sumi_contract_v1`)
    let evidence_bundle = build_technical_evidence_bundle(
        &sym,
        &pit_daily,
        last_date,
        qmv_bb,
        &caveats,
    )?;

    let qmv_blackbox_snapshot = qmv_bb.and_then(|b| serde_json::to_value(b).ok());
    let generated_at_utc = Utc::now().to_rfc3339();
    let verdict_id = format!("kt01_{}_{}", sym.to_ascii_lowercase(), as_of_str.replace('-', ""));

    let mut result = KT01GateVerdictV1 {
        schema_version: SUMI_KT01_VERDICT_VERSION_V1.to_owned(),
        verdict_id,
        symbol: sym.clone(),
        ticker: ticker.to_ascii_uppercase(),
        as_of: as_of_str,
        generated_at_utc,
        advisory_only: true,
        flow_provenance,
        verdict,
        verdict_label_vi: verdict.label_vi().to_owned(),
        confidence_score: (confidence_score * 10.0).round() / 10.0,
        mtf_alignment: MtfAlignmentSummary {
            weekly_trend,
            daily_regime: daily_regime.full_label().to_owned(),
            daily_health_score: health_d1.map(|v| (v * 10.0).round() / 10.0),
            hourly_timing,
            ma20_vnd: ma20.map(f64::round),
            ma50_vnd: ma50.map(f64::round),
            ma200_vnd: ma200.map(f64::round),
            rsi_14: rsi_14.map(|v| (v * 10.0).round() / 10.0),
            rvol_20: rvol_20.map(|v| (v * 100.0).round() / 100.0),
            active_signals,
        },
        execution_levels: Kt01ExecutionLevels {
            last_close_vnd: last_close,
            last_close_kvnd: normalize_sumi_vnd_to_kvnd(last_close, &sym),
            entry_zone_low_vnd: entry_zone_low.round(),
            entry_zone_high_vnd: entry_zone_high.round(),
            stop_loss_vnd: sl_vnd.round(),
            stop_loss_kvnd: normalize_sumi_vnd_to_kvnd(sl_vnd, &sym),
            take_profit_1_vnd: tp1_vnd.round(),
            take_profit_1_kvnd: normalize_sumi_vnd_to_kvnd(tp1_vnd, &sym),
            take_profit_2_vnd: tp2_vnd.round(),
            take_profit_2_kvnd: normalize_sumi_vnd_to_kvnd(tp2_vnd, &sym),
            risk_pct: (risk_pct * 100.0).round() / 100.0,
            reward_tp1_pct: (reward_tp1_pct * 100.0).round() / 100.0,
            rr_ratio_tp1: (rr_ratio_tp1 * 100.0).round() / 100.0,
            levels_source,
        },
        afc_summary: afc,
        qmv_blackbox_snapshot,
        dm01_is_real_holding: topdown.is_some_and(|t| t.dm01_real_holding.is_some()),
        dm01_is_yumeko_watchlist: topdown.is_some_and(|t| t.dm01_yumeko_watchlist.is_some()),
        invalidation_triggers,
        caveats,
        evidence_bundle: Some(evidence_bundle),
        verdict_sha256: String::new(),
    };

    let hash_val = serde_json::to_value(&result)?;
    result.verdict_sha256 = canonical_sha256_value(&hash_val);
    Ok(result)
}

/// Renders a concise Vietnamese Markdown evidence brief ready for Yumeko / `agy acp`.
#[must_use]
pub fn render_kt01_markdown(v: &KT01GateVerdictV1) -> String {
    let mut md = String::new();
    md.push_str(&format!(
        "## Bằng Chứng Kỹ Thuật KT01 — `{}` (`as_of: {}`)\n\n",
        v.ticker, v.as_of
    ));
    md.push_str(&format!(
        "- **Kết luận cổng KT01:** **{}** (Độ tin cậy: `{:.1}/100`, `advisory_only = true`)\n",
        v.verdict_label_vi, v.confidence_score
    ));
    md.push_str(&format!(
        "- **Nguồn dòng tiền:** `{}` · **Danh mục DM01:** {}\n",
        v.flow_provenance,
        if v.dm01_is_real_holding {
            "Đang nắm giữ thật (Real Holding)"
        } else if v.dm01_is_yumeko_watchlist {
            "Trong Watchlist Yumeko (★ — Chưa nắm giữ)"
        } else {
            "Ngoài danh mục"
        }
    ));
    md.push_str(&format!(
        "- **Đa khung thời gian (MTF):** 1W `{}` | 1D `{}` (Health `{}`) | 1H `{}`\n",
        v.mtf_alignment.weekly_trend,
        v.mtf_alignment.daily_regime,
        v.mtf_alignment
            .daily_health_score
            .map_or_else(|| "—".to_owned(), |h| format!("{h:.1}")),
        v.mtf_alignment
            .hourly_timing
            .as_deref()
            .unwrap_or("Chưa nạp 1H")
    ));
    md.push_str(&format!(
        "- **Kế hoạch giá ({}):** Đóng cửa `{:.0}` VND (`{:.2}` kVND) | Vùng mua `{:.0}–{:.0}` | SL `{:.0}` (`-{:.2}%`) | TP1 `{:.0}` (`+{:.2}%`, **R:R `{:.2}`**) | TP2 `{:.0}`\n",
        v.execution_levels.levels_source,
        v.execution_levels.last_close_vnd,
        v.execution_levels.last_close_kvnd,
        v.execution_levels.entry_zone_low_vnd,
        v.execution_levels.entry_zone_high_vnd,
        v.execution_levels.stop_loss_vnd,
        v.execution_levels.risk_pct,
        v.execution_levels.take_profit_1_vnd,
        v.execution_levels.reward_tp1_pct,
        v.execution_levels.rr_ratio_tp1,
        v.execution_levels.take_profit_2_vnd,
    ));
    if let Some(ref afc) = v.afc_summary {
        md.push_str(&format!(
            "- **Nón xác suất lịch sử AFC (H={}):** Win-rate `{:.1}%`, Trung vị P50 `{:+.2}%`, Dải P10..P90 `{:+.2}% .. {:+.2}%`\n",
            afc.horizon_bars, afc.win_rate_pct, afc.expected_p50_pct, afc.p10_pct, afc.p90_pct
        ));
    }
    if !v.mtf_alignment.active_signals.is_empty() {
        md.push_str(&format!(
            "- **Tín hiệu KT01 kích hoạt:** {}\n",
            v.mtf_alignment.active_signals.join(" · ")
        ));
    }
    if !v.invalidation_triggers.is_empty() {
        md.push_str("\n### Điểm Gãy Luận Điểm (Invalidation)\n");
        for trig in &v.invalidation_triggers {
            md.push_str(&format!("- {trig}\n"));
        }
    }
    md.push_str(&format!(
        "\n> `verdict_sha256`: `{}` · `schema`: `{}`\n",
        v.verdict_sha256, v.schema_version
    ));
    md
}

/// Persists `KT01GateVerdictV1` (`.json` + `.md`) under `<sumi_data_dir>/kt01_verdicts/<as_of>/`
/// and optionally copies the `.json` to `<YUMEKO_DATA_DIR>/sumi_evidence/` (never touching `yumeko-app.sqlite`!).
pub fn export_kt01_verdict_to_disk(
    verdict: &KT01GateVerdictV1,
    base_data_dir: Option<&Path>,
    copy_to_yumeko_dropzone: bool,
) -> BridgeResult<ExportedVerdictReceipt> {
    let data_root = base_data_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(sumi_data_dir);
    let out_dir = data_root.join("kt01_verdicts").join(&verdict.as_of);
    fs::create_dir_all(&out_dir)?;

    let sha8: String = verdict.verdict_sha256.chars().take(8).collect();
    let stem = format!("{}_{}", verdict.symbol.to_ascii_uppercase(), sha8);
    let json_path = out_dir.join(format!("{stem}.json"));
    let md_path = out_dir.join(format!("{stem}.md"));

    let json_pretty = serde_json::to_string_pretty(verdict)?;
    let markdown = render_kt01_markdown(verdict);
    write_atomic(&json_path, json_pretty.as_bytes())?;
    write_atomic(&md_path, markdown.as_bytes())?;

    let mut yumeko_dropzone_path = None;
    if copy_to_yumeko_dropzone {
        let drop_dir = yumeko_data_dir().join("sumi_evidence").join(&verdict.as_of);
        if fs::create_dir_all(&drop_dir).is_ok() {
            let drop_json = drop_dir.join(format!("{stem}.json"));
            let drop_md = drop_dir.join(format!("{stem}.md"));
            if write_atomic(&drop_json, json_pretty.as_bytes()).is_ok() {
                let _ = write_atomic(&drop_md, markdown.as_bytes());
                yumeko_dropzone_path = Some(drop_json.display().to_string());
            }
        }
    }

    Ok(ExportedVerdictReceipt {
        verdict: verdict.clone(),
        json_path: json_path.display().to_string(),
        markdown_path: md_path.display().to_string(),
        yumeko_dropzone_path,
        markdown_summary: markdown,
    })
}

/// Lists recently exported KT01 verdicts from `<sumi_data_dir>/kt01_verdicts/`.
pub fn list_exported_verdicts(
    base_data_dir: Option<&Path>,
    limit: usize,
) -> BridgeResult<Vec<ExportedVerdictListItem>> {
    let root = base_data_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(sumi_data_dir)
        .join("kt01_verdicts");
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut items = Vec::new();
    for day_entry in fs::read_dir(&root)?.flatten() {
        let day_path = day_entry.path();
        if !day_path.is_dir() {
            continue;
        }
        for file_entry in fs::read_dir(&day_path)?.flatten() {
            let p = file_entry.path();
            if p.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Ok(raw) = fs::read_to_string(&p) else {
                continue;
            };
            let Ok(v) = serde_json::from_str::<KT01GateVerdictV1>(&raw) else {
                continue;
            };
            let md = p.with_extension("md");
            items.push(ExportedVerdictListItem {
                verdict_id: v.verdict_id,
                symbol: v.symbol,
                as_of: v.as_of,
                verdict: v.verdict,
                confidence_score: v.confidence_score,
                flow_provenance: v.flow_provenance,
                json_path: p.display().to_string(),
                markdown_path: md.display().to_string(),
                generated_at_utc: v.generated_at_utc,
            });
        }
    }
    items.sort_by(|a, b| b.generated_at_utc.cmp(&a.generated_at_utc));
    items.truncate(limit.clamp(1, 200));
    Ok(items)
}

fn build_technical_evidence_bundle(
    sym: &str,
    bars: &[OhlcvBar],
    as_of: NaiveDate,
    qmv_bb: Option<&crate::reader::QmvStockBlackboxSnapshotDto>,
    gaps: &[String],
) -> BridgeResult<TechnicalEvidenceBundleV1> {
    let tail = &bars[bars.len().saturating_sub(120)..];
    let is_idx = is_index_or_futures_symbol(sym);
    let (price_unit, price_scale) = if is_idx {
        (QuantityUnit::Points, Scale::Unit)
    } else {
        (QuantityUnit::ThousandVnd, Scale::Thousand)
    };

    let mut by_date: BTreeMap<NaiveDate, CandleBarV1> = BTreeMap::new();
    for b in tail {
        let Some(dt) = Utc.timestamp_millis_opt(b.timestamp).single() else {
            continue;
        };
        let d = dt.date_naive();
        let o = normalize_sumi_vnd_to_kvnd(b.open, sym);
        let h = normalize_sumi_vnd_to_kvnd(b.high, sym);
        let l = normalize_sumi_vnd_to_kvnd(b.low, sym);
        let c = normalize_sumi_vnd_to_kvnd(b.close, sym);
        by_date.insert(
            d,
            CandleBarV1 {
                bar_date: d,
                timestamp_utc: None,
                open: o,
                high: h.max(o).max(c),
                low: l.min(o).min(c),
                close: c,
                volume: b.volume.max(0.0),
                value_traded: (b.turnover > 0.0).then_some(b.turnover),
            },
        );
    }
    let contract_bars: Vec<CandleBarV1> = by_date.into_values().collect();
    let series_payload = serde_json::to_value(&contract_bars)?;
    let source_sha256 = canonical_sha256_value(&series_payload);

    let candle_series = CandleSeriesV1 {
        schema_version: SUMI_CONTRACT_VERSION_V1.to_owned(),
        symbol: sym.to_owned(),
        timeframe: TimeframeV1::Daily1d,
        price_unit,
        price_scale,
        basis: Basis::AdjustedClose,
        historical_mode: HistoricalMode::PointInTime,
        source_provider: "sumi_supercharts_v3".to_owned(),
        source_sha256: source_sha256.clone(),
        as_of,
        bars: contract_bars,
    };

    let dates: Vec<NaiveDate> = candle_series.bars.iter().map(|b| b.bar_date).collect();
    let closes: Vec<f64> = candle_series.bars.iter().map(|b| b.close).collect();
    let sma = |w: usize| -> Vec<Option<f64>> {
        closes
            .iter()
            .enumerate()
            .map(|(i, _)| {
                if i + 1 < w {
                    None
                } else {
                    let s: f64 = closes[(i + 1 - w)..=i].iter().sum();
                    Some((s / (w as f64) * 100.0).round() / 100.0)
                }
            })
            .collect()
    };
    let mut indicators = BTreeMap::new();
    for (key, label, w) in [
        ("sumi_sma_20", "SMA(20) [Sumi Deterministic]", 20),
        ("sumi_sma_50", "SMA(50) [Sumi Deterministic]", 50),
    ] {
        indicators.insert(
            key.to_owned(),
            IndicatorColumnV1 {
                name: label.to_owned(),
                formula_version: format!("sumi.sma.v1(window={w})"),
                unit: price_unit,
                values: sma(w),
                is_provider_proprietary: false,
            },
        );
    }

    let frame = IndicatorFrameV1 {
        schema_version: SUMI_CONTRACT_VERSION_V1.to_owned(),
        symbol: sym.to_owned(),
        timeframe: TimeframeV1::Daily1d,
        source_series_sha256: source_sha256,
        dates,
        indicators,
    };

    let mut raw_qmv_signals = BTreeMap::new();
    if let Some(bb) = qmv_bb {
        raw_qmv_signals.insert(
            "qmv_blackbox_real".to_owned(),
            serde_json::to_value(bb).unwrap_or(json!({})),
        );
    }

    let bundle = TechnicalEvidenceBundleV1 {
        schema_version: SUMI_CONTRACT_VERSION_V1.to_owned(),
        symbol: sym.to_owned(),
        as_of,
        candle_series,
        indicator_frame: Some(frame),
        backtest_summaries: Vec::new(),
        raw_qmv_signals,
        gaps: gaps.to_vec(),
    };
    bundle.validate()?;
    Ok(bundle)
}

fn sma_tail(candles: &[Candle], window: usize) -> Option<f64> {
    if candles.len() < window || window == 0 {
        return None;
    }
    let sum: f64 = candles[candles.len() - window..]
        .iter()
        .map(|c| c.close)
        .sum();
    Some(sum / window as f64)
}

fn compute_atr14(candles: &[Candle]) -> Option<f64> {
    if candles.len() < 15 {
        return None;
    }
    let start = candles.len() - 14;
    let mut tr_sum = 0.0;
    for i in start..candles.len() {
        let cur = &candles[i];
        let prev_close = candles[i - 1].close;
        let tr = (cur.high - cur.low)
            .max((cur.high - prev_close).abs())
            .max((cur.low - prev_close).abs());
        tr_sum += tr;
    }
    Some(tr_sum / 14.0)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> BridgeResult<()> {
    let tmp = path.with_extension(format!(
        "tmp.{}",
        Utc::now().timestamp_nanos_opt().unwrap_or(0)
    ));
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn evaluate_and_export_kt01_verdict_enforces_advisory_only_and_sha256_pin() {
        let base_ms = NaiveDate::from_ymd_opt(2026, 5, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();
        let bars: Vec<OhlcvBar> = (0..90_i64)
            .map(|i| {
                let p = 110_000.0 + (i as f64) * 280.0;
                OhlcvBar::new(
                    base_ms + i * 86_400_000,
                    p - 400.0,
                    p + 900.0,
                    p - 900.0,
                    p + 200.0,
                    1_200_000.0,
                )
            })
            .collect();

        let verdict = evaluate_kt01_gate("HOSE:FPT", &bars, None, None, None, None).unwrap();
        assert!(verdict.advisory_only, "Hard Guard: advisory_only MUST always be true");
        assert_eq!(verdict.verdict_sha256.len(), 64);
        assert!(verdict.execution_levels.last_close_vnd > 100_000.0);
        assert!(verdict.execution_levels.last_close_kvnd > 100.0 && verdict.execution_levels.last_close_kvnd < 300.0);
        assert!(verdict.evidence_bundle.as_ref().unwrap().validate().is_ok());

        let tmp = tempfile::tempdir().unwrap();
        let receipt = export_kt01_verdict_to_disk(&verdict, Some(tmp.path()), false).unwrap();
        assert!(PathBuf::from(&receipt.json_path).is_file());
        assert!(PathBuf::from(&receipt.markdown_path).is_file());
        assert!(receipt.markdown_summary.contains("HOSE:FPT"));

        let listed = list_exported_verdicts(Some(tmp.path()), 10).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].symbol, "FPT");
    }

    #[test]
    fn wave6_kt01_verdict_filters_future_1h_bars_strictly_at_vn_as_of_cutoff() {
        let base_ms = NaiveDate::from_ymd_opt(2026, 5, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();
        let daily: Vec<OhlcvBar> = (0..60_i64)
            .map(|i| {
                let p = 110_000.0 + (i as f64) * 200.0;
                OhlcvBar::new(base_ms + i * 86_400_000, p, p + 500.0, p - 500.0, p, 1_000_000.0)
            })
            .collect();
        // Put 25 hourly bars strictly AFTER `as_of = 2026-06-01` (future!) and only 5 BEFORE -> must return `None`!
        let cutoff_ms = vn_as_of_end_of_day_ms("2026-06-01").unwrap();
        let mut hourly = Vec::new();
        for i in 0..5_i64 {
            hourly.push(OhlcvBar::new(
                cutoff_ms - (5 - i) * 3_600_000,
                115_000.0,
                116_000.0,
                114_000.0,
                115_500.0,
                100_000.0,
            ));
        }
        for i in 1..=25_i64 {
            hourly.push(OhlcvBar::new(
                cutoff_ms + i * 3_600_000,
                140_000.0,
                145_000.0,
                139_000.0,
                144_000.0,
                200_000.0,
            ));
        }

        let verdict = evaluate_kt01_gate(
            "HOSE:FPT",
            &daily,
            Some(&hourly),
            None,
            None,
            Some("2026-06-01"),
        )
        .unwrap();
        assert!(
            verdict.mtf_alignment.hourly_timing.is_none(),
            "Future 1H bars after as_of MUST be stripped so <20 pre-cutoff bars yield None!"
        );

        // Now supply 22 pre-cutoff 1H bars (bearish) + 25 future 1H bars (bullish spike) -> verdict MUST reflect bearish pre-cutoff!
        let mut hourly_bear_pre = Vec::new();
        for i in 0..22_i64 {
            let p = 120_000.0 - (i as f64) * 500.0;
            hourly_bear_pre.push(OhlcvBar::new(
                cutoff_ms - (22 - i) * 3_600_000,
                p,
                p + 200.0,
                p - 200.0,
                p,
                100_000.0,
            ));
        }
        for i in 1..=25_i64 {
            hourly_bear_pre.push(OhlcvBar::new(
                cutoff_ms + i * 3_600_000,
                180_000.0,
                185_000.0,
                179_000.0,
                184_000.0,
                300_000.0,
            ));
        }
        let v2 = evaluate_kt01_gate(
            "HOSE:FPT",
            &daily,
            Some(&hourly_bear_pre),
            None,
            None,
            Some("2026-06-01"),
        )
        .unwrap();
        assert_eq!(
            v2.mtf_alignment.hourly_timing.as_deref(),
            Some("1H đang điều chỉnh (< MA20H)"),
            "1H timing must evaluate strictly on <= as_of bars and ignore future bullish spike!"
        );
    }
}
