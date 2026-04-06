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

/// Whether streaming (non-memoized) layout mode is active.
/// When true, all `#[comemo::memoize]` layout functions bypass their cache.
/// Set during Phase 2 of two-phase compilation, after convergence.
/// Must be AtomicBool (not thread-local) because engine.parallelize() uses rayon.
static STREAMING_MODE: AtomicBool = AtomicBool::new(false);

/// Enable streaming layout mode (Phase 2: no memoization).
pub fn enable_streaming_mode() {
    STREAMING_MODE.store(true, Ordering::Relaxed);
}

/// Disable streaming layout mode (back to normal memoized layout).
pub fn disable_streaming_mode() {
    STREAMING_MODE.store(false, Ordering::Relaxed);
}

/// Check if streaming mode is currently active.
pub fn is_streaming_mode() -> bool {
    STREAMING_MODE.load(Ordering::Relaxed)
}
