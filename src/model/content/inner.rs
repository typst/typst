use std::{
    alloc::Layout,
    cmp,
    fmt::Debug,
    hash::Hash,
    mem,
    process::abort,
    ptr,
    ptr::NonNull,
    sync::atomic::{self, AtomicUsize, Ordering},
};

use comemo::Prehashed;
use ecow::EcoString;

use crate::{
    eval::Value,
    model::{Guard, Location, Style, Styles},
    syntax::Span,
};

use super::Content;

pub struct ContentInner {
    ptr: NonNull<ContentHeader>,
}

unsafe impl Send for ContentInner {}
unsafe impl Sync for ContentInner {}

impl Debug for ContentInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContentInner")
            .field("inner", &self.inner())
            .field("tail", &self.slice())
            .finish()
    }
}

impl Default for ContentInner {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentInner {
    pub const BASE_CAPACITY: usize = 4;

    /// Creates a new content inner with a base capacity.
    pub fn new() -> Self {
        Self::with_capacity(Self::BASE_CAPACITY)
    }

    /// Create a new dangling inner.
    /// Calling any method on the returned element is undefined behaviour.
    #[doc(hidden)]
    pub const unsafe fn dangling() -> Self {
        Self { ptr: NonNull::dangling() }
    }

    /// Creates a new content inner with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_header_iter_cap(
            ContentHeader::new(capacity),
            std::iter::empty(),
            capacity,
        )
    }

    /// Creates a new content inner with the given iterator.
    pub fn with_iter(iter: impl IntoIterator<Item = ContentTailItem>) -> Self {
        let iter = iter.into_iter();
        let (low, high) = iter.size_hint();
        let cap = high.unwrap_or(low).max(Self::BASE_CAPACITY);

        Self::with_header_iter_cap(ContentHeader::new(cap), iter, cap)
    }

    /// Creates a new content inner with the given iterator and capacity.
    pub fn with_header_iter_cap(
        mut header: ContentHeader,
        iter: impl IntoIterator<Item = ContentTailItem>,
        capacity: usize,
    ) -> Self {
        let layout = Self::layout(capacity.max(Self::BASE_CAPACITY));
        let ptr = unsafe { std::alloc::alloc(layout) as *mut ContentHeader };

        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }

        let ptr: NonNull<ContentHeader> = unsafe { NonNull::new_unchecked(ptr) };

        header.cap = capacity;
        header.strong = AtomicUsize::new(1);
        unsafe {
            ptr::write(ptr.as_ptr(), header);
        }

        let mut this = Self { ptr };

        for item in iter {
            this.push(item);
        }

        this
    }

    /// Creates a deep copy of the content inner. This means that the new
    /// copy will have its own strong reference count and be decoupled
    /// from the original.
    pub fn deep_clone(&self) -> Self {
        Self::with_header_iter_cap(
            self.inner().cloned(),
            self.slice().iter().cloned(),
            self.capacity(),
        )
    }

    /// Creates a new content inner if there are more than one strong
    /// references to the content inner otherwise returns a mutable
    /// reference to the inner value.
    pub fn make_mut(&mut self) -> &mut ContentHeader {
        if self.strong_count() > 1 {
            *self = self.deep_clone();
        }

        unsafe { self.inner_mut_unchecked() }
    }

    /// Creates a new content inner.
    pub fn capacity(&self) -> usize {
        self.inner().cap
    }

    /// Returns the number of items in the content inner.
    pub fn len(&self) -> usize {
        self.inner().len
    }

    /// Returns the number of strong references to the content inner.
    pub fn strong_count(&self) -> usize {
        self.inner().strong.load(Ordering::Acquire)
    }

    /// Returns the inner value.
    pub fn inner(&self) -> &ContentHeader {
        unsafe { self.ptr.as_ref() }
    }

    /// Returns a mutable reference to the inner value.
    pub unsafe fn inner_mut_unchecked(&mut self) -> &mut ContentHeader {
        self.ptr.as_mut()
    }

    /// Returns a slice of the content tail items.
    pub fn slice(&self) -> &[ContentTailItem] {
        unsafe { std::slice::from_raw_parts(self.data(), self.len()) }
    }

    /// Returns a mutable slice of the content tail items if there are no
    /// other strong references to the content inner.
    pub fn slice_mut(&mut self) -> Option<&mut [ContentTailItem]> {
        if self.strong_count() > 1 {
            return None;
        }

        Some(unsafe { std::slice::from_raw_parts_mut(self.data_mut(), self.len()) })
    }

    /// Returns a reference to the style of the content.
    pub fn style(&self) -> Option<&Styles> {
        self.inner().style.as_ref()
    }

    /// Returns a reference to the location of the content.
    pub fn location(&self) -> Option<Location> {
        self.inner().location
    }

    /// Returns a reference to the span of the content.
    pub fn span(&self) -> Option<&Span> {
        self.inner().span.as_ref()
    }

    /// Sets the span of the content.
    pub fn push_span(&mut self, span: Span) {
        let inner = self.make_mut();
        inner.span = Some(span);
    }

    /// Applies the given styles to the content.
    pub fn apply_style(&mut self, style: Style) {
        let inner = self.make_mut();
        if let Some(local) = &mut inner.style {
            local.apply_one(style);
        } else {
            inner.style = Some(style.into());
        }
    }

    /// Applies the given styles to the content.
    pub fn apply_styles(&mut self, styles: Styles) {
        let inner = self.make_mut();
        if let Some(local) = &mut inner.style {
            local.apply(styles);
        } else {
            inner.style = Some(styles);
        }
    }

    /// Sets the location of the content.
    pub fn push_location(&mut self, location: Location) {
        if self.location().is_some() {
            return;
        }

        let inner = self.make_mut();
        inner.location = Some(location);
    }

    /// Whether the content is prepared.
    pub fn is_prepared(&self) -> bool {
        self.inner().prepared
    }

    /// Sets the prepared flag of the content.
    pub fn set_prepared(&mut self, prepared: bool) {
        let inner = self.make_mut();
        inner.prepared = prepared;
    }

    /// Grows the content inner to the given capacity.
    fn grow(&mut self, target: usize) {
        debug_assert!(target > self.capacity());

        // Maintain the `capacity <= isize::MAX` invariant.
        if target > isize::MAX as usize {
            capacity_overflow();
        }

        self.make_mut();

        let ptr = self.ptr.as_ptr();
        let cap = self.capacity();

        if target >= cap {
            unsafe {
                let layout = Self::layout(target);
                let new_size = Self::size(target);

                let new_ptr: *mut ContentHeader = std::alloc::realloc(
                    ptr.cast(),
                    Self::layout(self.capacity()),
                    new_size,
                )
                .cast();

                if new_ptr.is_null() {
                    std::alloc::handle_alloc_error(layout);
                }

                self.ptr = NonNull::new_unchecked(new_ptr);
                self.inner_mut_unchecked().cap = target;
            }
        }
    }

    /// Extends this content with the given content.
    pub fn extend(&mut self, mut other: Self) {
        let other_len = other.len();

        if self.capacity() <= self.len() + other_len {
            self.grow((self.len() + other_len).max(self.capacity() * 2));
        }

        if other.strong_count() > 1 {
            for value in other.slice() {
                self.push(value.clone());
            }

            return;
        }

        unsafe {
            // We make sure that there is some room available
            // in the content inner.

            // We write the data
            ptr::copy_nonoverlapping(
                other.data(),
                self.data_mut().add(self.len()),
                other_len,
            );

            self.inner_mut_unchecked().len += other_len;

            // We prevent the other content from dropping its data.
            other.inner_mut_unchecked().len = 0;
        }
    }

    /// Inserts a new child at a given position in the tail.
    pub fn insert(&mut self, index: usize, child: ContentTailItem) {
        if index > self.len() {
            out_of_bounds(index, self.len());
        }

        unsafe {
            // We make sure that there is some room available
            // in the content inner.
            if self.len() == self.capacity() {
                self.grow(self.capacity() * 2);
            } else {
                self.make_mut();
            }

            let first = self.data_mut().add(index);
            ptr::copy(first, first.add(1), self.len() - index);

            first.write(child);

            self.inner_mut_unchecked().len += 1;
        }
    }

    /// Pushes a new item to the tail.
    pub fn push(&mut self, item: ContentTailItem) {
        self.make_mut();

        unsafe {
            // We make sure that there is some room available
            // in the content inner.
            if self.capacity() <= self.len() + 1 {
                self.grow(self.capacity() * 2);
            }

            // We write the data
            self.data_mut().add(self.len()).write(item);

            self.inner_mut_unchecked().len += 1;
        }
    }

    /// Pushes a new field to the content.
    pub fn push_field(&mut self, name: EcoString, value: Value) {
        self.make_mut();

        if let Some((_, local)) =
            self.fields_mut().and_then(|mut i| i.find(|(key, _)| *key == &name))
        {
            local.update(|local| *local = value);
            return;
        }

        unsafe {
            // We make sure that there is some room available
            // in the content inner.
            if self.capacity() < self.len() + 1 {
                self.grow(self.capacity() * 2);
            }

            // We write the data
            self.data_mut().add(self.len()).write(ContentTailItem::Field(
                Prehashed::new(name),
                Prehashed::new(value),
            ));

            self.inner_mut_unchecked().len += 1;
        }
    }

    /// Pushes a new child to the content.
    pub fn push_child(&mut self, child: Content) {
        self.push(ContentTailItem::Child(Prehashed::new(child)));
    }

    /// Pushes a new guard to the content.
    pub fn push_guard(&mut self, guard: Guard) {
        self.push(ContentTailItem::Guard(guard));
    }

    /// Returns an iterator over the children of the content.
    pub fn children(&self) -> impl Iterator<Item = &Content> {
        self.slice()
            .iter()
            .filter_map(ContentTailItem::child)
            .map(|child| &**child)
    }

    /// Returns true if the content has no children.
    pub fn is_childless(&self) -> bool {
        self.children().next().is_none()
    }

    /// Returns an iterator over the fields of the content.
    pub fn fields(&self) -> impl Iterator<Item = (&EcoString, &Value)> {
        self.slice()
            .iter()
            .filter_map(ContentTailItem::field)
            .map(|(key, value)| (&**key, &**value))
    }

    /// Returns an iterator over the fields of the content.
    pub fn fields_mut(
        &mut self,
    ) -> Option<impl Iterator<Item = (&EcoString, &mut Prehashed<Value>)>> {
        Some(
            self.slice_mut()?
                .iter_mut()
                .filter_map(ContentTailItem::field_mut)
                .map(|(key, value)| (&**key, &mut *value)),
        )
    }

    /// Returns an iterator over the guards of the content.
    pub fn guards(&self) -> impl Iterator<Item = &Guard> {
        self.slice().iter().filter_map(ContentTailItem::guard)
    }

    /// Returns an immutable pointer to the tail.
    fn data(&self) -> *const ContentTailItem {
        unsafe { self.ptr.as_ptr().add(1).cast::<ContentTailItem>() }
    }

    /// Returns a mutable pointer to the tail.
    fn data_mut(&mut self) -> *mut ContentTailItem {
        unsafe { self.ptr.as_ptr().add(1).cast::<ContentTailItem>() }
    }

    /// The size of a backing allocation for the given capacity.
    ///
    /// Always `> 0`. When rounded up to the next multiple of `Self::align()` is
    /// guaranteed to be `<= isize::MAX`.
    #[inline]
    fn size(capacity: usize) -> usize {
        mem::size_of::<ContentTailItem>()
            .checked_mul(capacity)
            .and_then(|a| a.checked_add(mem::size_of::<ContentHeader>()))
            .filter(|&size| size < isize::MAX as usize - Self::align())
            .unwrap_or_else(|| capacity_overflow())
    }

    /// The alignment of the backing allocation.
    #[inline]
    fn align() -> usize {
        cmp::max(mem::align_of::<ContentHeader>(), mem::align_of::<ContentTailItem>())
    }

    /// The layout of a backing allocation for the given capacity.
    #[inline]
    fn layout(capacity: usize) -> Layout {
        // Safety:
        // - `Self::size(capacity)` guarantees that it rounded up the alignment
        //   does not overflow `isize::MAX`.
        // - Since `Self::align()` is the header's alignment or T's alignment,
        //   it fulfills the requirements of a valid alignment.
        unsafe { Layout::from_size_align_unchecked(Self::size(capacity), Self::align()) }
    }
}

impl Clone for ContentInner {
    fn clone(&self) -> Self {
        // Note: See [`Arc::clone`] for the reasoning behind the ordering.

        let old_size = self.inner().strong.fetch_add(1, Ordering::Relaxed);

        if old_size > (isize::MAX) as usize {
            abort();
        }

        Self { ptr: self.ptr }
    }
}

impl Drop for ContentInner {
    fn drop(&mut self) {
        // Note: See [`Arc::drop`] for the reasoning behind the ordering.
        // Note: See [`EcoVec::drop`] for the reasoning behind the dealloc.

        if self.inner().strong.fetch_sub(1, Ordering::Release) != 1 {
            return;
        }

        // See Arc's drop impl for details.
        atomic::fence(Ordering::Acquire);

        // Ensures that the backing storage is deallocated even if one of the
        // element drops panics.
        struct Dealloc(*mut u8, Layout);

        impl Drop for Dealloc {
            fn drop(&mut self) {
                // Safety: See below.
                unsafe {
                    std::alloc::dealloc(self.0, self.1);
                }
            }
        }

        let _dealloc = Dealloc(self.ptr.as_ptr().cast(), Self::layout(self.capacity()));

        // Deallocate the header:
        unsafe {
            ptr::drop_in_place(self.ptr.as_ptr());
        }
        
        // Deallocate the children:
        unsafe {
            ptr::drop_in_place(ptr::slice_from_raw_parts_mut(
                self.data_mut(),
                self.len(),
            ));
        }
    }
}

impl Hash for ContentInner {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner().hash(state);
        self.slice().hash(state);
    }
}

#[derive(Debug)]
pub struct ContentHeader {
    /// The strong reference count of this element.
    strong: AtomicUsize,

    /// The length of the initialized tail.
    len: usize,

    /// The capacity of the tail.
    cap: usize,

    /// The span of this element.
    span: Option<Span>,

    /// The style chain of this element.
    style: Option<Styles>,

    /// The location of this element.
    location: Option<Location>,

    /// Whether this element is prepared or not.
    prepared: bool,
}

impl ContentHeader {
    pub fn new(cap: usize) -> Self {
        Self {
            span: None,
            style: None,
            location: None,
            prepared: false,
            strong: AtomicUsize::new(1),
            len: 0,
            cap,
        }
    }

    pub fn cloned(&self) -> Self {
        Self {
            span: self.span,
            style: self.style.clone(),
            location: self.location,
            prepared: self.prepared,
            strong: AtomicUsize::new(1),
            len: 0,
            cap: self.cap,
        }
    }
}

impl Hash for ContentHeader {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Note: we hash the slice separately and therefore we don't need
        // to hash the length and capacity.
        // Note: we don't hash the strong reference count.
        self.span.hash(state);
        self.style.hash(state);
        self.location.hash(state);
        self.prepared.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ContentTailItem {
    Child(Prehashed<Content>),
    Field(Prehashed<EcoString>, Prehashed<Value>),
    Guard(Guard),
}

impl ContentTailItem {
    pub fn child(&self) -> Option<&Prehashed<Content>> {
        match self {
            Self::Child(child) => Some(child),
            _ => None,
        }
    }

    pub fn field(&self) -> Option<(&Prehashed<EcoString>, &Prehashed<Value>)> {
        match self {
            Self::Field(key, value) => Some((key, value)),
            _ => None,
        }
    }

    pub fn field_mut(
        &mut self,
    ) -> Option<(&mut Prehashed<EcoString>, &mut Prehashed<Value>)> {
        match self {
            Self::Field(key, value) => Some((key, value)),
            _ => None,
        }
    }

    pub fn guard(&self) -> Option<&Guard> {
        match self {
            Self::Guard(guard) => Some(guard),
            _ => None,
        }
    }
}

#[cold]
fn capacity_overflow() -> ! {
    panic!("capacity overflow");
}

#[cold]
fn out_of_bounds(index: usize, len: usize) -> ! {
    panic!("index is out bounds (index: {index}, len: {len})");
}
