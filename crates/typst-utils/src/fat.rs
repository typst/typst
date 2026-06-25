//! Fat pointer handling.
//!
//! This assumes the memory representation of fat pointers. Although it is not
//! guaranteed by Rust, it's improbable that it will change. Still, when the
//! pointer metadata APIs are stable, we should definitely move to them:
//! <https://github.com/rust-lang/rust/issues/81513>

use std::alloc::Layout;
use std::any::Any;
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

/// Extract the address to a trait object's data pointer.
///
/// # Safety
/// Must only be called when `T` is a `dyn Trait`.
#[track_caller]
pub unsafe fn data<T: ?Sized>(ptr: *const T) -> NonNull<()> {
    unsafe {
        debug_assert_eq!(Layout::new::<*const T>(), Layout::new::<FatPointer>());
        NonNull::new_unchecked(
            mem::transmute_copy::<*const T, FatPointer>(&ptr).data as *mut (),
        )
    }
}

/// Extract the address to a trait object's data pointer.
///
/// # Safety
/// Must only be called when `T` is a `dyn Trait`.
#[track_caller]
pub unsafe fn data_mut<T: ?Sized>(ptr: *mut T) -> NonNull<()> {
    unsafe {
        debug_assert_eq!(Layout::new::<*mut T>(), Layout::new::<FatPointer>());
        NonNull::new_unchecked(
            mem::transmute_copy::<*mut T, FatPointer>(&ptr).data as *mut (),
        )
    }
}

/// Convert a `Box<dyn Any>` into a `Box<dyn Trait>` using the vtable of a
/// reference of the exact concrete type that implements that trait.
///
/// # Safety
/// Must only be called when:
/// 1. `T` is a `dyn Trait`
/// 2. `obj_ref` and `cloned` are of the exact same concrete type
pub unsafe fn cast_box<T: ?Sized>(obj_ref: &T, cloned: Box<dyn Any>) -> Box<T> {
    let object_ptr = obj_ref as *const T;
    let vtable = unsafe { vtable::<T>(object_ptr) };

    let raw_box_ptr = Box::into_raw(cloned);
    let data = unsafe { data_mut(raw_box_ptr) };

    let fat = unsafe { from_raw_parts_mut::<T>(data.as_ptr(), vtable.as_ptr()) };

    unsafe { Box::from_raw(fat) }
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
