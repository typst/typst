use std::hash::{Hash, Hasher};
use std::sync::Arc;

use once_cell::sync::OnceCell;

/// A deferred value.
///
/// This is a value that is being executed in parallel and can be waited on.
pub struct Deferred<A, B> {
    initial: A,
    inner: Arc<OnceCell<B>>,
}

impl<A: Clone + Send + Sync + 'static, B: Send + Sync + 'static> Deferred<A, B> {
    /// Creates a new deferred value.
    pub fn new(initial: A, handler: impl FnOnce(A) -> B + Send + Sync + 'static) -> Self {
        let inner = Arc::new(OnceCell::new());
        let inner2 = Arc::clone(&inner);

        let initial_clone = initial.clone();
        rayon::spawn(move || {
            // Initialize the value if it hasn't been initialized yet.
            // We do this to avoid panicking in case it was set externally.
            inner2.get_or_init(|| handler(initial_clone));
        });

        Self { initial, inner }
    }

    /// Waits on the value to be initialized.
    pub fn wait(&self) -> &B {
        // Ensure that we yield until the deferred is done for WASM compatibility.
        while let Some(rayon::Yield::Executed) = rayon::yield_now() {}

        self.inner.wait()
    }
}

impl<A: Hash, B> Hash for Deferred<A, B> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.initial.hash(state);
    }
}

impl<A: Clone, B> Clone for Deferred<A, B> {
    fn clone(&self) -> Self {
        Self {
            initial: self.initial.clone(),
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<A: PartialEq, B> PartialEq for Deferred<A, B> {
    fn eq(&self, other: &Self) -> bool {
        self.initial == other.initial
    }
}

impl<A: Eq, B> Eq for Deferred<A, B> {}
