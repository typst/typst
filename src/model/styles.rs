use std::any::Any;
use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::iter;
use std::marker::PhantomData;
use std::sync::Arc;

use comemo::{Prehashed, Tracked};

use super::{capability, Args, Content, Dict, Func, NodeId, Regex, Smart, Value};
use crate::diag::SourceResult;
use crate::geom::{
    Abs, Align, Axes, Corners, Em, GenAlign, Length, Numeric, PartialStroke, Rel, Sides,
};
use crate::syntax::Span;
use crate::util::ReadableTypeId;
use crate::World;

/// A map of style properties.
#[derive(Default, Clone, PartialEq, Hash)]
pub struct StyleMap(Vec<Style>);

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
        self.0.push(Style::Property(Property::new(key, value)));
    }

    /// Set an inner value for a style property if it is `Some(_)`.
    pub fn set_opt<'a, K: Key<'a>>(&mut self, key: K, value: Option<K::Value>) {
        if let Some(value) = value {
            self.set(key, value);
        }
    }

    /// Whether the map contains a style property for the given key.
    pub fn contains<'a, K: Key<'a>>(&self, _: K) -> bool {
        self.0
            .iter()
            .filter_map(|entry| entry.property())
            .any(|property| property.is::<K>())
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

    /// Set an outer style property.
    ///
    /// Like [`chain`](Self::chain) or [`apply_map`](Self::apply_map), but with
    /// only a entry.
    pub fn apply(&mut self, entry: Style) {
        self.0.insert(0, entry);
    }

    /// Apply styles from `tail` in-place. The resulting style map is equivalent
    /// to the style chain created by `self.chain(StyleChain::new(tail))`.
    ///
    /// This is useful over `chain` when you want to combine two maps, but you
    /// still need an owned map without a lifetime.
    pub fn apply_map(&mut self, tail: &Self) {
        self.0.splice(0..0, tail.0.iter().cloned());
    }

    /// Mark all contained properties as _scoped_. This means that they only
    /// apply to the first descendant node (of their type) in the hierarchy and
    /// not its children, too. This is used by
    /// [constructors](super::Node::construct).
    pub fn scoped(mut self) -> Self {
        for entry in &mut self.0 {
            if let Style::Property(property) = entry {
                property.make_scoped();
            }
        }
        self
    }

    /// Whether this map contains styles for the given `node.`
    pub fn interrupts<T: 'static>(&self) -> bool {
        self.0.iter().any(|entry| entry.is_of(NodeId::of::<T>()))
    }
}

impl From<Style> for StyleMap {
    fn from(entry: Style) -> Self {
        Self(vec![entry])
    }
}

impl Debug for StyleMap {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        for entry in self.0.iter() {
            writeln!(f, "{:?}", entry)?;
        }
        Ok(())
    }
}

/// A single style property, recipe or barrier.
#[derive(Clone, PartialEq, Hash)]
pub enum Style {
    /// A style property originating from a set rule or constructor.
    Property(Property),
    /// A show rule recipe.
    Recipe(Recipe),
    /// A barrier for scoped styles.
    Barrier(NodeId),
    /// Guards against recursive show rules.
    Guard(RecipeId),
    /// Allows recursive show rules again.
    Unguard(RecipeId),
}

impl Style {
    /// Make this style the first link of the `tail` chain.
    pub fn chain<'a>(&'a self, tail: &'a StyleChain) -> StyleChain<'a> {
        if let Style::Barrier(id) = self {
            if !tail
                .entries()
                .filter_map(Style::property)
                .any(|p| p.scoped() && *id == p.node())
            {
                return *tail;
            }
        }

        StyleChain { head: std::slice::from_ref(self), tail: Some(tail) }
    }

    /// If this is a property, return it.
    pub fn property(&self) -> Option<&Property> {
        match self {
            Self::Property(property) => Some(property),
            _ => None,
        }
    }

    /// If this is a recipe, return it.
    pub fn recipe(&self) -> Option<&Recipe> {
        match self {
            Self::Recipe(recipe) => Some(recipe),
            _ => None,
        }
    }

    /// Whether this entry contains styles for the given `node.`
    pub fn is_of(&self, node: NodeId) -> bool {
        match self {
            Self::Property(property) => property.is_of(node),
            Self::Recipe(recipe) => recipe.is_of(node),
            _ => false,
        }
    }
}

impl Debug for Style {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("#[")?;
        match self {
            Self::Property(property) => property.fmt(f)?,
            Self::Recipe(recipe) => recipe.fmt(f)?,
            Self::Barrier(id) => write!(f, "Barrier for {id:?}")?,
            Self::Guard(sel) => write!(f, "Guard against {sel:?}")?,
            Self::Unguard(sel) => write!(f, "Unguard against {sel:?}")?,
        }
        f.write_str("]")
    }
}

/// A style property key.
///
/// This trait is not intended to be implemented manually, but rather through
/// the `#[node]` proc-macro.
pub trait Key<'a>: Copy + 'static {
    /// The unfolded type which this property is stored as in a style map.
    type Value: Debug + Clone + Hash + Sync + Send + 'static;

    /// The folded type of value that is returned when reading this property
    /// from a style chain.
    type Output;

    /// The name of the property, used for debug printing.
    const NAME: &'static str;

    /// The id of the node the key belongs to.
    fn node() -> NodeId;

    /// Compute an output value from a sequence of values belonging to this key,
    /// folding if necessary.
    fn get(
        chain: StyleChain<'a>,
        values: impl Iterator<Item = &'a Self::Value>,
    ) -> Self::Output;
}

/// A style property originating from a set rule or constructor.
#[derive(Clone, Hash)]
pub struct Property {
    /// The id of the property's [key](Key).
    key: KeyId,
    /// The id of the node the property belongs to.
    node: NodeId,
    /// Whether the property should only affect the first node down the
    /// hierarchy. Used by constructors.
    scoped: bool,
    /// The property's value.
    value: Arc<Prehashed<dyn Bounds>>,
    /// The name of the property.
    #[cfg(debug_assertions)]
    name: &'static str,
}

impl Property {
    /// Create a new property from a key-value pair.
    pub fn new<'a, K: Key<'a>>(_: K, value: K::Value) -> Self {
        Self {
            key: KeyId::of::<K>(),
            node: K::node(),
            value: Arc::new(Prehashed::new(value)),
            scoped: false,
            #[cfg(debug_assertions)]
            name: K::NAME,
        }
    }

    /// Whether this property has the given key.
    pub fn is<'a, K: Key<'a>>(&self) -> bool {
        self.key == KeyId::of::<K>()
    }

    /// Whether this property belongs to the node with the given id.
    pub fn is_of(&self, node: NodeId) -> bool {
        self.node == node
    }

    /// Access the property's value if it is of the given key.
    pub fn downcast<'a, K: Key<'a>>(&'a self) -> Option<&'a K::Value> {
        if self.key == KeyId::of::<K>() {
            (**self.value).as_any().downcast_ref()
        } else {
            None
        }
    }

    /// The node this property is for.
    pub fn node(&self) -> NodeId {
        self.node
    }

    /// Whether the property is scoped.
    pub fn scoped(&self) -> bool {
        self.scoped
    }

    /// Make the property scoped.
    pub fn make_scoped(&mut self) {
        self.scoped = true;
    }
}

impl Debug for Property {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        #[cfg(debug_assertions)]
        write!(f, "{} = ", self.name)?;
        write!(f, "{:?}", self.value)?;
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

/// A unique identifier for a property key.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
struct KeyId(ReadableTypeId);

impl KeyId {
    /// The id of the given key.
    pub fn of<'a, T: Key<'a>>() -> Self {
        Self(ReadableTypeId::of::<T>())
    }
}

impl Debug for KeyId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A node that can be realized given some styles.
#[capability]
pub trait Show: 'static + Sync + Send {
    /// Unguard nested content against a specific recipe.
    fn unguard_parts(&self, id: RecipeId) -> Content;

    /// The base recipe for this node that is executed if there is no
    /// user-defined show rule.
    fn show(
        &self,
        world: Tracked<dyn World>,
        styles: StyleChain,
    ) -> SourceResult<Content>;
}

/// Post-process a node after it was realized.
#[capability]
pub trait Finalize: 'static + Sync + Send {
    /// Finalize this node given the realization of a base or user recipe. Use
    /// this for effects that should work even in the face of a user-defined
    /// show rule, for example the linking behaviour of a link node.
    fn finalize(
        &self,
        world: Tracked<dyn World>,
        styles: StyleChain,
        realized: Content,
    ) -> SourceResult<Content>;
}

/// A show rule recipe.
#[derive(Clone, PartialEq, Hash)]
pub struct Recipe {
    /// The span errors are reported with.
    pub span: Span,
    /// Determines whether the recipe applies to a node.
    pub selector: Option<Selector>,
    /// The transformation to perform on the match.
    pub transform: Transform,
}

impl Recipe {
    /// Whether the recipe is applicable to the target.
    pub fn applicable(&self, target: &Content) -> bool {
        self.selector
            .as_ref()
            .map_or(false, |selector| selector.matches(target))
    }

    /// Try to apply the recipe to the target.
    pub fn apply(
        &self,
        world: Tracked<dyn World>,
        sel: RecipeId,
        target: &Content,
    ) -> SourceResult<Option<Content>> {
        let content = match &self.selector {
            Some(Selector::Node(id, _)) => {
                if target.id() != *id {
                    return Ok(None);
                }

                self.transform.apply(world, self.span, || {
                    Value::Content(target.to::<dyn Show>().unwrap().unguard_parts(sel))
                })?
            }

            Some(Selector::Regex(regex)) => {
                let Some(text) = item!(text_str)(target) else {
                    return Ok(None);
                };

                let make = item!(text);
                let mut result = vec![];
                let mut cursor = 0;

                for mat in regex.find_iter(text) {
                    let start = mat.start();
                    if cursor < start {
                        result.push(make(text[cursor..start].into()));
                    }

                    let transformed = self
                        .transform
                        .apply(world, self.span, || Value::Str(mat.as_str().into()))?;
                    result.push(transformed);
                    cursor = mat.end();
                }

                if result.is_empty() {
                    return Ok(None);
                }

                if cursor < text.len() {
                    result.push(make(text[cursor..].into()));
                }

                Content::sequence(result)
            }

            None => return Ok(None),
        };

        Ok(Some(content.styled_with_entry(Style::Guard(sel))))
    }

    /// Whether this recipe is for the given node.
    pub fn is_of(&self, node: NodeId) -> bool {
        match self.selector {
            Some(Selector::Node(id, _)) => id == node,
            _ => false,
        }
    }
}

impl Debug for Recipe {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Recipe matching {:?}", self.selector)
    }
}

/// A selector in a show rule.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Selector {
    /// Matches a specific type of node.
    ///
    /// If there is a dictionary, only nodes with the fields from the
    /// dictionary match.
    Node(NodeId, Option<Dict>),
    /// Matches text through a regular expression.
    Regex(Regex),
}

impl Selector {
    /// Define a simple text selector.
    pub fn text(text: &str) -> Self {
        Self::Regex(Regex::new(&regex::escape(text)).unwrap())
    }

    /// Whether the selector matches for the target.
    pub fn matches(&self, target: &Content) -> bool {
        match self {
            Self::Node(id, dict) => {
                *id == target.id()
                    && dict
                        .iter()
                        .flat_map(|dict| dict.iter())
                        .all(|(name, value)| target.field(name).as_ref() == Some(value))
            }
            Self::Regex(_) => target.id() == item!(text_id),
        }
    }
}

/// A show rule transformation that can be applied to a match.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Transform {
    /// Replacement content.
    Content(Content),
    /// A function to apply to the match.
    Func(Func),
}

impl Transform {
    /// Apply the transform.
    pub fn apply<F>(
        &self,
        world: Tracked<dyn World>,
        span: Span,
        arg: F,
    ) -> SourceResult<Content>
    where
        F: FnOnce() -> Value,
    {
        match self {
            Transform::Content(content) => Ok(content.clone()),
            Transform::Func(func) => {
                let args = Args::new(span, [arg()]);
                Ok(func.call_detached(world, args)?.display(world))
            }
        }
    }
}

/// Identifies a show rule recipe.
#[derive(Debug, Copy, Clone, PartialEq, Hash)]
pub enum RecipeId {
    /// The nth recipe from the top of the chain.
    Nth(usize),
    /// The base recipe for a kind of node.
    Base(NodeId),
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
    head: &'a [Style],
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

    /// Whether the style chain has a matching recipe for the content.
    pub fn applicable(self, target: &Content) -> bool {
        // Find out how many recipes there any and whether any of them match.
        let mut n = 0;
        let mut any = true;
        for recipe in self.entries().filter_map(Style::recipe) {
            n += 1;
            any |= recipe.applicable(target);
        }

        // Find an applicable recipe.
        if any {
            for recipe in self.entries().filter_map(Style::recipe) {
                if recipe.applicable(target) {
                    let sel = RecipeId::Nth(n);
                    if !self.guarded(sel) {
                        return true;
                    }
                }
                n -= 1;
            }
        }

        false
    }

    /// Apply show recipes in this style chain to a target.
    pub fn apply(
        self,
        world: Tracked<dyn World>,
        target: &Content,
    ) -> SourceResult<Option<Content>> {
        // Find out how many recipes there any and whether any of them match.
        let mut n = 0;
        let mut any = true;
        for recipe in self.entries().filter_map(Style::recipe) {
            n += 1;
            any |= recipe.applicable(target);
        }

        // Find an applicable recipe.
        let mut realized = None;
        let mut guarded = false;
        if any {
            for recipe in self.entries().filter_map(Style::recipe) {
                if recipe.applicable(target) {
                    let sel = RecipeId::Nth(n);
                    if self.guarded(sel) {
                        guarded = true;
                    } else if let Some(content) = recipe.apply(world, sel, target)? {
                        realized = Some(content);
                        break;
                    }
                }
                n -= 1;
            }
        }

        if let Some(showable) = target.to::<dyn Show>() {
            // Realize if there was no matching recipe.
            if realized.is_none() {
                let sel = RecipeId::Base(target.id());
                if self.guarded(sel) {
                    guarded = true;
                } else {
                    let content = showable
                        .unguard_parts(sel)
                        .to::<dyn Show>()
                        .unwrap()
                        .show(world, self)?;

                    realized = Some(content.styled_with_entry(Style::Guard(sel)));
                }
            }

            // Finalize only if guarding didn't stop any recipe.
            if !guarded {
                if let Some(node) = target.to::<dyn Finalize>() {
                    if let Some(content) = realized {
                        realized = Some(node.finalize(world, self, content)?);
                    }
                }
            }
        }

        Ok(realized)
    }

    /// Whether the recipe identified by the selector is guarded.
    fn guarded(self, sel: RecipeId) -> bool {
        for entry in self.entries() {
            match *entry {
                Style::Guard(s) if s == sel => return true,
                Style::Unguard(s) if s == sel => return false,
                _ => {}
            }
        }

        false
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
            suffix.0.splice(0..0, link.iter().cloned());
        }
        suffix
    }

    /// Iterate over all values for the given property in the chain.
    fn values<K: Key<'a>>(self, _: K) -> Values<'a, K> {
        Values {
            entries: self.entries(),
            key: PhantomData,
            barriers: 0,
            guarded: false,
        }
    }

    /// Iterate over the entries of the chain.
    fn entries(self) -> Entries<'a> {
        Entries { inner: [].as_slice().iter(), links: self.links() }
    }

    /// Iterate over the links of the chain.
    fn links(self) -> Links<'a> {
        Links(Some(self))
    }
}

impl Debug for StyleChain<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        for entry in self.entries().collect::<Vec<_>>().into_iter().rev() {
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
    key: PhantomData<K>,
    barriers: usize,
    guarded: bool,
}

impl<'a, K: Key<'a>> Iterator for Values<'a, K> {
    type Item = &'a K::Value;

    fn next(&mut self) -> Option<Self::Item> {
        for entry in &mut self.entries {
            match entry {
                Style::Property(property) => {
                    if let Some(value) = property.downcast::<K>() {
                        if !property.scoped()
                            || if self.guarded {
                                self.barriers == 1
                            } else {
                                self.barriers <= 1
                            }
                        {
                            return Some(value);
                        }
                    }
                }
                Style::Barrier(id) => {
                    self.barriers += (*id == K::node()) as usize;
                }
                Style::Guard(RecipeId::Base(id)) => {
                    self.guarded |= *id == K::node();
                }
                _ => {}
            }
        }

        None
    }
}

/// An iterator over the entries in a style chain.
struct Entries<'a> {
    inner: std::slice::Iter<'a, Style>,
    links: Links<'a>,
}

impl<'a> Iterator for Entries<'a> {
    type Item = &'a Style;

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
    type Item = &'a [Style];

    fn next(&mut self) -> Option<Self::Item> {
        let StyleChain { head, tail } = self.0?;
        self.0 = tail.copied();
        Some(head)
    }
}

/// A sequence of items with associated styles.
#[derive(Clone, Hash)]
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

    /// Map the contained items.
    pub fn map<F, U>(&self, f: F) -> StyleVec<U>
    where
        F: FnMut(&T) -> U,
    {
        StyleVec {
            items: self.items.iter().map(f).collect(),
            maps: self.maps.clone(),
        }
    }

    /// Iterate over references to the contained items and associated style maps.
    pub fn iter(&self) -> impl Iterator<Item = (&T, &StyleMap)> + '_ {
        self.items().zip(
            self.maps
                .iter()
                .flat_map(|(map, count)| iter::repeat(map).take(*count)),
        )
    }

    /// Iterate over the contained items.
    pub fn items(&self) -> std::slice::Iter<'_, T> {
        self.items.iter()
    }

    /// Iterate over the contained maps. Note that zipping this with `items()`
    /// does not yield the same result as calling `iter()` because this method
    /// only returns maps once that are shared by consecutive items. This method
    /// is designed for use cases where you want to check, for example, whether
    /// any of the maps fulfills a specific property.
    pub fn styles(&self) -> impl Iterator<Item = &StyleMap> {
        self.maps.iter().map(|(map, _)| map)
    }
}

impl<T> Default for StyleVec<T> {
    fn default() -> Self {
        Self { items: vec![], maps: vec![] }
    }
}

impl<T> FromIterator<T> for StyleVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let items: Vec<_> = iter.into_iter().collect();
        let maps = vec![(StyleMap::new(), items.len())];
        Self { items, maps }
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

    /// Whether the builder is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
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
                for _ in 0..shared - len {
                    trunk.pop();
                }
                shared = len;
            } else if len > shared {
                for _ in 0..len - shared {
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

/// A property that is resolved with other properties from the style chain.
pub trait Resolve {
    /// The type of the resolved output.
    type Output;

    /// Resolve the value using the style chain.
    fn resolve(self, styles: StyleChain) -> Self::Output;
}

impl Resolve for Em {
    type Output = Abs;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        if self.is_zero() {
            Abs::zero()
        } else {
            self.at(item!(em)(styles))
        }
    }
}

impl Resolve for Length {
    type Output = Abs;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.abs + self.em.resolve(styles)
    }
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

impl<T: Resolve> Resolve for Axes<T> {
    type Output = Axes<T::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|v| v.resolve(styles))
    }
}

impl<T: Resolve> Resolve for Sides<T> {
    type Output = Sides<T::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|v| v.resolve(styles))
    }
}

impl<T: Resolve> Resolve for Corners<T> {
    type Output = Corners<T::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|v| v.resolve(styles))
    }
}

impl<T> Resolve for Rel<T>
where
    T: Resolve + Numeric,
    <T as Resolve>::Output: Numeric,
{
    type Output = Rel<<T as Resolve>::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|abs| abs.resolve(styles))
    }
}

impl Resolve for GenAlign {
    type Output = Align;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        let dir = item!(dir)(styles);
        match self {
            Self::Start => dir.start().into(),
            Self::End => dir.end().into(),
            Self::Specific(align) => align,
        }
    }
}

impl Resolve for PartialStroke {
    type Output = PartialStroke<Abs>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        PartialStroke {
            paint: self.paint,
            thickness: self.thickness.resolve(styles),
        }
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

impl<T> Fold for Sides<T>
where
    T: Fold,
{
    type Output = Sides<T::Output>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.zip(outer, |inner, outer| inner.fold(outer))
    }
}

impl Fold for Sides<Option<Rel<Abs>>> {
    type Output = Sides<Rel<Abs>>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.zip(outer, |inner, outer| inner.unwrap_or(outer))
    }
}

impl Fold for Sides<Option<Smart<Rel<Length>>>> {
    type Output = Sides<Smart<Rel<Length>>>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.zip(outer, |inner, outer| inner.unwrap_or(outer))
    }
}

impl<T> Fold for Corners<T>
where
    T: Fold,
{
    type Output = Corners<T::Output>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.zip(outer, |inner, outer| inner.fold(outer))
    }
}

impl Fold for Corners<Option<Rel<Abs>>> {
    type Output = Corners<Rel<Abs>>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.zip(outer, |inner, outer| inner.unwrap_or(outer))
    }
}

impl Fold for PartialStroke<Abs> {
    type Output = Self;

    fn fold(self, outer: Self::Output) -> Self::Output {
        Self {
            paint: self.paint.or(outer.paint),
            thickness: self.thickness.or(outer.thickness),
        }
    }
}
