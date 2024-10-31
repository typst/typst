/// Create a lazy initialized, globally unique `'static` reference to a value.
#[macro_export]
macro_rules! singleton {
    ($ty:ty, $value:expr) => {{
        static VALUE: ::std::sync::LazyLock<$ty> = ::std::sync::LazyLock::new(|| $value);
        &*VALUE
    }};
}

/// Implement the `Sub` trait based on existing `Neg` and `Add` impls.
#[macro_export]
macro_rules! sub_impl {
    ($a:ident - $b:ident -> $c:ident) => {
        impl ::core::ops::Sub<$b> for $a {
            type Output = $c;

            fn sub(self, other: $b) -> $c {
                self + -other
            }
        }
    };
}

/// Implement an assign trait based on an existing non-assign trait.
#[macro_export]
macro_rules! assign_impl {
    ($a:ident += $b:ident) => {
        impl ::core::ops::AddAssign<$b> for $a {
            fn add_assign(&mut self, other: $b) {
                *self = *self + other;
            }
        }
    };

    ($a:ident -= $b:ident) => {
        impl ::core::ops::SubAssign<$b> for $a {
            fn sub_assign(&mut self, other: $b) {
                *self = *self - other;
            }
        }
    };

    ($a:ident *= $b:ident) => {
        impl ::core::ops::MulAssign<$b> for $a {
            fn mul_assign(&mut self, other: $b) {
                *self = *self * other;
            }
        }
    };

    ($a:ident /= $b:ident) => {
        impl ::core::ops::DivAssign<$b> for $a {
            fn div_assign(&mut self, other: $b) {
                *self = *self / other;
            }
        }
    };
}
