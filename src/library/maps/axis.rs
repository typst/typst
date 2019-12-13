use super::*;
use AxisKey::*;

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

key!(AxisKey, "axis",
    "horizontal" | "h" => Specific(Horizontal),
    "vertical"   | "v" => Specific(Vertical),
    "primary"    | "p" => Generic(Primary),
    "secondary"  | "s" => Generic(Secondary),
);

/// A map for storing extents along axes.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtentMap<E: ExpressionKind + Copy>(ConsistentMap<AxisKey, E>);

impl<E: ExpressionKind + Copy> ExtentMap<E> {
    /// Parse an extent map from the function args.
    ///
    /// If `enforce` is true other arguments will create an error, otherwise
    /// they are left intact.
    pub fn new(args: &mut FuncArgs, enforce: bool) -> ParseResult<ExtentMap<E>> {
        let mut map = ConsistentMap::new();

        for arg in args.keys() {
            let key = match arg.v.key.v.0.as_str() {
                "width"          | "w"  => AxisKey::Specific(Horizontal),
                "height"         | "h"  => AxisKey::Specific(Vertical),
                "primary-size"   | "ps" => AxisKey::Generic(Primary),
                "secondary-size" | "ss" => AxisKey::Generic(Secondary),

                _ => if enforce {
                    error!("expected dimension")
                } else {
                    args.add_key(arg);
                    continue;
                }
            };

            let e = E::from_expr(arg.v.value)?;
            map.add(key, e)?;
        }

        Ok(ExtentMap(map))
    }

    /// Deduplicate from generic to specific axes.
    pub fn dedup(&self, axes: LayoutAxes) -> LayoutResult<ConsistentMap<SpecificAxis, E>> {
        self.0.dedup(|key, &val| Ok((key.to_specific(axes), val)))
    }
}

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

/// A map for storing some data for via keyword or positionally given axes.
#[derive(Debug, Clone, PartialEq)]
pub struct PosAxisMap<E: ExpressionKind + Copy>(ConsistentMap<PosAxisKey, E>);

impl<E: ExpressionKind + Copy> PosAxisMap<E> {
    /// Parse a positional axis map from the function args.
    pub fn new(args: &mut FuncArgs) -> ParseResult<PosAxisMap<E>> {
        let mut map = ConsistentMap::new();

        map.add_opt(PosAxisKey::First, args.get_pos_opt::<E>()?)?;
        map.add_opt(PosAxisKey::Second, args.get_pos_opt::<E>()?)?;

        for arg in args.keys() {
            let axis = AxisKey::from_ident(&arg.v.key)?;
            let value = E::from_expr(arg.v.value)?;

            map.add(PosAxisKey::Keyword(axis), value)?;
        }

        Ok(PosAxisMap(map))
    }

    /// Deduplicate from positional or specific to generic axes.
    pub fn dedup<F>(&self, axes: LayoutAxes, f: F) -> LayoutResult<ConsistentMap<GenericAxis, E>>
    where F: Fn(E) -> Option<GenericAxis> {
        self.0.dedup(|key, &e| {
            Ok((match key {
                PosAxisKey::First => f(e).unwrap_or(Primary),
                PosAxisKey::Second => f(e).unwrap_or(Secondary),
                PosAxisKey::Keyword(AxisKey::Specific(axis)) => axis.to_generic(axes),
                PosAxisKey::Keyword(AxisKey::Generic(axis)) => *axis,
            }, e))
        })
    }
}
