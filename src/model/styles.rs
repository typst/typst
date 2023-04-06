use std::any::{Any, TypeId};
use std::fmt::{self, Debug, Formatter, Write};
use std::iter;
use std::mem;

use ecow::{eco_format, eco_vec, EcoString, EcoVec};

use super::{Content, ElemFunc, Element, Label, Vt};
use crate::diag::{SourceResult, StrResult, Trace, Tracepoint};
use crate::eval::{cast_from_value, Args, Cast, CastInfo, Dict, Func, Regex, Value, Vm};
use crate::model::Locatable;
use crate::syntax::Span;
use crate::util::pretty_array_like;

/// A list of style properties.
#[derive(Default, PartialEq, Clone, Hash)]
pub struct Styles(EcoVec<Style>);

impl Styles {
    /// Create a new, empty style list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether this contains no styles.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Set an inner value for a style property.
    ///
    /// If the property needs folding and the value is already contained in the
    /// style map, `self` contributes the outer values and `value` is the inner
    /// one.
    pub fn set(&mut self, style: impl Into<Style>) {
        self.0.push(style.into());
    }

    /// Remove the style that was last set.
    pub fn unset(&mut self) {
        self.0.pop();
    }

    /// Apply outer styles. Like [`chain`](StyleChain::chain), but in-place.
    pub fn apply(&mut self, mut outer: Self) {
        outer.0.extend(mem::take(self).0.into_iter());
        *self = outer;
    }

    /// Apply one outer styles. Like [`chain_one`](StyleChain::chain_one), but
    /// in-place.
    pub fn apply_one(&mut self, outer: Style) {
        self.0.insert(0, outer);
    }

    /// Apply a slice of outer styles.
    pub fn apply_slice(&mut self, outer: &[Style]) {
        self.0 = outer.iter().cloned().chain(mem::take(self).0.into_iter()).collect();
    }

    /// Add an origin span to all contained properties.
    pub fn spanned(mut self, span: Span) -> Self {
        for entry in self.0.make_mut() {
            if let Style::Property(property) = entry {
                property.span = Some(span);
            }
        }
        self
    }

    /// Returns `Some(_)` with an optional span if this list contains
    /// styles for the given element.
    pub fn interruption<T: Element>(&self) -> Option<Option<Span>> {
        let func = T::func();
        self.0.iter().find_map(|entry| match entry {
            Style::Property(property) => property.is_of(func).then_some(property.span),
            Style::Recipe(recipe) => recipe.is_of(func).then_some(Some(recipe.span)),
        })
    }
}

impl From<Style> for Styles {
    fn from(entry: Style) -> Self {
        Self(eco_vec![entry])
    }
}

impl Debug for Styles {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("..")
    }
}

/// A single style property or recipe.
#[derive(Clone, PartialEq, Hash)]
pub enum Style {
    /// A style property originating from a set rule or constructor.
    Property(Property),
    /// A show rule recipe.
    Recipe(Recipe),
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
#[derive(Clone, PartialEq, Hash)]
pub struct Property {
    /// The element the property belongs to.
    element: ElemFunc,
    /// The property's name.
    name: EcoString,
    /// The property's value.
    value: Value,
    /// The span of the set rule the property stems from.
    span: Option<Span>,
}

impl Property {
    /// Create a new property from a key-value pair.
    pub fn new(element: ElemFunc, name: EcoString, value: Value) -> Self {
        Self { element, name, value, span: None }
    }

    /// Whether this property is the given one.
    pub fn is(&self, element: ElemFunc, name: &str) -> bool {
        self.element == element && self.name == name
    }

    /// Whether this property belongs to the given element.
    pub fn is_of(&self, element: ElemFunc) -> bool {
        self.element == element
    }
}

impl Debug for Property {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "set {}({}: {:?})", self.element.name(), self.name, self.value)?;
        Ok(())
    }
}

/// A show rule recipe.
#[derive(Clone, PartialEq, Hash)]
pub struct Recipe {
    /// The span errors are reported with.
    pub span: Span,
    /// Determines whether the recipe applies to an element.
    pub selector: Option<Selector>,
    /// The transformation to perform on the match.
    pub transform: Transform,
}

impl Recipe {
    /// Whether this recipe is for the given type of element.
    pub fn is_of(&self, element: ElemFunc) -> bool {
        match self.selector {
            Some(Selector::Elem(own, _)) => own == element,
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
    pub fn apply_vm(&self, vm: &mut Vm, content: Content) -> SourceResult<Content> {
        match &self.transform {
            Transform::Content(content) => Ok(content.clone()),
            Transform::Func(func) => {
                let args = Args::new(self.span, [Value::Content(content.clone())]);
                let mut result = func.call_vm(vm, args);
                // For selector-less show rules, a tracepoint makes no sense.
                if self.selector.is_some() {
                    let point = || Tracepoint::Show(content.func().name().into());
                    result = result.trace(vm.world(), point, content.span());
                }
                Ok(result?.display())
            }
            Transform::Style(styles) => Ok(content.styled_with_map(styles.clone())),
        }
    }

    /// Apply the recipe to the given content.
    pub fn apply_vt(&self, vt: &mut Vt, content: Content) -> SourceResult<Content> {
        match &self.transform {
            Transform::Content(content) => Ok(content.clone()),
            Transform::Func(func) => {
                let mut result = func.call_vt(vt, [Value::Content(content.clone())]);
                if self.selector.is_some() {
                    let point = || Tracepoint::Show(content.func().name().into());
                    result = result.trace(vt.world, point, content.span());
                }
                Ok(result?.display())
            }
            Transform::Style(styles) => Ok(content.styled_with_map(styles.clone())),
        }
    }
}

impl Debug for Recipe {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("show")?;
        if let Some(selector) = &self.selector {
            f.write_char(' ')?;
            selector.fmt(f)?;
        }
        f.write_str(": ")?;
        self.transform.fmt(f)
    }
}

/// A selector in a show rule.
#[derive(Clone, PartialEq, Hash)]
pub enum Selector {
    /// Matches a specific type of element.
    ///
    /// If there is a dictionary, only elements with the fields from the
    /// dictionary match.
    Elem(ElemFunc, Option<Dict>),
    /// Matches elements with a specific label.
    Label(Label),
    /// Matches text elements through a regular expression.
    Regex(Regex),
    /// Matches elements with a specific capability.
    Can(TypeId),
    /// Matches if any of the subselectors match.
    Any(EcoVec<Self>),
    /// Matches if all of the subselectors match.
    All(EcoVec<Self>),
}

impl Selector {
    /// Define a simple text selector.
    pub fn text(text: &str) -> Self {
        Self::Regex(Regex::new(&regex::escape(text)).unwrap())
    }

    /// Define a simple [`Selector::Can`] selector.
    pub fn can<T: ?Sized + Any>() -> Self {
        Self::Can(TypeId::of::<T>())
    }

    /// Whether the selector matches for the target.
    pub fn matches(&self, target: &Content) -> bool {
        match self {
            Self::Elem(element, dict) => {
                target.func() == *element
                    && dict
                        .iter()
                        .flat_map(|dict| dict.iter())
                        .all(|(name, value)| target.field_ref(name) == Some(value))
            }
            Self::Label(label) => target.label() == Some(label),
            Self::Regex(regex) => {
                target.func() == item!(text_func)
                    && item!(text_str)(target).map_or(false, |text| regex.is_match(&text))
            }
            Self::Can(cap) => target.can_type_id(*cap),
            Self::Any(selectors) => selectors.iter().any(|sel| sel.matches(target)),
            Self::All(selectors) => selectors.iter().all(|sel| sel.matches(target)),
        }
    }
}

impl Debug for Selector {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Elem(elem, dict) => {
                f.write_str(elem.name())?;
                if let Some(dict) = dict {
                    f.write_str(".where")?;
                    dict.fmt(f)?;
                }
                Ok(())
            }
            Self::Label(label) => label.fmt(f),
            Self::Regex(regex) => regex.fmt(f),
            Self::Can(cap) => cap.fmt(f),
            Self::Any(selectors) | Self::All(selectors) => {
                f.write_str(if matches!(self, Self::Any(_)) { "any" } else { "all" })?;
                let pieces: Vec<_> =
                    selectors.iter().map(|sel| eco_format!("{sel:?}")).collect();
                f.write_str(&pretty_array_like(&pieces, false))
            }
        }
    }
}

cast_from_value! {
    Selector: "selector",
    func: Func => func
        .element()
        .ok_or("only element functions can be used as selectors")?
        .select(),
    label: Label => Self::Label(label),
    text: EcoString => Self::text(&text),
    regex: Regex => Self::Regex(regex),
}

/// A selector that can be used with `query`. Hopefully, this is made obsolote
/// by a more powerful query mechanism in the future.
#[derive(Clone, PartialEq, Hash)]
pub struct LocatableSelector(pub Selector);

impl Cast for LocatableSelector {
    fn is(value: &Value) -> bool {
        matches!(value, Value::Label(_) | Value::Func(_))
            || value.type_name() == "selector"
    }

    fn cast(value: Value) -> StrResult<Self> {
        fn validate(selector: &Selector) -> StrResult<()> {
            match &selector {
                Selector::Elem(elem, _) if !elem.can::<dyn Locatable>() => {
                    Err(eco_format!("{} is not locatable", elem.name()))?
                }
                Selector::Regex(_) => Err("text is not locatable")?,
                Selector::Any(list) | Selector::All(list) => {
                    for selector in list {
                        validate(selector)?;
                    }
                }
                _ => {}
            }
            Ok(())
        }

        if !Self::is(&value) {
            return <Self as Cast>::error(value);
        }

        let selector = Selector::cast(value)?;
        validate(&selector)?;
        Ok(Self(selector))
    }

    fn describe() -> CastInfo {
        CastInfo::Union(vec![
            CastInfo::Type("label"),
            CastInfo::Type("function"),
            CastInfo::Type("selector"),
        ])
    }
}
/// A show rule transformation that can be applied to a match.
#[derive(Clone, PartialEq, Hash)]
pub enum Transform {
    /// Replacement content.
    Content(Content),
    /// A function to apply to the match.
    Func(Func),
    /// Apply styles to the content.
    Style(Styles),
}

impl Debug for Transform {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Content(content) => content.fmt(f),
            Self::Func(func) => func.fmt(f),
            Self::Style(styles) => styles.fmt(f),
        }
    }
}

cast_from_value! {
    Transform,
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
    head: &'a [Style],
    /// The remaining links in the chain.
    tail: Option<&'a Self>,
}

impl<'a> StyleChain<'a> {
    /// Start a new style chain with root styles.
    pub fn new(root: &'a Styles) -> Self {
        Self { head: &root.0, tail: None }
    }

    /// Make the given style list the first link of this chain.
    ///
    /// The resulting style chain contains styles from `local` as well as
    /// `self`. The ones from `local` take precedence over the ones from
    /// `self`. For folded properties `local` contributes the inner value.
    pub fn chain<'b>(&'b self, local: &'b Styles) -> StyleChain<'b> {
        if local.is_empty() {
            *self
        } else {
            StyleChain { head: &local.0, tail: Some(self) }
        }
    }

    /// Make the given style the first link of the this chain.
    pub fn chain_one<'b>(&'b self, style: &'b Style) -> StyleChain<'b> {
        StyleChain {
            head: std::slice::from_ref(style),
            tail: Some(self),
        }
    }

    /// Cast the first value for the given property in the chain.
    pub fn get<T: Cast>(
        self,
        func: ElemFunc,
        name: &'a str,
        inherent: Option<Value>,
        default: impl Fn() -> T,
    ) -> T {
        self.properties::<T>(func, name, inherent)
            .next()
            .unwrap_or_else(default)
    }

    /// Cast the first value for the given property in the chain.
    pub fn get_resolve<T: Cast + Resolve>(
        self,
        func: ElemFunc,
        name: &'a str,
        inherent: Option<Value>,
        default: impl Fn() -> T,
    ) -> T::Output {
        self.get(func, name, inherent, default).resolve(self)
    }

    /// Cast the first value for the given property in the chain.
    pub fn get_fold<T: Cast + Fold>(
        self,
        func: ElemFunc,
        name: &'a str,
        inherent: Option<Value>,
        default: impl Fn() -> T::Output,
    ) -> T::Output {
        fn next<T: Fold>(
            mut values: impl Iterator<Item = T>,
            styles: StyleChain,
            default: &impl Fn() -> T::Output,
        ) -> T::Output {
            values
                .next()
                .map(|value| value.fold(next(values, styles, default)))
                .unwrap_or_else(|| default())
        }
        next(self.properties::<T>(func, name, inherent), self, &default)
    }

    /// Cast the first value for the given property in the chain.
    pub fn get_resolve_fold<T>(
        self,
        func: ElemFunc,
        name: &'a str,
        inherent: Option<Value>,
        default: impl Fn() -> <T::Output as Fold>::Output,
    ) -> <T::Output as Fold>::Output
    where
        T: Cast + Resolve,
        T::Output: Fold,
    {
        fn next<T>(
            mut values: impl Iterator<Item = T>,
            styles: StyleChain,
            default: &impl Fn() -> <T::Output as Fold>::Output,
        ) -> <T::Output as Fold>::Output
        where
            T: Resolve,
            T::Output: Fold,
        {
            values
                .next()
                .map(|value| value.resolve(styles).fold(next(values, styles, default)))
                .unwrap_or_else(|| default())
        }
        next(self.properties::<T>(func, name, inherent), self, &default)
    }

    /// Iterate over all style recipes in the chain.
    pub fn recipes(self) -> impl Iterator<Item = &'a Recipe> {
        self.entries().filter_map(Style::recipe)
    }

    /// Iterate over all values for the given property in the chain.
    pub fn properties<T: Cast + 'a>(
        self,
        func: ElemFunc,
        name: &'a str,
        inherent: Option<Value>,
    ) -> impl Iterator<Item = T> + '_ {
        inherent
            .into_iter()
            .chain(
                self.entries()
                    .filter_map(Style::property)
                    .filter(move |property| property.is(func, name))
                    .map(|property| property.value.clone()),
            )
            .map(move |value| {
                value.cast().unwrap_or_else(|err| {
                    panic!("{} (for {}.{})", err, func.name(), name)
                })
            })
    }

    /// Convert to a style map.
    pub fn to_map(self) -> Styles {
        let mut suffix = Styles::new();
        for link in self.links() {
            suffix.apply_slice(link);
        }
        suffix
    }

    /// Iterate over the entries of the chain.
    fn entries(self) -> Entries<'a> {
        Entries { inner: [].as_slice().iter(), links: self.links() }
    }

    /// Iterate over the links of the chain.
    fn links(self) -> Links<'a> {
        Links(Some(self))
    }

    /// Build owned styles from the suffix (all links beyond the `len`) of the
    /// chain.
    fn suffix(self, len: usize) -> Styles {
        let mut suffix = Styles::new();
        let take = self.links().count().saturating_sub(len);
        for link in self.links().take(take) {
            suffix.apply_slice(link);
        }
        suffix
    }

    /// Remove the last link from the chain.
    fn pop(&mut self) {
        *self = self.tail.copied().unwrap_or_default();
    }

    /// Whether two style chains contain the same pointers.
    fn ptr_eq(self, other: Self) -> bool {
        std::ptr::eq(self.head, other.head)
            && match (self.tail, other.tail) {
                (Some(a), Some(b)) => std::ptr::eq(a, b),
                (None, None) => true,
                _ => false,
            }
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
        self.ptr_eq(*other) || crate::util::hash128(self) == crate::util::hash128(other)
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
    styles: Vec<(Styles, usize)>,
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

    /// Insert an item in the front. The item will share the style of the
    /// current first item.
    ///
    /// This method has no effect if the vector is empty.
    pub fn push_front(&mut self, item: T) {
        if !self.styles.is_empty() {
            self.items.insert(0, item);
            self.styles[0].1 += 1;
        }
    }

    /// Map the contained items.
    pub fn map<F, U>(&self, f: F) -> StyleVec<U>
    where
        F: FnMut(&T) -> U,
    {
        StyleVec {
            items: self.items.iter().map(f).collect(),
            styles: self.styles.clone(),
        }
    }

    /// Iterates (taking ownership) over the contained items and associated styles.
    pub fn into_iter(self) -> impl Iterator<Item = (T, Styles)> {
        self.items.into_iter().zip(
            self.styles
                .into_iter()
                .flat_map(|(map, count)| iter::repeat(map).take(count)),
        )
    }

    /// Iterate over references to the contained items and associated styles.
    pub fn iter(&self) -> impl Iterator<Item = (&T, &Styles)> + '_ {
        self.items().zip(
            self.styles
                .iter()
                .flat_map(|(map, count)| iter::repeat(map).take(*count)),
        )
    }

    /// Iterate over the contained items.
    pub fn items(&self) -> std::slice::Iter<'_, T> {
        self.items.iter()
    }

    /// Iterate over the contained style lists. Note that zipping this with
    /// `items()` does not yield the same result as calling `iter()` because
    /// this method only returns lists once that are shared by consecutive
    /// items. This method is designed for use cases where you want to check,
    /// for example, whether any of the lists fulfills a specific property.
    pub fn styles(&self) -> impl Iterator<Item = &Styles> {
        self.styles.iter().map(|(map, _)| map)
    }
}

impl StyleVec<Content> {
    pub fn to_vec(self) -> Vec<Content> {
        self.items
            .into_iter()
            .zip(
                self.styles
                    .iter()
                    .flat_map(|(map, count)| iter::repeat(map).take(*count)),
            )
            .map(|(content, styles)| content.styled_with_map(styles.clone()))
            .collect()
    }
}

impl<T> Default for StyleVec<T> {
    fn default() -> Self {
        Self { items: vec![], styles: vec![] }
    }
}

impl<T> FromIterator<T> for StyleVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let items: Vec<_> = iter.into_iter().collect();
        let styles = vec![(Styles::new(), items.len())];
        Self { items, styles }
    }
}

impl<T: Debug> Debug for StyleVec<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.iter().map(|(item, styles)| {
                crate::util::debug(|f| {
                    styles.fmt(f)?;
                    item.fmt(f)
                })
            }))
            .finish()
    }
}

/// Assists in the construction of a [`StyleVec`].
#[derive(Debug)]
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
    pub fn elems(&self) -> std::slice::Iter<'_, T> {
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

        let styles = self
            .chains
            .into_iter()
            .map(|(chain, count)| (chain.suffix(shared), count))
            .collect();

        (StyleVec { items: self.items, styles }, trunk)
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
