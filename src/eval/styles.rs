use std::any::{Any, TypeId};
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use super::{Args, Content, Func, Span, Value};
use crate::diag::{At, TypResult};
use crate::library::layout::PageNode;
use crate::library::text::ParNode;
use crate::Context;

/// A map of style properties.
#[derive(Default, Clone, PartialEq, Hash)]
pub struct StyleMap {
    /// Settable properties.
    props: Vec<Entry>,
    /// Show rule recipes.
    recipes: Vec<Recipe>,
}

impl StyleMap {
    /// Create a new, empty style map.
    pub fn new() -> Self {
        Self { props: vec![], recipes: vec![] }
    }

    /// Whether this map contains no styles.
    pub fn is_empty(&self) -> bool {
        self.props.is_empty() && self.recipes.is_empty()
    }

    /// Create a style map from a single property-value pair.
    pub fn with<K: Key>(key: K, value: K::Value) -> Self {
        let mut styles = Self::new();
        styles.set(key, value);
        styles
    }

    /// Set the value for a style property.
    pub fn set<K: Key>(&mut self, key: K, value: K::Value) {
        self.props.push(Entry::new(key, value));
    }

    /// Set a value for a style property if it is `Some(_)`.
    pub fn set_opt<K: Key>(&mut self, key: K, value: Option<K::Value>) {
        if let Some(value) = value {
            self.set(key, value);
        }
    }

    /// Set a recipe.
    pub fn set_recipe(&mut self, node: TypeId, func: Func, span: Span) {
        self.recipes.push(Recipe { node, func, span });
    }

    /// Mark all contained properties as _scoped_. This means that they only
    /// apply to the first descendant node (of their type) in the hierarchy and
    /// not its children, too. This is used by class constructors.
    pub fn scoped(mut self) -> Self {
        for entry in &mut self.props {
            entry.scoped = true;
        }
        self
    }

    /// Whether this map contains scoped styles.
    pub fn has_scoped(&self) -> bool {
        self.props.iter().any(|e| e.scoped)
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
                link: Some(Link::Map(self)),
                outer: Some(outer),
            }
        }
    }

    /// Apply styles from `outer` in-place. The resulting style map is
    /// equivalent to the style chain created by
    /// `self.chain(StyleChain::new(outer))`.
    ///
    /// This is useful over `chain` when you need an owned map without a
    /// lifetime, for example, because you want to store the style map inside a
    /// packed node.
    pub fn apply(&mut self, outer: &Self) {
        self.props.splice(0 .. 0, outer.props.iter().cloned());
        self.recipes.splice(0 .. 0, outer.recipes.iter().cloned());
    }

    /// The highest-level interruption of the map.
    pub fn interruption(&self) -> Option<Interruption> {
        self.props.iter().filter_map(|entry| entry.interruption()).max()
    }
}

impl Debug for StyleMap {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        for entry in self.props.iter().rev() {
            writeln!(f, "{:#?}", entry)?;
        }
        for recipe in self.recipes.iter().rev() {
            writeln!(f, "#[Recipe for {:?} from {:?}]", recipe.node, recipe.span)?;
        }
        Ok(())
    }
}

/// Determines whether a style could interrupt some composable structure.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Interruption {
    /// The style forces a paragraph break.
    Par,
    /// The style forces a page break.
    Page,
}

/// Style property keys.
///
/// This trait is not intended to be implemented manually, but rather through
/// the `#[node]` proc-macro.
pub trait Key: Sync + Send + 'static {
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
    /// `#[node]` macro. This way, expensive defaults don't need to be
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

/// Marker trait indicating that a property can be accessed by reference.
///
/// This is implemented by a key if and only if `K::FOLDING` if false.
/// Unfortunately, Rust's type system doesn't allow use to use an associated
/// constant to bound a function, so we need this trait.
pub trait Referencable {}

/// An entry for a single style property.
#[derive(Clone)]
struct Entry {
    pair: Arc<dyn Bounds>,
    scoped: bool,
}

impl Entry {
    fn new<K: Key>(key: K, value: K::Value) -> Self {
        Self {
            pair: Arc::new((key, value)),
            scoped: false,
        }
    }

    fn is<P: Key>(&self) -> bool {
        self.pair.style_id() == TypeId::of::<P>()
    }

    fn is_of<T: 'static>(&self) -> bool {
        self.pair.node_id() == TypeId::of::<T>()
    }

    fn is_of_id(&self, node: TypeId) -> bool {
        self.pair.node_id() == node
    }

    fn downcast<K: Key>(&self) -> Option<&K::Value> {
        self.pair.as_any().downcast_ref()
    }

    fn interruption(&self) -> Option<Interruption> {
        if self.is_of::<PageNode>() {
            Some(Interruption::Page)
        } else if self.is_of::<ParNode>() {
            Some(Interruption::Par)
        } else {
            None
        }
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

impl<K: Key> Bounds for (K, K::Value) {
    fn as_any(&self) -> &dyn Any {
        &self.1
    }

    fn dyn_fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} = {:?}", K::NAME, self.1)
    }

    fn dyn_eq(&self, other: &Entry) -> bool {
        self.style_id() == other.pair.style_id()
            && if let Some(other) = other.downcast::<K>() {
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
        K::node_id()
    }

    fn style_id(&self) -> TypeId {
        TypeId::of::<K>()
    }
}

/// A show rule recipe.
#[derive(Debug, Clone, PartialEq, Hash)]
struct Recipe {
    node: TypeId,
    func: Func,
    span: Span,
}

/// A chain of style maps, similar to a linked list.
///
/// A style chain allows to conceptually merge (and fold) properties from
/// multiple style maps in a node hierarchy in a non-allocating way. Rather than
/// eagerly merging the maps, each access walks the hierarchy from the innermost
/// to the outermost map, trying to find a match and then folding it with
/// matches further up the chain.
#[derive(Default, Clone, Copy, Hash)]
pub struct StyleChain<'a> {
    /// The first link of this chain.
    link: Option<Link<'a>>,
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
    pub fn new(map: &'a StyleMap) -> Self {
        Self { link: Some(Link::Map(map)), outer: None }
    }

    /// The number of links in the chain.
    pub fn len(self) -> usize {
        self.links().count()
    }

    /// Build a style map from the suffix (all links beyond the `len`) of the
    /// chain.
    ///
    /// Panics if the suffix contains barrier links.
    pub fn suffix(self, len: usize) -> StyleMap {
        let mut suffix = StyleMap::new();
        let remove = self.len().saturating_sub(len);
        for link in self.links().take(remove) {
            match link {
                Link::Map(map) => suffix.apply(map),
                Link::Barrier(_) => panic!("suffix contains barrier"),
            }
        }
        suffix
    }

    /// Remove the last link from the chain.
    pub fn pop(&mut self) {
        *self = self.outer.copied().unwrap_or_default();
    }

    /// Return the chain, but without the last link if that one contains only
    /// scoped styles. This is a hack.
    pub(crate) fn unscoped(mut self, node: TypeId) -> Self {
        if let Some(Link::Map(map)) = self.link {
            if map.props.iter().all(|e| e.scoped && e.is_of_id(node))
                && map.recipes.is_empty()
            {
                self.pop();
            }
        }
        self
    }
}

impl<'a> StyleChain<'a> {
    /// Get the (folded) value of a copyable style property.
    ///
    /// This is the method you should reach for first. If it doesn't work
    /// because your property is not copyable, use `get_ref`. If that doesn't
    /// work either because your property needs folding, use `get_cloned`.
    ///
    /// Returns the property's default value if no map in the chain contains an
    /// entry for it.
    pub fn get<K: Key>(self, key: K) -> K::Value
    where
        K::Value: Copy,
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
    pub fn get_ref<K: Key>(self, key: K) -> &'a K::Value
    where
        K: Referencable,
    {
        self.values(key).next().unwrap_or_else(|| K::default_ref())
    }

    /// Get the (folded) value of any style property.
    ///
    /// While this works for all properties, you should prefer `get` or
    /// `get_ref` where possible. This is only needed for non-`Copy` properties
    /// that need folding.
    ///
    /// Returns the property's default value if no map in the chain contains an
    /// entry for it.
    pub fn get_cloned<K: Key>(self, key: K) -> K::Value {
        if K::FOLDING {
            self.values(key)
                .cloned()
                .chain(std::iter::once(K::default()))
                .reduce(K::fold)
                .unwrap()
        } else {
            self.values(key).next().cloned().unwrap_or_else(K::default)
        }
    }

    /// Execute a user recipe for a node.
    pub fn show(
        self,
        node: &dyn Any,
        ctx: &mut Context,
        values: impl IntoIterator<Item = Value>,
    ) -> TypResult<Option<Content>> {
        Ok(if let Some(recipe) = self.recipes(node.type_id()).next() {
            let args = Args::from_values(recipe.span, values);
            Some(recipe.func.call(ctx, args)?.cast().at(recipe.span)?)
        } else {
            None
        })
    }

    /// Insert a barrier into the style chain.
    ///
    /// Barriers interact with [scoped](StyleMap::scoped) styles: A scoped style
    /// can still be read through a single barrier (the one of the node it
    /// _should_ apply to), but a second barrier will make it invisible.
    pub fn barred<'b>(&'b self, node: TypeId) -> StyleChain<'b> {
        if self
            .maps()
            .any(|map| map.props.iter().any(|entry| entry.scoped && entry.is_of_id(node)))
        {
            StyleChain {
                link: Some(Link::Barrier(node)),
                outer: Some(self),
            }
        } else {
            *self
        }
    }
}

impl<'a> StyleChain<'a> {
    /// Iterate over all values for the given property in the chain.
    fn values<K: Key>(self, _: K) -> impl Iterator<Item = &'a K::Value> {
        let mut depth = 0;
        self.links().flat_map(move |link| {
            let mut entries: &[Entry] = &[];
            match link {
                Link::Map(map) => entries = &map.props,
                Link::Barrier(id) => depth += (id == K::node_id()) as usize,
            }
            entries
                .iter()
                .rev()
                .filter(move |entry| entry.is::<K>() && (!entry.scoped || depth <= 1))
                .filter_map(|entry| entry.downcast::<K>())
        })
    }

    /// Iterate over the recipes for the given node.
    fn recipes(self, node: TypeId) -> impl Iterator<Item = &'a Recipe> {
        self.maps()
            .flat_map(|map| map.recipes.iter().rev())
            .filter(move |recipe| recipe.node == node)
    }

    /// Iterate over the map links of the chain.
    fn maps(self) -> impl Iterator<Item = &'a StyleMap> {
        self.links().filter_map(|link| match link {
            Link::Map(map) => Some(map),
            Link::Barrier(_) => None,
        })
    }

    /// Iterate over the links of the chain.
    fn links(self) -> impl Iterator<Item = Link<'a>> {
        let mut cursor = Some(self);
        std::iter::from_fn(move || {
            let Self { link, outer } = cursor?;
            cursor = outer.copied();
            link
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

impl PartialEq for StyleChain<'_> {
    fn eq(&self, other: &Self) -> bool {
        let as_ptr = |s| s as *const _;
        self.link == other.link && self.outer.map(as_ptr) == other.outer.map(as_ptr)
    }
}

impl PartialEq for Link<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (*self, *other) {
            (Self::Map(a), Self::Map(b)) => std::ptr::eq(a, b),
            (Self::Barrier(a), Self::Barrier(b)) => a == b,
            _ => false,
        }
    }
}

/// A sequence of items with associated styles.
#[derive(Hash)]
pub struct StyleVec<T> {
    items: Vec<T>,
    maps: Vec<(StyleMap, usize)>,
}

impl<T> StyleVec<T> {
    /// Whether there are any items in the sequence.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Iterate over the contained items.
    pub fn items(&self) -> std::slice::Iter<'_, T> {
        self.items.iter()
    }

    /// Iterate over the contained items and associated style maps.
    pub fn iter(&self) -> impl Iterator<Item = (&T, &StyleMap)> + '_ {
        let styles = self
            .maps
            .iter()
            .flat_map(|(map, count)| std::iter::repeat(map).take(*count));
        self.items().zip(styles)
    }

    /// Insert an element in the front. The element will share the style of the
    /// current first element.
    ///
    /// This method has no effect if the vector is empty.
    pub fn push_front(&mut self, item: T) {
        if !self.maps.is_empty() {
            self.items.insert(0, item);
            self.maps[0].1 += 1;
        }
    }
}

impl<T> Default for StyleVec<T> {
    fn default() -> Self {
        Self { items: vec![], maps: vec![] }
    }
}

impl<T: Debug> Debug for StyleVec<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.iter().map(|(item, map)| {
                crate::util::debug(|f| {
                    map.fmt(f)?;
                    item.fmt(f)
                })
            }))
            .finish()
    }
}

/// Assists in the construction of a [`StyleVec`].
pub struct StyleVecBuilder<'a, T> {
    items: Vec<T>,
    chains: Vec<(StyleChain<'a>, usize)>,
}

impl<'a, T> StyleVecBuilder<'a, T> {
    /// Create a new style-vec builder.
    pub fn new() -> Self {
        Self { items: vec![], chains: vec![] }
    }

    /// Push a new item into the style vector.
    pub fn push(&mut self, item: T, styles: StyleChain<'a>) {
        self.items.push(item);

        if let Some((prev, count)) = self.chains.last_mut() {
            if *prev == styles {
                *count += 1;
                return;
            }
        }

        self.chains.push((styles, 1));
    }

    /// Access the last item mutably and its chain by value.
    pub fn last_mut(&mut self) -> Option<(&mut T, StyleChain<'a>)> {
        let item = self.items.last_mut()?;
        let chain = self.chains.last()?.0;
        Some((item, chain))
    }

    /// Iterate over the contained items.
    pub fn items(&self) -> std::slice::Iter<'_, T> {
        self.items.iter()
    }

    /// Finish building, returning a pair of two things:
    /// - a style vector of items with the non-shared styles
    /// - a shared prefix chain of styles that apply to all items
    pub fn finish(self) -> (StyleVec<T>, StyleChain<'a>) {
        let mut iter = self.chains.iter();
        let mut trunk = match iter.next() {
            Some(&(chain, _)) => chain,
            None => return Default::default(),
        };

        let mut shared = trunk.len();
        for &(mut chain, _) in iter {
            let len = chain.len();
            if len < shared {
                for _ in 0 .. shared - len {
                    trunk.pop();
                }
                shared = len;
            } else if len > shared {
                for _ in 0 .. len - shared {
                    chain.pop();
                }
            }

            while shared > 0 && chain != trunk {
                trunk.pop();
                chain.pop();
                shared -= 1;
            }
        }

        let maps = self
            .chains
            .into_iter()
            .map(|(chain, count)| (chain.suffix(shared), count))
            .collect();

        (StyleVec { items: self.items, maps }, trunk)
    }
}

impl<'a, T> Default for StyleVecBuilder<'a, T> {
    fn default() -> Self {
        Self::new()
    }
}
