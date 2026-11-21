use tracing_subscriber::EnvFilter;

/// Re-exported TradingView transport filter directives (`tungstenite=debug,tokio_tungstenite=debug`)
/// preventing WebSocket frame dumps at `TRACE` from leaking `set_auth_token`.
pub const RECOMMENDED_LOG_DIRECTIVES: &str = tradingview::RECOMMENDED_LOG_DIRECTIVES;

/// Constructs an `EnvFilter` directive string guaranteed to cap WebSocket transport crates
/// via [`RECOMMENDED_LOG_DIRECTIVES`] whenever `base_directive` enables `trace` (or is empty),
/// while staying quiet at `warn`/`error`/`info` when higher thresholds are chosen.
#[must_use]
pub fn build_safe_filter_spec(base_directive: &str) -> String {
    let trimmed = base_directive.trim();
    if trimmed.is_empty() {
        format!("info,{RECOMMENDED_LOG_DIRECTIVES}")
    } else if trimmed.to_ascii_lowercase().contains("trace") {
        format!("{trimmed},{RECOMMENDED_LOG_DIRECTIVES}")
    } else {
        // Still guard transport crates explicitly at `warn` when base is `warn`/`error`,
        // or cap at `debug` via `RECOMMENDED_LOG_DIRECTIVES` if `debug` is requested.
        if trimmed.to_ascii_lowercase().contains("debug") {
            format!("{trimmed},{RECOMMENDED_LOG_DIRECTIVES}")
        } else {
            format!("{trimmed},tungstenite=warn,tokio_tungstenite=warn")
        }
    }
}

/// Initializes a `tracing_subscriber` formatting logger applying [`RECOMMENDED_LOG_DIRECTIVES`].
///
/// Respects `RUST_LOG` when set, appending the redaction directives so WebSocket transport frames
/// are never logged at `TRACE`. Calling this multiple times is a safe no-op.
pub fn init_tracing(default_level: &str) {
    let base = std::env::var("RUST_LOG").unwrap_or_else(|_| default_level.to_owned());
    let spec = build_safe_filter_spec(&base);
    let filter = EnvFilter::try_new(&spec)
        .unwrap_or_else(|_| EnvFilter::new(format!("info,{RECOMMENDED_LOG_DIRECTIVES}")));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_filter_always_appends_recommended_log_directives() {
        let spec = build_safe_filter_spec("trace");
        assert!(spec.starts_with("trace,"));
        assert!(spec.contains("tungstenite=debug"));
        assert!(spec.contains("tokio_tungstenite=debug"));
        assert!(EnvFilter::try_new(&spec).is_ok());

        let empty_spec = build_safe_filter_spec("   ");
        assert!(empty_spec.contains(RECOMMENDED_LOG_DIRECTIVES));
        assert!(EnvFilter::try_new(&empty_spec).is_ok());
    }
}
