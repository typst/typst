//! Deduplicating maps for argument parsing.

use std::collections::HashMap;
use std::hash::Hash;

use super::*;

/// A deduplicating map type useful for storing possibly redundant arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct ConsistentMap<K, V> where K: Hash + Eq {
    map: HashMap<K, V>,
}

impl<K, V> ConsistentMap<K, V> where K: Hash + Eq {
    pub fn new() -> ConsistentMap<K, V> {
        ConsistentMap { map: HashMap::new() }
    }

    /// Add a key-value pair.
    pub fn add(&mut self, key: K, value: V) -> ParseResult<()> {
        match self.map.insert(key, value) {
            Some(_) => error!("duplicate arguments"),
            None => Ok(())
        }
    }

    /// Add a key-value pair if the value is not `None`.
    pub fn add_opt(&mut self, key: K, value: Option<V>) -> ParseResult<()> {
        Ok(if let Some(value) = value {
            self.add(key, value)?;
        })
    }

    /// Add a key-spanned-value pair the value is not `None`.
    pub fn add_opt_span(&mut self, key: K, value: Option<Spanned<V>>) -> ParseResult<()> {
        Ok(if let Some(spanned) = value {
            self.add(key, spanned.v)?;
        })
    }

    /// Call a function with the value if the key is present.
    pub fn with<F>(&self, key: K, callback: F) where F: FnOnce(&V) {
        if let Some(value) = self.map.get(&key) {
            callback(value);
        }
    }

    /// Create a new consistent map where keys and values are mapped to new keys
    /// and values.
    ///
    /// Returns an error if a new key is duplicate.
    pub fn dedup<F, K2, V2>(&self, f: F) -> LayoutResult<ConsistentMap<K2, V2>>
    where
        F: Fn(&K, &V) -> ParseResult<(K2, V2)>,
        K2: Hash + Eq
    {
        let mut map = ConsistentMap::new();

        for (key, value) in self.map.iter() {
            let (key, value) = f(key, value)?;
            map.add(key, value)?;
        }

        Ok(map)
    }

    /// Iterate over the (key, value) pairs.
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, K, V> {
        self.map.iter()
    }
}

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
                "width"          | "w" => AxisKey::Horizontal,
                "height"         | "h" => AxisKey::Vertical,
                "primary-size"   | "ps" => AxisKey::Primary,
                "secondary-size" | "ss" => AxisKey::Secondary,

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

    /// Apply the extents on the dimensions.
    pub fn apply<F>(
        &self,
        axes: LayoutAxes,
        dimensions: &mut Size2D,
        size: F
    ) -> LayoutResult<()> where F: Fn(&E) -> Size {
        let map = self.dedup(axes)?;
        map.with(Horizontal, |val| dimensions.x = size(val));
        map.with(Vertical, |val| dimensions.y = size(val));
        Ok(())
    }

    /// Map from any axis key to the specific axis kind.
    pub fn apply_with<F>(&self, axes: LayoutAxes, mut f: F) -> LayoutResult<()>
    where F: FnMut(SpecificAxis, &E) {
        for (&key, value) in self.dedup(axes)?.iter() {
            f(key, value);
        }
        Ok(())
    }

    fn dedup(&self, axes: LayoutAxes) -> LayoutResult<ConsistentMap<SpecificAxis, E>> {
        self.0.dedup(|key, &val| Ok((key.to_specific(axes), val)))
    }
}

/// A map for storing padding at sides.
#[derive(Debug, Clone, PartialEq)]
pub struct PaddingMap(ConsistentMap<PaddingKey<AxisKey>, Size>);

impl PaddingMap {
    /// Parse an extent map from the function args.
    ///
    /// If `enforce` is true other arguments will create an error, otherwise
    /// they are left intact.
    pub fn new(args: &mut FuncArgs, enforce: bool) -> ParseResult<PaddingMap> {
        let mut map = ConsistentMap::new();

        map.add_opt_span(PaddingKey::All, args.get_pos_opt::<Size>()?)?;

        for arg in args.keys() {
            let key = match PaddingKey::from_ident(&arg.v.key) {
                Ok(key) => key,
                e => if enforce { e? } else { args.add_key(arg); continue; }
            };

            let size = Size::from_expr(arg.v.value)?;

            map.add(key, size)?;
        }

        Ok(PaddingMap(map))
    }

    /// Map from any axis key to the specific axis kind.
    pub fn apply(&self, axes: LayoutAxes, padding: &mut SizeBox) -> LayoutResult<()> {
        use PaddingKey::*;

        let map = self.0.dedup(|key, &val| {
            Ok((match key {
                All => All,
                Axis(axis) => Axis(axis.to_specific(axes)),
                AxisAligned(axis, alignment) => {
                    let axis = axis.to_specific(axes);
                    AxisAligned(axis, alignment.to_specific(axes, axis))
                }
            }, val))
        })?;

        map.with(All, |&val| padding.set_all(val));
        map.with(Axis(Horizontal), |&val| padding.set_horizontal(val));
        map.with(Axis(Vertical), |&val| padding.set_vertical(val));

        for (key, &val) in map.iter() {
            if let AxisAligned(_, alignment) = key {
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
