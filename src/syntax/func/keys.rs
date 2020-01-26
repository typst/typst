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
/// A key type has an associated output type, which is returned when parsing
/// this key from a string. Most of the time, the output type is simply the key
/// itself, as in the implementation for the [`AxisKey`]:
/// ```
/// # use typstc::syntax::func::Key;
/// # use typstc::syntax::span::Spanned;
/// # #[derive(Eq, PartialEq)] enum Axis { Horizontal, Vertical, Primary, Secondary }
/// # #[derive(Eq, PartialEq)] enum AxisKey { Specific(Axis), Generic(Axis) }
/// # use Axis::*;
/// # use AxisKey::*;
/// impl Key for AxisKey {
///     type Output = Self;
///
///     fn parse(key: Spanned<&str>) -> Option<Self::Output> {
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
///
/// The axis key would also be useful to identify axes when describing
/// dimensions of objects, as in `width=3cm`, because these are also properties
/// that are stored per axis. However, here the used keyword arguments are
/// actually different (`width` instead of `horizontal`)! Therefore we cannot
/// just use the axis key.
///
/// To fix this, there is another type [`ExtentKey`] which implements `Key` and
/// has the associated output type axis key. The extent key struct itself has no
/// fields and is only used to extract the axis key. This way, we can specify
/// which argument kind we want without duplicating the type in the background.
pub trait Key {
    /// The type to parse into.
    type Output: Eq;

    /// Parse a key string into the output type if the string is valid for this
    /// key.
    fn parse(key: Spanned<&str>) -> Option<Self::Output>;
}

impl<K: Key> Key for Spanned<K> {
    type Output = Spanned<K::Output>;

    fn parse(key: Spanned<&str>) -> Option<Self::Output> {
        K::parse(key).map(|v| Spanned { v, span: key.span })
    }
}

/// Implements [`Key`] for types that just need to match on strings.
macro_rules! key {
    ($type:ty, $output:ty, $($($p:pat)|* => $r:expr),* $(,)?) => {
        impl Key for $type {
            type Output = $output;

            fn parse(key: Spanned<&str>) -> Option<Self::Output> {
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

key!(AxisKey, Self,
    "horizontal" | "h" => Specific(Horizontal),
    "vertical"   | "v" => Specific(Vertical),
    "primary"    | "p" => Generic(Primary),
    "secondary"  | "s" => Generic(Secondary),
);

/// A key which parses into an [`AxisKey`] but uses typical extent keywords
/// instead of axis keywords, e.g. `width` instead of `horizontal`.
pub struct ExtentKey;

key!(ExtentKey, AxisKey,
    "width"          | "w"  => Specific(Horizontal),
    "height"         | "h"  => Specific(Vertical),
    "primary-size"   | "ps" => Generic(Primary),
    "secondary-size" | "ss" => Generic(Secondary),
);

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

key!(PaddingKey<AxisKey>, Self,
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
