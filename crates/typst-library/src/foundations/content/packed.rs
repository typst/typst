use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use typst_syntax::Span;

use crate::foundations::{Content, Label, NativeElement};
use crate::introspection::Location;

/// A packed element of a static type.
#[derive(Clone)]
#[repr(transparent)]
pub struct Packed<T: NativeElement>(
    /// Invariant: Must be of type `T`.
    Content,
    PhantomData<T>,
);

impl<T: NativeElement> Packed<T> {
    /// Pack element while retaining its static type.
    pub fn new(element: T) -> Self {
        // Safety: The element is known to be of type `T`.
        Packed(element.pack(), PhantomData)
    }

    /// Try to cast type-erased content into a statically known packed element.
    pub fn from_ref(content: &Content) -> Option<&Self> {
        if content.is::<T>() {
            // Safety:
            // - We have checked the type.
            // - Packed<T> is repr(transparent).
            return Some(unsafe { std::mem::transmute::<&Content, &Packed<T>>(content) });
        }
        None
    }

    /// Try to cast type-erased content into a statically known packed element.
    pub fn from_mut(content: &mut Content) -> Option<&mut Self> {
        if content.is::<T>() {
            // Safety:
            // - We have checked the type.
            // - Packed<T> is repr(transparent).
            return Some(unsafe {
                std::mem::transmute::<&mut Content, &mut Packed<T>>(content)
            });
        }
        None
    }

    /// Try to cast type-erased content into a statically known packed element.
    pub fn from_owned(content: Content) -> Result<Self, Content> {
        if content.is::<T>() {
            // Safety:
            // - We have checked the type.
            // - Packed<T> is repr(transparent).
            return Ok(unsafe { std::mem::transmute::<Content, Packed<T>>(content) });
        }
        Err(content)
    }

    /// Pack back into content.
    pub fn pack(self) -> Content {
        self.0
    }

    /// Extract the raw underlying element.
    pub fn unpack(self) -> T {
        // This function doesn't yet need owned self, but might in the future.
        (*self).clone()
    }

    /// The element's span.
    pub fn span(&self) -> Span {
        self.0.span()
    }

    /// Set the span of the element.
    pub fn spanned(self, span: Span) -> Self {
        Self(self.0.spanned(span), PhantomData)
    }

    /// Accesses the label of the element.
    pub fn label(&self) -> Option<Label> {
        self.0.label()
    }

    /// Accesses the location of the element.
    pub fn location(&self) -> Option<Location> {
        self.0.location()
    }

    /// Sets the location of the element.
    pub fn set_location(&mut self, location: Location) {
        self.0.set_location(location);
    }

    pub fn as_content(&self) -> &Content {
        &self.0
    }
}

impl<T: NativeElement> AsRef<T> for Packed<T> {
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T: NativeElement> AsMut<T> for Packed<T> {
    fn as_mut(&mut self) -> &mut T {
        self
    }
}

impl<T: NativeElement> Deref for Packed<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Safety: Packed<T> guarantees that the content is of element type `T`.
        unsafe { (self.0).0.data::<T>() }
    }
}

impl<T: NativeElement> DerefMut for Packed<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Safety: Packed<T> guarantees that the content is of element type `T`.
        unsafe { (self.0).0.data_mut::<T>() }
    }
}

impl<T: NativeElement + Debug> Debug for Packed<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: NativeElement> PartialEq for Packed<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: NativeElement> Hash for Packed<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}
