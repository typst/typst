use std::any::TypeId;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ptr::NonNull;
use std::sync::atomic::{self, AtomicUsize, Ordering};

use typst_syntax::Span;
use typst_utils::{fat, HashLock, SmallBitSet};

use super::vtable;
use crate::foundations::{Element, Label, NativeElement, Packed};
use crate::introspection::Location;

/// The raw, low-level implementation of content.
///
/// The `ptr` + `elem` fields implement a fat pointer setup similar to an
/// `Arc<Inner<dyn Trait>>`, but in a manual way, allowing us to have a custom
/// [vtable].
pub struct RawContent {
    /// A type-erased pointer to an allocation containing two things:
    /// - A header that is the same for all elements
    /// - Element-specific `data` that holds the specific element
    ///
    /// This pointer is valid for both a `Header` and an `Inner<E>` where
    /// `E::ELEM == self.elem` and can be freely cast between both. This is
    /// possible because
    /// - `Inner<E>` is `repr(C)`
    /// - The first field of `Inner<E>` is `Header`
    /// - ISO/IEC 9899:TC2 C standard § 6.7.2.1 - 13 states that a pointer to a
    ///   structure "points to its initial member" with no padding at the start
    ptr: NonNull<Header>,
    /// Describes which kind of element this content holds. This is used for
    ///
    /// - Direct comparisons, e.g. `is::<HeadingElem>()`
    /// - Behavior: An `Element` is just a pointer to a `ContentVtable`
    ///   containing not just data, but also function pointers for various
    ///   element-specific operations that can be performed
    ///
    /// It is absolutely crucial that `elem == <E as NativeElement>::ELEM` for
    /// `Inner<E>` pointed to by `ptr`. Otherwise, things will go very wrong
    /// since we'd be using the wrong vtable.
    elem: Element,
    /// The content's span.
    span: Span,
}

/// The allocated part of an element's representation.
///
/// This is `repr(C)` to ensure that a pointer to the whole structure may be
/// cast to a pointer to its first field.
#[repr(C)]
struct Inner<E> {
    /// It is crucial that this is the first field because we cast between
    /// pointers to `Inner<E>` and pointers to `Header`. See the documentation
    /// of `RawContent::ptr` for more details.
    header: Header,
    /// The element struct. E.g. `E = HeadingElem`.
    data: E,
}

/// The header that is shared by all elements.
struct Header {
    /// The element's reference count. This works just like for `Arc`.
    /// Unfortunately, we have to reimplement reference counting because we
    /// have a custom fat pointer and `Arc` wouldn't know how to drop its
    /// contents. Something with `ManuallyDrop<Arc<_>>` might also work, but at
    /// that point we're not gaining much and with the way it's implemented now
    /// we can also skip the unnecessary weak reference count.
    refs: AtomicUsize,
    /// Metadata for the element.
    meta: Meta,
    /// A cell for memoizing the hash of just the `data` part of the content.
    hash: HashLock,
}

/// Metadata that elements can hold.
#[derive(Clone, Hash)]
pub(super) struct Meta {
    /// An optional label attached to the element.
    pub label: Option<Label>,
    /// The element's location which identifies it in the laid-out output.
    pub location: Option<Location>,
    /// Manages the element during realization.
    /// - If bit 0 is set, the element is prepared.
    /// - If bit n is set, the element is guarded against the n-th show rule
    ///   recipe from the top of the style chain (counting from 1).
    pub lifecycle: SmallBitSet,
}

impl RawContent {
    /// Creates raw content wrapping an element, with all metadata set to
    /// default (including a detached span).
    pub(super) fn new<E: NativeElement>(data: E) -> Self {
        Self::create(
            data,
            Meta {
                label: None,
                location: None,
                lifecycle: SmallBitSet::new(),
            },
            HashLock::new(),
            Span::detached(),
        )
    }

    /// Creates and allocates raw content.
    fn create<E: NativeElement>(data: E, meta: Meta, hash: HashLock, span: Span) -> Self {
        let raw = Box::into_raw(Box::<Inner<E>>::new(Inner {
            header: Header { refs: AtomicUsize::new(1), meta, hash },
            data,
        }));

        // Safety: `Box` always holds a non-null pointer. See also
        // `Box::into_non_null` (which is unstable).
        let non_null = unsafe { NonNull::new_unchecked(raw) };

        // Safety: See `RawContent::ptr`.
        let ptr = non_null.cast::<Header>();

        Self { ptr, elem: E::ELEM, span }
    }

    /// Destroys raw content and deallocates.
    ///
    /// # Safety
    /// - The reference count must be zero.
    /// - The raw content must be be of type `E`.
    pub(super) unsafe fn drop_impl<E: NativeElement>(&mut self) {
        debug_assert_eq!(self.header().refs.load(Ordering::Relaxed), 0);

        // Safety:
        // - The caller guarantees that the content is of type `E`.
        // - Thus, `ptr` must have been created from `Box<Inner<E>>` (see
        //   `RawContent::ptr`).
        // - And to clean it up, we can just reproduce our box.
        unsafe {
            let ptr = self.ptr.cast::<Inner<E>>();
            drop(Box::<Inner<E>>::from_raw(ptr.as_ptr()));
        }
    }

    /// Clones a packed element into new raw content.
    pub(super) fn clone_impl<E: NativeElement>(elem: &Packed<E>) -> Self {
        let raw = &elem.as_content().0;
        let header = raw.header();
        RawContent::create(
            elem.as_ref().clone(),
            header.meta.clone(),
            header.hash.clone(),
            raw.span,
        )
    }

    /// Accesses the header part of the raw content.
    fn header(&self) -> &Header {
        // Safety: `self.ptr` is a valid pointer to a header structure.
        unsafe { self.ptr.as_ref() }
    }

    /// Mutably accesses the header part of the raw content.
    fn header_mut(&mut self) -> &mut Header {
        self.make_unique();

        // Safety:
        // - `self.ptr` is a valid pointer to a header structure.
        // - We have unique access to the backing allocation (just ensured).
        unsafe { self.ptr.as_mut() }
    }

    /// Retrieves the contained element **without checking that the content is
    /// of the correct type.**
    ///
    /// # Safety
    /// This must be preceded by a check to [`is`]. The safe API for this is
    /// [`Content::to_packed`] and the [`Packed`] struct.
    pub(super) unsafe fn data<E: NativeElement>(&self) -> &E {
        debug_assert!(self.is::<E>());

        // Safety:
        // - The caller guarantees that the content is of type `E`.
        // - `self.ptr` is a valid pointer to an `Inner<E>` (see
        //   `RawContent::ptr`).
        unsafe { &self.ptr.cast::<Inner<E>>().as_ref().data }
    }

    /// Retrieves the contained element mutably **without checking that the
    /// content is of the correct type.**
    ///
    /// Ensures that the element's allocation is unique.
    ///
    /// # Safety
    /// This must be preceded by a check to [`is`]. The safe API for this is
    /// [`Content::to_packed_mut`] and the [`Packed`] struct.
    pub(super) unsafe fn data_mut<E: NativeElement>(&mut self) -> &mut E {
        debug_assert!(self.is::<E>());

        // Ensure that the memoized hash is reset because we may mutate the
        // element.
        self.header_mut().hash.reset();

        // Safety:
        // - The caller guarantees that the content is of type `E`.
        // - `self.ptr` is a valid pointer to an `Inner<E>` (see
        //   `RawContent::ptr`).
        // - We have unique access to the backing allocation (due to header_mut).
        unsafe { &mut self.ptr.cast::<Inner<E>>().as_mut().data }
    }

    /// Ensures that we have unique access to the backing allocation by cloning
    /// if the reference count exceeds 1. This is used before performing
    /// mutable operations, implementing a clone-on-write scheme.
    fn make_unique(&mut self) {
        if self.header().refs.load(Ordering::Relaxed) > 1 {
            *self = self.handle().clone();
        }
    }

    /// Retrieves the element this content is for.
    pub(super) fn elem(&self) -> Element {
        self.elem
    }

    /// Whether this content holds an element of type `E`.
    pub(super) fn is<E: NativeElement>(&self) -> bool {
        self.elem == E::ELEM
    }

    /// Retrieves the content's span.
    pub(super) fn span(&self) -> Span {
        self.span
    }

    /// Retrieves the content's span mutably.
    pub(super) fn span_mut(&mut self) -> &mut Span {
        &mut self.span
    }

    /// Retrieves the content's metadata.
    pub(super) fn meta(&self) -> &Meta {
        &self.header().meta
    }

    /// Retrieves the content's metadata mutably.
    pub(super) fn meta_mut(&mut self) -> &mut Meta {
        &mut self.header_mut().meta
    }

    /// Casts into a trait object for a given trait if the packed element
    /// implements said trait.
    pub(super) fn with<C>(&self) -> Option<&C>
    where
        C: ?Sized + 'static,
    {
        // Safety: The vtable comes from the `Capable` implementation which
        // guarantees to return a matching vtable for `Packed<T>` and `C`. Since
        // any `Packed<T>` is repr(transparent) with `Content` and `RawContent`,
        // we can also use a `*const RawContent` pointer.
        let vtable = (self.elem.vtable().capability)(TypeId::of::<C>())?;
        let data = self as *const Self as *const ();
        Some(unsafe { &*fat::from_raw_parts(data, vtable.as_ptr()) })
    }

    /// Casts into a mutable trait object for a given trait if the packed
    /// element implements said trait.
    pub(super) fn with_mut<C>(&mut self) -> Option<&mut C>
    where
        C: ?Sized + 'static,
    {
        // Safety: The vtable comes from the `Capable` implementation which
        // guarantees to return a matching vtable for `Packed<T>` and `C`. Since
        // any `Packed<T>` is repr(transparent) with `Content` and `RawContent`,
        // we can also use a `*const Content` pointer.
        //
        // The resulting trait object contains an `&mut Packed<T>`. We do _not_
        // need to ensure that we hold the only reference to the `Arc` here
        // because `Packed<T>`'s DerefMut impl will take care of that if mutable
        // access is required.
        let vtable = (self.elem.vtable().capability)(TypeId::of::<C>())?;
        let data = self as *mut Self as *mut ();
        Some(unsafe { &mut *fat::from_raw_parts_mut(data, vtable.as_ptr()) })
    }
}

impl RawContent {
    /// Retrieves the element's vtable.
    pub(super) fn handle(&self) -> vtable::ContentHandle<&RawContent> {
        // Safety `self.elem.vtable()` is a matching vtable for `self`.
        unsafe { vtable::Handle::new(self, self.elem.vtable()) }
    }

    /// Retrieves the element's vtable.
    pub(super) fn handle_mut(&mut self) -> vtable::ContentHandle<&mut RawContent> {
        // Safety `self.elem.vtable()` is a matching vtable for `self`.
        unsafe { vtable::Handle::new(self, self.elem.vtable()) }
    }

    /// Retrieves the element's vtable.
    pub(super) fn handle_pair<'a, 'b>(
        &'a self,
        other: &'b RawContent,
    ) -> Option<vtable::ContentHandle<(&'a RawContent, &'b RawContent)>> {
        (self.elem == other.elem).then(|| {
            // Safety:
            // - `self.elem.vtable()` is a matching vtable for `self`.
            // - It's also matching for `other` because `self.elem == other.elem`.
            unsafe { vtable::Handle::new((self, other), self.elem.vtable()) }
        })
    }
}

impl Debug for RawContent {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.handle().debug(f)
    }
}

impl Clone for RawContent {
    fn clone(&self) -> Self {
        // See Arc's clone impl for details about memory ordering.
        let prev = self.header().refs.fetch_add(1, Ordering::Relaxed);

        // See Arc's clone impl details about guarding against incredibly
        // degenerate programs.
        if prev > isize::MAX as usize {
            ref_count_overflow(self.ptr, self.elem, self.span);
        }

        Self { ptr: self.ptr, elem: self.elem, span: self.span }
    }
}

impl Drop for RawContent {
    fn drop(&mut self) {
        // Drop our ref-count. If there was more than one content before
        // (including this one), we shouldn't deallocate. See Arc's drop impl
        // for details about memory ordering.
        if self.header().refs.fetch_sub(1, Ordering::Release) != 1 {
            return;
        }

        // See Arc's drop impl for details.
        atomic::fence(Ordering::Acquire);

        // Safety:
        // No other content references the backing allocation (just checked)
        unsafe {
            self.handle_mut().drop();
        }
    }
}

impl PartialEq for RawContent {
    fn eq(&self, other: &Self) -> bool {
        let Some(handle) = self.handle_pair(other) else { return false };
        handle
            .eq()
            .unwrap_or_else(|| handle.fields().all(|handle| handle.eq()))
    }
}

impl Hash for RawContent {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.elem.hash(state);
        let header = self.header();
        header.meta.hash(state);
        header.hash.get_or_insert_with(|| self.handle().hash()).hash(state);
        self.span.hash(state);
    }
}

// Safety:
// - Works like `Arc`.
// - `NativeElement` implies `Send` and `Sync`, see below.
unsafe impl Sync for RawContent {}
unsafe impl Send for RawContent {}

fn _ensure_send_sync<T: NativeElement>() {
    fn needs_send_sync<T: Send + Sync>() {}
    needs_send_sync::<T>();
}

#[cold]
fn ref_count_overflow(ptr: NonNull<Header>, elem: Element, span: Span) -> ! {
    // Drop to decrement the ref count to counter the increment in `clone()`
    drop(RawContent { ptr, elem, span });
    panic!("reference count overflow");
}
