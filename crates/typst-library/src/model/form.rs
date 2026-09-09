use ecow::{EcoString, eco_format};
use typst_macros::cast;
use typst_syntax::Span;

use crate::{
    diag::{At, Hint, HintedStrResult, SourceResult, StrResult},
    engine::Engine,
    foundations::{
        Array, Content, Dict, IntoValue, Label, NativeElement, OneOrMultiple, Packed,
        Repr, elem, scope,
    },
    introspection::{Introspector, Location, QueryLabelIntrospection},
    text::LocalName,
};

#[elem(scope, since = "0.16.0", LocalName)]
pub struct FormElem {
    /// The fields belonging to this form, along with any surrounding content.
    #[required]
    pub body: Content,
}

#[scope]
impl FormElem {
    #[elem]
    type FormCheckboxField;
    #[elem]
    type FormTextField;
    #[elem]
    type FormDropdownField;
    #[elem]
    type FormListField;
    #[elem]
    type FormLabel;
}

impl LocalName for Packed<FormElem> {
    const KEY: &'static str = "form";
}

#[elem(name = "checkbox", since = "0.16.0", Locatable)]
pub struct FormCheckboxField {
    /// The name of this field.
    #[required]
    pub name: EcoString,

    /// Whether this checkbox should be checked.
    pub checked: bool,

    /// Whether this checkbox is read-only and its value cannot be changed.
    pub read_only: bool,
}

#[elem(name = "text", since = "0.16.0", Locatable)]
pub struct FormTextField {
    /// The name of this field.
    #[required]
    pub name: EcoString,

    /// The value of this field.
    pub value: Option<EcoString>,

    /// Whether the value of this field can span multiple lines.
    pub multiline: bool,

    /// Whether it is mandatory to fill out this field (i.e., it cannot be empty).
    pub required: bool,

    /// The max length, in characters, of the value of this field.
    pub max_length: Option<u32>,

    /// Whether this text field is read-only and its value cannot be changed.
    pub read_only: bool,

    /// Whether the spellchecker should be enabled when filling out this field.
    /// In HTML export, this also controls text auto correction.
    #[default(true)]
    pub allow_spellcheck: bool,
}

// TODO: pdf allows `Edit` flag for free-form text entry
#[elem(name = "dropdown", since = "0.16.0", Locatable)]
pub struct FormDropdownField {
    /// The name of this field.
    #[required]
    pub name: EcoString,

    /// The value of this field.
    /// It must be present in @form.dropdown.options[`options`].
    pub value: Option<EcoString>,

    /// The available options on this dropdown.
    ///
    /// - If given a dictionary, the keys correspond to the value of each option,
    ///   while the values correspond to the user-facing display name
    ///   (or `none` to use the value as the display name).
    /// - If given an array, its elements are treated as both the value of the option and its
    ///   display name.
    ///
    /// #example(
    ///   title: "Give an array of strings",
    ///   ```
    ///   #form.dropdown("...", options: ("red-car", "blue-car"))
    ///   ```
    /// )
    ///
    /// #example(
    ///   title: "Give a dictionary mapping to display name",
    ///   ```
    ///   #form.dropdown("...", options: ("red-car": "Red Car", "blue-car": "Blue Car"))
    ///   ```
    /// )
    pub options: ChoiceOptions,

    /// Whether it is mandatory to fill out this field (i.e., it cannot be empty).
    pub required: bool,

    /// Whether this text field is read-only and its value cannot be changed.
    pub read_only: bool,
}

// TODO: PDF allows non-multiple list, but HTML does not
#[elem(name = "list", since = "0.16.0", Locatable)]
pub struct FormListField {
    /// The name of this field.
    #[required]
    pub name: EcoString,

    /// The value of this field.
    /// All entries must be present in @form.list.options[`options`].
    pub value: OneOrMultiple<EcoString>,

    /// The available options on this dropdown.
    ///
    /// - If given a dictionary, the keys correspond to the value of each option,
    ///   while the values correspond to the user-facing display name
    ///   (or `none` to use the value as the display name).
    /// - If given an array, its elements are treated as both the value of the option and its
    ///   display name.
    ///
    /// #example(
    ///   title: "Give an array of strings",
    ///   ```
    ///   #form.dropdown("...", options: ("red-car", "blue-car"))
    ///   ```
    /// )
    ///
    /// #example(
    ///   title: "Give a dictionary mapping to display name",
    ///   ```
    ///   #form.dropdown("...", options: ("red-car": "Red Car", "blue-car": "Blue Car"))
    ///   ```
    /// )
    pub options: ChoiceOptions,

    /// Whether it is mandatory to fill out this field (i.e., it cannot be empty).
    pub required: bool,

    /// Whether this text field is read-only and its value cannot be changed.
    pub read_only: bool,
}

/// Available options in a dropdown or list form field.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct ChoiceOptions(pub Vec<(EcoString, Option<EcoString>)>);

// TODO: how do we prevent duplicate keys when using arrays?
cast! {
    ChoiceOptions,
    self => self.0
        .into_iter()
        .map(|(value, display_name)| {
            (value.into(), display_name.into_value())
        })
        .collect::<Dict>()
        .into_value(),
    values: Array => Self(values
        .into_iter()
        .enumerate()
        .map(|(i, v)| Ok((
            v.clone().cast::<EcoString>().hint(str_hint_helper(i, &v))?,
            None,
        )))
        .collect::<HintedStrResult<_>>()?),
    values: Dict => Self(values
        .into_iter()
        .enumerate()
        .map(|(i, (k, v))| Ok((
            k.clone().into_value().cast::<EcoString>().hint(str_hint_helper(i, &k))?,
            v.cast::<Option<EcoString>>().hint(str_hint_helper(i, &k))?,
        )))
        .collect::<HintedStrResult<_>>()?),
}

fn str_hint_helper(index: usize, key: &impl Repr) -> EcoString {
    eco_format!("occurred in option at index {index} (`{}`)", key.repr())
}

#[elem(name = "label", since = "0.16.0", Locatable)]
pub struct FormLabel {
    /// The form field that this label describes.
    #[required]
    pub target: Label,

    /// The description of the linked form field.
    #[required]
    pub body: Content,
}

impl FormLabel {
    /// Resolves the destination.
    pub fn resolve_early(
        &self,
        engine: &mut Engine,
        span: Span,
    ) -> SourceResult<Location> {
        let elem = engine
            .introspect(QueryLabelIntrospection(self.target, span))
            .at(span)?;
        Ok(elem.location().unwrap())
    }

    /// Resolves the destination without an engine.
    pub fn resolve_late(&self, introspector: &dyn Introspector) -> StrResult<Location> {
        let elem = introspector.query_label(self.target)?;
        Ok(elem.location().unwrap())
    }

    /// Finds all linked-to inputs referenced in an introspector.
    pub fn find_destinations(
        introspector: &dyn Introspector,
    ) -> impl Iterator<Item = Location> {
        introspector
            .query(&Self::ELEM.select())
            .into_iter()
            .map(|elem| elem.into_packed::<Self>().unwrap())
            .filter_map(|elem| elem.resolve_late(introspector).ok())
    }
}
