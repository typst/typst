use std::any::{Any, TypeId};
use std::fmt::{self, Debug, Formatter, Write};
use std::sync::Arc;

use ecow::{eco_format, EcoString, EcoVec};

use super::{Content, ElemFunc, Label, Location};
use crate::diag::{bail, StrResult};
use crate::eval::{
    cast, CastInfo, Dict, FromValue, Func, IntoValue, Reflect, Regex, Value,
};
use crate::model::Locatable;
use crate::util::pretty_array_like;

/// A selector in a show rule.
#[derive(Clone, PartialEq, Hash)]
pub enum Selector {
    /// Matches a specific type of element.
    ///
    /// If there is a dictionary, only elements with the fields from the
    /// dictionary match.
    Elem(ElemFunc, Option<Dict>),
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
    pub fn text(text: &str) -> Self {
        Self::Regex(Regex::new(&regex::escape(text)).unwrap())
    }

    /// Define a simple [`Selector::Can`] selector.
    pub fn can<T: ?Sized + Any>() -> Self {
        Self::Can(TypeId::of::<T>())
    }

    /// Transforms this selector and an iterator of other selectors into a
    /// [`Selector::Or`] selector.
    pub fn and(self, others: impl IntoIterator<Item = Self>) -> Self {
        Self::And(others.into_iter().chain(Some(self)).collect())
    }

    /// Transforms this selector and an iterator of other selectors into a
    /// [`Selector::And`] selector.
    pub fn or(self, others: impl IntoIterator<Item = Self>) -> Self {
        Self::Or(others.into_iter().chain(Some(self)).collect())
    }

    /// Transforms this selector into a [`Selector::Before`] selector.
    pub fn before(self, location: impl Into<Self>, inclusive: bool) -> Self {
        Self::Before {
            selector: Arc::new(self),
            end: Arc::new(location.into()),
            inclusive,
        }
    }

    /// Transforms this selector into a [`Selector::After`] selector.
    pub fn after(self, location: impl Into<Self>, inclusive: bool) -> Self {
        Self::After {
            selector: Arc::new(self),
            start: Arc::new(location.into()),
            inclusive,
        }
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
            Self::Or(selectors) => selectors.iter().any(move |sel| sel.matches(target)),
            Self::And(selectors) => selectors.iter().all(move |sel| sel.matches(target)),
            Self::Location(location) => target.location() == Some(*location),
            // Not supported here.
            Self::Before { .. } | Self::After { .. } => false,
        }
    }
}

impl From<Location> for Selector {
    fn from(value: Location) -> Self {
        Self::Location(value)
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
            Self::Or(selectors) | Self::And(selectors) => {
                f.write_str(if matches!(self, Self::Or(_)) { "or" } else { "and" })?;
                let pieces: Vec<_> =
                    selectors.iter().map(|sel| eco_format!("{sel:?}")).collect();
                f.write_str(&pretty_array_like(&pieces, false))
            }
            Self::Location(loc) => loc.fmt(f),
            Self::Before { selector, end: split, inclusive }
            | Self::After { selector, start: split, inclusive } => {
                selector.fmt(f)?;

                if matches!(self, Self::Before { .. }) {
                    f.write_str(".before(")?;
                } else {
                    f.write_str(".after(")?;
                }

                split.fmt(f)?;
                if !*inclusive {
                    f.write_str(", inclusive: false")?;
                }
                f.write_char(')')
            }
        }
    }
}

cast! {
    type Selector: "selector",
    func: Func => func
        .element()
        .ok_or("only element functions can be used as selectors")?
        .select(),
    label: Label => Self::Label(label),
    text: EcoString => Self::text(&text),
    regex: Regex => Self::Regex(regex),
    location: Location => Self::Location(location),
}

/// A selector that can be used with `query`.
///
/// Hopefully, this is made obsolete by a more powerful query mechanism in the
/// future.
#[derive(Clone, PartialEq, Hash)]
pub struct LocatableSelector(pub Selector);

impl Reflect for LocatableSelector {
    fn describe() -> CastInfo {
        CastInfo::Union(vec![
            CastInfo::Type("function"),
            CastInfo::Type("label"),
            CastInfo::Type("selector"),
        ])
    }

    fn castable(value: &Value) -> bool {
        matches!(value.type_name(), "function" | "label" | "selector")
    }
}

impl IntoValue for LocatableSelector {
    fn into_value(self) -> Value {
        self.0.into_value()
    }
}

impl FromValue for LocatableSelector {
    fn from_value(value: Value) -> StrResult<Self> {
        fn validate(selector: &Selector) -> StrResult<()> {
            match selector {
                Selector::Elem(elem, _) => {
                    if !elem.can::<dyn Locatable>() {
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

/// A selector that can be used with show rules.
///
/// Hopefully, this is made obsolete by a more powerful showing mechanism in the
/// future.
#[derive(Clone, PartialEq, Hash)]
pub struct ShowableSelector(pub Selector);

impl Reflect for ShowableSelector {
    fn describe() -> CastInfo {
        CastInfo::Union(vec![
            CastInfo::Type("function"),
            CastInfo::Type("label"),
            CastInfo::Type("string"),
            CastInfo::Type("regular expression"),
            CastInfo::Type("symbol"),
            CastInfo::Type("selector"),
        ])
    }

    fn castable(value: &Value) -> bool {
        matches!(
            value.type_name(),
            "symbol"
                | "string"
                | "label"
                | "function"
                | "regular expression"
                | "selector"
        )
    }
}

impl IntoValue for ShowableSelector {
    fn into_value(self) -> Value {
        self.0.into_value()
    }
}

impl FromValue for ShowableSelector {
    fn from_value(value: Value) -> StrResult<Self> {
        fn validate(selector: &Selector) -> StrResult<()> {
            match selector {
                Selector::Elem(_, _) => {}
                Selector::Label(_) => {}
                Selector::Regex(_) => {}
                Selector::Or(_)
                | Selector::And(_)
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
        validate(&selector)?;
        Ok(Self(selector))
    }
}
