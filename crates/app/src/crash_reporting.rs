//! Crash reporting system for MCLRE.
//!
//! Provides local-only crash report generation with zero telemetry.
//! On panic, writes a detailed crash report to the crash-reports directory
//! with stack trace, system info, game state, and relevant logs.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Utc;
use mclre_core::Identifier;

static CRASH_REPORT_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Initialize the crash reporting system.
/// Call this early in main() before any other initialization.
pub fn init_crash_reporting(crash_dir: Option<PathBuf>) {
    let dir = crash_dir.unwrap_or_else(|| {
        // Default to <world_dir>/crash-reports or ./crash-reports
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("mclre")
            .join("crash-reports")
    });

    let _ = fs::create_dir_all(&dir);
    let _ = CRASH_REPORT_DIR.set(dir);

    // Set up panic hook
    std::panic::set_hook(Box::new(|panic_info| {
        generate_crash_report(panic_info);
    }));
}

/// Get the crash report directory
fn get_crash_dir() -> PathBuf {
    CRASH_REPORT_DIR
        .get()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("crash-reports"))
}

/// Generate a crash report from panic info
fn generate_crash_report(panic_info: &std::panic::PanicHookInfo<'_>) {
    let crash_dir = get_crash_dir();
    let _ = fs::create_dir_all(&crash_dir);

    let timestamp = Utc::now().format("%Y-%m-%d_%H.%M.%S").to_string();
    let report_name = format!("crash-{}.txt", timestamp);
    let report_path = crash_dir.join(&report_name);

    let mut report = String::new();

    // Header
    report.push_str("==================================================\n");
    report.push_str("MCLRE Crash Report\n");
    report.push_str("==================================================\n\n");

    // Time
    report.push_str(&format!("Time: {}\n", Utc::now().to_rfc3339()));
    report.push_str(&format!("Game Version: {}\n", env!("CARGO_PKG_VERSION")));
    report.push_str(&format!(
        "Git Commit: {}\n",
        option_env!("GIT_COMMIT").unwrap_or("unknown")
    ));
    report.push_str("\n");

    // System Info
    report.push_str("--- System Info ---\n");
    report.push_str(&format!("OS: {}\n", std::env::consts::OS));
    report.push_str(&format!("Arch: {}\n", std::env::consts::ARCH));
    report.push_str(&format!(
        "Rust Version: {}\n",
        option_env!("RUSTC_VERSION").unwrap_or("unknown")
    ));
    report.push_str("\n");

    // Panic Info
    report.push_str("--- Panic Info ---\n");
    if let Some(payload) = panic_info.payload().downcast_ref::<&str>() {
        report.push_str(&format!("Message: {}\n", payload));
    } else if let Some(payload) = panic_info.payload().downcast_ref::<String>() {
        report.push_str(&format!("Message: {}\n", payload));
    } else {
        report.push_str("Message: <non-string payload>\n");
    }

    if let Some(location) = panic_info.location() {
        report.push_str(&format!(
            "Location: {}:{}:{}\n",
            location.file(),
            location.line(),
            location.column()
        ));
    }
    report.push_str("\n");

    // Stack trace (if available)
    report.push_str("--- Stack Trace ---\n");
    #[cfg(feature = "backtrace")]
    {
        use std::backtrace::Backtrace;
        let bt = Backtrace::capture();
        report.push_str(&format!("{:?}\n", bt));
    }
    #[cfg(not(feature = "backtrace"))]
    {
        report.push_str("<backtrace feature not enabled>\n");
    }
    report.push_str("\n");

    // Game State (if available)
    report.push_str("--- Game State ---\n");
    report.push_str("<game state capture not yet implemented>\n");
    report.push_str("\n");

    // Recent Logs (if available)
    report.push_str("--- Recent Logs ---\n");
    report.push_str("<log capture not yet implemented>\n");
    report.push_str("\n");

    // Mods loaded
    report.push_str("--- Loaded Mods ---\n");
    report.push_str("<mod list not yet implemented>\n");
    report.push_str("\n");

    // Write report
    if let Err(e) = fs::write(&report_path, report) {
        eprintln!("Failed to write crash report: {}", e);
    } else {
        eprintln!("Crash report written to: {}", report_path.display());
    }

    // Also print to stderr for immediate visibility
    eprintln!("\n=== MCLRE CRASH ===");
    eprintln!(
        "A crash report has been saved to: {}",
        report_path.display()
    );
    eprintln!("Please include this file when reporting the issue.\n");
}

/// Generate a crash report from an error with context
pub fn generate_error_report(error: &dyn std::error::Error, context: &str) -> PathBuf {
    let crash_dir = get_crash_dir();
    let _ = fs::create_dir_all(&crash_dir);

    let timestamp = Utc::now().format("%Y-%m-%d_%H.%M.%S").to_string();
    let report_name = format!("error-{}.txt", timestamp);
    let report_path = crash_dir.join(&report_name);

    let mut report = String::new();

    report.push_str("==================================================\n");
    report.push_str("MCLRE Error Report\n");
    report.push_str("==================================================\n\n");

    report.push_str(&format!("Time: {}\n", Utc::now().to_rfc3339()));
    report.push_str(&format!("Context: {}\n", context));
    report.push_str(&format!("Error: {}\n", error));
    report.push_str("\n");

    // Error chain
    let mut source = error.source();
    let mut depth = 0;
    while let Some(err) = source {
        report.push_str(&format!("  Caused by ({}): {}\n", depth, err));
        source = err.source();
        depth += 1;
    }

    if let Err(e) = fs::write(&report_path, report) {
        eprintln!("Failed to write error report: {}", e);
    } else {
        eprintln!("Error report written to: {}", report_path.display());
    }

    report_path
}

/// Get list of recent crash reports
pub fn list_crash_reports() -> Vec<PathBuf> {
    let crash_dir = get_crash_dir();
    if !crash_dir.exists() {
        return Vec::new();
    }

    let mut reports: Vec<PathBuf> = fs::read_dir(&crash_dir)
        .unwrap_or_else(|_| fs::read_dir(".").unwrap())
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(|s| s.to_str()) == Some("txt")
                && path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.starts_with("crash-") || s.starts_with("error-"))
                    .unwrap_or(false)
        })
        .collect();

    // Sort by modification time (newest first)
    reports.sort_by(|a, b| {
        let a_time = fs::metadata(a)
            .and_then(|m| m.modified())
            .unwrap_or(UNIX_EPOCH);
        let b_time = fs::metadata(b)
            .and_then(|m| m.modified())
            .unwrap_or(UNIX_EPOCH);
        b_time.cmp(&a_time)
    });

    reports
}

/// Clean up old crash reports (keep last N)
pub fn cleanup_crash_reports(keep: usize) {
    let mut reports = list_crash_reports();
    if reports.len() > keep {
        for report in reports.drain(keep..) {
            let _ = fs::remove_file(&report);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic;

    #[test]
    fn test_crash_report_generation() {
        let temp_dir = tempfile::tempdir().unwrap();
        init_crash_reporting(Some(temp_dir.path().to_path_buf()));

        // Trigger a controlled panic
        let result = panic::catch_unwind(|| {
            panic!("Test panic for crash reporting");
        });

        assert!(result.is_err());

        // Check that crash report was created
        let reports = list_crash_reports();
        assert!(!reports.is_empty());

        let report_content = fs::read_to_string(&reports[0]).unwrap();
        assert!(report_content.contains("MCLRE Crash Report"));
        assert!(report_content.contains("Test panic for crash reporting"));
    }

    #[test]
    fn test_error_report_generation() {
        let temp_dir = tempfile::tempdir().unwrap();
        init_crash_reporting(Some(temp_dir.path().to_path_buf()));

        #[derive(Debug, thiserror::Error)]
        enum TestError {
            #[error("IO error: {0}")]
            Io(#[from] std::io::Error),
            #[error("Custom error: {msg}")]
            Custom { msg: String },
        }

        let error = TestError::Custom {
            msg: "Test error".to_string(),
        };
        let report_path = generate_error_report(&error, "Testing error reporting");

        assert!(report_path.exists());
        let content = fs::read_to_string(&report_path).unwrap();
        assert!(content.contains("MCLRE Error Report"));
        assert!(content.contains("Test error"));
        assert!(content.contains("Testing error reporting"));
    }
}
