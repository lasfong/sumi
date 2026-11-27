//! Safe Arithmetic Expression Parser & Synthetic OHLCV Ratio/Spread Engine (Wave 4).
//!
//! Enables TradingView-grade synthetic spread, ratio, and basket formulas:
//! - `HOSE:SSI / HOSE:VNINDEX` (Relative Strength candlestick chart supporting Trendlines / Ichimoku / AVWAP)
//! - `HNX:VN30F1! - HOSE:VN30` (Live VN30 Futures Basis spread)
//! - `(HOSE:HPG + HOSE:HSG) / 2` (Equal-weight sector basket)
//!
//! Strictly pure, deterministic, 100% offline & Bar Replay compatible (zero future lookahead).

use std::collections::{BTreeSet, HashMap};

use crate::error::{DataError, Result};
use crate::types::{OhlcvBar, SymbolKey, infer_vn_exchange_for_ticker};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprNode {
    Number(f64),
    Symbol(SymbolKey),
    Neg(Box<ExprNode>),
    Binary {
        op: BinOp,
        left: Box<ExprNode>,
        right: Box<ExprNode>,
    },
}

#[derive(Debug, Clone)]
pub struct ParsedExpression {
    pub raw: String,
    pub canonical: String,
    pub ast: ExprNode,
    pub symbols: Vec<SymbolKey>,
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Symbol(SymbolKey),
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
}

/// Returns `true` when `raw` looks like a synthetic arithmetic formula rather than a plain symbol ticker.
///
/// Avoids false positives on plain single tickers such as `HOSE:FPT`, `HNX:VN30F1!`, or `NASDAQ:BRK-B`.
#[must_use]
pub fn is_synthetic_expression(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.contains('/')
        || trimmed.contains('*')
        || trimmed.contains('+')
        || trimmed.contains('(')
        || trimmed.contains(')')
    {
        return true;
    }
    // Subtraction requires whitespace around `-` (e.g. `HNX:VN30F1! - HOSE:VN30`) so hyphenated
    // tickers like `NYSE:BRK-B` remain plain symbols.
    if trimmed.contains(" - ") || trimmed.contains(" -") || trimmed.contains("- ") {
        return true;
    }
    // Also detect multiple `EXCHANGE:SYMBOL` tokens separated by `-` (e.g. `HNX:VN30F1!-HOSE:VN30`).
    trimmed.matches(':').count() >= 2 && trimmed.contains('-')
}

fn tokenize_expression(raw: &str) -> Result<Vec<Token>> {
    let bytes = raw.as_bytes();
    let n = bytes.len();
    let mut i = 0_usize;
    let mut tokens = Vec::new();

    while i < n {
        let ch = bytes[i] as char;
        if ch.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        match ch {
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '-' => {
                tokens.push(Token::Minus);
                i += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                i += 1;
            }
            '/' => {
                tokens.push(Token::Slash);
                i += 1;
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            c if c.is_ascii_digit() || c == '.' => {
                let start = i;
                let mut seen_dot = c == '.';
                i += 1;
                while i < n {
                    let c2 = bytes[i] as char;
                    if c2.is_ascii_digit() {
                        i += 1;
                    } else if c2 == '.' && !seen_dot {
                        seen_dot = true;
                        i += 1;
                    } else {
                        break;
                    }
                }
                let slice = &raw[start..i];
                let val: f64 = slice.parse().map_err(|_| {
                    DataError::InvalidSymbol(format!("Số không hợp lệ trong công thức: {slice}"))
                })?;
                if !val.is_finite() {
                    return Err(DataError::InvalidSymbol(format!(
                        "Hằng số không hữu hạn trong công thức: {slice}"
                    )));
                }
                tokens.push(Token::Number(val));
            }
            c if c.is_ascii_alphabetic() => {
                let start = i;
                i += 1;
                while i < n {
                    let c2 = bytes[i] as char;
                    if c2.is_ascii_alphanumeric() || matches!(c2, ':' | '!' | '_' | '.') {
                        i += 1;
                    } else {
                        break;
                    }
                }
                let ident = &raw[start..i];
                let key = if ident.contains(':') {
                    SymbolKey::parse(ident)?
                } else {
                    let sym_up = ident.to_ascii_uppercase();
                    let ex = infer_vn_exchange_for_ticker(&sym_up, false);
                    SymbolKey::new(ex, &sym_up)?
                };
                tokens.push(Token::Symbol(key));
            }
            other => {
                return Err(DataError::InvalidSymbol(format!(
                    "Ký tự không hợp lệ '{other}' trong công thức biểu đồ"
                )));
            }
        }
    }

    if tokens.is_empty() {
        return Err(DataError::InvalidSymbol("Công thức biểu đồ trống".to_owned()));
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    depth: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(tok)
        } else {
            None
        }
    }

    fn parse_add_sub(&mut self) -> Result<ExprNode> {
        let mut left = self.parse_mul_div()?;
        while let Some(tok) = self.peek() {
            let op = match tok {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.next();
            let right = self.parse_mul_div()?;
            left = ExprNode::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_mul_div(&mut self) -> Result<ExprNode> {
        let mut left = self.parse_unary()?;
        while let Some(tok) = self.peek() {
            let op = match tok {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                _ => break,
            };
            self.next();
            let right = self.parse_unary()?;
            left = ExprNode::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<ExprNode> {
        if matches!(self.peek(), Some(Token::Minus)) {
            self.next();
            let inner = self.parse_unary()?;
            return Ok(ExprNode::Neg(Box::new(inner)));
        }
        if matches!(self.peek(), Some(Token::Plus)) {
            self.next();
            return self.parse_unary();
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<ExprNode> {
        if self.depth > 32 {
            return Err(DataError::InvalidSymbol(
                "Công thức lồng ngoặc quá sâu (> 32 cấp)".to_owned(),
            ));
        }
        match self.next() {
            Some(Token::Number(v)) => Ok(ExprNode::Number(v)),
            Some(Token::Symbol(k)) => Ok(ExprNode::Symbol(k)),
            Some(Token::LParen) => {
                self.depth += 1;
                let inner = self.parse_add_sub()?;
                self.depth -= 1;
                if !matches!(self.next(), Some(Token::RParen)) {
                    return Err(DataError::InvalidSymbol(
                        "Thiếu dấu đóng ngoặc ')' trong công thức".to_owned(),
                    ));
                }
                Ok(inner)
            }
            _ => Err(DataError::InvalidSymbol(
                "Cú pháp công thức không hợp lệ (VD: HOSE:SSI / HOSE:VNINDEX)".to_owned(),
            )),
        }
    }
}

fn collect_symbols(node: &ExprNode, out: &mut Vec<SymbolKey>) {
    match node {
        ExprNode::Number(_) => {}
        ExprNode::Symbol(k) => {
            if !out.contains(k) {
                out.push(k.clone());
            }
        }
        ExprNode::Neg(inner) => collect_symbols(inner, out),
        ExprNode::Binary { left, right, .. } => {
            collect_symbols(left, out);
            collect_symbols(right, out);
        }
    }
}

fn format_canonical(node: &ExprNode) -> String {
    match node {
        ExprNode::Number(v) => {
            if (v.fract()).abs() < 1e-9 {
                format!("{v:.0}")
            } else {
                format!("{v}")
            }
        }
        ExprNode::Symbol(k) => k.ticker(),
        ExprNode::Neg(inner) => format!("-{}", format_canonical(inner)),
        ExprNode::Binary { op, left, right } => {
            let sym = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
            };
            format!("{} {sym} {}", format_canonical(left), format_canonical(right))
        }
    }
}

/// Parses a safe arithmetic market expression into an AST + list of unique referenced symbols.
pub fn parse_expression(raw: &str) -> Result<ParsedExpression> {
    let tokens = tokenize_expression(raw)?;
    let mut parser = Parser {
        tokens,
        pos: 0,
        depth: 0,
    };
    let ast = parser.parse_add_sub()?;
    if parser.pos < parser.tokens.len() {
        return Err(DataError::InvalidSymbol(format!(
            "Dư ký tự không hợp lệ trong công thức: {raw}"
        )));
    }
    let mut symbols = Vec::new();
    collect_symbols(&ast, &mut symbols);
    if symbols.is_empty() {
        return Err(DataError::InvalidSymbol(
            "Công thức phải chứa ít nhất 1 mã chứng khoán".to_owned(),
        ));
    }
    if symbols.len() > 8 {
        return Err(DataError::InvalidSymbol(
            "Công thức tối đa 8 mã thành phần".to_owned(),
        ));
    }
    let canonical = format_canonical(&ast);
    Ok(ParsedExpression {
        raw: raw.trim().to_owned(),
        canonical,
        ast,
        symbols,
    })
}

fn eval_scalar(node: &ExprNode, vals: &HashMap<&SymbolKey, f64>) -> Option<f64> {
    match node {
        ExprNode::Number(v) => Some(*v),
        ExprNode::Symbol(k) => vals.get(k).copied().filter(|v| v.is_finite()),
        ExprNode::Neg(inner) => eval_scalar(inner, vals).map(|v| -v),
        ExprNode::Binary { op, left, right } => {
            let l = eval_scalar(left, vals)?;
            let r = eval_scalar(right, vals)?;
            let res = match op {
                BinOp::Add => l + r,
                BinOp::Sub => l - r,
                BinOp::Mul => l * r,
                BinOp::Div => {
                    if r.abs() <= 1e-12 {
                        return None;
                    }
                    l / r
                }
            };
            res.is_finite().then_some(res)
        }
    }
}

/// Evaluates `parsed` across aligned timestamps of all referenced symbols, synthesizing full
/// valid OHLCV bars (`low <= min(open, close) <= max(open, close) <= high`) so all indicators,
/// Trendlines, Ichimoku, AVWAP, and Bar Replay work seamlessly on ratio/spread charts!
#[must_use]
pub fn evaluate_expression_ohlcv(
    parsed: &ParsedExpression,
    series_map: &HashMap<SymbolKey, Vec<OhlcvBar>>,
) -> Vec<OhlcvBar> {
    if parsed.symbols.is_empty() {
        return Vec::new();
    }

    // Build per-symbol timestamp lookup maps & intersect timestamps across all referenced legs.
    let mut maps: Vec<(&SymbolKey, HashMap<i64, OhlcvBar>)> =
        Vec::with_capacity(parsed.symbols.len());
    let mut common_ts: Option<BTreeSet<i64>> = None;
    for sym in &parsed.symbols {
        let Some(bars) = series_map.get(sym) else {
            return Vec::new();
        };
        if bars.is_empty() {
            return Vec::new();
        }
        let mut ts_set = BTreeSet::new();
        let mut by_ts = HashMap::with_capacity(bars.len());
        for &b in bars {
            ts_set.insert(b.timestamp);
            by_ts.insert(b.timestamp, b);
        }
        common_ts = Some(match common_ts {
            None => ts_set,
            Some(prev) => prev.intersection(&ts_set).copied().collect(),
        });
        maps.push((sym, by_ts));
    }

    let Some(timestamps) = common_ts else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(timestamps.len());
    let mut opens: HashMap<&SymbolKey, f64> = HashMap::with_capacity(maps.len());
    let mut highs: HashMap<&SymbolKey, f64> = HashMap::with_capacity(maps.len());
    let mut lows: HashMap<&SymbolKey, f64> = HashMap::with_capacity(maps.len());
    let mut closes: HashMap<&SymbolKey, f64> = HashMap::with_capacity(maps.len());

    for ts in timestamps {
        opens.clear();
        highs.clear();
        lows.clear();
        closes.clear();
        let mut vol_sum = 0.0_f64;
        for (sym, map) in &maps {
            if let Some(b) = map.get(&ts) {
                opens.insert(*sym, b.open);
                highs.insert(*sym, b.high);
                lows.insert(*sym, b.low);
                closes.insert(*sym, b.close);
                vol_sum += b.volume.max(0.0);
            }
        }
        let (Some(o), Some(c)) = (
            eval_scalar(&parsed.ast, &opens),
            eval_scalar(&parsed.ast, &closes),
        ) else {
            continue;
        };
        let h_eval = eval_scalar(&parsed.ast, &highs).unwrap_or(o.max(c));
        let l_eval = eval_scalar(&parsed.ast, &lows).unwrap_or(o.min(c));
        let high = o.max(c).max(h_eval).max(l_eval);
        let low = o.min(c).min(h_eval).min(l_eval);
        let avg_vol = vol_sum / (maps.len().max(1) as f64);
        out.push(OhlcvBar::new(ts, o, high, low, c, avg_vol));
    }
    out
}

/// Computes a strictly causal `0.00%`-rebased comparison curve aligned onto `base_bars` timestamps.
/// Uses carry-forward on the latest causal bar `<= base_ts` (zero future lookahead).
#[must_use]
pub fn compute_rebased_comparison_series(
    base_bars: &[OhlcvBar],
    compare_bars: &[OhlcvBar],
) -> Vec<(i64, f64)> {
    if base_bars.is_empty() || compare_bars.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(base_bars.len());
    let mut j = 0_usize;
    let mut anchor: Option<f64> = None;

    for b in base_bars {
        while j + 1 < compare_bars.len() && compare_bars[j + 1].timestamp <= b.timestamp {
            j += 1;
        }
        if compare_bars[j].timestamp > b.timestamp {
            continue;
        }
        let c = compare_bars[j].close;
        if !c.is_finite() || c <= 0.0 {
            continue;
        }
        let base_c = *anchor.get_or_insert(c);
        let pct = ((c - base_c) / base_c) * 100.0;
        if pct.is_finite() {
            out.push((b.timestamp, pct));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_bar(ts: i64, o: f64, h: f64, l: f64, c: f64, v: f64) -> OhlcvBar {
        OhlcvBar::new(ts, o, h, l, c, v)
    }

    #[test]
    fn detects_synthetic_expressions_without_false_positives() {
        assert!(is_synthetic_expression("HOSE:SSI / HOSE:VNINDEX"));
        assert!(is_synthetic_expression("HNX:VN30F1! - HOSE:VN30"));
        assert!(is_synthetic_expression("(HOSE:HPG + HOSE:HSG) / 2"));
        assert!(!is_synthetic_expression("HOSE:FPT"));
        assert!(!is_synthetic_expression("HNX:VN30F1!"));
        assert!(!is_synthetic_expression("NYSE:BRK-B"));
    }

    #[test]
    fn evaluates_ratio_spread_and_basket_preserving_ohlc_invariants() {
        let ssi = SymbolKey::new("HOSE", "SSI").unwrap();
        let vni = SymbolKey::new("HOSE", "VNINDEX").unwrap();
        let mut map = HashMap::new();
        map.insert(
            ssi.clone(),
            vec![
                mk_bar(1_000, 26_000.0, 27_000.0, 25_500.0, 26_800.0, 10_000_000.0),
                mk_bar(2_000, 26_800.0, 28_000.0, 26_400.0, 27_500.0, 12_000_000.0),
            ],
        );
        map.insert(
            vni.clone(),
            vec![
                mk_bar(1_000, 1_250.0, 1_260.0, 1_240.0, 1_255.0, 800_000_000.0),
                mk_bar(2_000, 1_255.0, 1_270.0, 1_250.0, 1_265.0, 900_000_000.0),
            ],
        );

        let ratio = parse_expression("HOSE:SSI / HOSE:VNINDEX").unwrap();
        assert_eq!(ratio.symbols.len(), 2);
        let bars = evaluate_expression_ohlcv(&ratio, &map);
        assert_eq!(bars.len(), 2);
        assert!((bars[0].close - (26_800.0 / 1_255.0)).abs() < 1e-9);
        assert!(bars.iter().all(OhlcvBar::is_valid));

        let spread = parse_expression("HOSE:SSI - HOSE:VNINDEX * 20").unwrap();
        let sbars = evaluate_expression_ohlcv(&spread, &map);
        assert_eq!(sbars.len(), 2);
        assert!((sbars[0].close - (26_800.0 - 1_255.0 * 20.0)).abs() < 1e-9);

        let basket = parse_expression("(HOSE:SSI + HOSE:VNINDEX) / 2").unwrap();
        let bbars = evaluate_expression_ohlcv(&basket, &map);
        assert!((bbars[1].close - ((27_500.0 + 1_265.0) / 2.0)).abs() < 1e-9);
    }

    #[test]
    fn rebased_comparison_starts_at_zero_and_never_leaks_future_bars() {
        let base = vec![
            mk_bar(1_000, 100.0, 102.0, 99.0, 100.0, 1000.0),
            mk_bar(2_000, 100.0, 105.0, 100.0, 104.0, 1000.0),
            mk_bar(3_000, 104.0, 110.0, 103.0, 108.0, 1000.0),
        ];
        let cmp = vec![
            mk_bar(1_000, 50.0, 52.0, 49.0, 50.0, 500.0),
            mk_bar(3_000, 50.0, 56.0, 50.0, 55.0, 500.0),
        ];
        let pts = compute_rebased_comparison_series(&base, &cmp);
        assert_eq!(pts.len(), 3);
        assert!((pts[0].1 - 0.0).abs() < 1e-9);
        assert!((pts[1].1 - 0.0).abs() < 1e-9); // causal carry-forward from ts=1_000
        assert!((pts[2].1 - 10.0).abs() < 1e-9); // (55-50)/50 * 100
    }
}
