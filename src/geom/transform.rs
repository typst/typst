use super::*;

/// A scale-skew-translate transformation.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Transform {
    pub sx: Relative,
    pub ky: Relative,
    pub kx: Relative,
    pub sy: Relative,
    pub tx: Length,
    pub ty: Length,
}

impl Transform {
    /// The identity transformation.
    pub const fn identity() -> Self {
        Self {
            sx: Relative::one(),
            ky: Relative::zero(),
            kx: Relative::zero(),
            sy: Relative::one(),
            tx: Length::zero(),
            ty: Length::zero(),
        }
    }

    /// A translation transform.
    pub const fn translation(tx: Length, ty: Length) -> Self {
        Self { tx, ty, ..Self::identity() }
    }

    /// A scaling transform.
    pub const fn scale(sx: Relative, sy: Relative) -> Self {
        Self { sx, sy, ..Self::identity() }
    }

    /// A rotation transform.
    pub fn rotation(angle: Angle) -> Self {
        let cos = Relative::new(angle.cos());
        let sin = Relative::new(angle.sin());
        Self {
            sx: cos,
            ky: sin,
            kx: -sin,
            sy: cos,
            ..Self::default()
        }
    }

    /// Whether this is the identity transformation.
    pub fn is_identity(&self) -> bool {
        *self == Self::identity()
    }

    /// Pre-concatenate another transformation.
    pub fn pre_concat(&self, prev: Self) -> Self {
        Transform {
            sx: self.sx * prev.sx + self.kx * prev.ky,
            ky: self.ky * prev.sx + self.sy * prev.ky,
            kx: self.sx * prev.kx + self.kx * prev.sy,
            sy: self.ky * prev.kx + self.sy * prev.sy,
            tx: self.sx.resolve(prev.tx) + self.kx.resolve(prev.ty) + self.tx,
            ty: self.ky.resolve(prev.tx) + self.sy.resolve(prev.ty) + self.ty,
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}
