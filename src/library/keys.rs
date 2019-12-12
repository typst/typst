//! Keys for the consistent maps.

use super::*;

macro_rules! kind {
    ($type:ty, $name:expr, $($patterns:tt)*) => {
        impl $type {
            /// Parse this key from an identifier.
            pub fn from_ident(ident: &Spanned<Ident>) -> ParseResult<Self> {
                Ok(match ident.v.0.as_str() {
                    $($patterns)*
                    _ => error!("expected {}", <Self as ExpressionKind>::NAME),
                })
            }
        }

        impl ExpressionKind for $type {
            const NAME: &'static str = $name;

            fn from_expr(expr: Spanned<Expression>) -> ParseResult<Self> {
                if let Expression::Ident(ident) = expr.v {
                    Self::from_ident(&Spanned::new(ident, expr.span))
                } else {
                    error!("expected {}", Self::NAME);
                }
            }
        }
    };
}

/// An argument key which identifies a layouting axis.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum AxisKey {
    Primary,
    Secondary,
    Vertical,
    Horizontal,
}

impl AxisKey {
    /// The generic version of this axis key in the given system of axes.
    pub fn to_generic(self, axes: LayoutAxes) -> GenericAxis {
        match self {
            AxisKey::Primary => Primary,
            AxisKey::Secondary => Secondary,
            AxisKey::Vertical => Vertical.to_generic(axes),
            AxisKey::Horizontal => Horizontal.to_generic(axes),
        }
    }

    /// The specific version of this axis key in the given system of axes.
    pub fn to_specific(self, axes: LayoutAxes) -> SpecificAxis {
        match self {
            AxisKey::Primary => Primary.to_specific(axes),
            AxisKey::Secondary => Secondary.to_specific(axes),
            AxisKey::Vertical => Vertical,
            AxisKey::Horizontal => Horizontal,
        }
    }
}

/// An argument key which describes a target alignment.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum AlignmentKey {
    Left,
    Top,
    Right,
    Bottom,
    Origin,
    Center,
    End,
}

impl AlignmentKey {
    /// The generic axis this alignment key corresopnds to in the given system
    /// of layouting axes. Falls back to `default` if the alignment is generic.
    pub fn axis(self, axes: LayoutAxes, default: GenericAxis) -> GenericAxis {
        use AlignmentKey::*;
        match self {
            Origin | Center | End => default,
            Left | Right => Horizontal.to_generic(axes),
            Top | Bottom => Vertical.to_generic(axes),
        }
    }

    /// The generic version of this alignment in the given system of layouting
    /// axes.
    ///
    /// Returns an error if the alignment is invalid for the given axis.
    pub fn to_generic(self, axes: LayoutAxes, axis: GenericAxis) -> LayoutResult<Alignment> {
        let specific = axis.to_specific(axes);

        Ok(match (self, specific) {
            (AlignmentKey::Origin, _) => Origin,
            (AlignmentKey::Center, _) => Center,
            (AlignmentKey::End, _) => End,

            (AlignmentKey::Left, Horizontal) | (AlignmentKey::Top, Vertical) => {
                if axes.get_specific(specific).is_positive() { Origin } else { End }
            }

            (AlignmentKey::Right, Horizontal) | (AlignmentKey::Bottom, Vertical) => {
                if axes.get_specific(specific).is_positive() { End } else { Origin }
            }

            _ => error!(
                "invalid alignment `{}` for {} axis",
                format!("{:?}", self).to_lowercase(),
                format!("{:?}", axis).to_lowercase()
            )
        })
    }

    /// The specific version of this alignment in the given system of layouting
    /// axes.
    pub fn to_specific(self, axes: LayoutAxes, axis: SpecificAxis) -> AlignmentKey {
        use AlignmentKey::*;

        let positive = axes.get_specific(axis).is_positive();
        match (self, axis, positive) {
            (Origin, Horizontal, true) | (End, Horizontal, false) => Left,
            (End, Horizontal, true) | (Origin, Horizontal, false) => Right,
            (Origin, Vertical, true) | (End, Vertical, false) => Top,
            (End, Vertical, true) | (Origin, Vertical, false) => Bottom,
            _ => self,
        }
    }
}

/// An argument key which identifies a margin or padding target.
///
/// A is the used axis type.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum PaddingKey<A> {
    /// All four sides should have the specified padding.
    All,
    /// Both sides of the given axis should have the specified padding.
    Axis(A),
    /// Only the given side of the given axis should have the specified padding.
    AxisAligned(A, AlignmentKey),
}

kind!(AxisKey, "axis",
    "horizontal" | "h" => AxisKey::Horizontal,
    "vertical"   | "v" => AxisKey::Vertical,
    "primary"    | "p" => AxisKey::Primary,
    "secondary"  | "s" => AxisKey::Secondary,
);

kind!(AlignmentKey, "alignment",
    "left"   => AlignmentKey::Left,
    "top"    => AlignmentKey::Top,
    "right"  => AlignmentKey::Right,
    "bottom" => AlignmentKey::Bottom,
    "origin" => AlignmentKey::Origin,
    "center" => AlignmentKey::Center,
    "end"    => AlignmentKey::End,
);

kind!(PaddingKey<AxisKey>, "axis or side",
    "horizontal" | "h" => PaddingKey::Axis(AxisKey::Horizontal),
    "vertical"   | "v" => PaddingKey::Axis(AxisKey::Vertical),
    "primary"    | "p" => PaddingKey::Axis(AxisKey::Primary),
    "secondary"  | "s" => PaddingKey::Axis(AxisKey::Secondary),

    "left"   => PaddingKey::AxisAligned(AxisKey::Horizontal, AlignmentKey::Left),
    "right"  => PaddingKey::AxisAligned(AxisKey::Horizontal, AlignmentKey::Right),
    "top"    => PaddingKey::AxisAligned(AxisKey::Vertical,   AlignmentKey::Top),
    "bottom" => PaddingKey::AxisAligned(AxisKey::Vertical,   AlignmentKey::Bottom),

    "primary-origin"    => PaddingKey::AxisAligned(AxisKey::Primary,    AlignmentKey::Origin),
    "primary-end"       => PaddingKey::AxisAligned(AxisKey::Primary,    AlignmentKey::End),
    "secondary-origin"  => PaddingKey::AxisAligned(AxisKey::Secondary,  AlignmentKey::Origin),
    "secondary-end"     => PaddingKey::AxisAligned(AxisKey::Secondary,  AlignmentKey::End),
    "horizontal-origin" => PaddingKey::AxisAligned(AxisKey::Horizontal, AlignmentKey::Origin),
    "horizontal-end"    => PaddingKey::AxisAligned(AxisKey::Horizontal, AlignmentKey::End),
    "vertical-origin"   => PaddingKey::AxisAligned(AxisKey::Vertical,   AlignmentKey::Origin),
    "vertical-end"      => PaddingKey::AxisAligned(AxisKey::Vertical,   AlignmentKey::End),
);

kind!(Direction, "direction",
    "left-to-right" | "ltr" => LeftToRight,
    "right-to-left" | "rtl" => RightToLeft,
    "top-to-bottom" | "ttb" => TopToBottom,
    "bottom-to-top" | "btt" => BottomToTop,
);
