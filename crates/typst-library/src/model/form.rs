use std::sync::Arc;

use comemo::Track;
use ecow::{EcoString, EcoVec, eco_format, eco_vec};
use rustc_hash::FxHashMap;
use typst_macros::{Cast, cast};
use typst_syntax::Span;
use typst_utils::singleton;

use crate::diag::{
    At, CollectCombinedResult, Hint, HintedStrResult, SourceResult, StrResult, bail,
    error,
};
use crate::engine::Engine;
use crate::foundations::{
    Args, Array, Construct, Content, Context, Dict, FromValue, IntoValue, Label,
    NativeElement, OneOrMultiple, Packed, Repr, Selector, ShowSet, Smart, StyleChain,
    Styles, Synthesize, Value, dict, elem, scope,
};
use crate::introspection::{
    Counter, CounterKey, Introspector, Location, QueryIntrospection,
    QueryLabelIntrospection,
};
use crate::layout::{Frame, Length, Rel, Size, Sizing};

/// TODO: docs
#[elem(scope, since = "unreleased", Locatable, ShowSet)]
pub struct FormElem {
    /// The fields belonging to this form, along with any surrounding content.
    #[required]
    pub body: Content,

    /// Where to submit this form when a @form.button with a
    /// @form.button.action[submit action] is pressed.
    pub target: Option<FormTarget>,

    /// This form, to be inherited by field elements.
    #[internal]
    #[ghost]
    pub form: Option<Packed<FormElem>>,
}

impl ShowSet for Packed<FormElem> {
    fn show_set(&self, styles: StyleChain) -> Styles {
        let mut out = Styles::new();
        // Makes the form itself available to its field. Should be
        // superseded by a proper ancestry mechanism in the future.
        let mut elem = self.clone();
        elem.materialize(styles);
        out.set(FormElem::form, Some(elem));
        out
    }
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
    #[elem]
    type FormWidget;
    #[elem]
    type FormCursor;
}

impl FormElem {
    /// Get the ancestor form element that is present in the style chain, if any.
    /// Returns an error if not.
    pub fn try_from_styles<'a, T: NativeElement>(
        elem: &Packed<T>,
        styles: &'a StyleChain,
    ) -> SourceResult<&'a Packed<FormElem>> {
        styles.get_ref(FormElem::form).as_ref().ok_or_else(|| {
            eco_vec![error!(
                elem.span(),
                "this {} element must be placed inside a form element", T::ELEM.name();
                hint: "try surrounding this element with `#form[...]`";
            )]
        })
    }
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
        dict! {
            "url" => target.url,
            "content-type" => target.content_type,
            "method" => target.method,
        }
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

/// TODO: docs
#[elem(name = "button", since = "unreleased", Locatable, Tagged, ShowSet)]
pub struct FormButton {
    /// The label of this button.
    #[required]
    pub body: Content,

    /// The name of this field.
    ///
    /// If `{auto}`, will be automatically generated as `field-<n>`
    /// when exporting to PDF.
    pub name: Smart<EcoString>,

    /// The action that is triggered when this button is clicked, if any.
    pub action: Option<ButtonAction>,

    /// The button's width.
    pub width: Smart<Rel<Length>>,

    /// The button's height.
    pub height: Sizing,
}

impl ShowSet for Packed<FormButton> {
    fn show_set(&self, styles: StyleChain) -> Styles {
        let mut out = Styles::new();
        // Makes the field itself available to the `FieldWidget`. Should be
        // superseded by a proper ancestry mechanism in the future.
        let mut elem = self.clone().pack();
        elem.materialize(styles);
        out.set(FormWidget::parent, Some(elem));
        out
    }
}

/// Supported actions when clicking a button.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum ButtonAction {
    /// Submit the filled out form to the form's @form.target[submission target].
    Submit,
    /// Reset all fields in this form to their default values.
    Reset,
}

/// TODO: docs
#[elem(name = "checkbox", since = "unreleased", Locatable, Tagged, ShowSet)]
pub struct FormCheckbox {
    /// The name of this field.
    ///
    /// If `{auto}`, will be automatically generated as `field-<n>`
    /// when exporting to PDF.
    pub name: Smart<EcoString>,

    /// Whether this checkbox should be checked.
    pub checked: bool,

    /// Whether this checkbox is required and must be checked.
    pub required: bool,

    /// Whether this checkbox is read-only and its value cannot be changed.
    pub disabled: bool,

    /// The checkbox's width.
    pub width: Smart<Rel<Length>>,

    /// The checkbox's height.
    pub height: Sizing,
}

impl ShowSet for Packed<FormCheckbox> {
    fn show_set(&self, _: StyleChain) -> Styles {
        let mut out = Styles::new();
        // Makes the field itself available to the `FieldWidget`. Should be
        // superseded by a proper ancestry mechanism in the future.
        // No need to materialize since the widget show-rule does not access any property.
        out.set(FormWidget::parent, Some(self.clone().pack()));
        out
    }
}

/// TODO: docs
#[elem(name = "radio-group", since = "unreleased", Locatable, ShowSet)]
pub struct FormRadioGroup {
    /// The radio buttons belonging to this group, along with any surrounding content.
    #[required]
    pub body: Content,

    /// The name of this field.
    ///
    /// If `{auto}`, will be automatically generated as `field-<n>`
    /// when exporting to PDF.
    pub name: Smart<EcoString>,

    /// The label pointing to the radio button that is selected.
    pub selected: Option<Label>,

    /// Whether this radio group is required and one of the options must be selected.
    pub required: bool,

    /// The title of this radio group.
    ///
    /// In PDF, this is also used as the alt text for this field.
    /// In HTML, this becomes the `<legend>` element of the `<fieldset>`.
    pub title: Option<Content>,

    /// Whether this radio-group is read-only and its value cannot be changed.
    pub disabled: bool,
}

impl Packed<FormRadioGroup> {
    /// Resolves the selected radio button early.
    pub fn resolve_selected_early(
        &self,
        engine: &mut Engine,
        styles: StyleChain<'_>,
    ) -> SourceResult<Option<Packed<FormRadioButton>>> {
        let Some(selected) = self.selected.get_cloned(styles) else { return Ok(None) };
        let location = self.location().unwrap();
        let span = self.span();
        let matches = engine.introspect(QueryIntrospection(
            Selector::Label(selected).within(location.into()),
            span,
        ));
        Self::try_into_radio_button(selected, &matches, span).map(Some)
    }

    /// Resolves the selected radio button without an engine.
    pub fn resolve_selected_late(
        &self,
        introspector: &dyn Introspector,
    ) -> SourceResult<Option<Packed<FormRadioButton>>> {
        let Some(selected) = self.selected.get_cloned(StyleChain::default()) else {
            return Ok(None);
        };
        let location = self.location().unwrap();
        let span = self.span();
        let matches =
            introspector.query(&Selector::Label(selected).within(location.into()));
        Self::try_into_radio_button(selected, &matches, span).map(Some)
    }

    /// Attempts to convert the element returned by the query to a radio button,
    /// if there is only one.
    ///
    /// Returns an error instead if there is none or multiple,
    /// or if element is not a radio button.
    fn try_into_radio_button(
        selected: Label,
        matches: &[Content],
        span: Span,
    ) -> SourceResult<Packed<FormRadioButton>> {
        match matches {
            [content] => {
                if let Some(radio_button) = content.to_packed::<FormRadioButton>() {
                    Ok(radio_button.clone())
                } else {
                    bail!(
                        span, "label `{}` is not a radio button", selected.repr();
                        hint[content.span()]: "label refers to this {}", content.elem().name()
                    )
                }
            }
            [] => bail!(
                span,
                "label `{}` does not exist in the radio group",
                selected.repr()
            ),
            _ => bail!(
                span,
                "label `{}` occurs multiple times in the radio group",
                selected.repr()
            ),
        }
    }
}

impl ShowSet for Packed<FormRadioGroup> {
    fn show_set(&self, styles: StyleChain) -> Styles {
        let mut out = Styles::new();
        // Makes the radio group itself available to its radio buttons. Should be
        // superseded by a proper ancestry mechanism in the future.
        let mut elem = self.clone();
        elem.materialize(styles);
        out.set(FormRadioButton::parent, Some(elem));
        out
    }
}

impl FormRadioGroup {
    /// Get the ancestor radio group element that is present in the style chain, if any.
    ///
    /// Returns an error if not.
    pub fn try_from_styles<'a, T: NativeElement>(
        elem: &Packed<T>,
        styles: &'a StyleChain,
    ) -> SourceResult<&'a Packed<FormRadioGroup>> {
        styles.get_ref(FormRadioButton::parent).as_ref().ok_or_else(|| {
            eco_vec![error!(
                elem.span(),
                "this radio button must be placed inside a radio group";
                hint: "try surrounding this button with `#form.radio-group[...]`";
            )]
        })
    }
}

/// TODO: docs
#[elem(name = "radio", since = "unreleased", Locatable, Tagged, Synthesize, ShowSet)]
pub struct FormRadioButton {
    /// The value of the radio group when this option is selected.
    ///
    /// If `{auto}`, it is automatically generated on PDF export as `option-<n>`.
    pub value: Smart<EcoString>,

    /// The radio button's width.
    pub width: Smart<Rel<Length>>,

    /// The radio button's height.
    pub height: Sizing,

    /// Lets radio buttons access the radio group they are part of.
    /// This is a bit of a hack and should be superseded by a proper ancestry mechanism.
    #[ghost]
    #[internal]
    pub parent: Option<Packed<FormRadioGroup>>,

    /// Same as @form.radio.value[`value`], but with `{auto}` converted to an
    /// auto-generated value.
    #[internal]
    #[synthesized]
    pub resolved_value: EcoString,
}

impl Synthesize for Packed<FormRadioButton> {
    fn synthesize(
        &mut self,
        engine: &mut Engine,
        styles: StyleChain,
    ) -> SourceResult<()> {
        if let Smart::Custom(value) = self.value.get_ref(styles) {
            self.resolved_value = Some(value.clone());
            return Ok(());
        }

        if self.location().is_none() {
            return Ok(());
        }

        let span = self.span();

        let group = FormRadioGroup::try_from_styles(self, &styles)?;

        let selector = FormRadioButton::ELEM
            .select()
            .within(group.location().unwrap().into());
        let counter = Counter::new(CounterKey::Selector(selector));
        let context = Context::new(self.location(), Some(styles));
        let counter_state = counter.get(engine, context.track(), span)?;
        self.resolved_value = Some(eco_format!("option-{}", counter_state.first()));
        Ok(())
    }
}

impl ShowSet for Packed<FormRadioButton> {
    fn show_set(&self, _: StyleChain) -> Styles {
        let mut out = Styles::new();
        // Makes the field itself available to the `FieldWidget`. Should be
        // superseded by a proper ancestry mechanism in the future.
        // No need to materialize since the widget show-rule does not access any property.
        out.set(FormWidget::parent, Some(self.clone().pack()));
        out
    }
}

/// TODO: docs
#[elem(name = "textbox", since = "unreleased", Locatable, Tagged, ShowSet)]
pub struct FormTextbox {
    /// The name of this field.
    ///
    /// If `{auto}`, will be automatically generated as `field-<n>`
    /// when exporting to PDF.
    pub name: Smart<EcoString>,

    /// The value of this field.
    pub value: Option<EcoString>,

    /// Whether the value of this field can span multiple lines.
    pub multiline: bool,

    /// Whether it is mandatory to fill out this field (i.e., it cannot be empty).
    pub required: bool,

    /// The max length, in characters, of the value of this field.
    pub max_length: Option<u32>,

    /// Whether this textbox is divided into as many equally spaced positions
    /// as the value of `max-length`.
    ///
    /// The `max-length` property must be set.
    /// Only supported in PDF export.
    pub fixed_width: bool,

    /// The placeholder text to show when the textbox is empty.
    ///
    /// Only supported in HTML export.
    pub placeholder: Option<EcoString>,

    /// Whether this text field is read-only and its value cannot be changed.
    pub disabled: bool,

    /// Whether the spellchecker should be enabled when filling out this field.
    ///
    /// When set to `{auto}`, it is enabled in PDF export, and respects the browser
    /// defaults in HTML. In HTML export, this also controls text auto correction.
    pub spellcheck: Smart<bool>,

    /// The field's width.
    pub width: Smart<Rel<Length>>,

    /// The field's height.
    pub height: Sizing,
}

impl ShowSet for Packed<FormTextbox> {
    fn show_set(&self, styles: StyleChain) -> Styles {
        let mut out = Styles::new();
        // Makes the field itself available to the `FieldWidget`. Should be
        // superseded by a proper ancestry mechanism in the future.
        let mut elem = self.clone().pack();
        elem.materialize(styles);
        out.set(FormWidget::parent, Some(elem));
        out
    }
}

/// TODO: docs
#[elem(name = "choice", since = "unreleased", Locatable, Tagged, Synthesize, ShowSet)]
pub struct FormChoice {
    /// The name of this field.
    ///
    /// If `{auto}`, will be automatically generated as `field-<n>`
    /// when exporting to PDF.
    pub name: Smart<EcoString>,

    /// Whether this field allows multiple options to be selected at the same time.
    ///
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
    ///   (or `{auto}` to use the value as the display name).
    /// - If given an array, its elements are treated as both the value of the
    ///   option and its display name.
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
    ///
    /// Can be a list if @form.choice.multiple[`multiple`] is true.
    /// All entries must be present in @form.choice.options[`options`].
    pub value: OneOrMultiple<EcoString>,

    /// Whether it is mandatory to fill out this field (i.e., it cannot be empty).
    pub required: bool,

    /// Whether this text field is read-only and its value cannot be changed.
    pub disabled: bool,

    /// The field's width.
    pub width: Smart<Rel<Length>>,

    /// The field's height.
    pub height: Sizing,

    /// The selected @form.choice.value[`values`] of this field,
    /// but represented as indexes of @form.choice.options[`options`].
    #[internal]
    #[synthesized]
    pub value_indexes: EcoVec<usize>,
}

impl ShowSet for Packed<FormChoice> {
    fn show_set(&self, styles: StyleChain) -> Styles {
        let mut out = Styles::new();
        // Makes the field itself available to the `FieldWidget`. Should be
        // superseded by a proper ancestry mechanism in the future.
        let mut elem = self.clone().pack();
        elem.materialize(styles);
        out.set(FormWidget::parent, Some(elem));
        out
    }
}

impl Synthesize for Packed<FormChoice> {
    fn synthesize(&mut self, _: &mut Engine, styles: StyleChain) -> SourceResult<()> {
        let span = self.span();

        let mut seen_options = FxHashMap::default();
        for (i, option) in self.options.get_ref(styles).0.iter().enumerate() {
            if seen_options.insert(option.mapping_name.as_str(), i).is_some() {
                bail!(
                    span, "duplicate option";
                    hint: "occurred in option at index {i} (`{}`)", option.mapping_name.repr();
                )
            }
        }

        let value = &self.value.get_ref(styles).0;

        if !self.multiple.get(styles) && value.len() > 1 {
            bail!(
                span, "field cannot have more than one value when `multiple` is `false`";
                hint: "set `multiple` to `true` or set only a single value for this field";
            )
        }
        self.value_indexes = Some(value.iter().map(|v| {
            seen_options.get(v.as_str()).copied().ok_or_else(||{
                eco_vec![error!(
                    span, "unknown value `{}`", v.repr();
                    hint: "value is not one of the options of this field";
                    hint: "try adding it to the `options` property or setting a different value instead";
                )]
            })
        }).collect_combined_result()?);

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
    /// The user-facing name of this option.
    /// If [`Smart::Auto`], [`Self::mapping_name`] is used instead.
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

/// TODO: docs
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
        if !labelable_element_selector().matches(&elem, None) {
            bail!(span, "label `{}` does not point to a form field", self.target.repr())
        }
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

/// Generate an implicit field name based on a counter value.
pub fn gen_field_name(counter: u64) -> EcoString {
    eco_format!("field-{}", counter)
}

/// The representation of a form field in the document.
#[derive(Debug, Clone, Hash)]
pub struct FieldWidget {
    /// The span of the element declaring this widget.
    pub span: Span,
    /// The location of field this widget belongs to.
    /// If this widget is a radio button, this points to the radio group.
    pub field_location: Location,
    /// The size of the widget.
    pub size: Size,
    /// The action to perform when clicking this widget, if any.
    pub action: Option<WidgetAction>,
    /// The value this widget represents, if any. Used for radio buttons.
    pub value: Option<EcoString>,
    /// The appearance of this widget.
    pub appearance: Arc<FieldAppearanceKind<Frame>>,
}

/// The appearance of a field widget.
///
/// Contains a body of a generic type `T`, so that it can be reused with different types.
///
/// For checkboxes and radio buttons, the appearance has two bodies instead of just one,
/// corresponding to their "off" and "on" states.
/// Each body may also have different interactive states; see [`FieldAppearanceBody`].
#[derive(Debug, Clone, Hash)]
pub enum FieldAppearanceKind<T> {
    /// A static appearance (e.g., button).
    Single { variable_text: bool, body: FieldAppearanceBody<T> },
    /// A dual state appearance (e.g., checkbox).
    Dual {
        /// Whether this checkbox/radio is checked.
        active: bool,
        /// The body when
        body_off: FieldAppearanceBody<T>,
        body_on: FieldAppearanceBody<T>,
    },
}

impl<T> FieldAppearanceKind<T> {
    /// Get the primary body of this appearance.
    ///
    /// For single appearances, returns the only body.
    /// For dual appearances, returns the off body.
    pub fn primary_body(&self) -> &FieldAppearanceBody<T> {
        match self {
            Self::Single { body, .. } => body,
            Self::Dual { body_off, .. } => body_off,
        }
    }

    /// Get the active body of this appearance.
    pub fn active_body(&self) -> &FieldAppearanceBody<T> {
        match self {
            Self::Single { body, .. } => body,
            Self::Dual { active, body_on, .. } if *active => body_on,
            Self::Dual { body_off, .. } => body_off,
        }
    }

    /// Whether this appearance is of type 'variable text'.
    pub fn is_variable_text(&self) -> bool {
        match self {
            Self::Single { variable_text, .. } => *variable_text,
            Self::Dual { .. } => false,
        }
    }

    /// If this is of variant [`Self::Single`], convert to [`Option<T>`].
    pub fn single(self) -> Option<FieldAppearanceBody<T>> {
        match self {
            Self::Single { body, .. } => Some(body),
            Self::Dual { .. } => None,
        }
    }

    /// If this is of variant [`Self::Dual`], convert to [`Option<(T, T)>`],
    /// that is an [`Option`] of `(body_off, body_on)`.
    pub fn dual(self) -> Option<(FieldAppearanceBody<T>, FieldAppearanceBody<T>)> {
        match self {
            Self::Single { .. } => None,
            Self::Dual { body_off, body_on, .. } => Some((body_off, body_on)),
        }
    }

    /// Maps this [`FieldAppearanceKind`] to another body type by applying a
    /// function to its body/bodies.
    pub fn try_map<N, E>(
        &self,
        mut f: impl FnMut(&T) -> Result<N, E>,
    ) -> Result<FieldAppearanceKind<N>, E> {
        match self {
            Self::Single { variable_text, body } => Ok(FieldAppearanceKind::Single {
                variable_text: *variable_text,
                body: body.try_map(&mut f)?,
            }),
            Self::Dual { active, body_off, body_on } => Ok(FieldAppearanceKind::Dual {
                active: *active,
                body_off: body_off.try_map(&mut f)?,
                body_on: body_on.try_map(&mut f)?,
            }),
        }
    }
}

/// The body of a single state of a field widget.
///
/// May contain additional representations for the hover and down interactive states.
#[derive(Debug, Clone, Hash)]
pub struct FieldAppearanceBody<T> {
    /// The appearance in the normal state, where the widget is not being interacted with.
    pub normal: T,
    /// The appearance when the user's cursor is hovering over the widget.
    pub hover: Option<T>,
    /// The appearance when the user is clicking on the widget.
    pub down: Option<T>,
}

impl<T> FieldAppearanceBody<T> {
    /// Create a new [`FieldAppearanceBody`] with the given normal state,
    /// and [`None`] for hover and down state.
    pub fn new(normal: T) -> Self {
        FieldAppearanceBody { normal, hover: None, down: None }
    }

    /// Maps this [`FieldAppearanceBody`] to another body type by applying a
    /// function to its states.
    fn try_map<N, E>(
        &self,
        f: &mut impl FnMut(&T) -> Result<N, E>,
    ) -> Result<FieldAppearanceBody<N>, E> {
        Ok(FieldAppearanceBody {
            normal: f(&self.normal)?,
            hover: self.hover.as_ref().map(&mut *f).transpose()?,
            down: self.down.as_ref().map(f).transpose()?,
        })
    }
}

/// Action to perform when a widget is clicked on.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum WidgetAction {
    /// Submit all fields in a form to a given URL.
    Submit { form: Location, target: FormTarget },
    /// Reset all fields in the form to their default value.
    Reset { form: Location },
}

/// Get a selector that targets all form field types.
pub fn form_fields_selector() -> &'static Selector {
    singleton!(
        Selector,
        Selector::Or(eco_vec![
            FormButton::ELEM.select(),
            FormCheckbox::ELEM.select(),
            FormRadioGroup::ELEM.select(),
            FormTextbox::ELEM.select(),
            FormChoice::ELEM.select(),
        ])
    )
}

/// Get a selector that targets all elements that can be labeled via [`FormLabel`].
fn labelable_element_selector() -> &'static Selector {
    singleton!(
        Selector,
        Selector::Or(eco_vec![
            FormButton::ELEM.select(),
            FormCheckbox::ELEM.select(),
            FormRadioGroup::ELEM.select(),
            FormRadioButton::ELEM.select(),
            FormTextbox::ELEM.select(),
            FormChoice::ELEM.select(),
        ])
    )
}

/// Fragment element to allow styling of form fields in a specific state.
#[elem(name = "widget", since = "unreleased", Construct)]
pub struct FormWidget {
    /// Whether the form field is checked.
    ///
    /// This is [`None`] if the form field does not have a checked state.
    pub checked: Option<bool>,
    /// Whether the form field is hovered.
    ///
    /// This is [`None`] if the form field only has the normal state.
    pub state: Option<WidgetState>,

    /// The form field this widget belongs to.
    /// This is a bit of a hack and should be superseded by a proper ancestry mechanism.
    #[ghost]
    #[internal]
    pub parent: Option<Content>,
}

impl Construct for FormWidget {
    fn construct(_: &mut Engine, args: &mut Args) -> SourceResult<Content> {
        bail!(args.span, "cannot be constructed manually")
    }
}

/// The interactive state the widget represents.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum WidgetState {
    /// The normal state, where the widget is not being interacted with.
    Normal,
    /// The user's cursor is hovering over the widget.
    Hovered,
    /// The user is clicking on the widget.
    Down,
}

/// The visual representation of a textbox's value.
#[elem(name = "cursor", since = "unreleased")]
pub struct FormCursor {}
