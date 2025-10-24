use std::any::{Any, TypeId};
use std::sync::Arc;

use comemo::Tracked;
use ecow::{EcoString, EcoVec, eco_format};
use smallvec::SmallVec;

use crate::diag::{HintedStrResult, StrResult, bail};
use crate::foundations::{
    CastInfo, Content, Context, Dict, Element, FromValue, Func, Label, Reflect, Regex,
    Repr, Str, StyleChain, Symbol, Type, Value, cast, func, repr, scope, ty,
};
use crate::introspection::{Introspector, Locatable, Location, Unqueriable};

/// A helper macro to create a field selector used in [`Selector::Elem`]
#[macro_export]
#[doc(hidden)]
macro_rules! __select_where {
    ($ty:ty $(, $field:ident => $value:expr)* $(,)?) => {{
        #[allow(unused_mut)]
        let mut fields = $crate::foundations::SmallVec::new();
        $(
            fields.push((
                <$ty>::$field.index(),
                $crate::foundations::IntoValue::into_value($value),
            ));
        )*
        $crate::foundations::Selector::Elem(
            <$ty as $crate::foundations::NativeElement>::ELEM,
            Some(fields),
        )
    }};
}

#[doc(inline)]
pub use crate::__select_where as select_where;

/// A filter for selecting elements within the document.
///
/// To construct a selector you can:
/// - use an element [function]
/// - filter for an element function with [specific fields]($function.where)
/// - use a [string]($str) or [regular expression]($regex)
/// - use a [`{<label>}`]($label)
/// - use a [`location`]
/// - call the [`selector`] constructor to convert any of the above types into a
///   selector value and use the methods below to refine it
///
/// Selectors are used to [apply styling rules]($styling/#show-rules) to
/// elements. You can also use selectors to [query] the document for certain
/// types of elements.
///
/// Furthermore, you can pass a selector to several of Typst's built-in
/// functions to configure their behaviour. One such example is the [outline]
/// where it can be used to change which elements are listed within the outline.
///
/// Multiple selectors can be combined using the methods shown below. However,
/// not all kinds of selectors are supported in all places, at the moment.
///
/// # Example
/// ```example
/// #context query(
///   heading.where(level: 1)
///     .or(heading.where(level: 2))
/// )
///
/// = This will be found
/// == So will this
/// === But this will not.
/// ```
#[ty(scope, cast)]
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Selector {
    /// Matches a specific type of element.
    ///
    /// If there is a dictionary, only elements with the fields from the
    /// dictionary match.
    Elem(Element, Option<SmallVec<[(u8, Value); 1]>>),
    /// Matches the element at the specified location.
    Location(Location),
    /// Matches elements with a specific label.
    Label(Label),
    /// Matches text elements through a regular expression.
    Regex(Regex),
    /// Matches elements with a specific capability.
    Can(TypeId),
    /// Matches if any of the subselectors match.
    Or(EcoVec<Self>),
    /// Matches if all of the subselectors match.
    And(EcoVec<Self>),
    /// Matches all matches of `selector` before `end`.
    Before { selector: Arc<Self>, end: Arc<Self>, inclusive: bool },
    /// Matches all matches of `selector` after `start`.
    After { selector: Arc<Self>, start: Arc<Self>, inclusive: bool },
}

impl Selector {
    /// Define a simple text selector.
    pub fn text(text: &str) -> StrResult<Self> {
        if text.is_empty() {
            bail!("text selector is empty");
        }
        Ok(Self::Regex(Regex::new(&regex::escape(text)).unwrap()))
    }

    /// Define a regex selector.
    pub fn regex(regex: Regex) -> StrResult<Self> {
        if regex.as_str().is_empty() {
            bail!("regex selector is empty");
        }
        if regex.is_match("") {
            bail!("regex matches empty text");
        }
        Ok(Self::Regex(regex))
    }

    /// Define a simple [`Selector::Can`] selector.
    pub fn can<T: ?Sized + Any>() -> Self {
        Self::Can(TypeId::of::<T>())
    }

    /// Whether the selector matches for the target.
    pub fn matches(&self, target: &Content, styles: Option<StyleChain>) -> bool {
        match self {
            Self::Elem(element, dict) => {
                target.elem() == *element
                    && dict.iter().flat_map(|dict| dict.iter()).all(|(id, value)| {
                        target.get(*id, styles).as_ref().ok() == Some(value)
                    })
            }
            Self::Label(label) => target.label() == Some(*label),
            Self::Can(cap) => target.func().can_type_id(*cap),
            Self::Or(selectors) => {
                selectors.iter().any(move |sel| sel.matches(target, styles))
            }
            Self::And(selectors) => {
                selectors.iter().all(move |sel| sel.matches(target, styles))
            }
            Self::Location(location) => target.location() == Some(*location),
            // Not supported here.
            Self::Regex(_) | Self::Before { .. } | Self::After { .. } => false,
        }
    }
}

#[scope]
impl Selector {
    /// Turns a value into a selector. The following values are accepted:
    /// - An element function like a `heading` or `figure`.
    /// - A [string]($str) or [regular expression]($regex).
    /// - A `{<label>}`.
    /// - A [`location`].
    /// - A more complex selector like `{heading.where(level: 1)}`.
    #[func(constructor)]
    pub fn construct(
        /// Can be an element function like a `heading` or `figure`, a `{<label>}`
        /// or a more complex selector like `{heading.where(level: 1)}`.
        target: Selector,
    ) -> Selector {
        target
    }

    /// Selects all elements that match this or any of the other selectors.
    #[func]
    pub fn or(
        self,
        /// The other selectors to match on.
        #[variadic]
        others: Vec<Selector>,
    ) -> Selector {
        Self::Or(others.into_iter().chain(Some(self)).collect())
    }

    /// Selects all elements that match this and all of the other selectors.
    #[func]
    pub fn and(
        self,
        /// The other selectors to match on.
        #[variadic]
        others: Vec<Selector>,
    ) -> Selector {
        Self::And(others.into_iter().chain(Some(self)).collect())
    }

    /// Returns a modified selector that will only match elements that occur
    /// before the first match of `end`.
    #[func]
    pub fn before(
        self,
        /// The original selection will end at the first match of `end`.
        end: LocatableSelector,
        /// Whether `end` itself should match or not. This is only relevant if
        /// both selectors match the same type of element. Defaults to `{true}`.
        #[named]
        #[default(true)]
        inclusive: bool,
    ) -> Selector {
        Self::Before {
            selector: Arc::new(self),
            end: Arc::new(end.0),
            inclusive,
        }
    }

    /// Returns a modified selector that will only match elements that occur
    /// after the first match of `start`.
    #[func]
    pub fn after(
        self,
        /// The original selection will start at the first match of `start`.
        start: LocatableSelector,
        ///  Whether `start` itself should match or not. This is only relevant
        ///  if both selectors match the same type of element. Defaults to
        ///  `{true}`.
        #[named]
        #[default(true)]
        inclusive: bool,
    ) -> Selector {
        Self::After {
            selector: Arc::new(self),
            start: Arc::new(start.0),
            inclusive,
        }
    }
}

impl From<Location> for Selector {
    fn from(value: Location) -> Self {
        Self::Location(value)
    }
}

impl Repr for Selector {
    fn repr(&self) -> EcoString {
        match self {
            Self::Elem(elem, dict) => {
                if let Some(dict) = dict {
                    let dict = dict
                        .iter()
                        .map(|(id, value)| {
                            (
                                elem.field_name(*id).unwrap_or("internal"),
                                value.clone(),
                            )
                        })
                        .map(|(name, value)| (EcoString::from(name).into(), value))
                        .collect::<Dict>();
                    eco_format!("{}.where{}", elem.name(), dict.repr())
                } else {
                    elem.name().into()
                }
            }
            Self::Label(label) => label.repr(),
            Self::Regex(regex) => regex.repr(),
            Self::Can(cap) => eco_format!("{cap:?}"),
            Self::Or(selectors) | Self::And(selectors) => {
                let function = if matches!(self, Self::Or(_)) { "or" } else { "and" };
                let pieces: Vec<_> = selectors.iter().map(Selector::repr).collect();
                eco_format!("{}{}", function, repr::pretty_array_like(&pieces, false))
            }
            Self::Location(loc) => loc.repr(),
            Self::Before { selector, end: split, inclusive }
            | Self::After { selector, start: split, inclusive } => {
                let method =
                    if matches!(self, Self::Before { .. }) { "before" } else { "after" };
                let inclusive_arg = if !*inclusive { ", inclusive: false" } else { "" };
                eco_format!(
                    "{}.{}({}{})",
                    selector.repr(),
                    method,
                    split.repr(),
                    inclusive_arg
                )
            }
        }
    }
}

cast! {
    type Selector,
    text: EcoString => Self::text(&text)?,
    func: Func => func
        .element()
        .ok_or("only element functions can be used as selectors")?
        .select(),
    label: Label => Self::Label(label),
    regex: Regex => Self::regex(regex)?,
    location: Location => Self::Location(location),
}

/// A selector that can be used with `query`.
///
/// Hopefully, this is made obsolete by a more powerful query mechanism in the
/// future.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct LocatableSelector(pub Selector);

impl LocatableSelector {
    /// Resolve this selector into a location that is guaranteed to be unique.
    pub fn resolve_unique(
        &self,
        introspector: Tracked<Introspector>,
        context: Tracked<Context>,
    ) -> HintedStrResult<Location> {
        match &self.0 {
            Selector::Location(loc) => Ok(*loc),
            other => {
                context.introspect()?;
                Ok(introspector.query_unique(other).map(|c| c.location().unwrap())?)
            }
        }
    }
}

impl Reflect for LocatableSelector {
    fn input() -> CastInfo {
        CastInfo::Union(vec![
            CastInfo::Type(Type::of::<Label>()),
            CastInfo::Type(Type::of::<Func>()),
            CastInfo::Type(Type::of::<Location>()),
            CastInfo::Type(Type::of::<Selector>()),
        ])
    }

    fn output() -> CastInfo {
        CastInfo::Type(Type::of::<Selector>())
    }

    fn castable(value: &Value) -> bool {
        Label::castable(value)
            || Func::castable(value)
            || Location::castable(value)
            || Selector::castable(value)
    }
}

cast! {
    LocatableSelector,
    self => self.0.into_value(),
}

impl FromValue for LocatableSelector {
    fn from_value(value: Value) -> HintedStrResult<Self> {
        fn validate(selector: &Selector) -> StrResult<()> {
            match selector {
                Selector::Elem(elem, _) => {
                    if !elem.can::<dyn Locatable>() || elem.can::<dyn Unqueriable>() {
                        Err(eco_format!("{} is not locatable", elem.name()))?
                    }
                }
                Selector::Location(_) => {}
                Selector::Label(_) => {}
                Selector::Regex(_) => bail!("text is not locatable"),
                Selector::Can(_) => bail!("capability is not locatable"),
                Selector::Or(list) | Selector::And(list) => {
                    for selector in list {
                        validate(selector)?;
                    }
                }
                Selector::Before { selector, end: split, .. }
                | Selector::After { selector, start: split, .. } => {
                    for selector in [selector, split] {
                        validate(selector)?;
                    }
                }
            }
            Ok(())
        }

        if !Self::castable(&value) {
            return Err(Self::error(&value));
        }

        let selector = Selector::from_value(value)?;
        validate(&selector)?;
        Ok(Self(selector))
    }
}

impl From<Location> for LocatableSelector {
    fn from(loc: Location) -> Self {
        Self(Selector::Location(loc))
    }
}

/// A selector that can be used with show rules.
///
/// Hopefully, this is made obsolete by a more powerful showing mechanism in the
/// future.
#[derive(Clone, PartialEq, Hash)]
pub struct ShowableSelector(pub Selector);

impl Reflect for ShowableSelector {
    fn input() -> CastInfo {
        CastInfo::Union(vec![
            CastInfo::Type(Type::of::<Symbol>()),
            CastInfo::Type(Type::of::<Str>()),
            CastInfo::Type(Type::of::<Label>()),
            CastInfo::Type(Type::of::<Func>()),
            CastInfo::Type(Type::of::<Regex>()),
            CastInfo::Type(Type::of::<Selector>()),
        ])
    }

    fn output() -> CastInfo {
        CastInfo::Type(Type::of::<Selector>())
    }

    fn castable(value: &Value) -> bool {
        Symbol::castable(value)
            || Str::castable(value)
            || Label::castable(value)
            || Func::castable(value)
            || Regex::castable(value)
            || Selector::castable(value)
    }
}

cast! {
    ShowableSelector,
    self => self.0.into_value(),
}

impl FromValue for ShowableSelector {
    fn from_value(value: Value) -> HintedStrResult<Self> {
        fn validate(selector: &Selector, nested: bool) -> HintedStrResult<()> {
            match selector {
                Selector::Elem(_, _) => {}
                Selector::Label(_) => {}
                Selector::Regex(_) if !nested => {}
                Selector::Or(list) | Selector::And(list) => {
                    for selector in list {
                        validate(selector, true)?;
                    }
                }
                Selector::Regex(_)
                | Selector::Location(_)
                | Selector::Can(_)
                | Selector::Before { .. }
                | Selector::After { .. } => {
                    bail!("this selector cannot be used with show")
                }
            }
            Ok(())
        }

        if !Self::castable(&value) {
            return Err(Self::error(&value));
        }

        let selector = Selector::from_value(value)?;
        validate(&selector, false)?;
        Ok(Self(selector))
    }
}
