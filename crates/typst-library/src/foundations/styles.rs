use std::any::{Any, TypeId};
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::{mem, ptr};

use comemo::Tracked;
use ecow::{eco_vec, EcoString, EcoVec};
use smallvec::SmallVec;
use typst_syntax::Span;
use typst_utils::LazyHash;

use crate::diag::{SourceResult, Trace, Tracepoint};
use crate::engine::Engine;
use crate::foundations::{
    cast, ty, Content, Context, Element, Func, NativeElement, OneOrMultiple, Repr,
    Selector,
};
use crate::text::{FontFamily, FontList, TextElem};

/// A list of style properties.
#[ty(cast)]
#[derive(Default, PartialEq, Clone, Hash)]
pub struct Styles(EcoVec<LazyHash<Style>>);

impl Styles {
    /// Create a new, empty style list.
    pub const fn new() -> Self {
        Self(EcoVec::new())
    }

    /// Whether this contains no styles.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate over the contained styles.
    pub fn iter(&self) -> impl Iterator<Item = &Style> {
        self.0.iter().map(|style| &**style)
    }

    /// Iterate over the contained styles.
    pub fn as_slice(&self) -> &[LazyHash<Style>] {
        self.0.as_slice()
    }

    /// Set an inner value for a style property.
    ///
    /// If the property needs folding and the value is already contained in the
    /// style map, `self` contributes the outer values and `value` is the inner
    /// one.
    pub fn set(&mut self, style: impl Into<Style>) {
        self.0.push(LazyHash::new(style.into()));
    }

    /// Remove the style that was last set.
    pub fn unset(&mut self) {
        self.0.pop();
    }

    /// Apply outer styles. Like [`chain`](StyleChain::chain), but in-place.
    pub fn apply(&mut self, mut outer: Self) {
        outer.0.extend(mem::take(self).0);
        *self = outer;
    }

    /// Apply one outer styles.
    pub fn apply_one(&mut self, outer: Style) {
        self.0.insert(0, LazyHash::new(outer));
    }

    /// Add an origin span to all contained properties.
    pub fn spanned(mut self, span: Span) -> Self {
        for entry in self.0.make_mut() {
            if let Style::Property(property) = &mut **entry {
                property.span = span;
            }
        }
        self
    }

    /// Marks the styles as having been applied outside of any show rule.
    pub fn outside(mut self) -> Self {
        for entry in self.0.make_mut() {
            match &mut **entry {
                Style::Property(property) => property.outside = true,
                Style::Recipe(recipe) => recipe.outside = true,
                _ => {}
            }
        }
        self
    }

    /// Marks the styles as being allowed to be lifted up to the page level.
    pub fn liftable(mut self) -> Self {
        for entry in self.0.make_mut() {
            if let Style::Property(property) = &mut **entry {
                property.liftable = true;
            }
        }
        self
    }

    /// Whether there is a style for the given field of the given element.
    pub fn has<T: NativeElement>(&self, field: u8) -> bool {
        let elem = E::ELEM;
        self.0
            .iter()
            .filter_map(|style| style.property())
            .any(|property| property.is_of(elem) && property.id == field)
    }

    /// Set a font family composed of a preferred family and existing families
    /// from a style chain.
    pub fn set_family(&mut self, preferred: FontFamily, existing: StyleChain) {
        self.set(
            TextElem::font,
            FontList(
                std::iter::once(preferred)
                    .chain(existing.get_ref(TextElem::font).into_iter().cloned())
                    .collect(),
            ),
        );
    }
}

impl From<LazyHash<Style>> for Styles {
    fn from(style: LazyHash<Style>) -> Self {
        Self(eco_vec![style])
    }
}

impl From<Style> for Styles {
    fn from(style: Style) -> Self {
        Self(eco_vec![LazyHash::new(style)])
    }
}

impl IntoIterator for Styles {
    type Item = LazyHash<Style>;
    type IntoIter = ecow::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<LazyHash<Style>> for Styles {
    fn from_iter<T: IntoIterator<Item = LazyHash<Style>>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl Debug for Styles {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Styles ")?;
        f.debug_list().entries(&self.0).finish()
    }
}

impl Repr for Styles {
    fn repr(&self) -> EcoString {
        "..".into()
    }
}

/// A single style property or recipe.
#[derive(Clone, Hash)]
pub enum Style {
    /// A style property originating from a set rule or constructor.
    Property(Property),
    /// A show rule recipe.
    Recipe(Recipe),
    /// Disables a specific show rule recipe.
    ///
    /// Note: This currently only works for regex recipes since it's the only
    /// place we need it for the moment. Normal show rules use guards directly
    /// on elements instead.
    Revocation(RecipeIndex),
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

    /// The style's span, if any.
    pub fn span(&self) -> Span {
        match self {
            Self::Property(property) => property.span,
            Self::Recipe(recipe) => recipe.span,
            Self::Revocation(_) => Span::detached(),
        }
    }

    /// Returns `Some(_)` with an optional span if this style is for
    /// the given element.
    pub fn element(&self) -> Option<Element> {
        match self {
            Style::Property(property) => Some(property.elem),
            Style::Recipe(recipe) => match recipe.selector {
                Some(Selector::Elem(elem, _)) => Some(elem),
                _ => None,
            },
            Style::Revocation(_) => None,
        }
    }

    /// Whether the style is allowed to be lifted up to the page level. Only
    /// true for styles originating from set rules.
    pub fn liftable(&self) -> bool {
        match self {
            Self::Property(property) => property.liftable,
            Self::Recipe(_) => true,
            Self::Revocation(_) => false,
        }
    }

    /// Whether the style was applied outside of any show rule. This is set
    /// during realization.
    pub fn outside(&self) -> bool {
        match self {
            Self::Property(property) => property.outside,
            Self::Recipe(recipe) => recipe.outside,
            Self::Revocation(_) => false,
        }
    }

    /// Turn this style into prehashed style.
    pub fn wrap(self) -> LazyHash<Style> {
        LazyHash::new(self)
    }
}

impl Debug for Style {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Property(property) => property.fmt(f),
            Self::Recipe(recipe) => recipe.fmt(f),
            Self::Revocation(guard) => guard.fmt(f),
        }
    }
}

impl From<Property> for Style {
    fn from(property: Property) -> Self {
        Self::Property(property)
    }
}

impl From<Recipe> for Style {
    fn from(recipe: Recipe) -> Self {
        Self::Recipe(recipe)
    }
}

/// A style property originating from a set rule or constructor.
#[derive(Clone, Hash)]
pub struct Property {
    /// The element the property belongs to.
    elem: Element,
    /// The property's ID.
    id: u8,
    /// The property's value.
    value: Block,
    /// The span of the set rule the property stems from.
    span: Span,
    /// Whether the property is allowed to be lifted up to the page level.
    liftable: bool,
    /// Whether the property was applied outside of any show rule.
    outside: bool,
}

impl Property {
    /// Create a new property from a key-value pair.
    pub fn new<E, T>(id: u8, value: T) -> Self
    where
        E: NativeElement,
        T: Debug + Clone + Hash + Send + Sync + 'static,
    {
        Self {
            elem: E::ELEM,
            id,
            value: Block::new(value),
            span: Span::detached(),
            liftable: false,
            outside: false,
        }
    }

    /// Whether this property is the given one.
    pub fn is(&self, elem: Element, id: u8) -> bool {
        self.elem == elem && self.id == id
    }

    /// Whether this property belongs to the given element.
    pub fn is_of(&self, elem: Element) -> bool {
        self.elem == elem
    }

    /// Turn this property into prehashed style.
    pub fn wrap(self) -> LazyHash<Style> {
        Style::Property(self).wrap()
    }
}

impl Debug for Property {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "Set({}.{}: ",
            self.elem.name(),
            self.elem.field_name(self.id).unwrap()
        )?;
        self.value.fmt(f)?;
        write!(f, ")")
    }
}

/// A block storage for storing style values.
///
/// We're using a `Box` since values will either be contained in an `Arc` and
/// therefore already on the heap or they will be small enough that we can just
/// clone them.
#[derive(Hash)]
struct Block(Box<dyn Blockable>);

impl Block {
    /// Creates a new block.
    fn new<T: Blockable>(value: T) -> Self {
        Self(Box::new(value))
    }

    /// Downcasts the block to the specified type.
    fn downcast<T: 'static>(&self) -> Option<&T> {
        self.0.as_any().downcast_ref()
    }
}

impl Debug for Block {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Clone for Block {
    fn clone(&self) -> Self {
        self.0.dyn_clone()
    }
}

/// A value that can be stored in a block.
///
/// Auto derived for all types that implement [`Any`], [`Clone`], [`Hash`],
/// [`Debug`], [`Send`] and [`Sync`].
trait Blockable: Debug + Send + Sync + 'static {
    /// Equivalent to `downcast_ref` for the block.
    fn as_any(&self) -> &dyn Any;

    /// Equivalent to [`Hash`] for the block.
    fn dyn_hash(&self, state: &mut dyn Hasher);

    /// Equivalent to [`Clone`] for the block.
    fn dyn_clone(&self) -> Block;
}

impl<T: Debug + Clone + Hash + Send + Sync + 'static> Blockable for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn dyn_hash(&self, mut state: &mut dyn Hasher) {
        // Also hash the TypeId since values with different types but
        // equal data should be different.
        TypeId::of::<Self>().hash(&mut state);
        self.hash(&mut state);
    }

    fn dyn_clone(&self) -> Block {
        Block(Box::new(self.clone()))
    }
}

impl Hash for dyn Blockable {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.dyn_hash(state);
    }
}

/// A show rule recipe.
#[derive(Clone, PartialEq, Hash)]
pub struct Recipe {
    /// Determines whether the recipe applies to an element.
    ///
    /// If this is `None`, then this recipe is from a show rule with
    /// no selector (`show: rest => ...`), which is [eagerly applied][Content::styled_with_recipe]
    /// to the rest of the content in the scope.
    selector: Option<Selector>,
    /// The transformation to perform on the match.
    transform: Transformation,
    /// The span that errors are reported with.
    span: Span,
    /// Relevant properties of the kind of construct the style originated from
    /// and where it was applied.
    outside: bool,
}

impl Recipe {
    /// Create a new recipe from a key-value pair.
    pub fn new(
        selector: Option<Selector>,
        transform: Transformation,
        span: Span,
    ) -> Self {
        Self { selector, transform, span, outside: false }
    }

    /// The recipe's selector.
    pub fn selector(&self) -> Option<&Selector> {
        self.selector.as_ref()
    }

    /// The recipe's transformation.
    pub fn transform(&self) -> &Transformation {
        &self.transform
    }

    /// The recipe's span.
    pub fn span(&self) -> Span {
        self.span
    }

    /// Apply the recipe to the given content.
    pub fn apply(
        &self,
        engine: &mut Engine,
        context: Tracked<Context>,
        content: Content,
    ) -> SourceResult<Content> {
        let mut content = match &self.transform {
            Transformation::Content(content) => content.clone(),
            Transformation::Func(func) => {
                let mut result = func.call(engine, context, [content.clone()]);
                if self.selector.is_some() {
                    let point = || Tracepoint::Show(content.func().name().into());
                    result = result.trace(engine.world, point, content.span());
                }
                result?.display()
            }
            Transformation::Style(styles) => content.styled_with_map(styles.clone()),
        };
        if content.span().is_detached() {
            content = content.spanned(self.span);
        }
        Ok(content)
    }
}

impl Debug for Recipe {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Show(")?;
        if let Some(selector) = &self.selector {
            selector.fmt(f)?;
            f.write_str(", ")?;
        }
        self.transform.fmt(f)?;
        f.write_str(")")
    }
}

/// Identifies a show rule recipe from the top of the chain.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct RecipeIndex(pub usize);

/// A show rule transformation that can be applied to a match.
#[derive(Clone, PartialEq, Hash)]
pub enum Transformation {
    /// Replacement content.
    Content(Content),
    /// A function to apply to the match.
    Func(Func),
    /// Apply styles to the content.
    Style(Styles),
}

impl Debug for Transformation {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Content(content) => content.fmt(f),
            Self::Func(func) => func.fmt(f),
            Self::Style(styles) => styles.fmt(f),
        }
    }
}

cast! {
    Transformation,
    content: Content => Self::Content(content),
    func: Func => Self::Func(func),
}

/// A chain of styles, similar to a linked list.
///
/// A style chain allows to combine properties from multiple style lists in a
/// element hierarchy in a non-allocating way. Rather than eagerly merging the
/// lists, each access walks the hierarchy from the innermost to the outermost
/// map, trying to find a match and then folding it with matches further up the
/// chain.
#[derive(Default, Clone, Copy, Hash)]
pub struct StyleChain<'a> {
    /// The first link of this chain.
    head: &'a [LazyHash<Style>],
    /// The remaining links in the chain.
    tail: Option<&'a Self>,
}

impl<'a> StyleChain<'a> {
    /// Start a new style chain with root styles.
    pub fn new(root: &'a Styles) -> Self {
        Self { head: &root.0, tail: None }
    }

    /// Make the given chainable the first link of this chain.
    ///
    /// The resulting style chain contains styles from `local` as well as
    /// `self`. The ones from `local` take precedence over the ones from
    /// `self`. For folded properties `local` contributes the inner value.
    pub fn chain<'b, C>(&'b self, local: &'b C) -> StyleChain<'b>
    where
        C: Chainable + ?Sized,
    {
        Chainable::chain(local, self)
    }

    /// Cast the first value for the given property in the chain.
    pub fn get<T: Clone + 'static>(
        self,
        func: Element,
        id: u8,
        inherent: Option<&T>,
        default: impl Fn() -> T,
    ) -> T {
        self.properties::<T>(func, id, inherent)
            .next()
            .cloned()
            .unwrap_or_else(default)
    }

    /// Cast the first value for the given property in the chain,
    /// returning a borrowed value.
    pub fn get_ref<T: 'static>(
        self,
        func: Element,
        id: u8,
        inherent: Option<&'a T>,
        default: impl Fn() -> &'a T,
    ) -> &'a T {
        self.properties::<T>(func, id, inherent)
            .next()
            .unwrap_or_else(default)
    }

    /// Cast the first value for the given property in the chain, taking
    /// `Fold` implementations into account.
    pub fn get_folded<T: Fold + Clone + 'static>(
        self,
        func: Element,
        id: u8,
        inherent: Option<&T>,
        default: impl Fn() -> T,
    ) -> T {
        fn next<T: Fold>(
            mut values: impl Iterator<Item = T>,
            default: &impl Fn() -> T,
        ) -> T {
            values
                .next()
                .map(|value| value.fold(next(values, default)))
                .unwrap_or_else(default)
        }
        next(self.properties::<T>(func, id, inherent).cloned(), &default)
    }

    /// Iterate over all values for the given property in the chain.
    fn properties<T: 'static>(
        self,
        func: Element,
        id: u8,
        inherent: Option<&'a T>,
    ) -> impl Iterator<Item = &'a T> {
        inherent.into_iter().chain(
            self.entries()
                .filter_map(|style| style.property())
                .filter(move |property| property.is(func, id))
                .map(|property| &property.value)
                .map(move |value| {
                    value.downcast().unwrap_or_else(|| {
                        panic!(
                            "attempted to read a value of a different type than was written {}.{}: {:?}",
                            func.name(),
                            func.field_name(id).unwrap(),
                            value
                        )
                    })
                }),
        )
    }

    /// Iterate over the entries of the chain.
    pub fn entries(self) -> Entries<'a> {
        Entries { inner: [].as_slice().iter(), links: self.links() }
    }

    /// Iterate over the recipes in the chain.
    pub fn recipes(self) -> impl Iterator<Item = &'a Recipe> {
        self.entries().filter_map(|style| style.recipe())
    }

    /// Iterate over the links of the chain.
    pub fn links(self) -> Links<'a> {
        Links(Some(self))
    }

    /// Convert to a style map.
    pub fn to_map(self) -> Styles {
        let mut styles: EcoVec<_> = self.entries().cloned().collect();
        styles.make_mut().reverse();
        Styles(styles)
    }

    /// Build owned styles from the suffix (all links beyond the `len`) of the
    /// chain.
    pub fn suffix(self, len: usize) -> Styles {
        let mut styles = EcoVec::new();
        let take = self.links().count().saturating_sub(len);
        for link in self.links().take(take) {
            styles.extend(link.iter().cloned().rev());
        }
        styles.make_mut().reverse();
        Styles(styles)
    }

    /// Remove the last link from the chain.
    pub fn pop(&mut self) {
        *self = self.tail.copied().unwrap_or_default();
    }

    /// Determine the shared trunk of a collection of style chains.
    pub fn trunk(iter: impl IntoIterator<Item = Self>) -> Option<Self> {
        // Determine shared style depth and first span.
        let mut iter = iter.into_iter();
        let mut trunk = iter.next()?;
        let mut depth = trunk.links().count();

        for mut chain in iter {
            let len = chain.links().count();
            if len < depth {
                for _ in 0..depth - len {
                    trunk.pop();
                }
                depth = len;
            } else if len > depth {
                for _ in 0..len - depth {
                    chain.pop();
                }
            }

            while depth > 0 && chain != trunk {
                trunk.pop();
                chain.pop();
                depth -= 1;
            }
        }

        Some(trunk)
    }
}

impl Debug for StyleChain<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("StyleChain ")?;
        f.debug_list()
            .entries(self.entries().collect::<Vec<_>>().into_iter().rev())
            .finish()
    }
}

impl PartialEq for StyleChain<'_> {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self.head, other.head)
            && match (self.tail, other.tail) {
                (Some(a), Some(b)) => ptr::eq(a, b),
                (None, None) => true,
                _ => false,
            }
    }
}

/// Things that can be attached to a style chain.
pub trait Chainable {
    /// Attach `self` as the first link of the chain.
    fn chain<'a>(&'a self, outer: &'a StyleChain<'_>) -> StyleChain<'a>;
}

impl Chainable for LazyHash<Style> {
    fn chain<'a>(&'a self, outer: &'a StyleChain<'_>) -> StyleChain<'a> {
        StyleChain {
            head: std::slice::from_ref(self),
            tail: Some(outer),
        }
    }
}

impl Chainable for [LazyHash<Style>] {
    fn chain<'a>(&'a self, outer: &'a StyleChain<'_>) -> StyleChain<'a> {
        if self.is_empty() {
            *outer
        } else {
            StyleChain { head: self, tail: Some(outer) }
        }
    }
}

impl<const N: usize> Chainable for [LazyHash<Style>; N] {
    fn chain<'a>(&'a self, outer: &'a StyleChain<'_>) -> StyleChain<'a> {
        Chainable::chain(self.as_slice(), outer)
    }
}

impl Chainable for Styles {
    fn chain<'a>(&'a self, outer: &'a StyleChain<'_>) -> StyleChain<'a> {
        Chainable::chain(self.0.as_slice(), outer)
    }
}

/// An iterator over the entries in a style chain.
pub struct Entries<'a> {
    inner: std::slice::Iter<'a, LazyHash<Style>>,
    links: Links<'a>,
}

impl<'a> Iterator for Entries<'a> {
    type Item = &'a LazyHash<Style>;

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
pub struct Links<'a>(Option<StyleChain<'a>>);

impl<'a> Iterator for Links<'a> {
    type Item = &'a [LazyHash<Style>];

    fn next(&mut self) -> Option<Self::Item> {
        let StyleChain { head, tail } = self.0?;
        self.0 = tail.copied();
        Some(head)
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
///
/// In the example below, the chain of stroke values is folded into a single
/// value: `4pt + red`.
///
/// ```example
/// #set rect(stroke: red)
/// #set rect(stroke: 4pt)
/// #rect()
/// ```
pub trait Fold {
    /// Fold this inner value with an outer folded value.
    fn fold(self, outer: Self) -> Self;
}

impl Fold for bool {
    fn fold(self, _: Self) -> Self {
        self
    }
}

impl<T: Fold> Fold for Option<T> {
    fn fold(self, outer: Self) -> Self {
        match (self, outer) {
            (Some(inner), Some(outer)) => Some(inner.fold(outer)),
            // An explicit `None` should be respected, thus we don't do
            // `inner.or(outer)`.
            (inner, _) => inner,
        }
    }
}

impl<T> Fold for Vec<T> {
    fn fold(self, mut outer: Self) -> Self {
        outer.extend(self);
        outer
    }
}

impl<T, const N: usize> Fold for SmallVec<[T; N]> {
    fn fold(self, mut outer: Self) -> Self {
        outer.extend(self);
        outer
    }
}

impl<T> Fold for OneOrMultiple<T> {
    fn fold(self, mut outer: Self) -> Self {
        outer.0.extend(self.0);
        outer
    }
}

/// A variant of fold for foldable optional (`Option<T>`) values where an inner
/// `None` value isn't respected (contrary to `Option`'s usual `Fold`
/// implementation, with which folding with an inner `None` always returns
/// `None`). Instead, when either of the `Option` objects is `None`, the other
/// one is necessarily returned by `fold_or`. Normal folding still occurs when
/// both values are `Some`, using `T`'s `Fold` implementation.
///
/// This is useful when `None` in a particular context means "unspecified"
/// rather than "absent", in which case a specified value (`Some`) is chosen
/// over an unspecified one (`None`), while two specified values are folded
/// together.
pub trait AlternativeFold {
    /// Attempts to fold this inner value with an outer value. However, if
    /// either value is `None`, returns the other one instead of folding.
    fn fold_or(self, outer: Self) -> Self;
}

impl<T: Fold> AlternativeFold for Option<T> {
    fn fold_or(self, outer: Self) -> Self {
        match (self, outer) {
            (Some(inner), Some(outer)) => Some(inner.fold(outer)),
            // If one of values is `None`, return the other one instead of
            // folding.
            (inner, outer) => inner.or(outer),
        }
    }
}

/// A type that accumulates depth when folded.
#[derive(Debug, Default, Clone, Copy, PartialEq, Hash)]
pub struct Depth(pub usize);

impl Fold for Depth {
    fn fold(self, outer: Self) -> Self {
        Self(outer.0 + self.0)
    }
}
