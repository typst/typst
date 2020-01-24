use crate::layout::prelude::*;
use super::values::AlignmentValue::{self, *};
use super::*;

use self::AxisKey::*;
use self::PaddingKey::*;



pub trait Key {
    type Output: Eq;

    fn parse(key: Spanned<&str>) -> Option<Self::Output>;
}

impl<K: Key> Key for Spanned<K> {
    type Output = Spanned<K::Output>;

    fn parse(key: Spanned<&str>) -> Option<Self::Output> {
        K::parse(key).map(|v| Spanned { v, span: key.span })
    }
}

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

/// An argument key which identifies a layouting axis.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
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

pub struct ExtentKey;

key!(ExtentKey, AxisKey,
    "width"          | "w"  => Specific(Horizontal),
    "height"         | "h"  => Specific(Vertical),
    "primary-size"   | "ps" => Generic(Primary),
    "secondary-size" | "ss" => Generic(Secondary),
);

/// An argument key which identifies an axis, but allows for positional
/// arguments with unspecified axes.
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
