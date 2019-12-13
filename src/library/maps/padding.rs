use super::*;
use AxisKey::*;
use AlignmentKey::*;
use PaddingKey::*;

/// An argument key which identifies a margin or padding target.
///
/// A is the used axis type.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum PaddingKey<A> {
    /// All four sides should have the specified padding.
    All,
    /// Both sides of the given axis should have the specified padding.
    Both(A),
    /// Only the given side of the given axis should have the specified padding.
    Side(A, AlignmentKey),
}

key!(PaddingKey<AxisKey>, "axis or side",
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

/// A map for storing padding at sides.
#[derive(Debug, Clone, PartialEq)]
pub struct PaddingMap(ConsistentMap<PaddingKey<AxisKey>, Size>);

impl PaddingMap {
    /// Parse a padding map from the function args.
    pub fn new(args: &mut FuncArgs) -> ParseResult<PaddingMap> {
        let mut map = ConsistentMap::new();
        map.add_opt(PaddingKey::All, args.get_pos_opt::<Size>()?)?;

        for arg in args.keys() {
            let key = PaddingKey::from_ident(&arg.v.key)?;
            let size = Size::from_expr(arg.v.value)?;
            map.add(key, size)?;
        }

        Ok(PaddingMap(map))
    }

    /// Apply the specified padding on the size box.
    pub fn apply(&self, axes: LayoutAxes, padding: &mut SizeBox) -> LayoutResult<()> {
        use PaddingKey::*;

        let map = self.0.dedup(|key, &val| {
            Ok((match key {
                All => All,
                Both(axis) => Both(axis.to_specific(axes)),
                Side(axis, alignment) => {
                    let axis = axis.to_specific(axes);
                    Side(axis, alignment.to_specific(axes, axis))
                }
            }, val))
        })?;

        map.with(All, |&val| padding.set_all(val));
        map.with(Both(Horizontal), |&val| padding.set_horizontal(val));
        map.with(Both(Vertical), |&val| padding.set_vertical(val));

        for (key, &val) in map.iter() {
            if let Side(_, alignment) = key {
                match alignment {
                    AlignmentKey::Left => padding.left = val,
                    AlignmentKey::Right => padding.right = val,
                    AlignmentKey::Top => padding.top = val,
                    AlignmentKey::Bottom => padding.bottom = val,
                    _ => {},
                }
            }
        }

        Ok(())
    }
}
