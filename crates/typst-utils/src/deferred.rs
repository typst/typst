use std::sync::Arc;

use once_cell::sync::OnceCell;

/// A value that is lazily executed on another thread.
///
/// Execution will be started in the background and can be waited on.
pub struct Deferred<T>(Arc<OnceCell<T>>);

impl<T: Send + Sync + 'static> Deferred<T> {
    /// Creates a new deferred value.
    ///
    /// The closure will be called on a secondary thread such that the value
    /// can be initialized in parallel.
    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce() -> T + Send + Sync + 'static,
    {
        let inner = Arc::new(OnceCell::new());
        let cloned = Arc::clone(&inner);
        rayon::spawn(move || {
            // Initialize the value if it hasn't been initialized yet.
            // We do this to avoid panicking in case it was set externally.
            cloned.get_or_init(f);
        });
        Self(inner)
    }

    /// Waits on the value to be initialized.
    ///
    /// If the value has already been initialized, this will return
    /// immediately. Otherwise, this will block until the value is
    /// initialized in another thread.
    pub fn wait(&self) -> &T {
        // Fast path if the value is already available. We don't want to yield
        // to rayon in that case.
        if let Some(value) = self.0.get() {
            return value;
        }

        // Ensure that we yield to give the deferred value a chance to compute
        // single-threaded platforms (for WASM compatibility).
        while let Some(rayon::Yield::Executed) = rayon::yield_now() {}

        self.0.wait()
    }
}

impl<T> Clone for Deferred<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}
