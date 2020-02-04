//! Key types for identifying keyword arguments.

use crate::layout::prelude::*;
use super::values::AlignmentValue::{self, *};
use super::*;

use self::AxisKey::*;
use self::PaddingKey::*;


/// Key types are used to extract keyword arguments from
/// [`Objects`](crate::syntax::expr::Object). They represent the key part of a
/// keyword argument.
/// ```typst
/// [func: key=value]
///        ^^^
/// ```
///
/// # Example implementation
/// An implementation for the `AxisKey` that identifies layouting axes might
/// look as follows:
/// ```
/// # use typstc::syntax::func::Key;
/// # use typstc::syntax::span::Spanned;
/// # #[derive(Eq, PartialEq)] enum Axis { Horizontal, Vertical, Primary, Secondary }
/// # #[derive(Eq, PartialEq)] enum AxisKey { Specific(Axis), Generic(Axis) }
/// # use Axis::*;
/// # use AxisKey::*;
/// impl Key for AxisKey {
///     fn parse(key: Spanned<&str>) -> Option<Self> {
///         match key.v {
///             "horizontal" | "h" => Some(Specific(Horizontal)),
///             "vertical"   | "v" => Some(Specific(Vertical)),
///             "primary"    | "p" => Some(Generic(Primary)),
///             "secondary"  | "s" => Some(Generic(Secondary)),
///             _ => None,
///         }
///     }
/// }
/// ```
pub trait Key: Sized + Eq {
    /// Parse a key string into this type if it is valid for it.
    fn parse(key: Spanned<&str>) -> Option<Self>;
}

impl Key for String {
    fn parse(key: Spanned<&str>) -> Option<Self> {
        Some(key.v.to_string())
    }
}

impl<K: Key> Key for Spanned<K> {
    fn parse(key: Spanned<&str>) -> Option<Self> {
        K::parse(key).map(|v| Spanned { v, span: key.span })
    }
}

/// Implements [`Key`] for types that just need to match on strings.
macro_rules! key {
    ($type:ty, $($($p:pat)|* => $r:expr),* $(,)?) => {
        impl Key for $type {
            fn parse(key: Spanned<&str>) -> Option<Self> {
                match key.v {
                    $($($p)|* => Some($r)),*,
                    _ => None,
                }
            }
        }
    };
}

/// A key which identifies a layouting axis.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[allow(missing_docs)]
pub enum AxisKey {
    Generic(GenericAxis),
    Specific(SpecificAxis),
}

impl AxisKey {
    /// The generic version of this axis key in the given system of axes.
    pub fn to_generic(self, axes: LayoutAxes) -> GenericAxis {
        match self {
            Generic(axis) => axis,
            Specific(axis) => axis.to_generic(axes),
        }
    }

    /// The specific version of this axis key in the given system of axes.
    pub fn to_specific(self, axes: LayoutAxes) -> SpecificAxis {
        match self {
            Generic(axis) => axis.to_specific(axes),
            Specific(axis) => axis,
        }
    }
}

key!(AxisKey,
    "horizontal" | "h" => Specific(Horizontal),
    "vertical"   | "v" => Specific(Vertical),
    "primary"    | "p" => Generic(Primary),
    "secondary"  | "s" => Generic(Secondary),
);

/// A key which is equivalent to a [`AxisKey`] but uses typical extent keywords
/// instead of axis keywords, e.g. `width` instead of `horizontal`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ExtentKey(pub AxisKey);

key!(ExtentKey,
    "width"          | "w"  => ExtentKey(Specific(Horizontal)),
    "height"         | "h"  => ExtentKey(Specific(Vertical)),
    "primary-size"   | "ps" => ExtentKey(Generic(Primary)),
    "secondary-size" | "ss" => ExtentKey(Generic(Secondary)),
);

impl From<ExtentKey> for AxisKey {
    fn from(key: ExtentKey) -> AxisKey {
        key.0
    }
}

/// A key which identifies an axis, but alternatively allows for two positional
/// arguments with unspecified axes.
///
/// This type does not implement `Key` in itself since it cannot be parsed from
/// a string. Rather, [`AxisKeys`](AxisKey) and positional arguments should be
/// parsed separately and mapped onto this key, as happening in the
/// [`PosAxisMap`](super::maps::PosAxisMap).
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum PosAxisKey {
    /// The first positional argument.
    First,
    /// The second positional argument.
    Second,
    /// An axis keyword argument.
    Keyword(AxisKey),
}

/// An argument key which identifies a margin or padding target.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum PaddingKey<Axis> {
    /// All four sides should have the specified padding.
    All,
    /// Both sides of the given axis should have the specified padding.
    Both(Axis),
    /// Only the given side of the given axis should have the specified padding.
    Side(Axis, AlignmentValue),
}

key!(PaddingKey<AxisKey>,
    "horizontal" | "h" => Both(Specific(Horizontal)),
    "vertical"   | "v" => Both(Specific(Vertical)),
    "primary"    | "p" => Both(Generic(Primary)),
    "secondary"  | "s" => Both(Generic(Secondary)),

    "left"   => Side(Specific(Horizontal), Left),
    "right"  => Side(Specific(Horizontal), Right),
    "top"    => Side(Specific(Vertical),   Top),
    "bottom" => Side(Specific(Vertical),   Bottom),

    "primary-origin"    => Side(Generic(Primary),     Align(Origin)),
    "primary-end"       => Side(Generic(Primary),     Align(End)),
    "secondary-origin"  => Side(Generic(Secondary),   Align(Origin)),
    "secondary-end"     => Side(Generic(Secondary),   Align(End)),
    "horizontal-origin" => Side(Specific(Horizontal), Align(Origin)),
    "horizontal-end"    => Side(Specific(Horizontal), Align(End)),
    "vertical-origin"   => Side(Specific(Vertical),   Align(Origin)),
    "vertical-end"      => Side(Specific(Vertical),   Align(End)),
);
