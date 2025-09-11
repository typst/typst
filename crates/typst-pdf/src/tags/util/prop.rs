//! Convenience methods to retrieve a property value by passing the default
//! stylechain.
//! Since in the PDF export all elements are materialized, meaning all of their
//! fields have been copied from the stylechain, there is no point in providing
//! any other stylechain.

use typst_library::foundations::{
    NativeElement, RefableProperty, Settable, SettableProperty, StyleChain,
};

pub trait PropertyValCopied<E, T, const I: u8> {
    /// Get the copied value.
    fn val(&self) -> T;
}

impl<E, T: Copy, const I: u8> PropertyValCopied<E, T, I> for Settable<E, I>
where
    E: NativeElement,
    E: SettableProperty<I, Type = T>,
{
    fn val(&self) -> T {
        self.get(StyleChain::default())
    }
}

pub trait PropertyValCloned<E, T, const I: u8> {
    /// Get the cloned value.
    fn val_cloned(&self) -> T;
}

impl<E, T: Clone, const I: u8> PropertyValCloned<E, T, I> for Settable<E, I>
where
    E: NativeElement,
    E: SettableProperty<I, Type = T>,
{
    fn val_cloned(&self) -> T {
        self.get_cloned(StyleChain::default())
    }
}

pub trait PropertyOptRef<E, T, const I: u8> {
    fn opt_ref(&self) -> Option<&T>;
}

impl<E, T, const I: u8> PropertyOptRef<E, T, I> for Settable<E, I>
where
    E: NativeElement,
    E: SettableProperty<I, Type = Option<T>>,
    E: RefableProperty<I>,
{
    /// Get an `Option` with a reference to the contained value.
    fn opt_ref(&self) -> Option<&T> {
        self.get_ref(StyleChain::default()).as_ref()
    }
}
