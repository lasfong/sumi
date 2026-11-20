//! Self-contained, zero-dependency, `wasm32-unknown-unknown`-compatible HTML / Print-to-PDF
//! Quantitative Strategy Backtest Report Generator (`v1.1 / Wave 4`).
//!
//! Renders an offline-first institutional tearsheet containing:
//! - Deterministic `RunManifest` metadata & `SplitMix64` digest
//! - Full KPI Tearsheet grid (+ optional In-Sample / Out-of-Sample robustness comparison)
//! - 2 inline vector SVG charts: Cumulative Equity Curve (Strategy vs Buy & Hold) and Underwater Drawdown (%)
//! - Robustness attribution tables (`OutlierImpact`, `DrawdownPeriod`s, UTC Yearly, Signal-Bar Regime, Multi-Phase Batch)
//! - Complete Trade Log table (XSS-escaped & print-paginated for A4/Letter via `@media print`)

use std::fmt::Write as _;

use crate::data::format_utc_date;
use crate::journal::ExchangeVenue;
use crate::model::Candle;
use crate::trading::PositionSide;

use super::analytics::{AdvancedTearsheetAnalytics, RunManifest};
use super::engine::{BacktestRunOptions, BacktestRunResult};
use super::metrics::{BacktestEquityPoint, MIN_STATISTICAL_SAMPLE_TRADES};

/// Maximum trade rows emitted verbatim in the HTML table so huge intraday runs stay print-snappy.
const MAX_HTML_TRADE_ROWS: usize = 1_000;

/// Maximum polyline vertices per SVG chart after deterministic stride downsampling.
const MAX_SVG_POINTS: usize = 420;

/// Escapes all HTML-significant characters (`&`, `<`, `>`, `"`, `'`) to prevent XSS injection.
#[must_use]
pub fn escape_html(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

fn fmt_money(val: f64, is_vn: bool, signed: bool) -> String {
    if !val.is_finite() {
        return "—".to_owned();
    }
    if is_vn {
        if signed {
            format!("{val:+.2} kVND")
        } else {
            format!("{val:.2} kVND")
        }
    } else if signed {
        if val < 0.0 {
            format!("-${:.2}", val.abs())
        } else {
            format!("+${val:.2}")
        }
    } else if val < 0.0 {
        format!("-${:.2}", val.abs())
    } else {
        format!("${val:.2}")
    }
}

fn fmt_opt_f64(opt: Option<f64>, decimals: usize, suffix: &str, signed: bool) -> String {
    match opt {
        Some(v) if v.is_finite() => {
            if signed {
                format!("{v:+.decimals$}{suffix}")
            } else {
                format!("{v:.decimals$}{suffix}")
            }
        }
        _ => "—".to_owned(),
    }
}

fn value_tone_class(val: f64) -> &'static str {
    if !val.is_finite() || val.abs() < 1.0e-9 {
        "neu"
    } else if val > 0.0 {
        "pos"
    } else {
        "neg"
    }
}

fn is_vn_venue(venue: ExchangeVenue) -> bool {
    matches!(
        venue,
        ExchangeVenue::Hose
            | ExchangeVenue::Hnx
            | ExchangeVenue::Upcom
            | ExchangeVenue::Vn30Futures
            | ExchangeVenue::VnIndexBenchmark
    )
}

/// Deterministic min-max preserving downsample indices for SVG polylines.
fn downsample_indices(len: usize, max_points: usize) -> Vec<usize> {
    if len == 0 {
        return Vec::new();
    }
    if len <= max_points {
        return (0..len).collect();
    }
    let step = (len - 1) as f64 / (max_points.saturating_sub(1).max(1)) as f64;
    let mut out = Vec::with_capacity(max_points);
    for k in 0..max_points {
        let idx = ((k as f64) * step).round() as usize;
        let clamped = idx.min(len - 1);
        if out.last().copied() != Some(clamped) {
            out.push(clamped);
        }
    }
    if out.last().copied() != Some(len - 1) {
        out.push(len - 1);
    }
    out
}

fn render_equity_svg(
    equity_curve: &[BacktestEquityPoint],
    candles: &[Candle],
    initial_cash: f64,
    is_vn: bool,
) -> String {
    let w = 920.0_f64;
    let h = 260.0_f64;
    let pad_l = 78.0_f64;
    let pad_r = 22.0_f64;
    let pad_t = 22.0_f64;
    let pad_b = 30.0_f64;
    let plot_w = w - pad_l - pad_r;
    let plot_h = h - pad_t - pad_b;

    if equity_curve.is_empty() {
        return "<div class=\"empty-chart\">No equity curve data available.</div>".to_owned();
    }

    let init = if initial_cash.is_finite() && initial_cash > 0.0 {
        initial_cash
    } else {
        100_000.0
    };
    let sample_ids = downsample_indices(equity_curve.len(), MAX_SVG_POINTS);

    // Compute aligned Buy & Hold curve normalized to `initial_cash`.
    let first_close = equity_curve
        .first()
        .and_then(|pt| candles.get(pt.bar_index))
        .map(|c| c.close)
        .filter(|&p| p.is_finite() && p > 0.0);

    let mut strat_pts: Vec<(usize, f64, i64)> = Vec::with_capacity(sample_ids.len());
    let mut bnh_pts: Vec<(usize, f64)> = Vec::with_capacity(sample_ids.len());
    let mut min_v = init;
    let mut max_v = init;

    for &si in &sample_ids {
        let pt = &equity_curve[si];
        let eq = if pt.equity.is_finite() {
            pt.equity
        } else {
            init
        };
        min_v = min_v.min(eq);
        max_v = max_v.max(eq);
        strat_pts.push((si, eq, pt.timestamp));

        if let Some(c0) = first_close
            && let Some(c) = candles.get(pt.bar_index)
            && c.close.is_finite()
            && c.close > 0.0
        {
            let bnh_eq = init * (c.close / c0);
            if bnh_eq.is_finite() {
                min_v = min_v.min(bnh_eq);
                max_v = max_v.max(bnh_eq);
                bnh_pts.push((si, bnh_eq));
            }
        }
    }

    let span = (max_v - min_v).max(init * 0.01).max(1.0);
    let y_min = min_v - span * 0.06;
    let y_max = max_v + span * 0.06;
    let y_span = (y_max - y_min).max(1e-6);
    let max_idx = (equity_curve.len().saturating_sub(1)).max(1) as f64;

    let to_x = |idx: usize| -> f64 { pad_l + ((idx as f64) / max_idx) * plot_w };
    let to_y = |val: f64| -> f64 { pad_t + (1.0 - (val - y_min) / y_span) * plot_h };

    let final_eq = strat_pts.last().map_or(init, |p| p.1);
    let strat_color = if final_eq >= init {
        "#26A69A"
    } else {
        "#EF5350"
    };

    let mut svg = String::with_capacity(4096);
    let _ = writeln!(
        svg,
        "<svg class=\"chart-svg\" viewBox=\"0 0 {w:.0} {h:.0}\" role=\"img\" aria-label=\"Equity Curve vs Buy and Hold\">"
    );

    // Horizontal Y gridlines (5 ticks)
    for k in 0..=4 {
        let frac = (k as f64) / 4.0;
        let val = y_min + frac * y_span;
        let y = to_y(val);
        let _ = writeln!(
            svg,
            "  <line class=\"grid-line\" x1=\"{pad_l:.1}\" y1=\"{y:.1}\" x2=\"{:.1}\" y2=\"{y:.1}\" />",
            pad_l + plot_w
        );
        let lbl = if is_vn {
            format!("{val:.0}")
        } else {
            format!("${val:.0}")
        };
        let _ = writeln!(
            svg,
            "  <text class=\"axis-lbl\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\">{}</text>",
            pad_l - 8.0,
            y + 3.5,
            escape_html(&lbl)
        );
    }

    // Initial capital baseline
    let base_y = to_y(init);
    let _ = writeln!(
        svg,
        "  <line class=\"baseline\" x1=\"{pad_l:.1}\" y1=\"{base_y:.1}\" x2=\"{:.1}\" y2=\"{base_y:.1}\" />",
        pad_l + plot_w
    );

    // X-axis date labels (5 ticks)
    for k in 0..=4 {
        let idx = ((k as f64 / 4.0) * max_idx).round() as usize;
        let clamped = idx.min(equity_curve.len() - 1);
        let x = to_x(clamped);
        let date_s = format_utc_date(equity_curve[clamped].timestamp);
        let anchor = if k == 0 {
            "start"
        } else if k == 4 {
            "end"
        } else {
            "middle"
        };
        let _ = writeln!(
            svg,
            "  <text class=\"axis-lbl\" x=\"{x:.1}\" y=\"{:.1}\" text-anchor=\"{anchor}\">{}</text>",
            h - 8.0,
            escape_html(&date_s)
        );
    }

    // Buy & Hold dashed path
    if bnh_pts.len() >= 2 {
        let mut d = String::with_capacity(bnh_pts.len() * 14);
        for (i, &(idx, val)) in bnh_pts.iter().enumerate() {
            let cmd = if i == 0 { 'M' } else { 'L' };
            let _ = write!(d, "{cmd}{:.1},{:.1} ", to_x(idx), to_y(val));
        }
        let _ = writeln!(
            svg,
            "  <path class=\"bnh-line\" d=\"{}\" fill=\"none\" />",
            d.trim_end()
        );
    }

    // Strategy equity area + polyline
    if strat_pts.len() >= 2 {
        let mut d_line = String::with_capacity(strat_pts.len() * 14);
        for (i, &(idx, val, _)) in strat_pts.iter().enumerate() {
            let cmd = if i == 0 { 'M' } else { 'L' };
            let _ = write!(d_line, "{cmd}{:.1},{:.1} ", to_x(idx), to_y(val));
        }
        let first_x = to_x(strat_pts.first().unwrap().0);
        let last_x = to_x(strat_pts.last().unwrap().0);
        let area_d = format!(
            "{} L{last_x:.1},{base_y:.1} L{first_x:.1},{base_y:.1} Z",
            d_line.trim_end()
        );
        let _ = writeln!(
            svg,
            "  <path d=\"{area_d}\" fill=\"{strat_color}\" fill-opacity=\"0.12\" stroke=\"none\" />"
        );
        let _ = writeln!(
            svg,
            "  <path d=\"{}\" fill=\"none\" stroke=\"{strat_color}\" stroke-width=\"2.0\" stroke-linejoin=\"round\" />",
            d_line.trim_end()
        );
    }

    // Legend top-left inside plot
    let bnh_ret = bnh_pts
        .last()
        .map(|(_, v)| ((v / init) - 1.0) * 100.0)
        .filter(|v| v.is_finite());
    let strat_ret = ((final_eq / init) - 1.0) * 100.0;
    let legend_bnh = bnh_ret.map_or_else(
        || "Buy & Hold".to_owned(),
        |r| format!("Buy & Hold ({r:+.2}%)"),
    );
    let _ = writeln!(
        svg,
        "  <rect x=\"{:.1}\" y=\"5\" width=\"10\" height=\"3\" fill=\"{strat_color}\" />",
        pad_l + 6.0
    );
    let _ = writeln!(
        svg,
        "  <text class=\"legend-lbl\" x=\"{:.1}\" y=\"10\">Strategy ({strat_ret:+.2}%)</text>",
        pad_l + 20.0
    );
    let _ = writeln!(
        svg,
        "  <line x1=\"{:.1}\" y1=\"6.5\" x2=\"{:.1}\" y2=\"6.5\" class=\"bnh-line\" />",
        pad_l + 175.0,
        pad_l + 190.0
    );
    let _ = writeln!(
        svg,
        "  <text class=\"legend-lbl\" x=\"{:.1}\" y=\"10\">{}</text>",
        pad_l + 195.0,
        escape_html(&legend_bnh)
    );
    svg.push_str("</svg>");
    svg
}

fn render_underwater_svg(equity_curve: &[BacktestEquityPoint]) -> String {
    let w = 920.0_f64;
    let h = 175.0_f64;
    let pad_l = 78.0_f64;
    let pad_r = 22.0_f64;
    let pad_t = 18.0_f64;
    let pad_b = 28.0_f64;
    let plot_w = w - pad_l - pad_r;
    let plot_h = h - pad_t - pad_b;

    if equity_curve.is_empty() {
        return "<div class=\"empty-chart\">No drawdown data available.</div>".to_owned();
    }

    let sample_ids = downsample_indices(equity_curve.len(), MAX_SVG_POINTS);
    let mut worst_dd = 0.0_f64;
    let mut worst_idx = 0_usize;
    for (idx, pt) in equity_curve.iter().enumerate() {
        let uw = if pt.underwater_pct.is_finite() {
            pt.underwater_pct.min(0.0)
        } else {
            0.0
        };
        if uw < worst_dd {
            worst_dd = uw;
            worst_idx = idx;
        }
    }
    let y_min = (worst_dd * 1.12).min(-1.0);
    let y_span = (0.0 - y_min).max(1.0);
    let max_idx = (equity_curve.len().saturating_sub(1)).max(1) as f64;
    let to_x = |idx: usize| -> f64 { pad_l + ((idx as f64) / max_idx) * plot_w };
    let to_y = |uw: f64| -> f64 { pad_t + ((0.0 - uw) / y_span) * plot_h };

    let mut svg = String::with_capacity(3072);
    let _ = writeln!(
        svg,
        "<svg class=\"chart-svg\" viewBox=\"0 0 {w:.0} {h:.0}\" role=\"img\" aria-label=\"Underwater Drawdown Percentage\">"
    );

    for k in 0..=3 {
        let uw = y_min * ((k as f64) / 3.0);
        let y = to_y(uw);
        let _ = writeln!(
            svg,
            "  <line class=\"grid-line\" x1=\"{pad_l:.1}\" y1=\"{y:.1}\" x2=\"{:.1}\" y2=\"{y:.1}\" />",
            pad_l + plot_w
        );
        let _ = writeln!(
            svg,
            "  <text class=\"axis-lbl\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\">{uw:.1}%</text>",
            pad_l - 8.0,
            y + 3.5
        );
    }

    let y0 = to_y(0.0);
    let _ = writeln!(
        svg,
        "  <line class=\"baseline\" x1=\"{pad_l:.1}\" y1=\"{y0:.1}\" x2=\"{:.1}\" y2=\"{y0:.1}\" />",
        pad_l + plot_w
    );

    if sample_ids.len() >= 2 {
        let mut d_line = String::with_capacity(sample_ids.len() * 14);
        for (i, &si) in sample_ids.iter().enumerate() {
            let uw = equity_curve[si].underwater_pct;
            let uw_clean = if uw.is_finite() { uw.min(0.0) } else { 0.0 };
            let cmd = if i == 0 { 'M' } else { 'L' };
            let _ = write!(d_line, "{cmd}{:.1},{:.1} ", to_x(si), to_y(uw_clean));
        }
        let first_x = to_x(*sample_ids.first().unwrap());
        let last_x = to_x(*sample_ids.last().unwrap());
        let area_d = format!(
            "{} L{last_x:.1},{y0:.1} L{first_x:.1},{y0:.1} Z",
            d_line.trim_end()
        );
        let _ = writeln!(
            svg,
            "  <path d=\"{area_d}\" fill=\"#EF5350\" fill-opacity=\"0.22\" stroke=\"none\" />"
        );
        let _ = writeln!(
            svg,
            "  <path d=\"{}\" fill=\"none\" stroke=\"#EF5350\" stroke-width=\"1.6\" />",
            d_line.trim_end()
        );
    }

    // Highlight worst drawdown trough
    if worst_dd < -1e-6 {
        let wx = to_x(worst_idx);
        let wy = to_y(worst_dd);
        let _ = writeln!(
            svg,
            "  <circle cx=\"{wx:.1}\" cy=\"{wy:.1}\" r=\"3.5\" fill=\"#EF5350\" stroke=\"#FFFFFF\" stroke-width=\"1.0\" />"
        );
        let anchor = if wx > pad_l + plot_w * 0.72 {
            "end"
        } else {
            "start"
        };
        let tx = if anchor == "end" { wx - 6.0 } else { wx + 6.0 };
        let _ = writeln!(
            svg,
            "  <text class=\"trough-lbl\" x=\"{tx:.1}\" y=\"{:.1}\" text-anchor=\"{anchor}\">Max DD {worst_dd:.2}% ({})</text>",
            (wy - 4.0).max(pad_t + 10.0),
            escape_html(&format_utc_date(equity_curve[worst_idx].timestamp))
        );
    }

    for k in 0..=4 {
        let idx = ((k as f64 / 4.0) * max_idx).round() as usize;
        let clamped = idx.min(equity_curve.len() - 1);
        let x = to_x(clamped);
        let date_s = format_utc_date(equity_curve[clamped].timestamp);
        let anchor = if k == 0 {
            "start"
        } else if k == 4 {
            "end"
        } else {
            "middle"
        };
        let _ = writeln!(
            svg,
            "  <text class=\"axis-lbl\" x=\"{x:.1}\" y=\"{:.1}\" text-anchor=\"{anchor}\">{}</text>",
            h - 7.0,
            escape_html(&date_s)
        );
    }

    svg.push_str("</svg>");
    svg
}

/// Renders a complete, self-contained, offline-first HTML5 Strategy Lab Report ready for
/// browser inspection or Print-to-PDF (`Cmd+P -> Save as PDF`).
///
/// Purely deterministic: identical inputs always produce byte-for-byte identical HTML output.
#[must_use]
pub fn render_backtest_report_html(
    result: &BacktestRunResult,
    candles: &[Candle],
    venue: ExchangeVenue,
    analytics: Option<&AdvancedTearsheetAnalytics>,
) -> String {
    let ts = &result.tearsheet;
    let cfg = &result.config;
    let is_vn = is_vn_venue(venue);
    let fallback_manifest = RunManifest::build(result, BacktestRunOptions::default());
    let manifest = analytics.map_or(&fallback_manifest, |a| &a.manifest);

    let sym_esc = escape_html(&result.symbol);
    let strat_esc = escape_html(cfg.preset.full_label());
    let param_esc = escape_html(&manifest.param_summary);
    let venue_esc = escape_html(venue.full_label());
    let tf_esc = escape_html(result.timeframe.label());
    let start_date = result
        .first_timestamp
        .map_or_else(|| "—".to_owned(), format_utc_date);
    let end_date = result
        .last_timestamp
        .map_or_else(|| "—".to_owned(), format_utc_date);

    let eq_svg = render_equity_svg(&result.equity_curve, candles, cfg.initial_cash, is_vn);
    let uw_svg = render_underwater_svg(&result.equity_curve);

    let mut html = String::with_capacity(24 * 1024);
    let _ = write!(
        html,
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>Sumi Strategy Report — {sym_esc} · {strat_esc}</title>
<style>
  :root {{
    --bg: #0D111A;
    --panel: #131824;
    --panel2: #192030;
    --border: #273043;
    --text: #E2E8F0;
    --muted: #8B97AD;
    --bull: #26A69A;
    --bear: #EF5350;
    --accent: #2962FF;
    --warn: #FFB74D;
  }}
  * {{ box-sizing: border-box; }}
  body {{
    margin: 0; padding: 24px 28px;
    background: var(--bg); color: var(--text);
    font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI", Roboto, sans-serif;
    font-size: 13px; line-height: 1.45;
  }}
  .wrap {{ max-width: 1080px; margin: 0 auto; }}
  header {{
    display: flex; justify-content: space-between; align-items: flex-start; gap: 16px;
    border-bottom: 1px solid var(--border); padding-bottom: 14px; margin-bottom: 18px;
  }}
  .brand {{
    display: inline-block; background: var(--accent); color: #fff;
    font-weight: 700; font-size: 11px; letter-spacing: 0.08em;
    padding: 3px 8px; border-radius: 4px; margin-bottom: 6px;
  }}
  h1 {{ margin: 0 0 4px 0; font-size: 21px; font-weight: 700; }}
  .sub {{ color: var(--muted); font-size: 12px; font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }}
  .print-btn {{
    background: var(--panel2); color: var(--text); border: 1px solid var(--border);
    border-radius: 6px; padding: 7px 14px; font-size: 12px; font-weight: 600; cursor: pointer;
  }}
  .print-btn:hover {{ border-color: var(--accent); color: #fff; }}
  .manifest {{
    display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: 8px;
    background: var(--panel); border: 1px solid var(--border); border-radius: 8px;
    padding: 10px 14px; margin-bottom: 16px;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 11.5px;
  }}
  .manifest div span {{ display: block; color: var(--muted); font-size: 10px; text-transform: uppercase; }}
  .kpi-grid {{
    display: grid; grid-template-columns: repeat(4, 1fr); gap: 10px; margin-bottom: 18px;
  }}
  .kpi {{
    background: var(--panel); border: 1px solid var(--border); border-radius: 8px;
    padding: 10px 12px; break-inside: avoid;
  }}
  .kpi .lbl {{ color: var(--muted); font-size: 10.5px; text-transform: uppercase; letter-spacing: 0.04em; }}
  .kpi .val {{ font-size: 17px; font-weight: 700; margin-top: 3px; font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }}
  .kpi .hint {{ color: var(--muted); font-size: 11px; margin-top: 2px; font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }}
  .pos {{ color: var(--bull); }}
  .neg {{ color: var(--bear); }}
  .neu {{ color: var(--text); }}
  .warn-banner {{
    background: rgba(255, 183, 77, 0.12); border: 1px solid rgba(255, 183, 77, 0.5);
    color: var(--warn); border-radius: 6px; padding: 8px 12px; margin-bottom: 14px; font-size: 12px;
  }}
  .card {{
    background: var(--panel); border: 1px solid var(--border); border-radius: 8px;
    padding: 14px 16px; margin-bottom: 16px; break-inside: avoid;
  }}
  .card h2 {{
    margin: 0 0 10px 0; font-size: 13px; text-transform: uppercase; letter-spacing: 0.06em; color: var(--muted);
  }}
  .chart-svg {{ width: 100%; height: auto; display: block; }}
  .grid-line {{ stroke: #202838; stroke-width: 1; }}
  .baseline {{ stroke: #48546B; stroke-width: 1; stroke-dasharray: 4 4; }}
  .bnh-line {{ stroke: #7C8CA8; stroke-width: 1.5; stroke-dasharray: 5 4; }}
  .axis-lbl {{ fill: #8B97AD; font-size: 10px; font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }}
  .legend-lbl {{ fill: #CBD5E1; font-size: 10.5px; font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }}
  .trough-lbl {{ fill: #EF5350; font-size: 10.5px; font-weight: 700; font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }}
  table {{
    width: 100%; border-collapse: collapse; font-size: 11.5px;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }}
  th, td {{ padding: 6px 8px; border-bottom: 1px solid var(--border); text-align: right; }}
  th {{ color: var(--muted); font-weight: 600; font-size: 10.5px; text-transform: uppercase; background: var(--panel2); }}
  th:first-child, td:first-child, th.left, td.left {{ text-align: left; }}
  tr {{ break-inside: avoid; }}
  .badge {{
    display: inline-block; padding: 2px 7px; border-radius: 4px; font-size: 10.5px; font-weight: 700;
    background: var(--panel2); border: 1px solid var(--border);
  }}
  footer {{
    margin-top: 20px; padding-top: 10px; border-top: 1px solid var(--border);
    color: var(--muted); font-size: 11px; display: flex; justify-content: space-between;
  }}
  @page {{ size: A4 portrait; margin: 12mm; }}
  @media print {{
    :root {{
      --bg: #FFFFFF; --panel: #FFFFFF; --panel2: #F3F5F8;
      --border: #D5DCE6; --text: #111827; --muted: #4B5563;
      --bull: #0F766E; --bear: #B91C1C; --accent: #1D4ED8; --warn: #B45309;
    }}
    body {{ padding: 0; font-size: 11px; }}
    .no-print {{ display: none !important; }}
    .grid-line {{ stroke: #E5E7EB; }}
    .axis-lbl, .legend-lbl {{ fill: #374151; }}
    .card, .kpi, .manifest {{ box-shadow: none; }}
  }}
</style>
</head>
<body>
<div class="wrap">
  <header>
    <div>
      <div class="brand">SUMI TRADING WORKSTATION · v1.1</div>
      <h1>{sym_esc} · {strat_esc}</h1>
      <div class="sub">{venue_esc} · {tf_esc} · {start_date} → {end_date} ({} bars) · Params: {param_esc}</div>
    </div>
    <button class="print-btn no-print" onclick="window.print()">Print / Save PDF</button>
  </header>

  <section class="manifest">
    <div><span>Initial Capital</span>{}</div>
    <div><span>Final Equity</span>{}</div>
    <div><span>Direction</span>{:?}</div>
    <div><span>Comm / Tax / Slip</span>{:.1} / {:.1} / {:.1} bps</div>
    <div><span>Manifest Digest</span><code>{:016x}</code></div>
  </section>
"#,
        result.bar_count,
        escape_html(&fmt_money(cfg.initial_cash, is_vn, false)),
        escape_html(&fmt_money(ts.final_equity, is_vn, false)),
        cfg.direction_mode,
        cfg.commission_bps,
        cfg.tax_sell_bps,
        cfg.slippage_bps,
        manifest.manifest_digest,
    );

    if ts.low_sample_warning {
        let _ = writeln!(
            html,
            "  <div class=\"warn-banner\">⚠ Low Statistical Sample: {} closed trades (&lt; {MIN_STATISTICAL_SAMPLE_TRADES}). Interpret ratios cautiously.</div>",
            ts.num_trades
        );
    }

    // KPI Grid
    let pf_str = if let Some(pf) = ts.profit_factor {
        format!("{pf:.2}")
    } else if ts.num_winners > 0 && ts.num_losers == 0 {
        "∞".to_owned()
    } else {
        "—".to_owned()
    };
    let verdict_html = result.robustness.map_or_else(String::new, |v| {
        format!(" · <span class=\"badge\">{}</span>", escape_html(v.label()))
    });

    let _ = write!(
        html,
        r#"  <section class="kpi-grid">
    <div class="kpi"><div class="lbl">Total Return</div><div class="val {}">{:+.2}%</div><div class="hint">{}</div></div>
    <div class="kpi"><div class="lbl">CAGR</div><div class="val {}">{}</div><div class="hint">Annualized</div></div>
    <div class="kpi"><div class="lbl">Sharpe / Sortino</div><div class="val neu">{} / {}</div><div class="hint">Calmar {}</div></div>
    <div class="kpi"><div class="lbl">Max Drawdown</div><div class="val neg">-{:.2}%</div><div class="hint">{} ({} bars)</div></div>
    <div class="kpi"><div class="lbl">Win Rate</div><div class="val neu">{:.2}%</div><div class="hint">{}W · {}L · {}BE</div></div>
    <div class="kpi"><div class="lbl">Profit Factor</div><div class="val neu">{}</div><div class="hint">Exp {}</div></div>
    <div class="kpi"><div class="lbl">Avg Trade / R</div><div class="val {}">{:+.2}%</div><div class="hint">Avg R: {} · {:.1}b</div></div>
    <div class="kpi"><div class="lbl">Trades / Fees</div><div class="val neu">{}</div><div class="hint">Exp {:.1}% · Fees {}{}</div></div>
  </section>
"#,
        value_tone_class(ts.total_return_pct),
        ts.total_return_pct,
        escape_html(&fmt_money(ts.total_return_abs, is_vn, true)),
        ts.cagr_pct.map_or("neu", value_tone_class),
        escape_html(&fmt_opt_f64(ts.cagr_pct, 2, "%", true)),
        escape_html(&fmt_opt_f64(ts.sharpe_ratio, 2, "", false)),
        escape_html(&fmt_opt_f64(ts.sortino_ratio, 2, "", false)),
        escape_html(&fmt_opt_f64(ts.calmar_ratio, 2, "", false)),
        ts.max_drawdown_pct,
        escape_html(&fmt_money(-ts.max_drawdown_abs, is_vn, true)),
        ts.max_drawdown_duration_bars,
        ts.win_rate_pct,
        ts.num_winners,
        ts.num_losers,
        ts.num_breakeven,
        escape_html(&pf_str),
        escape_html(
            &ts.expectancy_net
                .map_or_else(|| "—".to_owned(), |e| fmt_money(e, is_vn, true))
        ),
        value_tone_class(ts.avg_trade_return_pct),
        ts.avg_trade_return_pct,
        escape_html(&fmt_opt_f64(ts.avg_r_multiple, 2, "R", true)),
        ts.avg_bars_held,
        ts.num_trades,
        ts.exposure_pct,
        escape_html(&fmt_money(ts.total_fees_and_tax, is_vn, false)),
        verdict_html,
    );

    // Inline SVG Charts
    let _ = write!(
        html,
        r#"  <section class="card">
    <h2>Cumulative Equity Curve (Strategy vs Buy &amp; Hold)</h2>
    {eq_svg}
  </section>
  <section class="card">
    <h2>Underwater Drawdown (%)</h2>
    {uw_svg}
  </section>
"#
    );

    // In-Sample vs Out-of-Sample Split table when available
    if let (Some(is_ts), Some(oos_ts)) =
        (&result.in_sample_tearsheet, &result.out_of_sample_tearsheet)
    {
        let _ = write!(
            html,
            r#"  <section class="card">
    <h2>In-Sample vs Out-of-Sample Split</h2>
    <table>
      <thead><tr><th class="left">Window</th><th>Return %</th><th>Max DD %</th><th>Sharpe</th><th>Trades</th><th>Win Rate</th><th>Profit Factor</th></tr></thead>
      <tbody>
        <tr><td class="left">In-Sample (IS)</td><td class="{}">{:+.2}%</td><td class="neg">-{:.2}%</td><td>{}</td><td>{}</td><td>{:.1}%</td><td>{}</td></tr>
        <tr><td class="left">Out-of-Sample (OOS)</td><td class="{}">{:+.2}%</td><td class="neg">-{:.2}%</td><td>{}</td><td>{}</td><td>{:.1}%</td><td>{}</td></tr>
      </tbody>
    </table>
  </section>
"#,
            value_tone_class(is_ts.total_return_pct),
            is_ts.total_return_pct,
            is_ts.max_drawdown_pct,
            escape_html(&fmt_opt_f64(is_ts.sharpe_ratio, 2, "", false)),
            is_ts.num_trades,
            is_ts.win_rate_pct,
            escape_html(&fmt_opt_f64(is_ts.profit_factor, 2, "", false)),
            value_tone_class(oos_ts.total_return_pct),
            oos_ts.total_return_pct,
            oos_ts.max_drawdown_pct,
            escape_html(&fmt_opt_f64(oos_ts.sharpe_ratio, 2, "", false)),
            oos_ts.num_trades,
            oos_ts.win_rate_pct,
            escape_html(&fmt_opt_f64(oos_ts.profit_factor, 2, "", false)),
        );
    }

    // Advanced Tearsheet Analytics tables (Outlier, Drawdowns, Yearly, Regime, Multi-Phase)
    if let Some(adv) = analytics {
        let oi = &adv.outlier_impact;
        let _ = write!(
            html,
            r#"  <section class="card">
    <h2>Outlier Dependency &amp; Tail Robustness</h2>
    {}
    <table>
      <thead><tr><th class="left">Metric</th><th>Value</th><th class="left">Metric</th><th>Value</th></tr></thead>
      <tbody>
        <tr>
          <td class="left">Full Expectancy</td><td>{}</td>
          <td class="left">Top-3 Winners Share</td><td>{}</td>
        </tr>
        <tr>
          <td class="left">Net PnL ex Top-3 Winners</td><td class="{}">{}</td>
          <td class="left">Trimmed 10% Expectancy</td><td>{}</td>
        </tr>
      </tbody>
    </table>
  </section>
"#,
            if oi.outlier_dependent_warning {
                "<div class=\"warn-banner\">⚠ Strategy net profit turns non-positive when excluding top 3 winning trades.</div>"
            } else {
                ""
            },
            escape_html(
                &oi.full_expectancy
                    .map_or_else(|| "—".to_owned(), |v| fmt_money(v, is_vn, true))
            ),
            escape_html(&fmt_opt_f64(
                oi.top_3_winners_share_of_gross_profit_pct,
                1,
                "%",
                false
            )),
            value_tone_class(oi.net_pnl_ex_top_3_winners),
            escape_html(&fmt_money(oi.net_pnl_ex_top_3_winners, is_vn, true)),
            escape_html(
                &oi.trimmed_expectancy_10pct
                    .map_or_else(|| "—".to_owned(), |v| fmt_money(v, is_vn, true))
            ),
        );

        if !adv.drawdown_periods.is_empty() {
            html.push_str(
                "  <section class=\"card\">\n    <h2>Top Drawdown Periods</h2>\n    <table>\n      <thead><tr><th class=\"left\">#</th><th class=\"left\">Peak</th><th class=\"left\">Trough</th><th class=\"left\">Recovery</th><th>Max DD %</th><th>Duration</th><th>Recovery</th></tr></thead>\n      <tbody>\n",
            );
            for (idx, dd) in adv.drawdown_periods.iter().take(10).enumerate() {
                let rec_date = dd
                    .recovery_timestamp
                    .map_or_else(|| "Active".to_owned(), format_utc_date);
                let rec_bars = dd
                    .recovery_bars
                    .map_or_else(|| "—".to_owned(), |b| format!("{b}b"));
                let _ = writeln!(
                    html,
                    "        <tr><td class=\"left\">{}</td><td class=\"left\">{}</td><td class=\"left\">{}</td><td class=\"left\">{}</td><td class=\"neg\">-{:.2}%</td><td>{}b</td><td>{}</td></tr>",
                    idx + 1,
                    escape_html(&format_utc_date(dd.peak_timestamp)),
                    escape_html(&format_utc_date(dd.trough_timestamp)),
                    escape_html(&rec_date),
                    dd.max_drawdown_pct,
                    dd.duration_bars,
                    escape_html(&rec_bars),
                );
            }
            html.push_str("      </tbody>\n    </table>\n  </section>\n");
        }

        if !adv.yearly_slices.is_empty() {
            html.push_str(
                "  <section class=\"card\">\n    <h2>Calendar Year Attribution (UTC)</h2>\n    <table>\n      <thead><tr><th class=\"left\">Year</th><th>Bars</th><th>Return %</th><th>Max DD %</th><th>Trades</th><th>Win %</th><th>Net PnL</th><th>PF</th><th>Avg R</th></tr></thead>\n      <tbody>\n",
            );
            for y in &adv.yearly_slices {
                let _ = writeln!(
                    html,
                    "        <tr><td class=\"left\">{}</td><td>{}</td><td class=\"{}\">{:+.2}%</td><td class=\"neg\">-{:.2}%</td><td>{}</td><td>{:.1}%</td><td class=\"{}\">{}</td><td>{}</td><td>{}</td></tr>",
                    y.year,
                    y.bar_count,
                    value_tone_class(y.equity_return_pct),
                    y.equity_return_pct,
                    y.max_drawdown_pct,
                    y.trade_count,
                    y.win_rate_pct,
                    value_tone_class(y.realized_net_pnl),
                    escape_html(&fmt_money(y.realized_net_pnl, is_vn, true)),
                    escape_html(&fmt_opt_f64(y.profit_factor, 2, "", false)),
                    escape_html(&fmt_opt_f64(y.avg_r_multiple, 2, "R", true)),
                );
            }
            html.push_str("      </tbody>\n    </table>\n  </section>\n");
        }

        if !adv.entry_regime_slices.is_empty() {
            html.push_str(
                "  <section class=\"card\">\n    <h2>Causal Signal-Bar Market Regime Attribution</h2>\n    <table>\n      <thead><tr><th class=\"left\">Regime</th><th>Bars</th><th>Trades</th><th>Win %</th><th>Net PnL</th><th>Avg Ret %</th><th>PF</th><th>Avg R</th></tr></thead>\n      <tbody>\n",
            );
            for reg in &adv.entry_regime_slices {
                let _ = writeln!(
                    html,
                    "        <tr><td class=\"left\">{}</td><td>{}</td><td>{}</td><td>{:.1}%</td><td class=\"{}\">{}</td><td class=\"{}\">{:+.2}%</td><td>{}</td><td>{}</td></tr>",
                    escape_html(reg.regime.short_label()),
                    reg.bar_count,
                    reg.trade_count,
                    reg.win_rate_pct,
                    value_tone_class(reg.net_pnl),
                    escape_html(&fmt_money(reg.net_pnl, is_vn, true)),
                    value_tone_class(reg.avg_return_pct),
                    reg.avg_return_pct,
                    escape_html(&fmt_opt_f64(reg.profit_factor, 2, "", false)),
                    escape_html(&fmt_opt_f64(reg.avg_r_multiple, 2, "R", true)),
                );
            }
            html.push_str("      </tbody>\n    </table>\n  </section>\n");
        }

        if !adv.multi_phase.phases.is_empty() {
            let mp = &adv.multi_phase;
            let _ = writeln!(
                html,
                "  <section class=\"card\">\n    <h2>Multi-Phase Batch ({}) — Consistency {:.1}% · Dispersion {:.2}% · Late-Early {:+.2}%</h2>\n    <table>\n      <thead><tr><th class=\"left\">Phase</th><th class=\"left\">Window</th><th>Return %</th><th>Max DD %</th><th>Trades</th><th>Win %</th><th>PF</th></tr></thead>\n      <tbody>",
                escape_html(mp.mode.short_label()),
                mp.consistency_pct,
                mp.return_dispersion_pct,
                mp.late_vs_early_return_delta_pct,
            );
            for ph in &mp.phases {
                let win_lbl = format!(
                    "{} → {}",
                    format_utc_date(ph.start_timestamp),
                    format_utc_date(ph.end_timestamp)
                );
                let _ = writeln!(
                    html,
                    "        <tr><td class=\"left\">{}</td><td class=\"left\">{}</td><td class=\"{}\">{:+.2}%</td><td class=\"neg\">-{:.2}%</td><td>{}</td><td>{:.1}%</td><td>{}</td></tr>",
                    escape_html(&ph.label),
                    escape_html(&win_lbl),
                    value_tone_class(ph.tearsheet.total_return_pct),
                    ph.tearsheet.total_return_pct,
                    ph.tearsheet.max_drawdown_pct,
                    ph.tearsheet.num_trades,
                    ph.tearsheet.win_rate_pct,
                    escape_html(&fmt_opt_f64(ph.tearsheet.profit_factor, 2, "", false)),
                );
            }
            html.push_str("      </tbody>\n    </table>\n  </section>\n");
        }
    }

    // Trade Log
    let shown_trades = result.trades.len().min(MAX_HTML_TRADE_ROWS);
    let _ = writeln!(
        html,
        "  <section class=\"card\">\n    <h2>Executed Trade Log ({}{} trades)</h2>",
        shown_trades,
        if result.trades.len() > MAX_HTML_TRADE_ROWS {
            format!(" of {}", result.trades.len())
        } else {
            String::new()
        }
    );
    if result.trades.is_empty() {
        html.push_str("    <div class=\"sub\">No closed trades executed in this window.</div>\n");
    } else {
        html.push_str(
            "    <table>\n      <thead><tr><th class=\"left\">#</th><th class=\"left\">Side</th><th class=\"left\">Entry</th><th class=\"left\">Exit</th><th>Qty</th><th>Bars</th><th>Net PnL</th><th>Ret %</th><th>R</th><th>MAE %</th><th>MFE %</th><th class=\"left\">Reason</th></tr></thead>\n      <tbody>\n",
        );
        for tr in result.trades.iter().take(MAX_HTML_TRADE_ROWS) {
            let side_s = match tr.side {
                PositionSide::Long => "LONG",
                PositionSide::Short => "SHORT",
            };
            let entry_s = format!(
                "{} @ {:.2}",
                format_utc_date(tr.entry_timestamp),
                tr.entry_price
            );
            let exit_s = format!(
                "{} @ {:.2}",
                format_utc_date(tr.exit_timestamp),
                tr.exit_price
            );
            let _ = writeln!(
                html,
                "        <tr><td class=\"left\">{}</td><td class=\"left\">{}</td><td class=\"left\">{}</td><td class=\"left\">{}</td><td>{:.2}</td><td>{}</td><td class=\"{}\">{}</td><td class=\"{}\">{:+.2}%</td><td>{}</td><td class=\"neg\">{:.2}%</td><td class=\"pos\">+{:.2}%</td><td class=\"left\">{}</td></tr>",
                tr.trade_id,
                side_s,
                escape_html(&entry_s),
                escape_html(&exit_s),
                tr.quantity,
                tr.bars_held,
                value_tone_class(tr.net_pnl),
                escape_html(&fmt_money(tr.net_pnl, is_vn, true)),
                value_tone_class(tr.return_pct),
                tr.return_pct,
                escape_html(&fmt_opt_f64(tr.r_multiple, 2, "R", true)),
                tr.mae_pct,
                tr.mfe_pct,
                escape_html(tr.exit_reason.short_label()),
            );
        }
        html.push_str("      </tbody>\n    </table>\n");
    }
    html.push_str("  </section>\n");

    let _ = write!(
        html,
        r#"  <footer>
    <span>Generated deterministically offline by Sumi Trading Workstation (Zero-Network Rust Engine)</span>
    <span>Causal Signal[T] → Open[T+1] · Digest {:016x}</span>
  </footer>
</div>
</body>
</html>
"#,
        manifest.manifest_digest
    );

    html
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backtest::{
        BacktestConfig, BacktestRunOptions, MultiPhaseSplitMode, StrategyPresetKind,
        compute_advanced_tearsheet_analytics, run_backtest_with_options,
    };
    use crate::data::{Timeframe, generate_synthetic_ohlcv};

    #[test]
    fn v1_1_wave4_html_report_deterministic_svg_kpis_and_xss_safe() {
        let candles = generate_synthetic_ohlcv(260);
        let mut cfg = BacktestConfig {
            preset: StrategyPresetKind::ALL[0],
            ..Default::default()
        };
        cfg.custom_rules.name = "<script>alert('xss')</script> & =CMD|'calc'".to_owned();
        let res = run_backtest_with_options(
            "<img src=x onerror=alert(1)>",
            Timeframe::D1,
            &candles,
            &cfg,
            BacktestRunOptions::default(),
        );
        let adv = compute_advanced_tearsheet_analytics(
            &candles,
            &res,
            MultiPhaseSplitMode::Tertiles,
            BacktestRunOptions::default(),
        );

        let html1 = render_backtest_report_html(&res, &candles, ExchangeVenue::Hose, Some(&adv));
        let html2 = render_backtest_report_html(&res, &candles, ExchangeVenue::Hose, Some(&adv));
        assert_eq!(
            html1, html2,
            "HTML report generator must be 100% deterministic"
        );

        // Verify structure, SVG charts, KPIs, print CSS, and manifest digest
        assert!(html1.starts_with("<!DOCTYPE html>"));
        assert!(html1.contains("<svg class=\"chart-svg\""));
        assert!(html1.contains("aria-label=\"Equity Curve vs Buy and Hold\""));
        assert!(html1.contains("aria-label=\"Underwater Drawdown Percentage\""));
        assert!(html1.contains("@media print"));
        assert!(html1.contains(&format!("{:016x}", adv.manifest.manifest_digest)));
        assert!(html1.contains(&format!("{:+.2}%", res.tearsheet.total_return_pct)));
        assert!(html1.contains("Calendar Year Attribution"));
        assert!(html1.contains("Causal Signal-Bar Market Regime Attribution"));
        assert!(html1.contains("Executed Trade Log"));

        // Verify strict XSS escaping (no unescaped `<img` or `<script>` injected!)
        assert!(!html1.contains("<img src=x"));
        assert!(!html1.contains("<script>"));
        assert!(html1.contains("&lt;img src=x onerror=alert(1)&gt;"));
    }
}
