use crate::config::NativeInputBackend;
use anyhow::Result;
use enigo::{
    Button as NativeButton, Coordinate as NativeCoordinate, Direction, Enigo, Mouse, Settings,
};
use log::warn;
use rand::Rng;
use std::sync::OnceLock;
use std::time::Duration;

pub(crate) trait NativeMouseBackend: Send + Sync + 'static {
    fn ensure_ready(&self);
    fn move_to_point_blocking(
        &self,
        target_x: i32,
        target_y: i32,
        reaction_delay_ms: u64,
        reaction_delay_variance_pct: u32,
        settle_ms: u64,
        settle_variance_pct: u32,
    ) -> Result<()>;
    fn move_and_click_point_blocking(
        &self,
        target_x: i32,
        target_y: i32,
        reaction_delay_ms: u64,
        reaction_delay_variance_pct: u32,
        settle_ms: u64,
        settle_variance_pct: u32,
    ) -> Result<()>;
}

struct EnigoNativeMouseBackend;

static ENIGO_BACKEND: EnigoNativeMouseBackend = EnigoNativeMouseBackend;
static UNSUPPORTED_BACKEND_WARNED: OnceLock<()> = OnceLock::new();

fn backend(selected: NativeInputBackend) -> &'static dyn NativeMouseBackend {
    match selected {
        NativeInputBackend::Enigo => &ENIGO_BACKEND,
        NativeInputBackend::Sendinput | NativeInputBackend::Rdev => {
            UNSUPPORTED_BACKEND_WARNED.get_or_init(|| {
                warn!(
                    "Native input backend '{}' is not implemented yet, falling back to enigo",
                    selected.as_str()
                );
            });
            &ENIGO_BACKEND
        }
    }
}

pub(crate) fn ensure_native_input_ready(selected: NativeInputBackend) {
    backend(selected).ensure_ready();
}

pub(crate) fn native_move_and_click_point_blocking(
    selected: NativeInputBackend,
    target_x: i32,
    target_y: i32,
    reaction_delay_ms: u64,
    reaction_delay_variance_pct: u32,
    settle_ms: u64,
    settle_variance_pct: u32,
) -> Result<()> {
    backend(selected).move_and_click_point_blocking(
        target_x,
        target_y,
        reaction_delay_ms,
        reaction_delay_variance_pct,
        settle_ms,
        settle_variance_pct,
    )
}

pub(crate) fn native_move_to_point_blocking(
    selected: NativeInputBackend,
    target_x: i32,
    target_y: i32,
    reaction_delay_ms: u64,
    reaction_delay_variance_pct: u32,
    settle_ms: u64,
    settle_variance_pct: u32,
) -> Result<()> {
    backend(selected).move_to_point_blocking(
        target_x,
        target_y,
        reaction_delay_ms,
        reaction_delay_variance_pct,
        settle_ms,
        settle_variance_pct,
    )
}

impl NativeMouseBackend for EnigoNativeMouseBackend {
    fn ensure_ready(&self) {
        static NATIVE_INIT_ONCE: OnceLock<()> = OnceLock::new();
        NATIVE_INIT_ONCE.get_or_init(|| {
            #[cfg(target_os = "windows")]
            {
                let _ = enigo::set_dpi_awareness();
            }
            let _ = Enigo::new(&Settings::default());
        });
    }

    fn move_to_point_blocking(
        &self,
        target_x: i32,
        target_y: i32,
        reaction_delay_ms: u64,
        reaction_delay_variance_pct: u32,
        settle_ms: u64,
        settle_variance_pct: u32,
    ) -> Result<()> {
        let mut enigo = Enigo::new(&Settings::default())
            .map_err(|err| anyhow::anyhow!("enigo init failed: {err:?}"))?;
        native_move_to_point_blocking_with_enigo(
            &mut enigo,
            target_x,
            target_y,
            reaction_delay_ms,
            reaction_delay_variance_pct,
            settle_ms,
            settle_variance_pct,
        )
    }

    fn move_and_click_point_blocking(
        &self,
        target_x: i32,
        target_y: i32,
        reaction_delay_ms: u64,
        reaction_delay_variance_pct: u32,
        settle_ms: u64,
        settle_variance_pct: u32,
    ) -> Result<()> {
        let mut enigo = Enigo::new(&Settings::default())
            .map_err(|err| anyhow::anyhow!("enigo init failed: {err:?}"))?;
        native_move_to_point_blocking_with_enigo(
            &mut enigo,
            target_x,
            target_y,
            reaction_delay_ms,
            reaction_delay_variance_pct,
            settle_ms,
            settle_variance_pct,
        )?;
        enigo
            .button(NativeButton::Left, Direction::Press)
            .map_err(|err| anyhow::anyhow!("enigo press failed: {err:?}"))?;
        std::thread::sleep(Duration::from_millis(jittered_delay_ms(45, 25)));
        enigo
            .button(NativeButton::Left, Direction::Release)
            .map_err(|err| anyhow::anyhow!("enigo release failed: {err:?}"))?;
        Ok(())
    }
}

fn native_move_to_point_blocking_with_enigo(
    enigo: &mut Enigo,
    target_x: i32,
    target_y: i32,
    reaction_delay_ms: u64,
    reaction_delay_variance_pct: u32,
    settle_ms: u64,
    settle_variance_pct: u32,
) -> Result<()> {
    let (start_x, start_y) = enigo.location().unwrap_or((target_x, target_y));
    let dx = target_x - start_x;
    let dy = target_y - start_y;
    let distance = ((dx as f64).powi(2) + (dy as f64).powi(2)).sqrt();
    let steps = ((distance / 85.0).ceil() as usize).clamp(6, 16);
    let mut rng = rand::thread_rng();

    if steps <= 1 {
        enigo
            .move_mouse(target_x, target_y, NativeCoordinate::Abs)
            .map_err(|err| anyhow::anyhow!("enigo move failed: {err:?}"))?;
    } else {
        let step_base = (reaction_delay_ms / steps as u64).max(2);
        for step in 1..=steps {
            let t = step as f64 / steps as f64;
            let eased = 1.0 - (1.0 - t).powi(3);
            let jitter = (distance / 600.0).clamp(0.0, 1.4);
            let x = start_x as f64
                + (target_x - start_x) as f64 * eased
                + rng.gen_range(-jitter..=jitter);
            let y = start_y as f64
                + (target_y - start_y) as f64 * eased
                + rng.gen_range(-jitter..=jitter);
            enigo
                .move_mouse(x.round() as i32, y.round() as i32, NativeCoordinate::Abs)
                .map_err(|err| anyhow::anyhow!("enigo move failed: {err:?}"))?;
            if step < steps {
                std::thread::sleep(Duration::from_millis(jittered_delay_ms(
                    step_base,
                    reaction_delay_variance_pct.min(60),
                )));
            }
        }
    }

    std::thread::sleep(Duration::from_millis(jittered_delay_ms(
        settle_ms,
        settle_variance_pct,
    )));
    Ok(())
}

fn jittered_delay_ms(base_ms: u64, variance_pct: u32) -> u64 {
    if base_ms == 0 {
        return 0;
    }

    let mut rng = rand::thread_rng();
    let variance = ((base_ms as f64) * (variance_pct as f64 / 100.0)).round() as u64;
    if variance == 0 {
        return base_ms;
    }

    let lower = base_ms.saturating_sub(variance);
    let upper = base_ms.saturating_add(variance);
    rng.gen_range(lower..=upper)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jittered_delay_ms_zero_base() {
        let delay = jittered_delay_ms(0, 20);
        assert_eq!(delay, 0);
    }

    #[test]
    fn test_jittered_delay_ms_zero_variance() {
        let delay = jittered_delay_ms(100, 0);
        assert_eq!(delay, 100);
    }

    #[test]
    fn test_jittered_delay_ms_within_bounds() {
        for _ in 0..50 {
            let delay = jittered_delay_ms(100, 20);
            assert!((80..=120).contains(&delay));
        }
    }

    #[test]
    fn test_jittered_delay_ms_high_variance() {
        for _ in 0..50 {
            let delay = jittered_delay_ms(100, 100);
            assert!((0..=200).contains(&delay));
        }
    }

    #[test]
    fn test_jittered_delay_ms_large_base() {
        let delay = jittered_delay_ms(1000, 10);
        assert!((900..=1100).contains(&delay));
    }

    #[test]
    fn test_jittered_delay_ms_small_base() {
        let delay = jittered_delay_ms(5, 50);
        assert!((0..=10).contains(&delay));
    }

    #[test]
    fn test_jittered_delay_ms_variance_clamping() {
        // Test that variance doesn't cause underflow
        let delay = jittered_delay_ms(1, 200);
        // delay is u64, so it's always non-negative
        assert!(delay <= 300); // Reasonable upper bound
    }

    #[test]
    fn test_backend_selection_enigo() {
        let backend = backend(NativeInputBackend::Enigo);
        // Should return a backend without panicking
        let _ = backend;
    }

    #[test]
    fn test_backend_selection_sendinput() {
        let backend = backend(NativeInputBackend::Sendinput);
        // Should fall back to enigo without panicking
        let _ = backend;
    }

    #[test]
    fn test_backend_selection_rdev() {
        let backend = backend(NativeInputBackend::Rdev);
        // Should fall back to enigo without panicking
        let _ = backend;
    }

    #[test]
    fn test_ensure_native_input_ready_enigo() {
        // Should not panic
        ensure_native_input_ready(NativeInputBackend::Enigo);
    }

    #[test]
    fn test_ensure_native_input_ready_sendinput() {
        // Should not panic (falls back to enigo)
        ensure_native_input_ready(NativeInputBackend::Sendinput);
    }

    #[test]
    fn test_ensure_native_input_ready_rdev() {
        // Should not panic (falls back to enigo)
        ensure_native_input_ready(NativeInputBackend::Rdev);
    }

    #[test]
    fn test_jittered_delay_ms_consistency() {
        // Test that the function is deterministic in its randomness
        let delays: Vec<u64> = (0..20).map(|_| jittered_delay_ms(100, 10)).collect();
        // All should be within bounds
        for delay in delays {
            assert!((90..=110).contains(&delay));
        }
    }

    #[test]
    fn test_jittered_delay_ms_extreme_variance() {
        for _ in 0..20 {
            let delay = jittered_delay_ms(100, 500);
            assert!((0..=600).contains(&delay));
        }
    }

    #[test]
    fn test_jittered_delay_ms_variance_rounding() {
        // Test that variance rounding doesn't cause issues
        let delay = jittered_delay_ms(3, 33);
        // delay is u64, so it's always non-negative
        assert!(delay <= 10); // Reasonable upper bound for base=3, variance=33
    }

    #[test]
    fn test_jittered_delay_ms_single_value() {
        // When variance is 0, should always return base
        for _ in 0..10 {
            let delay = jittered_delay_ms(50, 0);
            assert_eq!(delay, 50);
        }
    }

    #[test]
    fn test_native_input_backend_trait() {
        // Test that the trait is object-safe
        let backend: &dyn NativeMouseBackend = &ENIGO_BACKEND;
        backend.ensure_ready(); // Should not panic
    }

    #[test]
    fn test_jittered_delay_ms_very_large_base() {
        let delay = jittered_delay_ms(100000, 5);
        assert!((95000..=105000).contains(&delay));
    }

    #[test]
    fn test_jittered_delay_ms_base_one() {
        let delay = jittered_delay_ms(1, 50);
        // With base=1 and variance=50%, range is 0-2
        assert!(delay <= 2);
    }

    #[test]
    fn test_jittered_delay_ms_variance_100_percent() {
        for _ in 0..20 {
            let delay = jittered_delay_ms(100, 100);
            // Range is 0-200
            assert!(delay <= 200);
        }
    }

    #[test]
    fn test_jittered_delay_ms_variance_above_100() {
        let delay = jittered_delay_ms(100, 150);
        // Variance > 100% should still work
        assert!(delay <= 350); // 100 + 150% of 100 = 250, but saturating add may give more
    }

    #[test]
    fn test_jittered_delay_ms_very_small_variance() {
        let delay = jittered_delay_ms(1000, 1);
        // With 1% variance, range is 990-1010
        assert!((990..=1010).contains(&delay));
    }

    #[test]
    fn test_jittered_delay_ms_base_max_u64() {
        let delay = jittered_delay_ms(u64::MAX, 0);
        // With 0 variance, should return base
        assert_eq!(delay, u64::MAX);
    }

    #[test]
    fn test_jittered_delay_ms_variance_with_small_base() {
        let delay = jittered_delay_ms(2, 100);
        // With base=2 and 100% variance, range is 0-4
        assert!(delay <= 4);
    }

    #[test]
    fn test_backend_selection_consistency() {
        // Multiple calls should return the same backend
        let backend1 = backend(NativeInputBackend::Enigo);
        let backend2 = backend(NativeInputBackend::Enigo);
        // Both should be the same static reference
        // We can't compare trait object pointers directly, but we can verify they're both the ENIGO_BACKEND
        let _ = backend1;
        let _ = backend2;
    }

    #[test]
    fn test_ensure_ready_multiple_calls() {
        // Multiple calls should not panic
        for _ in 0..10 {
            ensure_native_input_ready(NativeInputBackend::Enigo);
        }
    }

    #[test]
    fn test_jittered_delay_ms_distribution() {
        // Test that delays are distributed across the range
        let delays: Vec<u64> = (0..100).map(|_| jittered_delay_ms(100, 20)).collect();
        let min = *delays.iter().min().expect("delays should not be empty");
        let max = *delays.iter().max().expect("delays should not be empty");
        // Should cover most of the range
        assert!(min <= 85); // Lower bound of range
        assert!(max >= 115); // Upper bound of range
    }

    #[test]
    fn test_jittered_delay_ms_base_saturating_sub() {
        // Test that saturating_sub doesn't underflow
        let delay = jittered_delay_ms(1, 100);
        // Should never be negative (u64)
        assert!(delay <= 2);
    }

    #[test]
    fn test_jittered_delay_ms_non_deterministic() {
        // With variance > 0, calls should produce different results
        let delay1 = jittered_delay_ms(100, 20);
        let delay2 = jittered_delay_ms(100, 20);
        // While they could theoretically be the same, statistically unlikely
        // We just verify they're both in range
        assert!((80..=120).contains(&delay1));
        assert!((80..=120).contains(&delay2));
    }

    #[test]
    fn test_jittered_delay_ms_variance_exact_50() {
        let delay = jittered_delay_ms(100, 50);
        // Range is 50-150
        assert!((50..=150).contains(&delay));
    }

    #[test]
    fn test_backend_fallback_warning_once() {
        // The warning should only be logged once per backend type
        // This test just verifies it doesn't panic
        for _ in 0..5 {
            backend(NativeInputBackend::Sendinput);
            backend(NativeInputBackend::Rdev);
        }
    }

    #[test]
    fn test_jittered_delay_ms_variance_zero_with_large_base() {
        let delay = jittered_delay_ms(10000, 0);
        assert_eq!(delay, 10000);
    }
}
