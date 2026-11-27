//! Cross-platform user directory, path normalization, secure config storage,
//! SQLite WAL-safe backup migration, and Vietnam trading-session freshness helpers.
//!
//! Aligns 100% with Yumeko's cross-platform convention (`macOS`, `Windows 10/11`, `Linux`):
//! - macOS / Linux: `~/.config/sumi/` (`0600`) & `~/.local/share/sumi/`
//! - Windows 10/11: `%APPDATA%\sumi\` & `%LOCALAPPDATA%\sumi\`
//! - Overrides: `SUMI_CONFIG_DIR`, `SUMI_DATA_DIR`, `YUMEKO_CONFIG_DIR`, `YUMEKO_DATA_DIR`

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{Datelike, NaiveDate, TimeZone, Timelike, Utc};
use rusqlite::{Connection, OpenFlags, backup::Backup};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{DataError, Result};

/// Resolves the current user's home directory across macOS/Linux (`HOME`) and Windows
/// (`USERPROFILE`, `HOMEDRIVE` + `HOMEPATH`). Zero hardcoded user paths.
#[must_use]
pub fn user_home_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Some(profile) = std::env::var_os("USERPROFILE")
            && !profile.is_empty()
        {
            return PathBuf::from(profile);
        }
        if let (Some(drive), Some(path)) =
            (std::env::var_os("HOMEDRIVE"), std::env::var_os("HOMEPATH"))
            && !drive.is_empty()
            && !path.is_empty()
        {
            let mut combined = drive;
            combined.push(path);
            return PathBuf::from(combined);
        }
        if let Some(home) = std::env::var_os("HOME")
            && !home.is_empty()
        {
            return PathBuf::from(home);
        }
        std::env::temp_dir()
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(home) = std::env::var_os("HOME")
            && !home.is_empty()
        {
            return PathBuf::from(home);
        }
        if let Some(profile) = std::env::var_os("USERPROFILE")
            && !profile.is_empty()
        {
            return PathBuf::from(profile);
        }
        std::env::temp_dir()
    }
}

/// Canonical per-user configuration directory for Sumi (`sync_config.json`, `alerts_secrets.json`, `doraemi_sync.json`).
///
/// - Override: `SUMI_CONFIG_DIR`
/// - Windows: `%APPDATA%\sumi` (fallback `%USERPROFILE%\.config\sumi`)
/// - macOS: `$HOME/.config/sumi`
/// - Linux: `$XDG_CONFIG_HOME/sumi` (fallback `$HOME/.config/sumi`)
#[must_use]
pub fn sumi_config_dir() -> PathBuf {
    if let Ok(explicit) = std::env::var("SUMI_CONFIG_DIR")
        && !explicit.trim().is_empty()
    {
        return PathBuf::from(explicit.trim());
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA")
            && !appdata.is_empty()
        {
            return PathBuf::from(appdata).join("sumi");
        }
        user_home_dir().join(".config").join("sumi")
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME")
            && !xdg.is_empty()
        {
            return PathBuf::from(xdg).join("sumi");
        }
        user_home_dir().join(".config").join("sumi")
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        user_home_dir().join(".config").join("sumi")
    }
}

/// Canonical per-user application data directory for Sumi (`sumi_market.db`, `sumi_workspace.db`,
/// `kt01_verdicts/`, `exports/`, `backups/`, `runtime.lock`).
///
/// - Override: `SUMI_DATA_DIR`
/// - Windows: `%LOCALAPPDATA%\sumi` (fallback `%USERPROFILE%\.local\share\sumi`)
/// - macOS: `$HOME/.local/share/sumi`
/// - Linux: `$XDG_DATA_HOME/sumi` (fallback `$HOME/.local/share/sumi`)
#[must_use]
pub fn sumi_data_dir() -> PathBuf {
    if let Ok(explicit) = std::env::var("SUMI_DATA_DIR")
        && !explicit.trim().is_empty()
    {
        return PathBuf::from(explicit.trim());
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(local_appdata) = std::env::var_os("LOCALAPPDATA")
            && !local_appdata.is_empty()
        {
            return PathBuf::from(local_appdata).join("sumi");
        }
        user_home_dir().join(".local").join("share").join("sumi")
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME")
            && !xdg.is_empty()
        {
            return PathBuf::from(xdg).join("sumi");
        }
        user_home_dir().join(".local").join("share").join("sumi")
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        user_home_dir().join(".local").join("share").join("sumi")
    }
}

/// Canonical per-user configuration directory for sibling workstation Yumeko (read-only lookup).
#[must_use]
pub fn yumeko_config_dir() -> PathBuf {
    if let Ok(explicit) = std::env::var("YUMEKO_CONFIG_DIR")
        && !explicit.trim().is_empty()
    {
        return PathBuf::from(explicit.trim());
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA")
            && !appdata.is_empty()
        {
            return PathBuf::from(appdata).join("yumeko");
        }
        user_home_dir().join(".config").join("yumeko")
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME")
            && !xdg.is_empty()
        {
            return PathBuf::from(xdg).join("yumeko");
        }
        user_home_dir().join(".config").join("yumeko")
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        user_home_dir().join(".config").join("yumeko")
    }
}

/// Canonical per-user data directory for sibling workstation Yumeko (`yumeko-app.sqlite`, Read-Only).
#[must_use]
pub fn yumeko_data_dir() -> PathBuf {
    if let Ok(explicit) = std::env::var("YUMEKO_DATA_DIR")
        && !explicit.trim().is_empty()
    {
        return PathBuf::from(explicit.trim());
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(local_appdata) = std::env::var_os("LOCALAPPDATA")
            && !local_appdata.is_empty()
        {
            return PathBuf::from(local_appdata).join("yumeko");
        }
        user_home_dir().join(".local").join("share").join("yumeko")
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME")
            && !xdg.is_empty()
        {
            return PathBuf::from(xdg).join("yumeko");
        }
        user_home_dir().join(".local").join("share").join("yumeko")
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        user_home_dir().join(".local").join("share").join("yumeko")
    }
}

/// Candidate legacy v2.x Tauri app data directories for non-destructive migration to `sumi_data_dir()`.
#[must_use]
pub fn legacy_sumi_app_data_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    #[cfg(target_os = "macos")]
    {
        dirs.push(
            user_home_dir()
                .join("Library")
                .join("Application Support")
                .join("com.sumi.supercharts"),
        );
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(roaming) = std::env::var_os("APPDATA")
            && !roaming.is_empty()
        {
            dirs.push(PathBuf::from(roaming).join("com.sumi.supercharts"));
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA")
            && !local.is_empty()
        {
            dirs.push(PathBuf::from(local).join("com.sumi.supercharts"));
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME")
            && !xdg.is_empty()
        {
            dirs.push(PathBuf::from(xdg).join("com.sumi.supercharts"));
        }
        dirs.push(
            user_home_dir()
                .join(".local")
                .join("share")
                .join("com.sumi.supercharts"),
        );
    }
    dirs
}

/// Computes lowercase hex SHA-256 of an in-memory byte slice.
#[must_use]
pub fn sha256_bytes_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Computes lowercase hex SHA-256 of a file on disk using streaming chunks.
pub fn sha256_file_hex(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65_536];
    loop {
        let n = std::io::Read::read(&mut file, &mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Writes sensitive configuration/secrets atomically with `0600` owner-only permissions on Unix.
pub fn write_private_file_0600(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
        }
    }
    let tmp_path = path.with_extension(format!(
        "tmp.{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    ));
    {
        let mut f = fs::File::create(&tmp_path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600))?;
        }
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    fs::rename(&tmp_path, path)?;
    Ok(())
}

/// Receipt emitted after migrating a legacy SQLite database into `sumi_data_dir()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqliteMigrationReceipt {
    pub source_path: String,
    pub target_path: String,
    pub target_sha256: String,
    pub target_bytes: u64,
    pub migrated_at_utc: String,
}

/// Performs a WAL-safe, non-destructive atomic copy of `source_db` into `target_db` via SQLite's
/// Online Backup API (`rusqlite::backup::Backup`), verifies `PRAGMA integrity_check == "ok"`,
/// computes SHA-256, and renames atomically into place. Never mutates or deletes `source_db`.
pub fn sqlite_backup_atomic(source_db: &Path, target_db: &Path) -> Result<SqliteMigrationReceipt> {
    if !source_db.is_file() {
        return Err(DataError::Storage(format!(
            "source SQLite file does not exist: {}",
            source_db.display()
        )));
    }
    if let Some(parent) = target_db.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    let tmp_target = target_db.with_extension("migrating.sqlite");
    if tmp_target.exists() {
        let _ = fs::remove_file(&tmp_target);
    }

    // Open source read-only (capturing any live WAL frames seamlessly via SQLite's WAL reader).
    let src_conn = Connection::open_with_flags(
        source_db,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    let _ = src_conn.pragma_update(None, "query_only", "ON");
    let _ = src_conn.pragma_update(None, "busy_timeout", 5_000_i64);

    {
        let mut dst_conn = Connection::open(&tmp_target)?;
        {
            let backup = Backup::new(&src_conn, &mut dst_conn)?;
            backup.run_to_completion(256, Duration::from_millis(5), None)?;
        }
        let integrity: String = dst_conn.pragma_query_value(None, "integrity_check", |r| r.get(0))?;
        if !integrity.eq_ignore_ascii_case("ok") {
            let _ = fs::remove_file(&tmp_target);
            return Err(DataError::Storage(format!(
                "SQLite migration integrity check failed for {}: {integrity}",
                source_db.display()
            )));
        }
    }

    let target_sha256 = sha256_file_hex(&tmp_target)?;
    let target_bytes = fs::metadata(&tmp_target)?.len();
    fs::rename(&tmp_target, target_db)?;

    Ok(SqliteMigrationReceipt {
        source_path: source_db.display().to_string(),
        target_path: target_db.display().to_string(),
        target_sha256,
        target_bytes,
        migrated_at_utc: Utc::now().to_rfc3339(),
    })
}

/// Launches an `http://` or `https://` URL in the OS default system browser safely without shell
/// interpolation and without flashing a console window on Windows (`CREATE_NO_WINDOW`).
pub fn open_url_in_system_browser(url: &str) -> std::io::Result<()> {
    let trimmed = url.trim();
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "refusing_non_http_browser_url_scheme",
        ));
    }
    #[cfg(target_os = "macos")]
    let status = std::process::Command::new("/usr/bin/open")
        .arg(trimmed)
        .status()?;
    #[cfg(target_os = "linux")]
    let status = std::process::Command::new("xdg-open")
        .arg(trimmed)
        .status()?;
    #[cfg(target_os = "windows")]
    let status = {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        std::process::Command::new("rundll32")
            .args(["url.dll,FileProtocolHandler", trimmed])
            .creation_flags(CREATE_NO_WINDOW)
            .status()?
    };

    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "system_browser_exit_status:{:?}",
            status.code()
        )))
    }
}

/// Returns the most recent completed (or currently active >= 09:00 ICT) Vietnam exchange trading
/// date (`YYYY-MM-DD`) and its canonical `00:00:00 UTC` millisecond timestamp, accounting for
/// weekends and statutory `VIETNAM_HOLIDAYS_V1` in `sumi-core`.
#[must_use]
pub fn latest_expected_vn_trading_session_ms(now_utc_ms: i64) -> (String, i64) {
    const ICT_OFFSET_SECS: i64 = 7 * 3_600;
    let ict_secs = (now_utc_ms / 1_000).saturating_add(ICT_OFFSET_SECS);
    let dt = Utc
        .timestamp_opt(ict_secs, 0)
        .single()
        .unwrap_or_else(Utc::now);
    let mut date = dt.date_naive();
    // Before 09:00 ICT on a trading day, the latest closed EOD bar is the previous trading session.
    if dt.hour() < 9 {
        date = date.pred_opt().unwrap_or(date);
    }

    for _ in 0..16 {
        let epoch_days = sumi_core::data::days_from_civil(date.year(), date.month(), date.day());
        if sumi_core::journal::is_vn_exchange_trading_day_days(epoch_days) {
            let day_start_utc_ms = epoch_days.saturating_mul(86_400_000);
            return (date.format("%Y-%m-%d").to_string(), day_start_utc_ms);
        }
        date = date.pred_opt().unwrap_or(date);
    }

    let fallback_days = sumi_core::data::days_from_civil(date.year(), date.month(), date.day());
    (
        date.format("%Y-%m-%d").to_string(),
        fallback_days.saturating_mul(86_400_000),
    )
}

/// Counts Vietnam trading sessions strictly between `cached_latest_ms` and `expected_latest_ms`.
#[must_use]
pub fn count_missing_vn_sessions(cached_latest_ms: i64, expected_latest_ms: i64) -> usize {
    if cached_latest_ms >= expected_latest_ms {
        return 0;
    }
    let start_day = cached_latest_ms.div_euclid(86_400_000) + 1;
    let end_day = expected_latest_ms.div_euclid(86_400_000);
    if start_day > end_day {
        return 0;
    }
    let capped_end = end_day.min(start_day + 500);
    let mut missing = 0usize;
    for d in start_day..=capped_end {
        if sumi_core::journal::is_vn_exchange_trading_day_days(d) {
            missing += 1;
        }
    }
    missing
}

/// Formats a UTC millisecond timestamp as `YYYY-MM-DD` (ICT-adjusted for intraday, exact UTC date for daily).
#[must_use]
pub fn format_ymd_utc(ts_ms: i64) -> String {
    let secs = ts_ms.div_euclid(1_000);
    Utc.timestamp_opt(secs, 0)
        .single()
        .map_or_else(
            || NaiveDate::from_ymd_opt(1970, 1, 1).unwrap().to_string(),
            |d| d.format("%Y-%m-%d").to_string(),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_platform_dirs_without_hardcoded_user_paths() {
        let cfg = sumi_config_dir();
        let data = sumi_data_dir();
        let y_cfg = yumeko_config_dir();
        let y_data = yumeko_data_dir();
        assert!(cfg.to_string_lossy().to_ascii_lowercase().contains("sumi"));
        assert!(data.to_string_lossy().to_ascii_lowercase().contains("sumi"));
        assert!(
            y_cfg
                .to_string_lossy()
                .to_ascii_lowercase()
                .contains("yumeko")
        );
        assert!(
            y_data
                .to_string_lossy()
                .to_ascii_lowercase()
                .contains("yumeko")
        );
        assert!(open_url_in_system_browser("javascript:alert(1)").is_err());
    }

    #[test]
    fn sqlite_backup_atomic_captures_live_wal_and_preserves_source() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("legacy.db");
        let dst = dir.path().join("migrated/sumi_market.db");

        {
            let conn = Connection::open(&src).unwrap();
            conn.pragma_update(None, "journal_mode", "WAL").unwrap();
            conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)", [])
                .unwrap();
            conn.execute("INSERT INTO t (id, v) VALUES (1, 'FPT'), (2, 'VCB')", [])
                .unwrap();
            // Keep `conn` open so WAL frames are active during backup!
            let receipt = sqlite_backup_atomic(&src, &dst).unwrap();
            assert_eq!(receipt.target_sha256.len(), 64);
            assert!(receipt.target_bytes > 0);
        }

        assert!(src.is_file(), "source legacy DB must NEVER be deleted");
        assert!(dst.is_file());
        let check = Connection::open(&dst).unwrap();
        let cnt: i64 = check
            .query_row("SELECT COUNT(*) FROM t", [], |r| r.get(0))
            .unwrap();
        assert_eq!(cnt, 2);
    }

    #[test]
    fn vn_trading_session_freshness_skips_weekends() {
        // Sunday 2026-11-22 10:00 ICT (03:00 UTC) -> expects Friday 2026-11-20
        let sun_ms = Utc
            .with_ymd_and_hms(2026, 11, 22, 3, 0, 0)
            .unwrap()
            .timestamp_millis();
        let (ymd, ts_ms) = latest_expected_vn_trading_session_ms(sun_ms);
        assert_eq!(ymd, "2026-11-20");

        let nov_06_ms = Utc
            .with_ymd_and_hms(2026, 11, 6, 0, 0, 0)
            .unwrap()
            .timestamp_millis();
        assert_eq!(count_missing_vn_sessions(nov_06_ms, ts_ms), 10);
        assert_eq!(count_missing_vn_sessions(ts_ms, ts_ms), 0);
    }
}
