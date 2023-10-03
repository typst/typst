use super::*;

/// A scale-skew-translate transformation.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Transform {
    pub sx: Ratio,
    pub ky: Ratio,
    pub kx: Ratio,
    pub sy: Ratio,
    pub tx: Abs,
    pub ty: Abs,
}

impl Transform {
    /// The identity transformation.
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
    pub const fn translate(tx: Abs, ty: Abs) -> Self {
        Self { tx, ty, ..Self::identity() }
    }

    /// A scale transform.
    pub const fn scale(sx: Ratio, sy: Ratio) -> Self {
        Self { sx, sy, ..Self::identity() }
    }

    /// A rotate transform.
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
    pub fn is_identity(self) -> bool {
        self == Self::identity()
    }

    /// Pre-concatenate another transformation.
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

    /// Post-concatenate another transformation.
    pub fn post_concat(self, next: Self) -> Self {
        next.pre_concat(self)
    }

    /// Inverts the transformation.
    ///
    /// Returns `None` if the determinant of the matrix is zero.
    pub fn invert(self) -> Option<Self> {
        // Allow the trivial case to be inlined.
        if self.is_identity() {
            return Some(self);
        }

        // Fast path for scale-translate-only transforms.
        if self.kx.is_zero() && self.ky.is_zero() {
            if self.sx.is_zero() || self.sy.is_zero() {
                return Some(Self::translate(-self.tx, -self.ty));
            }

            let inv_x = 1.0 / self.sx;
            let inv_y = 1.0 / self.sy;
            return Some(Self {
                sx: Ratio::new(inv_x),
                ky: Ratio::zero(),
                kx: Ratio::zero(),
                sy: Ratio::new(inv_y),
                tx: -self.tx * inv_x,
                ty: -self.ty * inv_y,
            });
        }

        let det = self.sx * self.sy - self.kx * self.ky;
        if det.get() < 1e-12 {
            return None;
        }

        let inv_det = 1.0 / det;
        Some(Self {
            sx: (self.sy * inv_det),
            ky: (-self.ky * inv_det),
            kx: (-self.kx * inv_det),
            sy: (self.sx * inv_det),
            tx: Abs::pt(
                (self.kx.get() * self.ty.to_pt() - self.sy.get() * self.tx.to_pt())
                    * inv_det,
            ),
            ty: Abs::pt(
                (self.ky.get() * self.tx.to_pt() - self.sx.get() * self.ty.to_pt())
                    * inv_det,
            ),
        })
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}
