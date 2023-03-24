#[allow(clippy::wildcard_imports /* this module exists to reduce file size, not to introduce a new scope */)]
use super::*;

/// A scale-skew-translate transformation.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Transform {
    /// Scaling along the X axis.
    pub sx: Ratio,
    /// Skew along the Y axis.
    pub ky: Ratio,
    /// Skew along the X axis.
    pub kx: Ratio,
    /// Scaling along the Y axis.
    pub sy: Ratio,
    /// Translation along the X axis.
    pub tx: Abs,
    /// Translation along the Y axis.
    pub ty: Abs,
}

impl Transform {
    /// The identity transformation.
    #[must_use]
    #[inline]
    pub const fn identity() -> Self {
        Self {
            sx: Ratio::one(),
            ky: Ratio::zero(),
            kx: Ratio::zero(),
            sy: Ratio::one(),
            tx: Abs::zero(),
            ty: Abs::zero(),
        }
    }

    /// A translate transform.
    #[must_use]
    #[inline]
    pub const fn translate(tx: Abs, ty: Abs) -> Self {
        Self { tx, ty, ..Self::identity() }
    }

    /// A scale transform.
    #[must_use]
    #[inline]
    pub const fn scale(sx: Ratio, sy: Ratio) -> Self {
        Self { sx, sy, ..Self::identity() }
    }

    /// A rotate transform.
    #[must_use]
    #[inline]
    pub fn rotate(angle: Angle) -> Self {
        let cos = Ratio::new(angle.cos());
        let sin = Ratio::new(angle.sin());
        Self {
            sx: cos,
            ky: sin,
            kx: -sin,
            sy: cos,
            ..Self::default()
        }
    }

    /// Whether this is the identity transformation.
    #[must_use]
    #[inline]
    pub fn is_identity(self) -> bool {
        self == Self::identity()
    }

    /// Pre-concatenate another transformation, i.e., perform `prev` before `self`.
    #[must_use]
    #[inline]
    pub fn pre_concat(self, prev: Self) -> Self {
        Transform {
            sx: self.sx * prev.sx + self.kx * prev.ky,
            ky: self.ky * prev.sx + self.sy * prev.ky,
            kx: self.sx * prev.kx + self.kx * prev.sy,
            sy: self.ky * prev.kx + self.sy * prev.sy,
            tx: self.sx.of(prev.tx) + self.kx.of(prev.ty) + self.tx,
            ty: self.ky.of(prev.tx) + self.sy.of(prev.ty) + self.ty,
        }
    }

    /// Post-concatenate another transformation, i.e., perform `next` after `self`.
    #[must_use]
    #[inline]
    pub fn post_concat(self, next: Self) -> Self {
        next.pre_concat(self)
    }
}

impl Default for Transform {
    #[inline]
    fn default() -> Self {
        Self::identity()
    }
}
