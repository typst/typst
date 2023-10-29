use std::{
    any::{Any, TypeId},
    fmt::{self, Debug},
    hash::{Hash, Hasher},
    mem::size_of,
};

// Make `Style` 2 cache line length.
const DEFAULT_SIZE: usize = 56;

#[repr(C)]
pub struct Block<const N: usize = DEFAULT_SIZE> {
    type_: TypeId,
    storage: Storage<N>,
}

unsafe impl<const N: usize> Send for Block<N> {}
unsafe impl<const N: usize> Sync for Block<N> {}

impl<const N: usize> Block<N> {
    pub fn new<T: Blockable>(value: T) -> Self {
        Self {
            type_: TypeId::of::<T>(),
            storage: Storage::of(value),
        }
    }

    pub fn as_ptr(&self) -> *const () {
        match &self.storage {
            Storage::Stack(_, data) => data.as_ptr() as *const (),
            Storage::Heap(_, data) => data.as_ref() as *const dyn Any as *const (),
        }
    }

    pub fn as_mut_ptr(&mut self) -> *mut () {
        match &mut self.storage {
            Storage::Stack(_, data) => data.as_mut_ptr() as *mut (),
            Storage::Heap(_, data) => data.as_mut() as *mut dyn Any as *mut (),
        }
    }

    pub fn downcast<T: Any>(&self) -> Option<&T> {
        match &self.storage {
            Storage::Stack(_, _) if self.type_ == TypeId::of::<T>() => {
                Some(unsafe { &*(self.as_ptr() as *const T) })
            }
            Storage::Heap(_, data) => data.downcast_ref(),
            _ => None,
        }
    }

    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        match &mut self.storage {
            Storage::Stack(_, data) if self.type_ == TypeId::of::<T>() => {
                Some(unsafe { &mut *(data.as_mut_ptr() as *mut T) })
            }
            Storage::Heap(_, data) => data.downcast_mut(),
            _ => None,
        }
    }
}

impl<const N: usize> Clone for Block<N> {
    fn clone(&self) -> Self {
        Self {
            type_: self.type_,
            storage: match &self.storage {
                Storage::Stack(vtable, data) => {
                    Storage::Stack(*vtable, (vtable.clone)(data.as_ptr() as *const ()))
                }
                Storage::Heap(vtable, data) => Storage::Heap(
                    *vtable,
                    (vtable.clone)(data.as_ref() as *const dyn Any as *const ()),
                ),
            },
        }
    }
}

impl<const N: usize> Hash for Block<N> {
    fn hash<H: Hasher>(&self, mut state: &mut H) {
        match &self.storage {
            Storage::Stack(vtable, _) => (vtable.common.hash)(self.as_ptr(), &mut state),
            Storage::Heap(vtable, _) => (vtable.common.hash)(self.as_ptr(), &mut state),
        }
    }
}

impl<const N: usize> Debug for Block<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.storage {
            Storage::Stack(vtable, _) => (vtable.common.debug)(self.as_ptr(), f),
            Storage::Heap(vtable, _) => (vtable.common.debug)(self.as_ptr(), f),
        }
    }
}

pub trait Blockable: Any + Clone + Hash + Send + Sync + Debug {}

impl<T: Any + Clone + Hash + Send + Sync + Debug> Blockable for T {}

enum Storage<const N: usize> {
    Stack(StackVTable<N>, [u8; N]),
    Heap(HeapVTable, Box<dyn Any + Send + Sync>),
}

impl<const N: usize> Storage<N> {
    fn of<T: Blockable>(value: T) -> Self {
        if size_of::<T>() > N {
            Self::Heap(HeapVTable::of::<T>(), Box::new(value))
        } else {
            let mut stack = [0; N];
            unsafe { std::ptr::write(stack.as_mut_ptr() as *mut T, value) }

            Self::Stack(StackVTable::of::<T>(), stack)
        }
    }
}

impl<const N: usize> Drop for Storage<N> {
    fn drop(&mut self) {
        if let Self::Stack(vtable, data) = self {
            (vtable.drop)(data.as_mut_ptr() as *mut ());
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CommonVTable {
    hash: fn(*const (), &mut dyn Hasher),
    debug: fn(*const (), &mut fmt::Formatter<'_>) -> fmt::Result,
}

impl CommonVTable {
    fn of<T: Blockable>() -> Self {
        Self {
            hash: |ptr, mut hasher| {
                let ptr = ptr as *const T;
                let value = unsafe { &*ptr };
                value.hash(&mut hasher)
            },
            debug: |ptr, formatter| {
                let ptr = ptr as *const T;
                let value = unsafe { &*ptr };
                value.fmt(formatter)
            },
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StackVTable<const N: usize> {
    common: CommonVTable,
    drop: fn(*mut ()),
    clone: fn(*const ()) -> [u8; N],
}

impl<const N: usize> StackVTable<N> {
    fn of<T: Blockable>() -> Self {
        Self {
            common: CommonVTable::of::<T>(),
            drop: |ptr| {
                let ptr = ptr as *mut T;
                unsafe { ptr.drop_in_place() }
            },
            clone: |ptr| {
                let ptr = ptr as *const T;
                let value = unsafe { &*ptr };

                let mut stack = [0; N];
                unsafe { std::ptr::write(stack.as_mut_ptr() as *mut T, value.clone()) }
                stack
            },
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct HeapVTable {
    common: CommonVTable,
    clone: fn(*const ()) -> Box<dyn Any + Send + Sync>,
}

impl HeapVTable {
    fn of<T: Blockable>() -> Self {
        Self {
            common: CommonVTable::of::<T>(),
            clone: |ptr| {
                let ptr = ptr as *const T;
                let value = unsafe { &*ptr };
                Box::new(value.clone())
            },
        }
    }
}
