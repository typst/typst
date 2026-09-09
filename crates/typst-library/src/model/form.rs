use ecow::{EcoString, EcoVec, eco_format};
use rustc_hash::FxHashSet;
use typst_macros::{Cast, cast};
use typst_syntax::Span;

use crate::diag::{At, Hint, HintedStrResult, SourceResult, StrResult, bail};
use crate::engine::Engine;
use crate::foundations::{
    Array, Content, Dict, FromValue, IntoValue, Label, NativeElement, OneOrMultiple,
    Packed, Repr, Smart, Value, elem, scope,
};
use crate::introspection::{Introspector, Location, QueryLabelIntrospection};
use crate::layout::{Length, Rel, Sizing};
use crate::text::LocalName;

#[elem(scope, since = "unreleased", LocalName, Locatable)]
pub struct FormElem {
    /// The fields belonging to this form, along with any surrounding content.
    #[required]
    pub body: Content,

    /// Where to submit this form when a @form.button with a @form.button.action[submit action]
    /// is pressed.
    pub target: Option<FormTarget>,
}

#[scope]
impl FormElem {
    #[elem]
    type FormButton;
    #[elem]
    type FormCheckbox;
    #[elem]
    type FormRadioGroup;
    #[elem]
    type FormRadioButton;
    #[elem]
    type FormTextbox;
    #[elem]
    type FormChoice;
    #[elem]
    type FormLabel;
}

/// Where and how to submit a filled out form.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct FormTarget {
    /// The URL to send form data to.
    pub url: EcoString, // TODO: check and strong type URL?
    /// The format to submit the form data in.
    pub content_type: Smart<ContentType>,
    /// The HTTP method to use when submitting the form.
    pub method: HttpMethod,
}

cast! {
    FormTarget,
    self => Value::Dict(self.into()),
    url: EcoString => Self {
        url,
        content_type: Smart::Auto,
        method: HttpMethod::default(),
    },
    mut dict: Dict => {
        // Get a value by key, accepting either non-existence or something
        // convertible to type T.
        fn take<T: FromValue>(dict: &mut Dict, key: &str) -> HintedStrResult<Option<T>> {
            dict.take(key).ok().map(|v| v.cast()).transpose()
        }

        let url = take(&mut dict, "url")?
            .ok_or("missing the required `url` entry in the form target")?;
        let content_type = take(&mut dict, "content-type")?.unwrap_or_default();
        let method = take(&mut dict, "method")?.unwrap_or_default();
        dict.finish(&["url", "content-type", "method"])?;
        Self { url,  content_type, method }
    },
}

impl From<FormTarget> for Dict {
    fn from(target: FormTarget) -> Self {
        let mut dict = Dict::new();
        dict.insert("url".into(), target.url.into_value());
        dict.insert("content-type".into(), target.content_type.into_value());
        dict.insert("method".into(), target.method.into_value());
        dict
    }
}

/// The submission format.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum ContentType {
    /// URL encoded key-value pairs (`application/x-www-form-url-encoded`).
    UrlEncoded,
    /// Multipart form data (`multipart/form-data`).
    Multipart,
    /// Plain text (`text/plain`).
    Plain,
    /// The PDF file itself (`application/pdf`).
    Pdf,
    /// Forms Data Format (`application/fdf`).
    Fdf,
    /// XML Forms Data Format (`application/xfdf`).
    Xfdf,
}

/// The HTTP method.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum HttpMethod {
    /// GET.
    Get,
    #[default]
    /// POST.
    Post,
}

impl LocalName for Packed<FormElem> {
    const KEY: &'static str = "form";
}

#[elem(name = "button", since = "unreleased", Locatable, Tagged)]
pub struct FormButton {
    /// The label of this button.
    #[required]
    pub body: Content,

    /// The name of this field.
    /// If `auto`, will be automatically generated as `typst.field-<n>` when exporting to PDF.
    pub name: Smart<EcoString>,

    /// The action that is triggered when this button is clicked, if any.
    pub action: Option<ButtonAction>,

    /// The button's width.
    pub width: Sizing,

    /// The button's height.
    pub height: Smart<Rel<Length>>,
}

/// Supported actions when clicking a button.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum ButtonAction {
    /// Submit the filled out form to the form's @form.target[submission target].
    Submit,
    /// Reset all fields in this form to their default values.
    // TODO: in html, the `value` of each field is used as default, should we do the same in PDF
    // instead of just blanking everything?
    Reset,
}

#[elem(name = "checkbox", since = "unreleased", Locatable, Tagged)]
pub struct FormCheckbox {
    /// The name of this field.
    /// If `auto`, will be automatically generated as `typst.field-<n>` when exporting to PDF.
    pub name: Smart<EcoString>,

    /// Whether this checkbox should be checked.
    pub checked: bool,

    /// Whether this checkbox is required and must be checked.
    pub required: bool,

    /// Whether this checkbox is read-only and its value cannot be changed.
    pub disabled: bool,

    /// The checkbox's width.
    pub width: Sizing,

    /// The checkbox's height.
    pub height: Smart<Rel<Length>>,
}

#[elem(name = "radio-group", since = "unreleased", Locatable)]
pub struct FormRadioGroup {
    /// The radio buttons belonging to this group, along with any surrounding content.
    #[required]
    pub body: Content,

    /// The name of this field.
    /// If `auto`, will be automatically generated as `typst.field-<n>` when exporting to PDF.
    pub name: Smart<EcoString>,

    /// The label pointing to the radio button that is selected.
    pub selected: Option<Label>,

    /// Whether this radio group is required and one of the options must be selected.
    pub required: bool,

    /// Whether this radio-group is read-only and its value cannot be changed.
    pub disabled: bool,
}

#[elem(name = "radio", since = "unreleased", Locatable, Tagged)]
pub struct FormRadioButton {
    /// The value of the radio group when this option is selected.
    /// If `auto`, it is automatically generated on PDF export as `option-<n>`.
    pub value: Smart<EcoString>,

    /// The radio button's width.
    pub width: Sizing,

    /// The radio button's height.
    pub height: Smart<Rel<Length>>,
}

#[elem(name = "textbox", since = "unreleased", Locatable, Tagged)]
pub struct FormTextbox {
    /// The name of this field.
    /// If `auto`, will be automatically generated as `typst.field-<n>` when exporting to PDF.
    pub name: Smart<EcoString>,

    /// The value of this field.
    pub value: Option<EcoString>,

    /// Whether the value of this field can span multiple lines.
    pub multiline: bool,

    /// Whether it is mandatory to fill out this field (i.e., it cannot be empty).
    pub required: bool,

    /// The max length, in characters, of the value of this field.
    pub max_length: Option<u32>,

    /// Whether this textbox is divided into as many equally spaced positions as the value of `max-length`.
    /// The `max-length` property must be set.
    /// Only supported in PDF export.
    pub fixed_width: bool,

    /// The placeholder text to show when the textbox is empty. Only supported in HTML export.
    pub placeholder: Option<EcoString>,

    /// Whether this text field is read-only and its value cannot be changed.
    pub disabled: bool,

    /// Whether the spellchecker should be enabled when filling out this field.
    /// When set to `auto`, it is enabled in PDF export, and respects the browser defaults in HTML.
    /// In HTML export, this also controls text auto correction.
    pub spellcheck: Smart<bool>,

    /// The field's width.
    pub width: Sizing,

    /// The field's height.
    pub height: Smart<Rel<Length>>,
}

#[elem(name = "choice", since = "unreleased", Locatable, Tagged)]
pub struct FormChoice {
    /// The name of this field.
    /// If `auto`, will be automatically generated as `typst.field-<n>` when exporting to PDF.
    pub name: Smart<EcoString>,

    /// Whether this field allows multiple options to be selected at the same time.
    /// - When `false`, this field is displayed as a dropdown.
    /// - When `true`, this field is displayed as a listbox.
    ///
    /// #example(
    ///   title: "When multiple is true, a listbox is displayed",
    ///   ```
    ///   #form(
    ///     form.choice(options: ("red-car", "blue-car"), multiple: true)
    ///   )
    ///   ```
    /// )
    pub multiple: bool,

    /// The available options on this field.
    ///
    /// - If given a dictionary, the keys correspond to the value of each option,
    ///   while the values correspond to the user-facing display name
    ///   (or `auto` to use the value as the display name).
    /// - If given an array, its elements are treated as both the value of the option and its
    ///   display name.
    ///
    /// #example(
    ///   title: "Give an array of strings",
    ///   ```
    ///   #form(
    ///     form.choice(options: ("red-car", "blue-car"))
    ///   )
    ///   ```
    /// )
    ///
    /// #example(
    ///   title: "Give a dictionary mapping to display name",
    ///   ```
    ///   #form(
    ///     form.choice(options: ("red-car": "Red Car", "blue-car": "Blue Car"))
    ///   )
    ///   ```
    /// )
    pub options: ChoiceOptions,

    /// The value of this field.
    /// Can be a list if @form.choice.multiple[`multiple`] is true.
    /// All entries must be present in @form.choice.options[`options`].
    pub value: OneOrMultiple<EcoString>,

    /// Whether it is mandatory to fill out this field (i.e., it cannot be empty).
    pub required: bool,

    /// Whether this text field is read-only and its value cannot be changed.
    pub disabled: bool,

    /// The field's width.
    pub width: Sizing,

    /// The field's height.
    pub height: Smart<Rel<Length>>,
}

impl FormChoice {
    pub fn validate(
        span: Span,
        multiple: bool,
        options: &ChoiceOptions,
        value: &OneOrMultiple<EcoString>,
    ) -> SourceResult<()> {
        let mut seen_options = FxHashSet::default();
        for (i, option) in options.0.iter().enumerate() {
            if !seen_options.insert(option.mapping_name.clone()) {
                bail!(
                    span, "duplicate option";
                    hint: "occurred in option at index {i} (`{}`)", option.mapping_name.repr();
                )
            }
        }

        if !multiple && value.0.len() > 1 {
            bail!(
                span, "field cannot have more than one value when `multiple: false`";
                hint: "set `multiple: true` or set only a single value for this field";
            )
        }
        for v in &value.0 {
            if !seen_options.contains(v) {
                bail!(
                    span, "unknown value `{:?}` in field", v.repr();
                    hint: "value is not one of the options of this field";
                    hint: "try adding it to the `options` property or setting a different value instead";
                )
            }
        }
        Ok(())
    }
}

/// Available options in a choice form field.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct ChoiceOptions(pub EcoVec<ChoiceOption>);

cast! {
    ChoiceOptions,
    self => self.0
        .into_iter()
        .map(|option| {
            (option.mapping_name.into(), option.display_name.into_value())
        })
        .collect::<Dict>()
        .into_value(),
    values: Array => Self(values
        .into_iter()
        .enumerate()
        .map(|(i, v)| Ok(ChoiceOption::new(
            v.clone().cast::<EcoString>().hint(opt_hint_helper(i, &v))?,
            Smart::Auto,
        )))
        .collect::<HintedStrResult<_>>()?),
    values: Dict => Self(values
        .into_iter()
        .enumerate()
        .map(|(i, (k, v))| Ok(ChoiceOption::new(
            k.clone().into_value().cast::<EcoString>().hint(opt_hint_helper(i, &k))?,
            v.cast::<Smart<EcoString>>().hint(opt_hint_helper(i, &k))?,
        )))
        .collect::<HintedStrResult<_>>()?),
}

fn opt_hint_helper(index: usize, key: &impl Repr) -> EcoString {
    eco_format!("occurred in option at index {index} (`{}`)", key.repr())
}

/// An option in a choice form field.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct ChoiceOption {
    /// The name that uniquely identifies this option.
    pub mapping_name: EcoString,
    /// The user-facing name of this option. If [`Smart::Auto`], [`Self::mapping_name`] is used instead.
    pub display_name: Smart<EcoString>,
}

impl ChoiceOption {
    pub fn new(mapping_name: EcoString, display_name: Smart<EcoString>) -> Self {
        Self { mapping_name, display_name }
    }

    /// If present, get the display name, otherwise get the mapping name.
    pub fn display_or_mapping_name(&self) -> EcoString {
        self.display_name.as_ref().unwrap_or(&self.mapping_name).clone()
    }
}

#[elem(name = "label", since = "unreleased", Locatable)]
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
