use std::any::Any;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::iter;
use std::marker::PhantomData;

use comemo::Tracked;
use ecow::EcoString;

use super::{Content, Label, Node, NodeId};
use crate::diag::{SourceResult, Trace, Tracepoint};
use crate::eval::{cast_from_value, Args, Cast, Dict, Func, Regex, Value};
use crate::syntax::Span;
use crate::World;

/// A map of style properties.
#[derive(Default, Clone, Hash)]
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

    /// Set an inner value for a style property.
    ///
    /// If the property needs folding and the value is already contained in the
    /// style map, `self` contributes the outer values and `value` is the inner
    /// one.
    pub fn set<K: Key>(&mut self, key: K, value: K::Value) {
        self.0.push(Style::Property(Property::new(key, value)));
    }

    /// Set an inner value for a style property if it is `Some(_)`.
    pub fn set_opt<K: Key>(&mut self, key: K, value: Option<K::Value>) {
        if let Some(value) = value {
            self.set(key, value);
        }
    }

    /// Remove the style that was last set.
    pub fn unset(&mut self) {
        self.0.pop();
    }

    /// Whether the map contains a style property for the given key.
    pub fn contains<K: Key>(&self, _: K) -> bool {
        self.0
            .iter()
            .filter_map(|entry| entry.property())
            .any(|property| property.is::<K>())
    }

    /// Apply outer styles. Like [`chain`](StyleChain::chain), but in-place.
    pub fn apply(&mut self, outer: Self) {
        self.0.splice(0..0, outer.0.iter().cloned());
    }

    /// Set an outer style. Like [`chain_one`](StyleChain::chain_one), but
    /// in-place.
    pub fn apply_one(&mut self, outer: Style) {
        self.0.insert(0, outer);
    }

    /// Mark all contained properties as _scoped_. This means that they only
    /// apply to the first descendant node (of their type) in the hierarchy and
    /// not its children, too. This is used by
    /// [constructors](super::Construct::construct).
    pub fn scoped(mut self) -> Self {
        for entry in &mut self.0 {
            if let Style::Property(property) = entry {
                property.make_scoped();
            }
        }
        self
    }

    /// Add an origin span to all contained properties.
    pub fn spanned(mut self, span: Span) -> Self {
        for entry in &mut self.0 {
            if let Style::Property(property) = entry {
                property.origin = Some(span);
            }
        }
        self
    }

    /// Returns `Some(_)` with an optional span if this map contains styles for
    /// the given `node`.
    pub fn interruption<T: Node>(&self) -> Option<Option<Span>> {
        let node = NodeId::of::<T>();
        self.0.iter().find_map(|entry| match entry {
            Style::Property(property) => property.is_of(node).then(|| property.origin),
            Style::Recipe(recipe) => recipe.is_of(node).then(|| Some(recipe.span)),
            _ => None,
        })
    }
}

impl From<Style> for StyleMap {
    fn from(entry: Style) -> Self {
        Self(vec![entry])
    }
}

impl PartialEq for StyleMap {
    fn eq(&self, other: &Self) -> bool {
        crate::util::hash128(self) == crate::util::hash128(other)
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
#[derive(Clone, Hash)]
pub enum Style {
    /// A style property originating from a set rule or constructor.
    Property(Property),
    /// A show rule recipe.
    Recipe(Recipe),
    /// A barrier for scoped styles.
    Barrier(NodeId),
}

impl Style {
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
}

impl Debug for Style {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Property(property) => property.fmt(f),
            Self::Recipe(recipe) => recipe.fmt(f),
            Self::Barrier(id) => write!(f, "#[Barrier for {id:?}]"),
        }
    }
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
    value: Value,
    /// The span of the set rule the property stems from.
    origin: Option<Span>,
}

impl Property {
    /// Create a new property from a key-value pair.
    pub fn new<K: Key>(_: K, value: K::Value) -> Self {
        Self {
            key: KeyId::of::<K>(),
            node: K::node(),
            value: value.into(),
            scoped: false,
            origin: None,
        }
    }

    /// Whether this property has the given key.
    pub fn is<K: Key>(&self) -> bool {
        self.key == KeyId::of::<K>()
    }

    /// Whether this property belongs to the node with the given id.
    pub fn is_of(&self, node: NodeId) -> bool {
        self.node == node
    }

    /// Access the property's value if it is of the given key.
    #[track_caller]
    pub fn cast<K: Key>(&self) -> Option<K::Value> {
        if self.key == KeyId::of::<K>() {
            Some(self.value.clone().cast().unwrap_or_else(|err| {
                panic!("{} (for {} with value {:?})", err, self.key.name(), self.value)
            }))
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
        write!(f, "#set {}({}: {:?})", self.node.name(), self.key.name(), self.value)?;
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

/// A style property key.
///
/// This trait is not intended to be implemented manually.
pub trait Key: Copy + 'static {
    /// The unfolded type which this property is stored as in a style map.
    type Value: Cast + Into<Value>;

    /// The folded type of value that is returned when reading this property
    /// from a style chain.
    type Output;

    /// The id of the property.
    fn id() -> KeyId;

    /// The id of the node the key belongs to.
    fn node() -> NodeId;

    /// Compute an output value from a sequence of values belonging to this key,
    /// folding if necessary.
    fn get(chain: StyleChain, values: impl Iterator<Item = Self::Value>) -> Self::Output;
}

/// A unique identifier for a style key.
#[derive(Copy, Clone)]
pub struct KeyId(&'static KeyMeta);

impl KeyId {
    pub fn of<T: Key>() -> Self {
        T::id()
    }

    pub fn from_meta(meta: &'static KeyMeta) -> Self {
        Self(meta)
    }

    pub fn name(self) -> &'static str {
        self.0.name
    }
}

impl Debug for KeyId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(self.name())
    }
}

impl Hash for KeyId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.0 as *const _ as usize);
    }
}

impl Eq for KeyId {}

impl PartialEq for KeyId {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct KeyMeta {
    pub name: &'static str,
}

/// A show rule recipe.
#[derive(Clone, Hash)]
pub struct Recipe {
    /// The span errors are reported with.
    pub span: Span,
    /// Determines whether the recipe applies to a node.
    pub selector: Option<Selector>,
    /// The transformation to perform on the match.
    pub transform: Transform,
}

impl Recipe {
    /// Whether this recipe is for the given node.
    pub fn is_of(&self, node: NodeId) -> bool {
        match self.selector {
            Some(Selector::Node(id, _)) => id == node,
            _ => false,
        }
    }

    /// Whether the recipe is applicable to the target.
    pub fn applicable(&self, target: &Content) -> bool {
        self.selector
            .as_ref()
            .map_or(false, |selector| selector.matches(target))
    }

    /// Apply the recipe to the given content.
    pub fn apply(
        &self,
        world: Tracked<dyn World>,
        content: Content,
    ) -> SourceResult<Content> {
        match &self.transform {
            Transform::Content(content) => Ok(content.clone()),
            Transform::Func(func) => {
                let args = Args::new(self.span, [Value::Content(content.clone())]);
                let mut result = func.call_detached(world, args);
                if let Some(span) = content.span() {
                    // For selector-less show rules, a tracepoint makes no sense.
                    if self.selector.is_some() {
                        let point = || Tracepoint::Show(content.name().into());
                        result = result.trace(world, point, span);
                    }
                }
                Ok(result?.display())
            }
            Transform::Style(styles) => Ok(content.styled_with_map(styles.clone())),
        }
    }
}

impl Debug for Recipe {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "#show {:?}: {:?}", self.selector, self.transform)
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
    /// Matches nodes with a specific label.
    Label(Label),
    /// Matches text nodes through a regular expression.
    Regex(Regex),
}

impl Selector {
    /// Define a simple node selector.
    pub fn node<T: Node>() -> Self {
        Self::Node(NodeId::of::<T>(), None)
    }

    /// Define a simple text selector.
    pub fn text(text: &str) -> Self {
        Self::Regex(Regex::new(&regex::escape(text)).unwrap())
    }

    /// Whether the selector matches for the target.
    pub fn matches(&self, target: &Content) -> bool {
        match self {
            Self::Node(id, dict) => {
                target.id() == *id
                    && dict
                        .iter()
                        .flat_map(|dict| dict.iter())
                        .all(|(name, value)| target.field(name) == Some(value))
            }
            Self::Label(label) => target.label() == Some(label),
            Self::Regex(regex) => {
                target.id() == item!(text_id)
                    && item!(text_str)(target).map_or(false, |text| regex.is_match(&text))
            }
        }
    }
}

cast_from_value! {
    Selector: "selector",
    text: EcoString => Self::text(&text),
    label: Label => Self::Label(label),
    func: Func => func.select(None)?,
    regex: Regex => Self::Regex(regex),
}

/// A show rule transformation that can be applied to a match.
#[derive(Debug, Clone, Hash)]
pub enum Transform {
    /// Replacement content.
    Content(Content),
    /// A function to apply to the match.
    Func(Func),
    /// Apply styles to the content.
    Style(StyleMap),
}

cast_from_value! {
    Transform,
    content: Content => Self::Content(content),
    func: Func => {
        if func.argc().map_or(false, |count| count != 1) {
            Err("function must have exactly one parameter")?
        }
        Self::Func(func)
    },
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
    /// Start a new style chain with a root map.
    pub fn new(root: &'a StyleMap) -> Self {
        Self { head: &root.0, tail: None }
    }

    /// Make the given map the first link of this chain.
    ///
    /// The resulting style chain contains styles from `map` as well as
    /// `self`. The ones from `map` take precedence over the ones from
    /// `self`. For folded properties `map` contributes the inner value.
    pub fn chain<'b>(&'b self, map: &'b StyleMap) -> StyleChain<'b> {
        if map.is_empty() {
            *self
        } else {
            StyleChain { head: &map.0, tail: Some(self) }
        }
    }

    /// Make the given style the first link of the this chain.
    pub fn chain_one<'b>(&'b self, style: &'b Style) -> StyleChain<'b> {
        if let Style::Barrier(id) = style {
            if !self
                .entries()
                .filter_map(Style::property)
                .any(|p| p.scoped() && *id == p.node())
            {
                return *self;
            }
        }

        StyleChain {
            head: std::slice::from_ref(style),
            tail: Some(self),
        }
    }

    /// Get the output value of a style property.
    ///
    /// Returns the property's default value if no map in the chain contains an
    /// entry for it. Also takes care of resolving and folding and returns
    /// references where applicable.
    pub fn get<K: Key>(self, key: K) -> K::Output {
        K::get(self, self.values(key))
    }

    /// Iterate over all style recipes in the chain.
    pub fn recipes(self) -> impl Iterator<Item = &'a Recipe> {
        self.entries().filter_map(Style::recipe)
    }

    /// Iterate over all values for the given property in the chain.
    fn values<K: Key>(self, _: K) -> Values<'a, K> {
        Values {
            entries: self.entries(),
            key: PhantomData,
            barriers: 0,
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

    /// Remove the last link from the chain.
    fn pop(&mut self) {
        *self = self.tail.copied().unwrap_or_default();
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
        crate::util::hash128(self) == crate::util::hash128(other)
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

/// An iterator over the values in a style chain.
struct Values<'a, K> {
    entries: Entries<'a>,
    key: PhantomData<K>,
    barriers: usize,
}

impl<'a, K: Key> Iterator for Values<'a, K> {
    type Item = K::Value;

    #[track_caller]
    fn next(&mut self) -> Option<Self::Item> {
        for entry in &mut self.entries {
            match entry {
                Style::Property(property) => {
                    if let Some(value) = property.cast::<K>() {
                        if !property.scoped() || self.barriers <= 1 {
                            return Some(value);
                        }
                    }
                }
                Style::Barrier(id) => {
                    self.barriers += (*id == K::node()) as usize;
                }
                _ => {}
            }
        }

        None
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

impl StyleVec<Content> {
    pub fn to_vec(self) -> Vec<Content> {
        self.items
            .into_iter()
            .zip(
                self.maps
                    .iter()
                    .flat_map(|(map, count)| iter::repeat(map).take(*count)),
            )
            .map(|(content, map)| content.styled_with_map(map.clone()))
            .collect()
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

impl<T: Resolve> Resolve for Option<T> {
    type Output = Option<T::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|v| v.resolve(styles))
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
