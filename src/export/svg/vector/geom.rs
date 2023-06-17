use core::fmt;
use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

use typst::geom::{
    Abs as TypstAbs, Axes as TypstAxes, Point as TypstPoint, Ratio as TypstRatio,
    Scalar as TypstScalar, Transform as TypstTransform,
};

#[cfg(feature = "rkyv")]
use rkyv::{Archive, Deserialize as rDeser, Serialize as rSer};

/// Scalar value of Vector representation.
/// Note: Unlike Typst's Scalar, all lengths with Scalar type are in pt.
#[derive(Default, Clone, Copy)]
#[cfg_attr(feature = "rkyv", derive(Archive, rDeser, rSer))]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct Scalar(pub f32);

impl From<f32> for Scalar {
    fn from(float: f32) -> Self {
        Self(float)
    }
}

impl From<Scalar> for f32 {
    fn from(scalar: Scalar) -> Self {
        scalar.0
    }
}

impl From<TypstScalar> for Scalar {
    fn from(scalar: TypstScalar) -> Self {
        Self(scalar.0 as f32)
    }
}

impl From<Scalar> for TypstScalar {
    fn from(scalar: Scalar) -> Self {
        Self(scalar.0 as f64)
    }
}

impl From<TypstRatio> for Scalar {
    fn from(ratio: TypstRatio) -> Self {
        Self(ratio.get() as f32)
    }
}

impl From<TypstAbs> for Scalar {
    fn from(ratio: TypstAbs) -> Self {
        Self(ratio.to_pt() as f32)
    }
}

impl fmt::Debug for Scalar {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Eq for Scalar {}

impl PartialEq for Scalar {
    fn eq(&self, other: &Self) -> bool {
        assert!(!self.0.is_nan() && !other.0.is_nan(), "float is NaN");
        self.0 == other.0
    }
}

impl PartialEq<f32> for Scalar {
    fn eq(&self, other: &f32) -> bool {
        self == &Self(*other)
    }
}

impl Ord for Scalar {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).expect("float is NaN")
    }
}

impl PartialOrd for Scalar {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }

    fn lt(&self, other: &Self) -> bool {
        self.0 < other.0
    }

    fn le(&self, other: &Self) -> bool {
        self.0 <= other.0
    }

    fn gt(&self, other: &Self) -> bool {
        self.0 > other.0
    }

    fn ge(&self, other: &Self) -> bool {
        self.0 >= other.0
    }
}

impl std::ops::Neg for Scalar {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl Hash for Scalar {
    fn hash<H: Hasher>(&self, state: &mut H) {
        debug_assert!(!self.0.is_nan(), "float is NaN");
        // instead of bits we swap the bytes on platform with BigEndian
        self.0.to_le_bytes().hash(state);
    }
}

pub type Size = Axes<Scalar>;
// scalar in pt.
pub type Abs = Scalar;
pub type Point = Axes<Scalar>;
pub type Ratio = Scalar;

/// A container with a horizontal and vertical component.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "rkyv", derive(Archive, rDeser, rSer))]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct Axes<T> {
    /// The horizontal component.
    pub x: T,
    /// The vertical component.
    pub y: T,
}

impl<T> Axes<T> {
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl<U, T> From<TypstAxes<U>> for Axes<T>
where
    T: From<U>,
{
    fn from(typst_axes: TypstAxes<U>) -> Self {
        Self { x: typst_axes.x.into(), y: typst_axes.y.into() }
    }
}

impl<T, U> From<Axes<T>> for TypstAxes<U>
where
    T: Into<U>,
{
    fn from(axes: Axes<T>) -> Self {
        Self { x: axes.x.into(), y: axes.y.into() }
    }
}

impl From<TypstPoint> for Point {
    fn from(p: TypstPoint) -> Self {
        Self { x: p.x.into(), y: p.y.into() }
    }
}

/// A scale-skew-translate transformation.
#[repr(C)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "rkyv", derive(Archive, rDeser, rSer))]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct Transform {
    pub sx: Ratio,
    pub ky: Ratio,
    pub kx: Ratio,
    pub sy: Ratio,
    pub tx: Abs,
    pub ty: Abs,
}

impl From<TypstTransform> for Transform {
    fn from(typst_transform: TypstTransform) -> Self {
        Self {
            sx: typst_transform.sx.into(),
            ky: typst_transform.ky.into(),
            kx: typst_transform.kx.into(),
            sy: typst_transform.sy.into(),
            tx: typst_transform.tx.into(),
            ty: typst_transform.ty.into(),
        }
    }
}

impl From<tiny_skia::Transform> for Transform {
    fn from(skia_transform: tiny_skia::Transform) -> Self {
        Self {
            sx: skia_transform.sx.into(),
            ky: skia_transform.ky.into(),
            kx: skia_transform.kx.into(),
            sy: skia_transform.sy.into(),
            tx: skia_transform.tx.into(),
            ty: skia_transform.ty.into(),
        }
    }
}
