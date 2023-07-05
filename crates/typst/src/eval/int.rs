use std::num::{NonZeroI64, NonZeroIsize, NonZeroU64, NonZeroUsize};

use super::{cast, Value};

macro_rules! signed_int {
    ($($ty:ty)*) => {
        $(cast! {
            $ty,
            self => Value::Int(self as i64),
            v: i64 => v.try_into().map_err(|_| "number too large")?,
        })*
    }
}

macro_rules! unsigned_int {
    ($($ty:ty)*) => {
        $(cast! {
            $ty,
            self => Value::Int(self as i64),
            v: i64 => v.try_into().map_err(|_| {
                if v < 0 {
                    "number must be at least zero"
                } else {
                    "number too large"
                }
            })?,
        })*
    }
}

macro_rules! signed_nonzero {
    ($($ty:ty)*) => {
        $(cast! {
            $ty,
            self => Value::Int(self.get() as i64),
            v: i64 => v
                .try_into()
                .ok()
                .and_then($ty::new)
                .ok_or_else(|| if v == 0 {
                    "number must not be zero"
                } else {
                    "number too large"
                })?,
        })*
    }
}

macro_rules! unsigned_nonzero {
    ($($ty:ty)*) => {
        $(cast! {
            $ty,
            self => Value::Int(self.get() as i64),
            v: i64 => v
                .try_into()
                .ok()
                .and_then($ty::new)
                .ok_or_else(|| if v <= 0 {
                    "number must be positive"
                } else {
                    "number too large"
                })?,
        })*
    }
}

signed_int! {
    i8 i16 i32 isize
}

unsigned_int! {
    u8 u16 u32 u64 usize
}

signed_nonzero! {
    NonZeroI64 NonZeroIsize
}

unsigned_nonzero! {
    NonZeroU64  NonZeroUsize
}
