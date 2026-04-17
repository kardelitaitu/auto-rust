#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthState {
    Healthy,
    Unhealthy,
}

pub struct HealthMonitor {
    _session_id: String,
    failure_count: std::sync::atomic::AtomicUsize,
    is_healthy: std::sync::atomic::AtomicBool,
}

impl HealthMonitor {
    pub fn new(session_id: String) -> Self {
        Self {
            _session_id: session_id,
            failure_count: std::sync::atomic::AtomicUsize::new(0),
            is_healthy: std::sync::atomic::AtomicBool::new(true),
        }
    }

    pub fn state(&self) -> HealthState {
        if self.is_healthy.load(std::sync::atomic::Ordering::SeqCst) {
            HealthState::Healthy
        } else {
            HealthState::Unhealthy
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.is_healthy.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn mark_healthy(&self) {
        self.is_healthy.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn mark_unhealthy(&self) {
        self.is_healthy.store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn record_failure(&self) {
        self.failure_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn failure_count(&self) -> usize {
        self.failure_count.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn reset(&self) {
        self.failure_count.store(0, std::sync::atomic::Ordering::SeqCst);
        self.is_healthy.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}
