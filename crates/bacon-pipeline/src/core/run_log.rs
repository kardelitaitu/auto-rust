//! Persistent run log — records pipeline execution outcomes.
//!
//! Writes to `.bacon/sessions/run-log.json` after each pipeline run.
//! Uses JSON format to avoid `serde_yaml` C-FFI U+2028/U+2029 panics.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::Stage;

/// Duration and confidence for a single pipeline stage.
#[derive(Debug, Serialize, Deserialize)]
pub struct StageEntry {
    pub stage: String,
    pub duration_ms: u64,
    pub confidence: Option<String>,
}

/// One complete pipeline execution.
#[derive(Debug, Serialize, Deserialize)]
pub struct RunEntry {
    pub timestamp_secs: u64,
    pub spec_id: Option<String>,
    pub stages: Vec<StageEntry>,
    pub outcome: String,
}

/// Persistent run log stored as a JSON array of `RunEntry`.
#[derive(Debug, Serialize, Deserialize)]
pub struct RunLog {
    pub entries: Vec<RunEntry>,
}

impl RunLog {
    /// Load existing run log from disk, or create empty.
    fn load(path: &PathBuf) -> Self {
        if path.exists() {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
                .unwrap_or(Self {
                    entries: Vec::new(),
                })
        } else {
            Self {
                entries: Vec::new(),
            }
        }
    }

    fn save(&self, path: &PathBuf) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, content);
        }
    }
}

/// Append a run entry to the persistent log.
pub fn append_entry(entry: RunEntry) {
    let path = run_log_path();
    let mut log = RunLog::load(&path);
    log.entries.push(entry);
    log.save(&path);
}

/// Determine the run log path from project config.
fn run_log_path() -> PathBuf {
    let bacon_dir = crate::config::project_config().bacon_dir.clone();
    bacon_dir.join("sessions").join("run-log.json")
}

// ---------------------------------------------------------------------------
// GitHub Actions annotation helpers (--ci flag)
// ---------------------------------------------------------------------------

/// Whether CI mode is active (set via `--ci` flag).
static CI_MODE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Enable CI annotation mode.
pub fn set_ci_mode(enabled: bool) {
    CI_MODE.store(enabled, std::sync::atomic::Ordering::Relaxed);
}

/// Check if CI mode is active.
pub fn is_ci_mode() -> bool {
    CI_MODE.load(std::sync::atomic::Ordering::Relaxed)
}

/// Emit a GitHub Actions notice annotation.
pub fn ci_notice(msg: &str) {
    if is_ci_mode() {
        println!("::notice::{msg}");
    }
}

/// Emit a GitHub Actions warning annotation.
pub fn ci_warning(msg: &str) {
    if is_ci_mode() {
        println!("::warning::{msg}");
    }
}

/// Emit a GitHub Actions error annotation.
pub fn ci_error(msg: &str) {
    if is_ci_mode() {
        println!("::error::{msg}");
    }
}

/// Emit a GitHub Actions group start.
pub fn ci_group_start(title: &str) {
    if is_ci_mode() {
        println!("::group::{title}");
    }
}

/// Emit a GitHub Actions group end.
pub fn ci_group_end() {
    if is_ci_mode() {
        println!("::endgroup::");
    }
}

/// Build a `RunEntry` from pipeline stage durations and outcome.
///
/// Tests in `run_log_test.rs` verify field mapping and CI annotation helpers.
pub fn build_entry(
    spec_id: Option<String>,
    stages: &[(Stage, std::time::Duration)],
    outcome: &str,
) -> RunEntry {
    let timestamp_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let stage_entries: Vec<StageEntry> = stages
        .iter()
        .map(|(stage, dur)| StageEntry {
            stage: format!("{stage:?}"),
            duration_ms: dur.as_millis() as u64,
            confidence: None,
        })
        .collect();

    RunEntry {
        timestamp_secs,
        spec_id,
        stages: stage_entries,
        outcome: outcome.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_entry_sets_spec_id() {
        let entry = build_entry(Some("spec-001".to_string()), &[], "success");
        assert_eq!(entry.spec_id.as_deref(), Some("spec-001"));
    }

    #[test]
    fn build_entry_sets_outcome() {
        let entry = build_entry(None, &[], "failure");
        assert_eq!(entry.outcome, "failure");
    }

    #[test]
    fn build_entry_sets_stages_in_order() {
        let stages = &[
            (Stage::Observer, std::time::Duration::from_millis(100)),
            (Stage::Strategist, std::time::Duration::from_millis(200)),
            (Stage::Coder, std::time::Duration::from_millis(5000)),
            (Stage::Auditor, std::time::Duration::from_millis(300)),
        ];
        let entry = build_entry(None, stages, "success");
        assert_eq!(entry.stages.len(), 4);
        assert_eq!(entry.stages[0].stage, "Observer");
        assert_eq!(entry.stages[0].duration_ms, 100);
        assert_eq!(entry.stages[1].stage, "Strategist");
        assert_eq!(entry.stages[1].duration_ms, 200);
        assert_eq!(entry.stages[2].stage, "Coder");
        assert_eq!(entry.stages[2].duration_ms, 5000);
        assert_eq!(entry.stages[3].stage, "Auditor");
        assert_eq!(entry.stages[3].duration_ms, 300);
    }

    #[test]
    fn build_entry_timestamp_is_recent() {
        let entry = build_entry(None, &[], "success");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        assert!(entry.timestamp_secs <= now);
        assert!(entry.timestamp_secs > now - 10);
    }

    #[test]
    fn build_entry_empty_stages() {
        let entry = build_entry(None, &[], "no-op");
        assert!(entry.stages.is_empty());
    }

    #[test]
    fn build_entry_confidence_defaults_to_none() {
        let entry = build_entry(
            None,
            &[(Stage::Observer, std::time::Duration::from_millis(50))],
            "done",
        );
        assert_eq!(entry.stages.len(), 1);
        assert!(entry.stages[0].confidence.is_none());
    }

    #[test]
    fn ci_mode_toggle() {
        set_ci_mode(false);
        assert!(!is_ci_mode());
        set_ci_mode(true);
        assert!(is_ci_mode());
        set_ci_mode(false);
        assert!(!is_ci_mode());
    }
}
