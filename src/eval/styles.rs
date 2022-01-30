use std::any::{Any, TypeId};
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

// TODO(style): Possible optimizations:
// - Ref-count map for cheaper cloning and smaller footprint
// - Store map in `Option` to make empty maps non-allocating
// - Store small properties inline

/// An item with associated styles.
#[derive(PartialEq, Clone, Hash)]
pub struct Styled<T> {
    /// The item to apply styles to.
    pub item: T,
    /// The associated style map.
    pub map: StyleMap,
}

impl<T> Styled<T> {
    /// Create a new instance from an item and a style map.
    pub fn new(item: T, map: StyleMap) -> Self {
        Self { item, map }
    }

    /// Create a new instance with empty style map.
    pub fn bare(item: T) -> Self {
        Self { item, map: StyleMap::new() }
    }

    /// Map the item with `f`.
    pub fn map<F, U>(self, f: F) -> Styled<U>
    where
        F: FnOnce(T) -> U,
    {
        Styled { item: f(self.item), map: self.map }
    }
}

impl<T: Debug> Debug for Styled<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.map.fmt(f)?;
        self.item.fmt(f)
    }
}

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

    /// Mark all contained properties as _scoped_. This means that they only
    /// apply to the first descendant node (of their type) in the hierarchy and
    /// not its children, too. This is used by class constructors.
    pub fn scoped(mut self) -> Self {
        for entry in &mut self.0 {
            entry.scoped = true;
        }
        self
    }

    /// Make `self` the first link of the style chain `outer`.
    ///
    /// The resulting style chain contains styles from `self` as well as
    /// `outer`. The ones from `self` take precedence over the ones from
    /// `outer`. For folded properties `self` contributes the inner value.
    pub fn chain<'a>(&'a self, outer: &'a StyleChain<'a>) -> StyleChain<'a> {
        if self.is_empty() {
            *outer
        } else {
            StyleChain {
                first: Link::Map(self),
                outer: Some(outer),
            }
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
        for outer in &outer.0 {
            if let Some(inner) = self.0.iter_mut().find(|inner| inner.is_same(outer)) {
                *inner = inner.fold(outer);
                continue;
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

    /// Whether two style maps are equal when filtered down to properties of the
    /// node `T`.
    pub fn compatible<T: 'static>(&self, other: &Self) -> bool {
        let f = |entry: &&Entry| entry.is_of::<T>();
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
        self.0.len() == other.0.len() && self.0.iter().all(|x| other.0.contains(x))
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
    /// The first map in the chain.
    first: Link<'a>,
    /// The remaining maps in the chain.
    outer: Option<&'a Self>,
}

/// The two kinds of links in the chain.
#[derive(Clone, Copy, Hash)]
enum Link<'a> {
    Map(&'a StyleMap),
    Barrier(TypeId),
}

impl<'a> StyleChain<'a> {
    /// Start a new style chain with a root map.
    pub fn new(map: &'a StyleMap) -> Self {
        Self { first: Link::Map(map), outer: None }
    }

    /// Get the (folded) value of a copyable style property.
    ///
    /// This is the method you should reach for first. If it doesn't work
    /// because your property is not copyable, use `get_ref`. If that doesn't
    /// work either because your property needs folding, use `get_cloned`.
    ///
    /// Returns the property's default value if no map in the chain contains an
    /// entry for it.
    pub fn get<P: Property>(self, key: P) -> P::Value
    where
        P::Value: Copy,
    {
        self.get_impl(key, 0)
    }

    /// Get a reference to a style property's value.
    ///
    /// This is naturally only possible for properties that don't need folding.
    /// Prefer `get` if possible or resort to `get_cloned` for non-`Copy`
    /// properties that need folding.
    ///
    /// Returns a lazily-initialized reference to the property's default value
    /// if no map in the chain contains an entry for it.
    pub fn get_ref<P: Property>(self, key: P) -> &'a P::Value
    where
        P: Nonfolding,
    {
        self.get_ref_impl(key, 0)
    }

    /// Get the (folded) value of any style property.
    ///
    /// While this works for all properties, you should prefer `get` or
    /// `get_ref` where possible. This is only needed for non-`Copy` properties
    /// that need folding.
    ///
    /// Returns the property's default value if no map in the chain contains an
    /// entry for it.
    pub fn get_cloned<P: Property>(self, key: P) -> P::Value {
        self.get_impl(key, 0)
    }

    /// Insert a barrier into the style chain.
    ///
    /// Barriers interact with [scoped](StyleMap::scoped) styles: A scoped style
    /// can still be read through a single barrier (the one of the node it
    /// _should_ apply to), but a second barrier will make it invisible.
    pub fn barred<'b>(&'b self, node: TypeId) -> StyleChain<'b> {
        if self.needs_barrier(node) {
            StyleChain {
                first: Link::Barrier(node),
                outer: Some(self),
            }
        } else {
            *self
        }
    }
}

impl<'a> StyleChain<'a> {
    fn get_impl<P: Property>(self, key: P, depth: usize) -> P::Value {
        let (value, depth) = self.process(key, depth);
        if let Some(value) = value.cloned() {
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
            outer.get_impl(key, depth)
        } else {
            P::default()
        }
    }

    fn get_ref_impl<P: Property>(self, key: P, depth: usize) -> &'a P::Value
    where
        P: Nonfolding,
    {
        let (value, depth) = self.process(key, depth);
        if let Some(value) = value {
            value
        } else if let Some(outer) = self.outer {
            outer.get_ref_impl(key, depth)
        } else {
            P::default_ref()
        }
    }

    fn process<P: Property>(self, _: P, depth: usize) -> (Option<&'a P::Value>, usize) {
        match self.first {
            Link::Map(map) => (
                map.0
                    .iter()
                    .find(|entry| entry.is::<P>() && (!entry.scoped || depth <= 1))
                    .and_then(|entry| entry.downcast::<P>()),
                depth,
            ),
            Link::Barrier(node) => (None, depth + (P::node_id() == node) as usize),
        }
    }

    fn needs_barrier(self, node: TypeId) -> bool {
        if let Link::Map(map) = self.first {
            if map.0.iter().any(|entry| entry.is_of_same(node)) {
                return true;
            }
        }

        self.outer.map_or(false, |outer| outer.needs_barrier(node))
    }
}

impl Debug for StyleChain<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.first.fmt(f)?;
        if let Some(outer) = self.outer {
            outer.fmt(f)?;
        }
        Ok(())
    }
}

impl Debug for Link<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Map(map) => map.fmt(f),
            Self::Barrier(id) => writeln!(f, "Barrier({:?})", id),
        }
    }
}

/// An entry for a single style property.
#[derive(Clone)]
struct Entry {
    p: Rc<dyn Bounds>,
    scoped: bool,
}

impl Entry {
    fn new<P: Property>(key: P, value: P::Value) -> Self {
        Self { p: Rc::new((key, value)), scoped: false }
    }

    fn is<P: Property>(&self) -> bool {
        self.p.style_id() == TypeId::of::<P>()
    }

    fn is_same(&self, other: &Self) -> bool {
        self.p.style_id() == other.p.style_id()
    }

    fn is_of<T: 'static>(&self) -> bool {
        self.p.node_id() == TypeId::of::<T>()
    }

    fn is_of_same(&self, node: TypeId) -> bool {
        self.p.node_id() == node
    }

    fn downcast<P: Property>(&self) -> Option<&P::Value> {
        self.p.as_any().downcast_ref()
    }

    fn fold(&self, outer: &Self) -> Self {
        self.p.fold(outer)
    }
}

impl Debug for Entry {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.p.dyn_fmt(f)
    }
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.p.dyn_eq(other)
    }
}

impl Hash for Entry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.p.hash64());
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

    /// Whether the property needs folding.
    const FOLDABLE: bool = false;

    /// The type id of the node this property belongs to.
    fn node_id() -> TypeId;

    /// The default value of the property.
    fn default() -> Self::Value;

    /// A static reference to the default value of the property.
    ///
    /// This is automatically implemented through lazy-initialization in the
    /// `#[properties]` macro. This way, expensive defaults don't need to be
    /// recreated all the time.
    fn default_ref() -> &'static Self::Value;

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

/// This trait is implemented for pairs of zero-sized property keys and their
/// value types below. Although it is zero-sized, the property `P` must be part
/// of the implementing type so that we can use it in the methods (it must be a
/// constrained type parameter).
trait Bounds: 'static {
    fn as_any(&self) -> &dyn Any;
    fn dyn_fmt(&self, f: &mut Formatter) -> fmt::Result;
    fn dyn_eq(&self, other: &Entry) -> bool;
    fn hash64(&self) -> u64;
    fn node_id(&self) -> TypeId;
    fn style_id(&self) -> TypeId;
    fn fold(&self, outer: &Entry) -> Entry;
}

impl<P: Property> Bounds for (P, P::Value) {
    fn as_any(&self) -> &dyn Any {
        &self.1
    }

    fn dyn_fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "#[{} = {:?}]", P::NAME, self.1)
    }

    fn dyn_eq(&self, other: &Entry) -> bool {
        self.style_id() == other.p.style_id()
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

    fn node_id(&self) -> TypeId {
        P::node_id()
    }

    fn style_id(&self) -> TypeId {
        TypeId::of::<P>()
    }

    fn fold(&self, outer: &Entry) -> Entry {
        let outer = outer.downcast::<P>().unwrap();
        let combined = P::fold(self.1.clone(), outer.clone());
        Entry::new(self.0, combined)
    }
}
