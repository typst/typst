use std::any::{Any, TypeId};
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

// TODO(style): Possible optimizations:
// - Ref-count map for cheaper cloning and smaller footprint
// - Store map in `Option` to make empty maps non-allocating
// - Store small properties inline

/// A map of style properties.
#[derive(Default, Clone, Hash)]
pub struct StyleMap(Vec<Entry>);

impl StyleMap {
    /// Create a new, empty style map.
    pub fn new() -> Self {
        Self(vec![])
    }

    /// Whether this map contains no styles.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Create a style map from a single property-value pair.
    pub fn with<P: Property>(key: P, value: P::Value) -> Self {
        let mut styles = Self::new();
        styles.set(key, value);
        styles
    }

    /// Set the value for a style property.
    pub fn set<P: Property>(&mut self, key: P, value: P::Value) {
        for entry in &mut self.0 {
            if entry.is::<P>() {
                let prev = entry.downcast::<P>().unwrap();
                let folded = P::fold(value, prev.clone());
                *entry = Entry::new(key, folded);
                return;
            }
        }

        self.0.push(Entry::new(key, value));
    }

    /// Set a value for a style property if it is `Some(_)`.
    pub fn set_opt<P: Property>(&mut self, key: P, value: Option<P::Value>) {
        if let Some(value) = value {
            self.set(key, value);
        }
    }

    /// Toggle a boolean style property, removing it if it exists and inserting
    /// it with `true` if it doesn't.
    pub fn toggle<P: Property<Value = bool>>(&mut self, key: P) {
        for (i, entry) in self.0.iter_mut().enumerate() {
            if entry.is::<P>() {
                self.0.swap_remove(i);
                return;
            }
        }

        self.0.push(Entry::new(key, true));
    }

    /// Make `self` the first link of the style chain `outer`.
    ///
    /// The resulting style chain contains styles from `self` as well as
    /// `outer`. The ones from `self` take precedence over the ones from
    /// `outer`. For folded properties `self` contributes the inner value.
    pub fn chain<'a>(&'a self, outer: &'a StyleChain<'a>) -> StyleChain<'a> {
        if self.is_empty() {
            // No need to chain an empty map.
            *outer
        } else {
            StyleChain { inner: self, outer: Some(outer) }
        }
    }

    /// Apply styles from `outer` in-place. The resulting style map is
    /// equivalent to the style chain created by
    /// `self.chain(StyleChain::new(outer))`.
    ///
    /// This is useful in the evaluation phase while building nodes and their
    /// style maps, whereas `chain` would be used during layouting to combine
    /// immutable style maps from different levels of the hierarchy.
    pub fn apply(&mut self, outer: &Self) {
        'outer: for outer in &outer.0 {
            for inner in &mut self.0 {
                if inner.style_id() == outer.style_id() {
                    inner.fold(outer);
                    continue 'outer;
                }
            }

            self.0.push(outer.clone());
        }
    }

    /// Subtract `other` from `self` in-place, keeping only styles that are in
    /// `self` but not in `other`.
    pub fn erase(&mut self, other: &Self) {
        self.0.retain(|x| !other.0.contains(x));
    }

    /// Intersect `self` with `other` in-place, keeping only styles that are
    /// both in `self` and `other`.
    pub fn intersect(&mut self, other: &Self) {
        self.0.retain(|x| other.0.contains(x));
    }

    /// Whether two style maps are equal when filtered down to the given
    /// properties.
    pub fn compatible<F>(&self, other: &Self, filter: F) -> bool
    where
        F: Fn(StyleId) -> bool,
    {
        let f = |entry: &&Entry| filter(entry.style_id());
        self.0.iter().filter(f).count() == other.0.iter().filter(f).count()
            && self.0.iter().filter(f).all(|x| other.0.contains(x))
    }
}

impl Debug for StyleMap {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        for entry in &self.0 {
            writeln!(f, "{:#?}", entry)?;
        }
        Ok(())
    }
}

impl PartialEq for StyleMap {
    fn eq(&self, other: &Self) -> bool {
        self.compatible(other, |_| true)
    }
}

/// A chain of style maps, similar to a linked list.
///
/// A style chain allows to conceptually merge (and fold) properties from
/// multiple style maps in a node hierarchy in a non-allocating way. Rather than
/// eagerly merging the maps, each access walks the hierarchy from the innermost
/// to the outermost map, trying to find a match and then folding it with
/// matches further up the chain.
#[derive(Clone, Copy, Hash)]
pub struct StyleChain<'a> {
    inner: &'a StyleMap,
    outer: Option<&'a Self>,
}

impl<'a> StyleChain<'a> {
    /// Start a new style chain with a root map.
    pub fn new(map: &'a StyleMap) -> Self {
        Self { inner: map, outer: None }
    }

    /// Get the (folded) value of a copyable style property.
    ///
    /// Returns the property's default value if no map in the chain contains an
    /// entry for it.
    pub fn get<P>(self, key: P) -> P::Value
    where
        P: Property,
        P::Value: Copy,
    {
        // This exists separately to `get_cloned` for `Copy` types so that
        // people don't just naively use `get` / `get_cloned` where they should
        // use `get_ref`.
        self.get_cloned(key)
    }

    /// Get a reference to a style property's value.
    ///
    /// This is naturally only possible for properties that don't need folding.
    /// Prefer `get` if possible or resort to `get_cloned` for non-`Copy`
    /// properties that need folding.
    ///
    /// Returns a reference to the property's default value if no map in the
    /// chain contains an entry for it.
    pub fn get_ref<P>(self, key: P) -> &'a P::Value
    where
        P: Property + Nonfolding,
    {
        if let Some(value) = self.get_locally(key) {
            value
        } else if let Some(outer) = self.outer {
            outer.get_ref(key)
        } else {
            P::default_ref()
        }
    }

    /// Get the (folded) value of any style property.
    ///
    /// While this works for all properties, you should prefer `get` or
    /// `get_ref` where possible. This is only needed for non-`Copy` properties
    /// that need folding.
    pub fn get_cloned<P>(self, key: P) -> P::Value
    where
        P: Property,
    {
        if let Some(value) = self.get_locally(key).cloned() {
            if P::FOLDABLE {
                if let Some(outer) = self.outer {
                    P::fold(value, outer.get_cloned(key))
                } else {
                    P::fold(value, P::default())
                }
            } else {
                value
            }
        } else if let Some(outer) = self.outer {
            outer.get_cloned(key)
        } else {
            P::default()
        }
    }

    /// Find a property directly in the most local map.
    fn get_locally<P: Property>(&self, _: P) -> Option<&'a P::Value> {
        self.inner
            .0
            .iter()
            .find(|entry| entry.is::<P>())
            .and_then(|entry| entry.downcast::<P>())
    }
}

impl Debug for StyleChain<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.inner.fmt(f)?;
        if let Some(outer) = self.outer {
            outer.fmt(f)?;
        }
        Ok(())
    }
}

/// A unique identifier for a style property.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct StyleId(TypeId);

impl StyleId {
    /// The style id of the property.
    pub fn of<P: Property>() -> Self {
        Self(TypeId::of::<P>())
    }
}

/// Style property keys.
///
/// This trait is not intended to be implemented manually, but rather through
/// the `#[properties]` proc-macro.
pub trait Property: Copy + 'static {
    /// The type of value that is returned when getting this property from a
    /// style map. For example, this could be [`Length`](crate::geom::Length)
    /// for a `WIDTH` property.
    type Value: Debug + Clone + PartialEq + Hash + 'static;

    /// The name of the property, used for debug printing.
    const NAME: &'static str;

    /// The default value of the property.
    fn default() -> Self::Value;

    /// A static reference to the default value of the property.
    ///
    /// This is automatically implemented through lazy-initialization in the
    /// `#[properties]` macro. This way, expensive defaults don't need to be
    /// recreated all the time.
    fn default_ref() -> &'static Self::Value;

    /// Whether the property needs folding.
    const FOLDABLE: bool = false;

    /// Fold the property with an outer value.
    ///
    /// For example, this would fold a relative font size with an outer
    /// absolute font size.
    #[allow(unused_variables)]
    fn fold(inner: Self::Value, outer: Self::Value) -> Self::Value {
        inner
    }
}

/// Marker trait that indicates that a property doesn't need folding.
pub trait Nonfolding {}

/// An entry for a single style property.
#[derive(Clone)]
struct Entry(Rc<dyn Bounds>);

impl Entry {
    fn new<P: Property>(key: P, value: P::Value) -> Self {
        Self(Rc::new((key, value)))
    }

    fn style_id(&self) -> StyleId {
        self.0.style_id()
    }

    fn is<P: Property>(&self) -> bool {
        self.style_id() == StyleId::of::<P>()
    }

    fn downcast<P: Property>(&self) -> Option<&P::Value> {
        self.0.as_any().downcast_ref()
    }

    fn fold(&mut self, outer: &Self) {
        *self = self.0.fold(outer);
    }
}

impl Debug for Entry {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.dyn_fmt(f)
    }
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.0.dyn_eq(other)
    }
}

impl Hash for Entry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0.hash64());
    }
}

/// This trait is implemented for pairs of zero-sized property keys and their
/// value types below. Although it is zero-sized, the property `P` must be part
/// of the implementing type so that we can use it in the methods (it must be a
/// constrained type parameter).
trait Bounds: 'static {
    fn style_id(&self) -> StyleId;
    fn fold(&self, outer: &Entry) -> Entry;
    fn as_any(&self) -> &dyn Any;
    fn dyn_fmt(&self, f: &mut Formatter) -> fmt::Result;
    fn dyn_eq(&self, other: &Entry) -> bool;
    fn hash64(&self) -> u64;
}

impl<P: Property> Bounds for (P, P::Value) {
    fn style_id(&self) -> StyleId {
        StyleId::of::<P>()
    }

    fn fold(&self, outer: &Entry) -> Entry {
        let outer = outer.downcast::<P>().unwrap();
        let combined = P::fold(self.1.clone(), outer.clone());
        Entry::new(self.0, combined)
    }

    fn as_any(&self) -> &dyn Any {
        &self.1
    }

    fn dyn_fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "#[{} = {:?}]", P::NAME, self.1)
    }

    fn dyn_eq(&self, other: &Entry) -> bool {
        self.style_id() == other.style_id()
            && if let Some(other) = other.downcast::<P>() {
                &self.1 == other
            } else {
                false
            }
    }

    fn hash64(&self) -> u64 {
        let mut state = fxhash::FxHasher64::default();
        self.style_id().hash(&mut state);
        self.1.hash(&mut state);
        state.finish()
    }
}
