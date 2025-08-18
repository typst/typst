//! Fat pointer handling.
//!
//! This assumes the memory representation of fat pointers. Although it is not
//! guaranteed by Rust, it's improbable that it will change. Still, when the
//! pointer metadata APIs are stable, we should definitely move to them:
//! <https://github.com/rust-lang/rust/issues/81513>

use std::alloc::Layout;
use std::mem;
use std::ptr::NonNull;

/// Create a fat pointer from a data address and a vtable address.
///
/// # Safety
/// Must only be called when `T` is a `dyn Trait`. The data address must point
/// to a value whose type implements the trait of `T` and the `vtable` must have
/// been extracted with [`vtable`].
#[track_caller]
pub unsafe fn from_raw_parts<T: ?Sized>(data: *const (), vtable: *const ()) -> *const T {
    unsafe {
        let fat = FatPointer { data, vtable };
        debug_assert_eq!(Layout::new::<*const T>(), Layout::new::<FatPointer>());
        mem::transmute_copy::<FatPointer, *const T>(&fat)
    }
}

/// Create a mutable fat pointer from a data address and a vtable address.
///
/// # Safety
/// Must only be called when `T` is a `dyn Trait`. The data address must point
/// to a value whose type implements the trait of `T` and the `vtable` must have
/// been extracted with [`vtable`].
#[track_caller]
pub unsafe fn from_raw_parts_mut<T: ?Sized>(data: *mut (), vtable: *const ()) -> *mut T {
    unsafe {
        let fat = FatPointer { data, vtable };
        debug_assert_eq!(Layout::new::<*mut T>(), Layout::new::<FatPointer>());
        mem::transmute_copy::<FatPointer, *mut T>(&fat)
    }
}

/// Extract the address to a trait object's vtable.
///
/// # Safety
/// Must only be called when `T` is a `dyn Trait`.
#[track_caller]
pub unsafe fn vtable<T: ?Sized>(ptr: *const T) -> NonNull<()> {
    unsafe {
        debug_assert_eq!(Layout::new::<*const T>(), Layout::new::<FatPointer>());
        NonNull::new_unchecked(
            mem::transmute_copy::<*const T, FatPointer>(&ptr).vtable as *mut (),
        )
    }
}

/// The memory representation of a trait object pointer.
///
/// Although this is not guaranteed by Rust, it's improbable that it will
/// change.
#[repr(C)]
struct FatPointer {
    data: *const (),
    vtable: *const (),
}
