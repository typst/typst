use std::any::{Any, TypeId};
use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::Arc;

use super::{Args, Content, Func, Layout, Node, Smart, Span, Value};
use crate::diag::{At, TypResult};
use crate::geom::{Numeric, Relative, Sides, Spec};
use crate::library::layout::PageNode;
use crate::library::text::{FontFamily, ParNode, TextNode};
use crate::util::Prehashed;
use crate::Context;

/// A map of style properties.
#[derive(Default, Clone, PartialEq, Hash)]
pub struct StyleMap(Vec<Entry>);

impl StyleMap {
    /// Create a new, empty style map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether this map contains no styles.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Create a style map from a single property-value pair.
    pub fn with<'a, K: Key<'a>>(key: K, value: K::Value) -> Self {
        let mut styles = Self::new();
        styles.set(key, value);
        styles
    }

    /// Set an inner value for a style property.
    ///
    /// If the property needs folding and the value is already contained in the
    /// style map, `self` contributes the outer values and `value` is the inner
    /// one.
    pub fn set<'a, K: Key<'a>>(&mut self, key: K, value: K::Value) {
        self.0.push(Entry::Property(Property::new(key, value)));
    }

    /// Set an inner value for a style property if it is `Some(_)`.
    pub fn set_opt<'a, K: Key<'a>>(&mut self, key: K, value: Option<K::Value>) {
        if let Some(value) = value {
            self.set(key, value);
        }
    }

    /// Set a font family composed of a preferred family and existing families
    /// from a style chain.
    pub fn set_family(&mut self, preferred: FontFamily, existing: StyleChain) {
        self.set(
            TextNode::FAMILY,
            std::iter::once(preferred)
                .chain(existing.get(TextNode::FAMILY).iter().cloned())
                .collect(),
        );
    }

    /// Set a show rule recipe for a node.
    pub fn set_recipe<T: Node>(&mut self, func: Func, span: Span) {
        self.0.push(Entry::Recipe(Recipe::new::<T>(func, span)));
    }

    /// Make `self` the first link of the `tail` chain.
    ///
    /// The resulting style chain contains styles from `self` as well as
    /// `tail`. The ones from `self` take precedence over the ones from
    /// `tail`. For folded properties `self` contributes the inner value.
    pub fn chain<'a>(&'a self, tail: &'a StyleChain<'a>) -> StyleChain<'a> {
        if self.is_empty() {
            *tail
        } else {
            StyleChain { head: &self.0, tail: Some(tail) }
        }
    }

    /// Set an outer value for a style property.
    ///
    /// If the property needs folding and the value is already contained in the
    /// style map, `self` contributes the inner values and `value` is the outer
    /// one.
    ///
    /// Like [`chain`](Self::chain) or [`apply_map`](Self::apply_map), but with
    /// only a single property.
    pub fn apply<'a, K: Key<'a>>(&mut self, key: K, value: K::Value) {
        self.0.insert(0, Entry::Property(Property::new(key, value)));
    }

    /// Apply styles from `tail` in-place. The resulting style map is equivalent
    /// to the style chain created by `self.chain(StyleChain::new(tail))`.
    ///
    /// This is useful over `chain` when you want to combine two maps, but you
    /// still need an owned map without a lifetime.
    pub fn apply_map(&mut self, tail: &Self) {
        self.0.splice(0 .. 0, tail.0.iter().cloned());
    }

    /// Mark all contained properties as _scoped_. This means that they only
    /// apply to the first descendant node (of their type) in the hierarchy and
    /// not its children, too. This is used by [constructors](Node::construct).
    pub fn scoped(mut self) -> Self {
        for entry in &mut self.0 {
            if let Entry::Property(property) = entry {
                property.scoped = true;
            }
        }
        self
    }

    /// The highest-level kind of of structure the map interrupts.
    pub fn interruption(&self) -> Option<Interruption> {
        self.0
            .iter()
            .filter_map(|entry| entry.property())
            .filter_map(|property| property.interruption())
            .max()
    }
}

impl Debug for StyleMap {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        for entry in self.0.iter().rev() {
            writeln!(f, "{:?}", entry)?;
        }
        Ok(())
    }
}

/// An entry for a single style property, recipe or barrier.
#[derive(Clone, PartialEq, Hash)]
enum Entry {
    /// A style property originating from a set rule or constructor.
    Property(Property),
    /// A barrier for scoped styles.
    Barrier(TypeId, &'static str),
    /// A show rule recipe.
    Recipe(Recipe),
}

impl Entry {
    /// If this is a property, return it.
    fn property(&self) -> Option<&Property> {
        match self {
            Self::Property(property) => Some(property),
            _ => None,
        }
    }

    /// If this is a recipe, return it.
    fn recipe(&self) -> Option<&Recipe> {
        match self {
            Self::Recipe(recipe) => Some(recipe),
            _ => None,
        }
    }
}

impl Debug for Entry {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("#[")?;
        match self {
            Self::Property(property) => property.fmt(f)?,
            Self::Recipe(recipe) => recipe.fmt(f)?,
            Self::Barrier(_, name) => write!(f, "Barrier for {name}")?,
        }
        f.write_str("]")
    }
}

/// A style property originating from a set rule or constructor.
#[derive(Clone, Hash)]
struct Property {
    /// The type id of the property's [key](Key).
    key: TypeId,
    /// The type id of the node the property belongs to.
    node: TypeId,
    /// The name of the property.
    name: &'static str,
    /// The property's value.
    value: Arc<Prehashed<dyn Bounds>>,
    /// Whether the property should only affects the first node down the
    /// hierarchy. Used by constructors.
    scoped: bool,
}

impl Property {
    /// Create a new property from a key-value pair.
    fn new<'a, K: Key<'a>>(_: K, value: K::Value) -> Self {
        Self {
            key: TypeId::of::<K>(),
            node: K::node(),
            name: K::NAME,
            value: Arc::new(Prehashed::new(value)),
            scoped: false,
        }
    }

    /// What kind of structure the property interrupts.
    fn interruption(&self) -> Option<Interruption> {
        if self.is_of::<PageNode>() {
            Some(Interruption::Page)
        } else if self.is_of::<ParNode>() {
            Some(Interruption::Par)
        } else {
            None
        }
    }

    /// Access the property's value if it is of the given key.
    fn downcast<'a, K: Key<'a>>(&'a self) -> Option<&'a K::Value> {
        if self.key == TypeId::of::<K>() {
            (**self.value).as_any().downcast_ref()
        } else {
            None
        }
    }

    /// Whether this property belongs to the node `T`.
    fn is_of<T: Node>(&self) -> bool {
        self.node == TypeId::of::<T>()
    }
}

impl Debug for Property {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} = {:?}", self.name, self.value)?;
        if self.scoped {
            write!(f, " [scoped]")?;
        }
        Ok(())
    }
}

impl PartialEq for Property {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.value.eq(&other.value)
            && self.scoped == other.scoped
    }
}

trait Bounds: Debug + Sync + Send + 'static {
    fn as_any(&self) -> &dyn Any;
}

impl<T> Bounds for T
where
    T: Debug + Sync + Send + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Style property keys.
///
/// This trait is not intended to be implemented manually, but rather through
/// the `#[node]` proc-macro.
pub trait Key<'a>: 'static {
    /// The unfolded type which this property is stored as in a style map. For
    /// example, this is [`Toggle`](crate::geom::Length) for the
    /// [`STRONG`](TextNode::STRONG) property.
    type Value: Debug + Clone + Hash + Sync + Send + 'static;

    /// The folded type of value that is returned when reading this property
    /// from a style chain. For example, this is [`bool`] for the
    /// [`STRONG`](TextNode::STRONG) property. For non-copy, non-folding
    /// properties this is a reference type.
    type Output;

    /// The name of the property, used for debug printing.
    const NAME: &'static str;

    /// The type id of the node this property belongs to.
    fn node() -> TypeId;

    /// Compute an output value from a sequence of values belong to this key,
    /// folding if necessary.
    fn get(
        chain: StyleChain<'a>,
        values: impl Iterator<Item = &'a Self::Value>,
    ) -> Self::Output;
}

/// A property that is resolved with other properties from the style chain.
pub trait Resolve {
    /// The type of the resolved output.
    type Output;

    /// Resolve the value using the style chain.
    fn resolve(self, styles: StyleChain) -> Self::Output;
}

impl<T: Resolve> Resolve for Option<T> {
    type Output = Option<T::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|v| v.resolve(styles))
    }
}

impl<T: Resolve> Resolve for Smart<T> {
    type Output = Smart<T::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|v| v.resolve(styles))
    }
}

impl<T: Resolve> Resolve for Spec<T> {
    type Output = Spec<T::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|v| v.resolve(styles))
    }
}

impl<T: Resolve> Resolve for Sides<T> {
    type Output = Sides<T::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        Sides {
            left: self.left.resolve(styles),
            right: self.right.resolve(styles),
            top: self.top.resolve(styles),
            bottom: self.bottom.resolve(styles),
        }
    }
}

impl<T> Resolve for Relative<T>
where
    T: Resolve + Numeric,
    <T as Resolve>::Output: Numeric,
{
    type Output = Relative<<T as Resolve>::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|abs| abs.resolve(styles))
    }
}

/// A property that is folded to determine its final value.
pub trait Fold {
    /// The type of the folded output.
    type Output;

    /// Fold this inner value with an outer folded value.
    fn fold(self, outer: Self::Output) -> Self::Output;
}

impl<T> Fold for Option<T>
where
    T: Fold,
    T::Output: Default,
{
    type Output = Option<T::Output>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.map(|inner| inner.fold(outer.unwrap_or_default()))
    }
}

impl<T> Fold for Smart<T>
where
    T: Fold,
    T::Output: Default,
{
    type Output = Smart<T::Output>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.map(|inner| inner.fold(outer.unwrap_or_default()))
    }
}

/// A show rule recipe.
#[derive(Clone, PartialEq, Hash)]
struct Recipe {
    /// The affected node.
    node: TypeId,
    /// The name of the affected node.
    name: &'static str,
    /// The function that defines the recipe.
    func: Func,
    /// The span to report all erros with.
    span: Span,
}

impl Recipe {
    /// Create a new recipe for the node `T`.
    fn new<T: Node>(func: Func, span: Span) -> Self {
        Self {
            node: TypeId::of::<T>(),
            name: std::any::type_name::<T>(),
            func,
            span,
        }
    }
}

impl Debug for Recipe {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Recipe for {} from {:?}", self.name, self.span)
    }
}

/// A style chain barrier.
///
/// Barriers interact with [scoped](StyleMap::scoped) styles: A scoped style
/// can still be read through a single barrier (the one of the node it
/// _should_ apply to), but a second barrier will make it invisible.
#[derive(Clone, PartialEq, Hash)]
pub struct Barrier(Entry);

impl Barrier {
    /// Create a new barrier for the layout node `T`.
    pub fn new<T: Layout>() -> Self {
        Self(Entry::Barrier(
            TypeId::of::<T>(),
            std::any::type_name::<T>(),
        ))
    }

    /// Make this barrier the first link of the `tail` chain.
    pub fn chain<'a>(&'a self, tail: &'a StyleChain) -> StyleChain<'a> {
        // We have to store a full `Entry` enum inside the barrier because
        // otherwise the `slice::from_ref` trick below won't work.
        // Unfortunately, that also means we have to somehow extract the id
        // here.
        let id = match self.0 {
            Entry::Barrier(id, _) => id,
            _ => unreachable!(),
        };

        if tail
            .entries()
            .filter_map(Entry::property)
            .any(|p| p.scoped && p.node == id)
        {
            StyleChain {
                head: std::slice::from_ref(&self.0),
                tail: Some(tail),
            }
        } else {
            *tail
        }
    }
}

impl Debug for Barrier {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
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

/// A chain of style maps, similar to a linked list.
///
/// A style chain allows to combine properties from multiple style maps in a
/// node hierarchy in a non-allocating way. Rather than eagerly merging the
/// maps, each access walks the hierarchy from the innermost to the outermost
/// map, trying to find a match and then folding it with matches further up the
/// chain.
#[derive(Default, Clone, Copy, Hash)]
pub struct StyleChain<'a> {
    /// The first link of this chain.
    head: &'a [Entry],
    /// The remaining links in the chain.
    tail: Option<&'a Self>,
}

impl<'a> StyleChain<'a> {
    /// Create a new, empty style chain.
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a new style chain with a root map.
    pub fn with_root(root: &'a StyleMap) -> Self {
        Self { head: &root.0, tail: None }
    }

    /// Get the output value of a style property.
    ///
    /// Returns the property's default value if no map in the chain contains an
    /// entry for it. Also takes care of resolving and folding and returns
    /// references where applicable.
    pub fn get<K: Key<'a>>(self, key: K) -> K::Output {
        K::get(self, self.values(key))
    }

    /// Execute and return the result of a user recipe for a node if there is
    /// any.
    pub fn show<T, I>(self, ctx: &mut Context, values: I) -> TypResult<Option<Content>>
    where
        T: Node,
        I: IntoIterator<Item = Value>,
    {
        if let Some(recipe) = self
            .entries()
            .filter_map(Entry::recipe)
            .find(|recipe| recipe.node == TypeId::of::<T>())
        {
            let args = Args::from_values(recipe.span, values);
            Ok(Some(recipe.func.call(ctx, args)?.cast().at(recipe.span)?))
        } else {
            Ok(None)
        }
    }
}

impl<'a> StyleChain<'a> {
    /// Return the chain, but without the trailing scoped property for the given
    /// `node`. This is a 90% hack fix for show node constructor scoping.
    pub(super) fn unscoped(mut self, node: TypeId) -> Self {
        while self
            .head
            .last()
            .and_then(Entry::property)
            .map_or(false, |p| p.scoped && p.node == node)
        {
            let len = self.head.len();
            self.head = &self.head[.. len - 1]
        }
        self
    }

    /// Remove the last link from the chain.
    fn pop(&mut self) {
        *self = self.tail.copied().unwrap_or_default();
    }

    /// Build a style map from the suffix (all links beyond the `len`) of the
    /// chain.
    fn suffix(self, len: usize) -> StyleMap {
        let mut suffix = StyleMap::new();
        let take = self.links().count().saturating_sub(len);
        for link in self.links().take(take) {
            suffix.0.splice(0 .. 0, link.iter().cloned());
        }
        suffix
    }

    /// Iterate over all values for the given property in the chain.
    fn values<K: Key<'a>>(self, _: K) -> Values<'a, K> {
        Values {
            entries: self.entries(),
            depth: 0,
            key: PhantomData,
        }
    }

    /// Iterate over the entries of the chain.
    fn entries(self) -> Entries<'a> {
        Entries {
            inner: [].as_slice().iter(),
            links: self.links(),
        }
    }

    /// Iterate over the links of the chain.
    fn links(self) -> Links<'a> {
        Links(Some(self))
    }
}

impl Debug for StyleChain<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        for entry in self.entries() {
            writeln!(f, "{:?}", entry)?;
        }
        Ok(())
    }
}

impl PartialEq for StyleChain<'_> {
    fn eq(&self, other: &Self) -> bool {
        let as_ptr = |s| s as *const _;
        self.head.as_ptr() == other.head.as_ptr()
            && self.head.len() == other.head.len()
            && self.tail.map(as_ptr) == other.tail.map(as_ptr)
    }
}

/// An iterator over the values in a style chain.
struct Values<'a, K> {
    entries: Entries<'a>,
    depth: usize,
    key: PhantomData<K>,
}

impl<'a, K: Key<'a>> Iterator for Values<'a, K> {
    type Item = &'a K::Value;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(entry) = self.entries.next() {
            match entry {
                Entry::Property(property) => {
                    if let Some(value) = property.downcast::<K>() {
                        if !property.scoped || self.depth <= 1 {
                            return Some(value);
                        }
                    }
                }
                Entry::Barrier(id, _) => {
                    self.depth += (*id == K::node()) as usize;
                }
                Entry::Recipe(_) => {}
            }
        }

        None
    }
}

/// An iterator over the entries in a style chain.
struct Entries<'a> {
    inner: std::slice::Iter<'a, Entry>,
    links: Links<'a>,
}

impl<'a> Iterator for Entries<'a> {
    type Item = &'a Entry;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(entry) = self.inner.next_back() {
                return Some(entry);
            }

            match self.links.next() {
                Some(next) => self.inner = next.iter(),
                None => return None,
            }
        }
    }
}

/// An iterator over the links of a style chain.
struct Links<'a>(Option<StyleChain<'a>>);

impl<'a> Iterator for Links<'a> {
    type Item = &'a [Entry];

    fn next(&mut self) -> Option<Self::Item> {
        let StyleChain { head, tail } = self.0?;
        self.0 = tail.copied();
        Some(head)
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

    /// Number of items in the sequence.
    pub fn len(&self) -> usize {
        self.items.len()
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

        let mut shared = trunk.links().count();
        for &(mut chain, _) in iter {
            let len = chain.links().count();
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
