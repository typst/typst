use std::sync::Arc;

use once_cell::sync::OnceCell;

/// A deferred value.
///
/// This is a value that is being executed in parallel and can be waited on.
#[repr(transparent)]
pub struct Deferred<T>(Arc<OnceCell<T>>);

impl<T: Send + Sync + 'static> Deferred<T> {
    /// Creates a new deferred value.
    pub fn new<A>(
        initial: A,
        handler: impl FnOnce(A) -> T + Send + Sync + 'static,
    ) -> Self
    where
        A: Send + 'static,
    {
        let inner = Arc::new(OnceCell::new());
        let inner2 = Arc::clone(&inner);
        rayon::spawn(move || {
            // Initialize the value if it hasn't been initialized yet.
            // We do this to avoid panicking in case it was set externally.
            inner2.get_or_init(|| handler(initial));
        });

        Self(inner)
    }

    /// Waits on the value to be initialized.
    pub fn wait(&self) -> &T {
        // Ensure that we yield until the deferred is done for WASM compatibility.
        while let Some(rayon::Yield::Executed) = rayon::yield_now() {}

        self.0.wait()
    }
}

impl<T> Clone for Deferred<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}
