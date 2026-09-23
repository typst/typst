use std::ops::DerefMut;

/// Automatically calls a deferred function when the returned handle is dropped.
pub fn defer<T, F: FnOnce(&mut T)>(
    thing: &mut T,
    deferred: F,
) -> impl DerefMut<Target = T> {
    pub struct DeferHandle<'a, T, F: FnOnce(&mut T)> {
        thing: &'a mut T,
        deferred: Option<F>,
    }

    impl<T, F: FnOnce(&mut T)> Drop for DeferHandle<'_, T, F> {
        fn drop(&mut self) {
            std::mem::take(&mut self.deferred).expect("deferred function")(self.thing);
        }
    }

    impl<T, F: FnOnce(&mut T)> std::ops::Deref for DeferHandle<'_, T, F> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            self.thing
        }
    }

    impl<T, F: FnOnce(&mut T)> std::ops::DerefMut for DeferHandle<'_, T, F> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.thing
        }
    }

    DeferHandle { thing, deferred: Some(deferred) }
}
