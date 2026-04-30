//! Native OS-level mouse interaction and calibration.
//!
//! Provides infrastructure for native OS mouse input including:
//! - Screen coordinate calibration for browser-to-OS mapping
//! - Fingerprint-based calibration caching
//! - Test calibration helpers

use crate::config::NativeClickCalibrationMode;
use anyhow::Result;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use tokio::sync::Mutex as TokioMutex;

/// Native click probe ID constant.
pub const NATIVE_CLICK_PROBE_ID: &str = "__auto_rust_nativeclick_probe";

/// Native click probe hit flag constant.
pub const NATIVE_CLICK_PROBE_HIT_FLAG: &str = "__auto_rust_nativeclick_probe_hit";

/// Global lock for native click operations to prevent concurrent OS input conflicts.
pub static NATIVE_CLICK_LOCK: Lazy<TokioMutex<()>> = Lazy::new(|| TokioMutex::new(()));

/// Calibration cache keyed by session::target_id fingerprint.
pub static NATIVE_CLICK_CALIBRATION_CACHE: Lazy<
    Mutex<HashMap<String, NativeClickCalibrationEntry>>,
> = Lazy::new(|| Mutex::new(HashMap::new()));

/// Forced calibration override for integration tests.
pub static FORCED_NATIVECLICK_CALIBRATION: Lazy<Mutex<HashMap<String, NativeClickCalibration>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Trace hooks for testing native click flow.
pub static NATIVECLICK_TRACE_HOOKS: Lazy<Mutex<HashMap<String, Vec<String>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Counter for trace IDs.
static NATIVE_CLICK_TRACE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Counter for lock acquisitions.
static NATIVE_LOCK_ACQUISITIONS: AtomicU64 = AtomicU64::new(0);

/// Counter for lock contentions.
static NATIVE_LOCK_CONTENTIONS: AtomicU64 = AtomicU64::new(0);

/// Total wait time for locks.
static NATIVE_LOCK_TOTAL_WAIT_MS: AtomicU64 = AtomicU64::new(0);

/// Maximum wait time for locks.
static NATIVE_LOCK_MAX_WAIT_MS: AtomicU64 = AtomicU64::new(0);

/// Total hold time for locks.
static NATIVE_LOCK_TOTAL_HOLD_MS: AtomicU64 = AtomicU64::new(0);

/// Maximum hold time for locks.
static NATIVE_LOCK_MAX_HOLD_MS: AtomicU64 = AtomicU64::new(0);

/// Contention threshold in ms.
const NATIVE_LOCK_CONTENTION_THRESHOLD_MS: u64 = 50;

/// Warning threshold for long waits.
const NATIVE_LOCK_WARN_WAIT_THRESHOLD_MS: u64 = 500;

/// Dispatch options for native mouse input.
#[derive(Debug, Clone, Copy)]
pub struct NativeDispatchOptions {
    pub backend: crate::config::NativeInputBackend,
    pub reaction_delay_ms: u64,
    pub reaction_delay_variance_pct: u32,
    pub settle_ms: u64,
    pub settle_variance_pct: u32,
}

/// Screen point coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenPoint {
    pub x: i32,
    pub y: i32,
}

/// Native click calibration parameters for coordinate transformation.
#[derive(Debug, Clone, Copy)]
pub struct NativeClickCalibration {
    pub scale_x: f64,
    pub scale_y: f64,
    pub origin_adjust_x: f64,
    pub origin_adjust_y: f64,
    pub mode: NativeClickCalibrationMode,
}

/// Browser window metrics for calibration.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct BrowserWindowMetrics {
    pub screen_x: f64,
    pub screen_y: f64,
    pub outer_width: f64,
    pub outer_height: f64,
    pub inner_width: f64,
    pub inner_height: f64,
    pub device_pixel_ratio: f64,
    pub visual_viewport_scale: f64,
    pub visual_viewport_offset_left: f64,
    pub visual_viewport_offset_top: f64,
}

/// Calibration fingerprint for cache lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeClickFingerprint {
    pub mode: NativeClickCalibrationMode,
    pub screen_x: i32,
    pub screen_y: i32,
    pub outer_width: i32,
    pub outer_height: i32,
}

/// Cache entry combining fingerprint and calibration.
#[derive(Debug, Clone, Copy)]
pub struct NativeClickCalibrationEntry {
    pub fingerprint: NativeClickFingerprint,
    pub calibration: NativeClickCalibration,
}

impl Default for NativeClickCalibration {
    fn default() -> Self {
        Self {
            scale_x: 1.0,
            scale_y: 1.0,
            origin_adjust_x: 0.0,
            origin_adjust_y: 0.0,
            mode: NativeClickCalibrationMode::Windows,
        }
    }
}

/// Get cached calibration if fingerprint matches.
pub fn cached_native_click_calibration(
    cache_key: &str,
    fingerprint: NativeClickFingerprint,
) -> Option<NativeClickCalibration> {
    NATIVE_CLICK_CALIBRATION_CACHE
        .lock()
        .ok()
        .and_then(|cache| {
            cache.get(cache_key).and_then(|entry| {
                if entry.fingerprint == fingerprint {
                    Some(entry.calibration)
                } else {
                    None
                }
            })
        })
}

/// Store calibration in cache.
pub fn store_native_click_calibration(
    cache_key: &str,
    fingerprint: NativeClickFingerprint,
    calibration: NativeClickCalibration,
) {
    if let Ok(mut cache) = NATIVE_CLICK_CALIBRATION_CACHE.lock() {
        cache.insert(
            cache_key.to_string(),
            NativeClickCalibrationEntry {
                fingerprint,
                calibration,
            },
        );
    }
}

/// Get forced calibration for testing.
pub fn get_forced_calibration(session_id: &str) -> Option<NativeClickCalibration> {
    FORCED_NATIVECLICK_CALIBRATION
        .lock()
        .ok()
        .and_then(|map| map.get(session_id).copied())
}

/// Set forced calibration for integration tests.
pub fn set_nativeclick_forced_calibration_for_tests(
    session_id: &str,
    scale_x: f64,
    scale_y: f64,
    origin_adjust_x: f64,
    origin_adjust_y: f64,
    mode: NativeClickCalibrationMode,
) {
    if let Ok(mut map) = FORCED_NATIVECLICK_CALIBRATION.lock() {
        map.insert(
            session_id.to_string(),
            NativeClickCalibration {
                scale_x,
                scale_y,
                origin_adjust_x,
                origin_adjust_y,
                mode,
            },
        );
    }
}

/// Get next trace ID.
pub fn next_nativeclick_trace_id() -> u64 {
    NATIVE_CLICK_TRACE_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Record a trace phase for a session.
pub fn record_nativeclick_trace_phase(session_id: &str, phase: &str) {
    if let Ok(mut hooks) = NATIVECLICK_TRACE_HOOKS.lock() {
        hooks
            .entry(session_id.to_string())
            .or_default()
            .push(phase.to_string());
    }
}

/// Update max atomic value.
fn atomic_update_max(target: &AtomicU64, value: u64) {
    let mut current = target.load(Ordering::Relaxed);
    while value > current
        && target
            .compare_exchange(current, value, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
    {
        current = target.load(Ordering::Relaxed);
    }
}

/// Lock guard with metrics tracking.
pub struct NativeInputLockGuard {
    _guard: tokio::sync::MutexGuard<'static, ()>,
    acquired_at: Instant,
}

impl Drop for NativeInputLockGuard {
    fn drop(&mut self) {
        let hold_ms = self.acquired_at.elapsed().as_millis() as u64;
        NATIVE_LOCK_TOTAL_HOLD_MS.fetch_add(hold_ms, Ordering::Relaxed);
        atomic_update_max(&NATIVE_LOCK_MAX_HOLD_MS, hold_ms);
    }
}

/// Acquire native input lock with metrics.
pub async fn acquire_native_input_lock(
    session_id: &str,
    _trace_id: u64,
    op: &'static str,
) -> NativeInputLockGuard {
    let wait_started = Instant::now();
    let guard = NATIVE_CLICK_LOCK.lock().await;
    let wait_ms = wait_started.elapsed().as_millis() as u64;
    NATIVE_LOCK_ACQUISITIONS.fetch_add(1, Ordering::Relaxed);
    NATIVE_LOCK_TOTAL_WAIT_MS.fetch_add(wait_ms, Ordering::Relaxed);
    let contentions = if wait_ms > NATIVE_LOCK_CONTENTION_THRESHOLD_MS {
        NATIVE_LOCK_CONTENTIONS.fetch_add(1, Ordering::Relaxed);
        wait_ms
    } else {
        0
    };
    atomic_update_max(&NATIVE_LOCK_MAX_WAIT_MS, wait_ms);
    if wait_ms > NATIVE_LOCK_WARN_WAIT_THRESHOLD_MS {
        log::warn!(
            "[native-lock] High wait time for session_id={} op={}: {}ms",
            session_id,
            op,
            wait_ms
        );
    }
    if contentions > 0 {
        log::debug!(
            "[native-lock] Contention for session_id={} op={}: waited {}ms",
            session_id,
            op,
            contentions
        );
    }
    NativeInputLockGuard {
        _guard: guard,
        acquired_at: Instant::now(),
    }
}

/// Metrics snapshot for native input lock.
#[derive(Debug, Clone, Copy)]
pub struct NativeInputLockMetricsSnapshot {
    pub acquisitions: u64,
    pub contentions: u64,
    pub total_wait_ms: u64,
    pub max_wait_ms: u64,
    pub avg_wait_ms: f64,
    pub total_hold_ms: u64,
    pub max_hold_ms: u64,
    pub avg_hold_ms: f64,
}

/// Get metrics snapshot for native input lock.
pub fn native_input_lock_metrics_snapshot() -> NativeInputLockMetricsSnapshot {
    let acquisitions = NATIVE_LOCK_ACQUISITIONS.load(Ordering::Relaxed);
    let total_wait_ms = NATIVE_LOCK_TOTAL_WAIT_MS.load(Ordering::Relaxed);
    let total_hold_ms = NATIVE_LOCK_TOTAL_HOLD_MS.load(Ordering::Relaxed);
    let denom = acquisitions.max(1) as f64;
    NativeInputLockMetricsSnapshot {
        acquisitions,
        contentions: NATIVE_LOCK_CONTENTIONS.load(Ordering::Relaxed),
        total_wait_ms,
        max_wait_ms: NATIVE_LOCK_MAX_WAIT_MS.load(Ordering::Relaxed),
        avg_wait_ms: total_wait_ms as f64 / denom,
        total_hold_ms,
        max_hold_ms: NATIVE_LOCK_MAX_HOLD_MS.load(Ordering::Relaxed),
        avg_hold_ms: total_hold_ms as f64 / denom,
    }
}

/// Clear forced calibration for a session.
pub fn clear_nativeclick_forced_calibration_for_tests(session_id: &str) {
    if let Ok(mut map) = FORCED_NATIVECLICK_CALIBRATION.lock() {
        map.remove(session_id);
    }
}

/// Add trace hook entry for testing.
pub fn nativeclick_add_trace_hook(session_id: &str, phase: &str) {
    if let Ok(mut hooks) = NATIVECLICK_TRACE_HOOKS.lock() {
        hooks
            .entry(session_id.to_string())
            .or_default()
            .push(phase.to_string());
    }
}

/// Get and clear trace hooks for a session.
pub fn take_nativeclick_trace_hooks(session_id: &str) -> Vec<String> {
    NATIVECLICK_TRACE_HOOKS
        .lock()
        .ok()
        .map_or_else(Vec::new, |mut hooks| {
            hooks.remove(session_id).unwrap_or_default()
        })
}

/// Clear trace hooks for a session.
pub fn clear_nativeclick_trace_hooks(session_id: &str) {
    if let Ok(mut hooks) = NATIVECLICK_TRACE_HOOKS.lock() {
        hooks.remove(session_id);
    }
}

/// Generate fingerprint from browser metrics.
pub fn native_click_fingerprint(
    metrics: &BrowserWindowMetrics,
    mode: NativeClickCalibrationMode,
) -> NativeClickFingerprint {
    NativeClickFingerprint {
        mode,
        screen_x: metrics.screen_x.round() as i32,
        screen_y: metrics.screen_y.round() as i32,
        outer_width: metrics.outer_width.round() as i32,
        outer_height: metrics.outer_height.round() as i32,
    }
}

/// Calculate browser content origin offset.
pub fn browser_content_origin(
    metrics: &BrowserWindowMetrics,
    scale_x: f64,
    scale_y: f64,
    mode: NativeClickCalibrationMode,
) -> (f64, f64) {
    let chrome_y = (metrics.outer_height - metrics.inner_height).max(0.0);
    let chrome_x = match mode {
        NativeClickCalibrationMode::Windows => {
            ((metrics.outer_width - metrics.inner_width).max(0.0)) / 2.0
        }
        NativeClickCalibrationMode::Mac | NativeClickCalibrationMode::Linux => 0.0,
    };
    (
        metrics.screen_x + chrome_x + metrics.visual_viewport_offset_left * scale_x,
        metrics.screen_y + chrome_y + metrics.visual_viewport_offset_top * scale_y,
    )
}

/// Calculate browser scale factor.
pub fn browser_scale(metrics: &BrowserWindowMetrics, _mode: NativeClickCalibrationMode) -> f64 {
    (metrics.device_pixel_ratio / metrics.visual_viewport_scale.max(1.0)).clamp(0.5, 4.0)
}

/// Create calibration from browser metrics.
pub fn native_click_calibration_from_metrics(
    metrics: &BrowserWindowMetrics,
    mode: NativeClickCalibrationMode,
) -> NativeClickCalibration {
    let scale = browser_scale(metrics, mode);
    NativeClickCalibration {
        scale_x: scale,
        scale_y: scale,
        origin_adjust_x: 0.0,
        origin_adjust_y: 0.0,
        mode,
    }
}

/// Convert page coordinates to screen coordinates using calibration.
pub fn screen_point_from_calibration(
    metrics: &BrowserWindowMetrics,
    calibration: &NativeClickCalibration,
    x: f64,
    y: f64,
) -> ScreenPoint {
    let (origin_x, origin_y) = browser_content_origin(
        metrics,
        calibration.scale_x,
        calibration.scale_y,
        calibration.mode,
    );
    ScreenPoint {
        x: (origin_x + x * calibration.scale_x + calibration.origin_adjust_x).round() as i32,
        y: (origin_y + y * calibration.scale_y + calibration.origin_adjust_y).round() as i32,
    }
}

/// Validate calibration parameters are finite.
pub fn validate_native_calibration(calibration: &NativeClickCalibration) -> Result<()> {
    let finite = calibration.scale_x.is_finite()
        && calibration.scale_y.is_finite()
        && calibration.origin_adjust_x.is_finite()
        && calibration.origin_adjust_y.is_finite();
    if !finite {
        return Err(anyhow::anyhow!("Calibration contains non-finite values"));
    }
    if calibration.scale_x < 0.1 || calibration.scale_y < 0.1 {
        return Err(anyhow::anyhow!("Calibration scale too small: < 0.1"));
    }
    if calibration.scale_x > 5.0 || calibration.scale_y > 5.0 {
        return Err(anyhow::anyhow!("Calibration scale too large: > 5.0"));
    }
    Ok(())
}

/// Native cursor candidate for probe-based calibration.
#[derive(Debug, Clone, Deserialize)]
pub struct NativeCursorCandidate {
    pub x: f64,
    pub y: f64,
    pub label: Option<String>,
}

/// Sample from a native click probe.
#[derive(Debug, Clone, Copy)]
pub struct NativeClickProbeSample {
    pub desired_x: f64,
    pub desired_y: f64,
    pub hit_x: f64,
    pub hit_y: f64,
}

/// Solve calibration parameters from two probe samples.
pub fn solve_calibration_from_probe_samples(
    _metrics: &BrowserWindowMetrics,
    candidate: NativeClickCalibration,
    first: NativeClickProbeSample,
    second: NativeClickProbeSample,
) -> Option<NativeClickCalibration> {
    let desired_dx = second.desired_x - first.desired_x;
    let desired_dy = second.desired_y - first.desired_y;
    let hit_dx = second.hit_x - first.hit_x;
    let hit_dy = second.hit_y - first.hit_y;

    if hit_dx.abs() < 1.0 || hit_dy.abs() < 1.0 {
        return None;
    }

    let scale_x = desired_dx / hit_dx;
    let scale_y = desired_dy / hit_dy;

    if !scale_x.is_finite() || !scale_y.is_finite() {
        return None;
    }

    let origin_adjust_x =
        first.desired_x * candidate.scale_x - first.hit_x * scale_x + candidate.origin_adjust_x;
    let origin_adjust_y =
        first.desired_y * candidate.scale_y - first.hit_y * scale_y + candidate.origin_adjust_y;

    Some(NativeClickCalibration {
        scale_x,
        scale_y,
        origin_adjust_x,
        origin_adjust_y,
        mode: candidate.mode,
    })
}
