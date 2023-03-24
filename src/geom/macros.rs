/// Implement the `Sub` trait based on existing `Neg` and `Add` impls.
macro_rules! sub_impl {
    ($a:ident - $b:ident -> $c:ident) => {
        impl Sub<$b> for $a {
            type Output = $c;

            #[inline]
            fn sub(self, other: $b) -> $c {
                self + -other
            }
        }
    };
}

/// Implement an assign trait based on an existing non-assign trait.
macro_rules! assign_impl {
    ($a:ident += $b:ident) => {
        impl AddAssign<$b> for $a {
            #[inline]
            fn add_assign(&mut self, other: $b) {
                *self = *self + other;
            }
        }
    };

    ($a:ident -= $b:ident) => {
        impl SubAssign<$b> for $a {
            #[inline]
            fn sub_assign(&mut self, other: $b) {
                *self = *self - other;
            }
        }
    };

    ($a:ident *= $b:ident) => {
        impl MulAssign<$b> for $a {
            #[inline]
            fn mul_assign(&mut self, other: $b) {
                *self = *self * other;
            }
        }
    };

    ($a:ident /= $b:ident) => {
        impl DivAssign<$b> for $a {
            #[inline]
            fn div_assign(&mut self, other: $b) {
                *self = *self / other;
            }
        }
    };
}
