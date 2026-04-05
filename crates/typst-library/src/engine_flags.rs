//! Global flags for controlling optimization behavior during compilation.

use std::sync::atomic::{AtomicBool, Ordering};

/// Whether memory-saving eviction should be performed during layout.
/// Set to true during the first convergence iteration (when we can tolerate
/// cache misses) and false during subsequent iterations (where cache hits
/// from the first iteration speed up validation).
static EVICTION_ENABLED: AtomicBool = AtomicBool::new(true);

/// Enable layout-time cache eviction (first convergence iteration).
pub fn enable_layout_eviction() {
    EVICTION_ENABLED.store(true, Ordering::Relaxed);
}

/// Disable layout-time cache eviction (subsequent convergence iterations).
pub fn disable_layout_eviction() {
    EVICTION_ENABLED.store(false, Ordering::Relaxed);
}

/// Check if layout-time eviction is currently enabled.
pub fn is_layout_eviction_enabled() -> bool {
    EVICTION_ENABLED.load(Ordering::Relaxed)
}
