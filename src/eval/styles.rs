use std::any::{Any, TypeId};
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

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
        self.0.splice(0 .. 0, outer.0.clone());
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
        for entry in self.0.iter().rev() {
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
    /// The first link in the chain.
    first: Link<'a>,
    /// The remaining links in the chain.
    outer: Option<&'a Self>,
}

/// The two kinds of links in the chain.
#[derive(Clone, Copy, Hash)]
enum Link<'a> {
    /// Just a map with styles.
    Map(&'a StyleMap),
    /// A barrier that, in combination with one more such barrier, stops scoped
    /// styles for the node with this type id.
    Barrier(TypeId),
}

impl<'a> StyleChain<'a> {
    /// Start a new style chain with a root map.
    pub fn new(first: &'a StyleMap) -> Self {
        Self { first: Link::Map(first), outer: None }
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
        self.get_cloned(key)
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
        self.values(key).next().unwrap_or_else(|| P::default_ref())
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
        if P::FOLDING {
            self.values(key)
                .cloned()
                .chain(std::iter::once(P::default()))
                .reduce(P::fold)
                .unwrap()
        } else {
            self.values(key).next().cloned().unwrap_or_else(P::default)
        }
    }

    /// Insert a barrier into the style chain.
    ///
    /// Barriers interact with [scoped](StyleMap::scoped) styles: A scoped style
    /// can still be read through a single barrier (the one of the node it
    /// _should_ apply to), but a second barrier will make it invisible.
    pub fn barred<'b>(&'b self, node: TypeId) -> StyleChain<'b> {
        if self
            .maps()
            .any(|map| map.0.iter().any(|entry| entry.scoped && entry.is_of_same(node)))
        {
            StyleChain {
                first: Link::Barrier(node),
                outer: Some(self),
            }
        } else {
            *self
        }
    }

    /// Iterate over all values for the given property in the chain.
    fn values<P: Property>(self, _: P) -> impl Iterator<Item = &'a P::Value> {
        let mut depth = 0;
        self.links().flat_map(move |link| {
            let mut entries: &[Entry] = &[];
            match link {
                Link::Map(map) => entries = &map.0,
                Link::Barrier(id) => depth += (id == P::node_id()) as usize,
            }
            entries
                .iter()
                .rev()
                .filter(move |entry| entry.is::<P>() && (!entry.scoped || depth <= 1))
                .filter_map(|entry| entry.downcast::<P>())
        })
    }

    /// Iterate over the links of the chain.
    fn links(self) -> impl Iterator<Item = Link<'a>> {
        let mut cursor = Some(self);
        std::iter::from_fn(move || {
            let Self { first, outer } = cursor?;
            cursor = outer.copied();
            Some(first)
        })
    }

    /// Iterate over the map links of the chain.
    fn maps(self) -> impl Iterator<Item = &'a StyleMap> {
        self.links().filter_map(|link| match link {
            Link::Map(map) => Some(map),
            Link::Barrier(_) => None,
        })
    }
}

impl Debug for StyleChain<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        for link in self.links() {
            link.fmt(f)?;
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
    pair: Arc<dyn Bounds>,
    scoped: bool,
}

impl Entry {
    fn new<P: Property>(key: P, value: P::Value) -> Self {
        Self {
            pair: Arc::new((key, value)),
            scoped: false,
        }
    }

    fn is<P: Property>(&self) -> bool {
        self.pair.style_id() == TypeId::of::<P>()
    }

    fn is_of<T: 'static>(&self) -> bool {
        self.pair.node_id() == TypeId::of::<T>()
    }

    fn is_of_same(&self, node: TypeId) -> bool {
        self.pair.node_id() == node
    }

    fn downcast<P: Property>(&self) -> Option<&P::Value> {
        self.pair.as_any().downcast_ref()
    }
}

impl Debug for Entry {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("#[")?;
        self.pair.dyn_fmt(f)?;
        if self.scoped {
            f.write_str(" (scoped)")?;
        }
        f.write_str("]")
    }
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.pair.dyn_eq(other) && self.scoped == other.scoped
    }
}

impl Hash for Entry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.pair.hash64());
        state.write_u8(self.scoped as u8);
    }
}

/// Style property keys.
///
/// This trait is not intended to be implemented manually, but rather through
/// the `#[properties]` proc-macro.
pub trait Property: Sync + Send + 'static {
    /// The type of value that is returned when getting this property from a
    /// style map. For example, this could be [`Length`](crate::geom::Length)
    /// for a `WIDTH` property.
    type Value: Debug + Clone + PartialEq + Hash + Sync + Send + 'static;

    /// The name of the property, used for debug printing.
    const NAME: &'static str;

    /// Whether the property needs folding.
    const FOLDING: bool = false;

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
trait Bounds: Sync + Send + 'static {
    fn as_any(&self) -> &dyn Any;
    fn dyn_fmt(&self, f: &mut Formatter) -> fmt::Result;
    fn dyn_eq(&self, other: &Entry) -> bool;
    fn hash64(&self) -> u64;
    fn node_id(&self) -> TypeId;
    fn style_id(&self) -> TypeId;
}

impl<P: Property> Bounds for (P, P::Value) {
    fn as_any(&self) -> &dyn Any {
        &self.1
    }

    fn dyn_fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} = {:?}", P::NAME, self.1)
    }

    fn dyn_eq(&self, other: &Entry) -> bool {
        self.style_id() == other.pair.style_id()
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
}
