use std::sync::Arc;

use comemo::Track;
use ecow::{EcoString, EcoVec, eco_format, eco_vec};
use rustc_hash::FxHashMap;
use typst_macros::{Cast, cast};
use typst_syntax::{LooselySpanned, Span};
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
use crate::layout::{Abs, Em, FixedAlignment, Frame, Length, Rel, Sides, Size, Sizing};
use crate::text::TextElem;
use crate::visualize::{CircleElem, Paint, RectElem};

/// A form with interactive and fillable fields.
///
/// Forms fields provide a way for users to users to interact with your document.
/// For example, a user can fill out their name on a textbox
/// or indicate their dietary restrictions using checkboxes. // TODO better examples?
/// Typst supports many field types across both PDF and HTML,
/// such as @form.button[buttons], @form.checkbox[checkboxes],
/// @form.radio-group[radio buttons], @form.textbox[textboxes],
/// @form.choice[dropdowns], and @form.choice[listboxes].
/// When exporting to PDF, signature fields are also supported.
///
/// All form fields in a Typst document must be enclosed within a @form element.
///
/// = Example <example>
/// The example below demonstrates creating a simple form with various form
/// fields.
/// However, this example and the other ones on this page are not interactive.
///
/// ```example
/// #form[
///   #form.label(<name>)[Name:]
///   #form.textbox() <name>
///
///   Food preferences:
///   #grid(
///     columns: 2,
///     gutter: 0.4em,
///     align: horizon,
///     [#form.checkbox() <meat>],
///     form.label(<meat>)[Meat],
///     [#form.checkbox() <vegetarian>],
///     form.label(<vegetarian>)[Vegetarian],
///     [#form.checkbox() <vegan>],
///     form.label(<vegan>)[Vegan],
///     [#form.checkbox() <lactose>],
///     form.label(<lactose>)[Lactose-free],
///     [#form.checkbox() <gluten>],
///     form.label(<gluten>)[Gluten-free],
///   )
/// ]
/// ```
///
/// = Accessibility <accessibility>
///
/// Form fields need extra information to be properly consumed by users
/// of Assistive Technology (AT).
/// You can describe a form field via the @form.label element, which is
/// visible to sighted users and announced for users of AT.
/// We strongly recommend that all fields have an associated @form.label.
///
/// However, @form.radio-group[radio groups] cannot be described via @form.label
/// and must instead use the group's @form.radio-group.title[`title`]
/// argument.
///
/// = Compatibility <compatibility>
///
/// Not all PDF readers have full support for all interactive forms features.
/// The following compatibility table shows which features are available in
/// each reader we tested, and when exporting to HTML.
/// While testing, we used these PDF readers:
/// - Adobe Acrobat Reader (version 2026.3.22110)
/// - Pdfium (Chrome 158)
/// - PDF.js (Firefox 161)
/// - Foxit Reader (version 2026.2.1.39815)
/// - Poppler (version 26.11.0; Okular 26.08.3 & GNOME Papers 51.1)
/// - Apple Quartz (Preview, Safari; macOS v??)
///
/// #let yes = emoji.checkmark
/// #let partial = emoji.warning
/// #let no = emoji.crossmark
///
/// #docs-table(
///   table.header[][Acrobat][Pdfium][PDF.js][Foxit][Poppler][Apple][HTML],
///   [@form.target[Submission] in FDF],
///   yes, yes, no,
///   [#yes#footnote[
///     Foxit Reader incorrectly sends a `Content-Type: x-www-form-url-encoded`
///     header for FDF and PDF. When submitting in XFDF, it correctly sends
///     `Content-Type: application/vnd.adobe.xfdf`.] <foxit-submission>
///   ],
///   no, [?], no,
///
///   [@form.target[Submission] in XFDF or PDF],
///   yes, no, no,
///   [#yes#footnote(<foxit-submission>)],
///   no, [?], no,
///
///   [@form.target[Submission] in URL encoded],
///   yes, no, no, yes, no, [?], yes,
///
///   [@form.target[Submission] in multipart or plain text],
///   no, no, no, no, no, [?], yes,
///
///   [Handle @form.target[submission] response],
///   yes, no, no, yes, no, [?], yes,
///
///   [@form.button.action[Reset fields] button],
///   yes, yes, yes, yes,
///   [#partial#footnote[Okular resets fields when a reset button is clicked,
///     but GNOME Papers does not]],
///   [?], yes,
///
///   [@form.widget.state[Hover/down appearances]],
///   yes,
///   [#partial#footnote[Hover/down states are supported on checkboxes and
///     radio buttons, but not on regular buttons.] <btn-hover>],
///   no,
///   [#partial#footnote(<btn-hover>)],
///   no, [?], no,
///
///   [Text color in textboxes and choice fields],
///   yes, yes, yes, yes, yes, [?], no,
///
///   [Horizontal alignment in textboxes and choice fields],
///   yes, no, no, yes, yes, [?], no,
///
///   [@form.textbox.kind[Fixed-width textboxes]],
///   yes, yes, yes, yes, yes, [?], no,
///
///   [@form.textbox.placeholder[Placeholder in textboxes]],
///   no, no, no, no, no, no, yes,
///
///   [@form.signature[Signature fields]],
///   yes, no, no, yes, yes, [?], no,
/// )
///
/// Everything else is fully supported in all readers and in HTML.
#[elem(scope, since = "unreleased", Locatable, ShowSet)]
pub struct FormElem {
    /// The fields belonging to this form, along with any surrounding content.
    #[required]
    pub body: Content,

    /// Where to submit this form when a @form.button with a
    /// @form.button.action[submit action] is pressed.
    ///
    /// Can either be a URL as a string, or a dictionary with the following
    /// fields:
    /// - `url` (required): the URL where the form data is to be submitted.
    ///   Can also be a relative path when exporting to HTML.
    /// - `content-type` (optional, default `{auto}`): the format to submit form
    ///   data in.
    ///   Can be one of `{auto}`, `{"url-encoded"}`, `{"multipart"}`,
    ///   `{"plain"}`, `{"pdf"}`, `{"fdf"}`, or `{"xfdf"}`.
    ///   When set to `{auto}`, `{"fdf"}` is used for PDF export and
    ///   `{"url-encoded"}` is used for HTML export.
    /// - `method` (optional, default `{"post"}`): the HTTP method to use when
    ///   submitting form data. Can be one of `{"post"}` or `{"get"}`.
    ///   Only `{"post"}` is supported on paged export.
    ///
    /// We highly recommend explicitly setting each form field's name when you
    /// expect the form to be submitted. Otherwise, fields won't be submitted at
    /// all in HTML and will have auto-generated names in PDF.
    ///
    /// ```typ
    /// #form(
    ///   target: (
    ///     url: "https://example.com/submission",
    ///     content-type: "multipart",
    ///     method: "post",
    ///   )
    /// )
    /// ```
    pub target: LooselySpanned<Option<FormTarget>>,

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
    type FormSignature;
    #[elem]
    type FormLabel;
    #[elem]
    type FormWidget;
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

impl Packed<FormElem> {
    /// Check if this form is inside another form;
    /// if so, returns an error.
    pub fn assert_not_inside_form(&self, styles: StyleChain) -> SourceResult<()> {
        if let Some(parent) = styles
            .get_all_ref(FormElem::form)
            .filter_map(|opt| opt.as_ref())
            .find(|el| el.location() != self.location())
        {
            bail!(
                self.span(), "this form cannot be inside another form";
                hint[parent.span()]: "the outer form is here";
            )
        }
        Ok(())
    }
}

/// Where and how to submit a filled out form.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct FormTarget {
    /// The URL to send form data to.
    pub url: EcoString,
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
    /// URL encoded key-value pairs (`application/x-www-form-urlencoded`).
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

/// An interactive button.
///
/// A button can perform an action when clicked, such as
/// resetting all fields in a form to their default values,
/// or submitting the form.
/// In HTML, you may also add your own click listener via JavaScript.
///
/// ```example
/// >>> #form[
/// #form.button(action: "reset")[Reset Fields]
/// >>> ]
/// ```
///
/// === Styling <button-styling>
///
/// Buttons can be styled by a show rule targeting @form.widget.
/// The button's default show rule contains three widgets, depending on
/// the @form.widget.state[interactive state] of the button.
///
/// ```example
/// >>> #form[
/// #show form.button: it => {
///   show form.widget: ellipse(
///     fill: aqua,
///     it.body,
///   )
///   it
/// }
/// #form.button[Click me!]
/// >>> ]
/// ```
///
/// The table below shows the available selectors you can use to style
/// the @form.widget[widgets] of buttons:
///
/// #docs-table(
///   table.header[State][Selector],
///   [_Any_], `{form.widget}`,
///   [Normal], `{form.widget.where(state: "normal")}`,
///   [Hovered], `{form.widget.where(state: "hovered")}`,
///   [Down], `{form.widget.where(state: "down")}`,
/// )
///
/// Alternatively, you can use show-set rules to style the existing
/// @rect that encloses button:
///
/// ```example
/// >>> #form[
/// #show form.button: set rect(
///   fill: aqua,
///   stroke: blue,
///   radius: 5pt,
/// )
/// #form.button[Click me!]
/// >>> ]
/// ```
#[elem(name = "button", since = "unreleased", Locatable, Tagged, ShowSet)]
pub struct FormButton {
    /// The label of this button.
    #[required]
    pub body: Content,

    /// The name of this field.
    ///
    /// If `{auto}`, will be automatically generated as `{"field-<n>"}`
    /// when exporting to PDF.
    pub name: Smart<EcoString>,

    /// The action that is triggered when this button is clicked, if any.
    ///
    /// When set to `{"submit"}`, the surrounding form must have a
    /// @form.target[target].
    ///
    /// In HTML, you can set this to `{none}` and then use custom JavaScript to
    /// set your own click listener that performs another action instead.
    pub action: Option<ButtonAction>,

    /// Whether this button is disabled. When true, interacting with it does not
    /// trigger any action.
    pub disabled: bool,

    /// The button's width.
    ///
    /// When `{auto}`, the button will expand with its content.
    pub width: Smart<Rel<Length>>,

    /// The button's height.
    ///
    /// When `{auto}`, the button will expand with its content.
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

        let disabled = self.disabled.get(styles);
        out.set(
            RectElem::outset,
            Sides::splat(Some((styles::STROKE_THICKNESS / -2.0).into())),
        );
        out.set(
            RectElem::stroke,
            Smart::Custom(Sides::splat(Some(Some(styles::field_stroke(disabled))))),
        );
        // Set inset to rect's default + inset from inner stroke
        out.set(
            RectElem::inset,
            Sides::splat(Some(
                Rel::from(styles::STROKE_THICKNESS) + Rel::from(Abs::pt(5.0)),
            )),
        );
        if disabled {
            styles::set_disabled_styles(&mut out);
        }
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

/// An interactive checkbox.
///
/// A checkbox can be used for binary choices, for example "yes" and "no".
/// Checkboxes are toggled on and off when clicked.
///
/// ```example
/// >>> #form[
/// #form.checkbox(checked: false)
/// #form.checkbox(checked: true)
/// >>> ]
/// ```
///
/// === Styling <checkbox-styling>
///
/// Like @form.button:button-styling[buttons], checkboxes can be styled via a
/// show rule targeting @form.widget.
/// The checkbox's default show rule contains 6 widgets, depending on
/// whether it is checked and the @form.widget.state[interactive state] it is in.
///
/// The table below shows some of the available selectors you can use to style
/// the @form.widget[widgets] of checkboxes:
///
/// #docs-table(
///   table.header[Checked][State][Selector],
///   [No], [_Any_], `{form.widget.where(checked: false)}`,
///   [No], [Normal], `{form.widget.where(checked: false, state: "normal")}`,
///   [No], [Hovered], `{form.widget.where(checked: false, state: "hovered")}`,
///   [No], [Down], `{form.widget.where(checked: false, state: "down")}`,
///   [Yes], [_Any_], `{form.widget.where(checked: true)}`,
///   [Yes], [Normal], `{form.widget.where(checked: true, state: "normal")}`,
///   [Yes], [Hovered], `{form.widget.where(checked: true, state: "hovered")}`,
///   [Yes], [Down], `{form.widget.where(checked: true, state: "down")}`,
/// )
#[elem(name = "checkbox", since = "unreleased", Locatable, Tagged, ShowSet)]
pub struct FormCheckbox {
    /// The name of this field.
    ///
    /// If `{auto}`, will be automatically generated as `{"field-<n>"}`
    /// when exporting to PDF.
    pub name: Smart<EcoString>,

    /// Whether this checkbox is checked by default.
    pub checked: bool,

    /// Whether this checkbox is required and must be checked.
    pub required: bool,

    /// Whether this checkbox is read-only. When true, clicking on it does not
    /// toggle it.
    pub disabled: bool,

    /// The checkbox's width.
    ///
    /// When `{auto}`, the width defaults to `{1em}` when exporting to a paged
    /// format, and is unset when exporting to HTML.
    pub width: Smart<Rel<Length>>,

    /// The checkbox's height.
    ///
    /// When `{auto}`, the height defaults to `{1em}` when exporting to a paged
    /// format, and is unset when exporting to HTML.
    pub height: Sizing,
}

impl ShowSet for Packed<FormCheckbox> {
    fn show_set(&self, styles: StyleChain) -> Styles {
        let mut out = Styles::new();
        // Makes the field itself available to the `FieldWidget`. Should be
        // superseded by a proper ancestry mechanism in the future.
        // No need to materialize since the widget show-rule does not access any
        // property.
        out.set(FormWidget::parent, Some(self.clone().pack()));

        let disabled = self.disabled.get(styles);
        styles::set_inner_stroke(&mut out, disabled);
        if disabled {
            styles::set_disabled_styles(&mut out);
        }
        out
    }
}

/// A group of radio buttons.
///
/// Radio groups can be used for exclusive choices.
/// In a group of radio buttons, at most one button can be selected at a time,
/// that is, when selecting one option, all other options are automatically
/// unselected.
///
/// ```example
/// >>> #form[
/// #form.radio-group(
///   title: [Best typesetting software?],
///   selected: <typst>,
///   grid(
///     columns: 2,
///     gutter: 0.4em,
///     align: horizon,
///     [#form.radio() <typst>],
///     form.label(<typst>)[Typst],
///     [#form.radio() <latex>],
///     form.label(<latex>)[LaTeX],
///     [#form.radio() <md>],
///     form.label(<md>)[Markdown],
///   )
/// )
/// >>> ]
/// ```
#[elem(name = "radio-group", since = "unreleased", Locatable, ShowSet)]
pub struct FormRadioGroup {
    /// The radio buttons belonging to this group, along with any surrounding
    /// content.
    #[required]
    pub body: Content,

    /// The name of this field.
    ///
    /// If `{auto}`, will be automatically generated as `{"field-<n>"}`
    /// when exporting to PDF.
    pub name: Smart<EcoString>,

    /// The label pointing to the radio button that is selected by default.
    pub selected: Option<Label>,

    /// Whether this radio group is required and one of the options must be
    /// selected.
    pub required: bool,

    /// The title of this radio group.
    ///
    /// In PDF, this is also used as the @form:accessibility[alternative
    /// description] for this field.
    /// In HTML, this becomes the `<legend>` element of the `<fieldset>`.
    pub title: Option<Content>,

    /// Whether this radio group is read-only. When true, clicking on one of its
    /// buttons does not change the selected option.
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
        // Makes the radio group itself available to its radio buttons. Should
        // be superseded by a proper ancestry mechanism in the future.
        let mut elem = self.clone();
        elem.materialize(styles);
        out.set(FormRadioButton::parent, Some(elem));
        out
    }
}

impl FormRadioGroup {
    /// Get the ancestor radio group element that is present in the style chain,
    /// if any.
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

/// An interactive radio button.
///
/// Must appear inside of a @form.radio-group[radio group].
///
/// === Styling <radio-styling>
///
/// Like regular @form.button:button-styling[buttons], radio buttons can be
/// styled via a show rule targeting @form.widget.
/// The radio button's default show rule contains 6 widgets, depending on
/// whether it is selected and the @form.widget.state[interactive state] it is
/// in.
///
/// The table below shows some of the available selectors you can use to style
/// the @form.widget[widgets] of radio buttons:
///
/// #docs-table(
///   table.header[Selected][State][Selector],
///   [No], [_Any_], `{form.widget.where(checked: false)}`,
///   [No], [Normal], `{form.widget.where(checked: false, state: "normal")}`,
///   [No], [Hovered], `{form.widget.where(checked: false, state: "hovered")}`,
///   [No], [Down], `{form.widget.where(checked: false, state: "down")}`,
///   [Yes], [_Any_], `{form.widget.where(checked: true)}`,
///   [Yes], [Normal], `{form.widget.where(checked: true, state: "normal")}`,
///   [Yes], [Hovered], `{form.widget.where(checked: true, state: "hovered")}`,
///   [Yes], [Down], `{form.widget.where(checked: true, state: "down")}`,
/// )
///
/// Unlike regular buttons, the radio buttons' default show rule uses @circle
/// instead of @rect, so you should target `circle` in the show rules you write.
#[elem(name = "radio", since = "unreleased", Locatable, Tagged, Synthesize, ShowSet)]
pub struct FormRadioButton {
    /// The value of the radio group when this option is selected.
    ///
    /// If `{auto}`, it is automatically generated on PDF export as
    /// `{"option-<n>"}`.
    pub value: Smart<EcoString>,

    /// The radio button's width.
    ///
    /// When `{auto}`, the width defaults to `{1em}` when exporting to a paged
    /// format, and is unset when exporting to HTML.
    pub width: Smart<Rel<Length>>,

    /// The radio button's height.
    ///
    /// When `{auto}`, the height defaults to `{1em}` when exporting to a paged
    /// format, and is unset when exporting to HTML.
    pub height: Sizing,

    /// Lets radio buttons access the radio group they are part of.
    /// This is a bit of a hack and should be superseded by a proper ancestry
    /// mechanism.
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
    fn show_set(&self, styles: StyleChain) -> Styles {
        let mut out = Styles::new();
        // Makes the field itself available to the `FieldWidget`. Should be
        // superseded by a proper ancestry mechanism in the future.
        // No need to materialize since the widget show-rule does not access any
        // property.
        out.set(FormWidget::parent, Some(self.clone().pack()));

        let disabled = styles
            .get_ref(FormRadioButton::parent)
            .as_ref()
            .is_some_and(|group| group.disabled.get(styles));
        let stroke = styles::field_stroke(disabled);
        out.set(
            CircleElem::outset,
            Sides::splat(Some((styles::STROKE_THICKNESS / -2.0).into())),
        );
        out.set(CircleElem::stroke, Smart::Custom(Some(stroke)));
        if disabled {
            out.set(CircleElem::fill, Some(styles::DISABLED_FILL.into()));
            out.set(TextElem::fill, styles::DISABLED_TEXT_FILL.into());
        }
        out
    }
}

/// A free-form text input.
///
/// A textbox can be used when you want the user to input arbitrary text
/// into a field, and it comes in three variants:
///
/// - *single-line*, where the value cannot contain line breaks;
/// - *multi-line*, where the value can span more than one line, ideal for long
///   descriptions; and
/// - *fixed-width*, where the textbox is divided into sections of equal width
///   that take a single character each, ideal for text that is always the same
///   length.
///
/// ```example
/// >>> #form[
/// >>> #set form.textbox(width: 100%)
/// // Single-line
/// #form.label(<name>)[Name:]
/// #form.textbox() <name>
///
/// // Multi-line
/// #form.label(<about>)[About:]
/// #form.textbox(kind: "multi-line") <about>
///
/// // Fixed-width
/// #form.label(<id>)[ID number:]
/// #form.textbox(
///   kind: "fixed-width",
///   max-length: 9,
/// ) <id>
/// >>> ]
/// ```
///
/// === Styling <textbox-styling>
///
/// By default, Typst encloses the textbox in a @rect, and sets its
/// @rect.stroke[stroke] depending on the @form.textbox.kind[textbox's variant].
/// You can customize this rectangle via a show-set rule:
///
/// ```example
/// >>> #form[
/// >>> #set form.textbox(width: 100%)
/// #show form.textbox: set rect(stroke: (bottom: 1pt + blue))
/// #form.textbox()
/// >>> ]
/// ```
///
/// In the case of fixed-width textboxes, Typst additionally draws vertical
/// separators between the characters. These lines are drawn with the
/// rectangle's stroke, when all sides have the same stroke. If you change the
/// rectangle's stroke in a way that it is no longer equals on all sides,
/// Typst will not draw these vertical separators.
///
/// Furthermore, when exporting to PDF, Typst will emit styling information
/// to inform the PDF reader which @text.font[font], @text.size[text size],
/// @text.fill[text color], @text.weight[text weight], @text.style[text style],
/// and @alignment[horizontal alignment] to use. However, please note that not
/// all PDF readers respect all styles, and so your document should not depend
/// on this for readability.
///
/// To set which styles Typst will emit for textboxes, you can use a show-set
/// rule as such:
///
/// ```typ
/// #show form.textbox: set text(size: 12pt, weight: "bold")
/// ```
///
/// Moreover, Typst will not embed the current font in the emitted PDF, but
/// instead choose a replacement font out of the standard PDF fonts, depending
/// on whether your font is sans-serif, serif, or monospace.
/// Since Typst relies on standard PDF fonts, textboxes can only be used on PDF
/// 1.4 through 1.7, and Typst will throw an error when exporting to PDF 2.0.
///
/// Finally, the textbox contains only one @form.widget, which can be styled
/// via a show rule.
#[elem(name = "textbox", since = "unreleased", Locatable, Tagged, ShowSet)]
pub struct FormTextbox {
    /// The name of this field.
    ///
    /// If `{auto}`, will be automatically generated as `{"field-<n>"}`
    /// when exporting to PDF.
    pub name: Smart<EcoString>,

    /// The default value of this field.
    pub value: Option<EcoString>,

    /// The kind of this textbox.
    pub kind: TextboxKind,

    /// Whether it is mandatory to fill out this field (i.e., it cannot be
    /// empty).
    pub required: bool,

    /// The max length, in characters, of the value of this field.
    pub max_length: Option<u32>,

    /// The placeholder text to show when the textbox is empty.
    ///
    /// Only supported in HTML export.
    pub placeholder: Option<EcoString>,

    /// Whether this textbox field is read-only. When true, it is not possible
    /// to change the text in the textbox.
    pub disabled: bool,

    /// Whether the spellchecker should be enabled when filling out this field.
    ///
    /// When set to `{auto}`, it is enabled in PDF export, and respects the
    /// browser defaults in HTML. In HTML export, this also controls text auto
    /// correction.
    pub spellcheck: Smart<bool>,

    /// The field's width.
    ///
    /// When `{auto}`, the width defaults to:
    /// - `{18em}` if this is a single-line field and when exporting to a paged
    ///   format;
    /// - `{100%}` if this is a multi-line field and when exporting to a paged
    ///   format; or
    /// - unset when exporting to HTML.
    pub width: Smart<Rel<Length>>,

    /// The field's height.
    ///
    /// When `{auto}`, the height defaults to:
    /// - sizing according to the current font and text size if this is a
    ///   single-line field and when exporting to a paged format;
    /// - `{5em}` if this is a multi-line field and when exporting to a paged
    ///   format; or
    /// - unset when exporting to HTML.
    pub height: Sizing,
}

impl Packed<FormTextbox> {
    pub fn validate_value_length(&self, styles: StyleChain) -> SourceResult<()> {
        let Some(value) = self.value.get_ref(styles).as_ref() else { return Ok(()) };
        let Some(max_length) = self.max_length.get(styles) else { return Ok(()) };

        let length = value.chars().count();
        if length > max_length as usize {
            bail!(
                self.span(), "textbox's value is longer than `max-length`";
                hint: "the value has `{}` characters, but the maximum is `{}` characters", length, max_length;
            )
        }
        Ok(())
    }
}

impl ShowSet for Packed<FormTextbox> {
    fn show_set(&self, styles: StyleChain) -> Styles {
        let mut out = Styles::new();
        // Makes the field itself available to the `FieldWidget`. Should be
        // superseded by a proper ancestry mechanism in the future.
        let mut elem = self.clone().pack();
        elem.materialize(styles);
        out.set(FormWidget::parent, Some(elem));

        let single_line = self.kind.get(styles) == TextboxKind::SingleLine;
        let disabled = self.disabled.get(styles);

        let stroke = styles::sides_all_or_bottom(
            !single_line,
            Some(Some(styles::field_stroke(disabled))),
            Some(None),
        );
        let inset = styles::sides_all_or_bottom(
            !single_line,
            Some((styles::STROKE_THICKNESS / 2.0).into()),
            Some(Rel::zero()),
        );
        out.set(RectElem::stroke, Smart::Custom(stroke));
        out.set(RectElem::inset, inset);

        if disabled {
            styles::set_disabled_styles(&mut out);
        }
        out
    }
}

/// The kind of a textbox.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum TextboxKind {
    /// The textbox's value can only span a single line, that is, it cannot have
    /// line breaks.
    #[default]
    SingleLine,
    /// The textbox's value can span multiple lines.
    MultiLine,
    /// The textbox is divided into as many equally spaced positions as the
    /// value of `max-length`.
    ///
    /// The `max-length` property must be set.
    /// Only supported in PDF export.
    FixedWidth,
}

/// A dropdown or listbox field.
///
/// Choice fields can be used when a user is expected to select from a
/// pre-defined list of options.
/// This field comes in two variants: a dropdown, when only one option can be
/// selected at a time, or a listbox when selecting multiple options is allowed.
///
/// ```example
/// >>> #form[
///  #let options = (
///    "Chocolate", "Vanilla",
///    "Raspberry", "Strawberry",
///  )
///
///  // Dropdown
///  #form.choice(
///     options: options,
///     value: "Chocolate",
///  )
///
///  // Listbox
///  #form.choice(
///     options: options,
///     multiple: true,
///     value: ("Chocolate", "Raspberry"),
///  )
/// >>> ]
/// ```
///
/// === Styling <choice-styling>
///
/// PDF does not provide sufficient control over how readers will present choice
/// fields to the user. Typst provides default styles in a best-effort basis,
/// but most of these styles will be discarded when the user interacts with the
/// choice field.
///
/// Regardless, all of the @form.textbox:textbox-styling[styling considerations
/// for textboxes] also apply to choice fields, including PDF 2.0 limitations.
/// Likewise, choice fields only have one @form.widget in their default show
/// rule.
#[elem(name = "choice", since = "unreleased", Locatable, Tagged, Synthesize, ShowSet)]
pub struct FormChoice {
    /// The name of this field.
    ///
    /// If `{auto}`, will be automatically generated as `field-<n>`
    /// when exporting to PDF.
    pub name: Smart<EcoString>,

    /// Whether this field allows multiple options to be selected at the same
    /// time.
    ///
    /// - When `false`, this field is displayed as a dropdown.
    /// - When `true`, this field is displayed as a listbox.
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
    ///   >>> #form[
    ///   #form.choice(
    ///     options: ("red-car", "blue-car"),
    ///     value: "red-car",
    ///   )
    ///   >>> ]
    ///   ```
    /// )
    ///
    /// #example(
    ///   title: "Give a dictionary mapping to display name",
    ///   ```
    ///   >>> #form[
    ///   #form.choice(
    ///     options: (
    ///       "red-car": "Red Car",
    ///       "blue-car": "Blue Car",
    ///     ),
    ///     value: "red-car",
    ///   )
    ///   >>> ]
    ///   ```
    /// )
    ///
    /// While in PDF no option is selected when the document is opened,
    /// in HTML the browser will automatically select the first option.
    /// If you do not want any option to be selected, add an option with an
    /// empty string (`{""}`) as its value as the first element of this array or
    /// dictionary.
    pub options: ChoiceOptions,

    /// The default value of this field.
    ///
    /// Can be an array of string if @form.choice.multiple[`multiple`] is true.
    /// All entries must be present in @form.choice.options[`options`].
    pub value: OneOrMultiple<EcoString>,

    /// Whether it is mandatory to fill out this field (i.e., it cannot be
    /// empty).
    pub required: bool,

    /// Whether this choice field is read-only. When true, the selected
    /// option(s) cannot be changed.
    pub disabled: bool,

    /// The field's width.
    ///
    /// When `{auto}`, the width defaults to `{10em}` when exporting to a paged
    /// format, and is unset when exporting to HTML.
    pub width: Smart<Rel<Length>>,

    /// The field's height.
    ///
    /// When `{auto}`, the height defaults to:
    /// - sizing according to the current font and text size if this is a
    ///   dropdown and when exporting to a paged format;
    /// - sizing according to the number of options if this is a listbox and
    ///   when exporting to a paged format; or
    /// - unset when exporting to HTML.
    pub height: Sizing,

    /// The selected @form.choice.value[`values`] of this field,
    /// but represented as indices of @form.choice.options[`options`].
    #[internal]
    #[synthesized]
    pub value_indices: EcoVec<usize>,
}

impl FormChoice {
    /// Get the background of a selected option of a listbox,
    /// depending whether the listbox is disabled.
    pub fn listbox_selected_paint(disabled: bool) -> Paint {
        if disabled {
            styles::LISTBOX_SELECTED_DISABLED_FILL.into()
        } else {
            styles::LISTBOX_SELECTED_FILL.into()
        }
    }
}

impl Packed<FormChoice> {
    fn resolve_value(&self, styles: StyleChain) -> SourceResult<EcoVec<usize>> {
        let span = self.span();

        let mut seen_options = FxHashMap::default();
        for (i, option) in self.options.get_ref(styles).0.iter().enumerate() {
            if let Some(prev) = seen_options.insert(option.mapping_name.as_str(), i) {
                bail!(
                    span, "duplicate option `{}`", option.mapping_name.repr();
                    hint: "occurred in options at indices {prev} and {i}";
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
        value.iter().map(|v| {
            seen_options.get(v.as_str()).copied().ok_or_else(||{
                eco_vec![error!(
                    span, "unknown value `{}`", v.repr();
                    hint: "value is not one of the options of this field";
                    hint: "try adding it to the `options` property or setting a different value instead";
                )]
            })
        }).collect_combined_result()
    }
}

impl ShowSet for Packed<FormChoice> {
    fn show_set(&self, styles: StyleChain) -> Styles {
        let mut out = Styles::new();
        // Makes the field itself available to the `FieldWidget`. Should be
        // superseded by a proper ancestry mechanism in the future.
        let mut elem = self.clone();
        elem.value_indices = self.resolve_value(styles).ok();
        let mut elem = elem.pack();
        elem.materialize(styles);
        out.set(FormWidget::parent, Some(elem));

        let multiple = self.multiple.get(styles);
        let disabled = self.disabled.get(styles);

        let stroke = styles::sides_all_or_bottom(
            multiple,
            Some(Some(styles::field_stroke(disabled))),
            Some(None),
        );
        let inset = styles::sides_all_or_bottom(
            multiple,
            Some((styles::STROKE_THICKNESS / 2.0).into()),
            Some(Rel::zero()),
        );
        out.set(RectElem::stroke, Smart::Custom(stroke));
        out.set(RectElem::inset, inset);

        if disabled {
            styles::set_disabled_styles(&mut out);
        }
        out
    }
}

impl Synthesize for Packed<FormChoice> {
    fn synthesize(&mut self, _: &mut Engine, styles: StyleChain) -> SourceResult<()> {
        self.value_indices = Some(self.resolve_value(styles)?);
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

/// A signature field.
///
/// When used within a PDF file, this field allows the user to cryptographically
/// sign the present document and optionally place a visual signature where
/// this field is.
///
/// This form field is not supported in HTML and some PDF readers. See the
/// @form:compatibility[compatibility section] for more details.
///
/// ```example
/// >>> #form[
///   #form.signature()
/// >>> ]
/// ```
///
/// === Styling <signature-styling>
///
/// Similarly to @form.textbox:textbox-styling[textboxes], signature fields are
/// enclosed in a @rect, which can be styled via a show-set rule.
/// For example, you can change its @rect.stroke[stroke] and @rect.fill[fill]:
///
/// ```example
/// >>> #form[
/// #show form.signature: set rect(
///     stroke: 0.5pt,
///     fill: black.transparentize(95%),
/// )
/// #form.signature()
/// >>> ]
/// ```
///
/// Signature fields only contain one @form.widget, which will be replaced by
/// the PDF reader upon signing the document. Therefore, this can also be
/// used as a placeholder, for example:
///
/// ```example
/// >>> #form[
/// #show form.signature: it => {
///   show form.widget: align(left + bottom, sym.crossmark)
///   it
/// }
/// #form.signature()
/// >>> ]
/// ```
#[elem(name = "signature", since = "unreleased", Locatable, Tagged, ShowSet)]
pub struct FormSignature {
    /// The name of this field.
    ///
    /// If `{auto}`, will be automatically generated as `field-<n>`
    /// when exporting to PDF.
    pub name: Smart<EcoString>,

    /// The field's width.
    #[default(Em::new(10.0).into())]
    pub width: Rel<Length>,

    /// The field's height.
    #[default(Em::new(3.0).into())]
    pub height: Sizing, // TODO this still accepts auto
}

impl ShowSet for Packed<FormSignature> {
    fn show_set(&self, styles: StyleChain) -> Styles {
        let mut out = Styles::new();
        // Makes the field itself available to the `FieldWidget`. Should be
        // superseded by a proper ancestry mechanism in the future.
        let mut elem = self.clone().pack();
        elem.materialize(styles);
        out.set(FormWidget::parent, Some(elem));

        let stroke = styles::field_stroke(false);
        out.set(
            RectElem::stroke,
            Smart::Custom(Sides::new(None, None, None, Some(Some(stroke)))),
        );
        out.set(
            RectElem::inset,
            Sides::splat(Some((styles::STROKE_THICKNESS / 2.0).into())),
        );
        out
    }
}

/// A label describing a form field.
///
/// In order to create an @form:accessibility[accessible form], all fields must
/// have an accompanying label indicating what they represent.
/// While a label can be placed anywhere in the document,
/// it is recommended for it to be close to the field it describes.
///
/// Furthermore, when exporting to HTML, labels also act as focus target
/// for their linked field.
/// For example, clicking the label of a checkbox will toggle that checkbox,
/// and clicking the label of a textbox will focus the cursor in that textbox.
#[elem(name = "label", since = "unreleased", Locatable)]
pub struct FormLabel {
    /// The form field that this label describes.
    ///
    /// The following elements can be targeted by a form label:
    /// @form.button, @form.checkbox, @form.radio, @form.textbox, @form.choice,
    /// and @form.signature. Notably, @form.radio-group[radio groups] cannot
    /// and their @form.radio-group.title[title] should be used to
    /// describe them instead.
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
            if elem.is::<FormRadioGroup>() {
                bail!(
                    span, "label `{}` cannot point to a radio group", self.target.repr();
                    hint: "use the radio group's `title` argument instead";
                )
            } else {
                bail!(
                    span,
                    "label `{}` does not point to a form field",
                    self.target.repr()
                )
            }
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

/// Check if the given element is inside a form field;
/// if so, returns an error.
pub fn assert_not_inside_form_field<T: NativeElement>(
    elem: &Packed<T>,
    styles: StyleChain,
) -> SourceResult<()> {
    if let Some(parent) = styles
        .get_all_ref(FormWidget::parent)
        .filter_map(|opt| opt.as_ref())
        .find(|el| el.location() != elem.location())
    {
        bail!(
            elem.span(),
            "{} element cannot occur inside of another form field", T::ELEM.name();
            hint[parent.span()]: "the outer form field is here";
        )
    }
    Ok(())
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
/// Contains a body of a generic type `T`, so that it can be reused with
/// different types.
///
/// For checkboxes and radio buttons, the appearance has two bodies instead of
/// just one, corresponding to their "off" and "on" states.
/// Each body may also have different interactive states; see
/// [`FieldAppearanceBody`].
#[derive(Debug, Clone, Hash)]
pub enum FieldAppearanceKind<T> {
    /// A static appearance (e.g., button).
    Single { variable_text: Option<VariableText>, body: FieldAppearanceBody<T> },
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
            Self::Dual { active: true, body_on, .. } => body_on,
            Self::Dual { body_off, .. } => body_off,
        }
    }

    /// Whether this appearance is of type 'variable text'.
    pub fn variable_text(&self) -> Option<&VariableText> {
        match self {
            Self::Single { variable_text, .. } => variable_text.as_ref(),
            Self::Dual { .. } => None,
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
                variable_text: variable_text.clone(),
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
/// May contain additional representations for the hover and down interactive
/// states.
#[derive(Debug, Clone, Hash)]
pub struct FieldAppearanceBody<T> {
    /// The appearance in the normal state, where the widget is not being
    /// interacted with.
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
        mut f: impl FnMut(&T) -> Result<N, E>,
    ) -> Result<FieldAppearanceBody<N>, E> {
        Ok(FieldAppearanceBody {
            normal: f(&self.normal)?,
            hover: self.hover.as_ref().map(&mut f).transpose()?,
            down: self.down.as_ref().map(&mut f).transpose()?,
        })
    }
}

/// Styles to apply to variable text.
#[derive(Debug, Clone, Hash)]
pub struct VariableText {
    /// The type of font to use (sans-serif, serif, or monospace).
    pub font_type: FontType,
    /// Whether to use the bold variant of the font.
    pub font_bold: bool,
    /// Whether to use the italic variant of the font.
    pub font_italic: bool,
    /// Resolved font size.
    pub font_size: Abs,
    /// Text color. Only solid colors are supported.
    pub fill_color: Paint,
    /// Horizontal alignment.
    pub alignment: FixedAlignment,
}

/// The type of the font.
#[derive(Debug, Copy, Clone, Hash)]
pub enum FontType {
    /// Sans serif.
    Sans,
    /// Serif.
    Serif,
    /// Monospace.
    Mono,
}

/// Action to perform when a widget is clicked on.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum WidgetAction {
    /// Submit all fields in a form to a given URL.
    Submit { form: Location, target: LooselySpanned<FormTarget> },
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
            FormSignature::ELEM.select(),
        ])
    )
}

/// Get a selector that targets all elements that can be labeled via
/// [`FormLabel`].
fn labelable_element_selector() -> &'static Selector {
    singleton!(
        Selector,
        Selector::Or(eco_vec![
            FormButton::ELEM.select(),
            FormCheckbox::ELEM.select(),
            FormRadioButton::ELEM.select(),
            FormTextbox::ELEM.select(),
            FormChoice::ELEM.select(),
            FormSignature::ELEM.select(),
        ])
    )
}

/// Fragment element to allow styling of form fields in a specific state.
///
/// This element cannot be constructed manually and is part of the default
/// show rule for each form field. See the documentation of each field kind for
/// a list of widgets that exist in each element.
///
/// A widget corresponds to the content inside the interactive part of each form
/// field. Not all PDF readers will respect the content of widgets and may
/// overlay their own interactive elements on top instead. Furthermore, for
/// textboxes, choice, and signature fields, the widget is only preserved until
/// the form field is interacted with, after which the PDF reader will replace
/// its contents with the value the user has inputted.
///
/// You can use this element in @reference:styling:show-rules[show rule]
/// selectors to style a form field.
/// For example, to style a textbox:
///
/// ```typ
/// #show form.textbox: it => {
///   show form.widget: // ... your code here
///   it
/// }
/// ```
///
/// Furthermore, you can use a @function.where[`where` selector] to restrict
/// the show rule to only certain states when a form field has more than one,
/// as is the case with buttons, checkboxes, and radio buttons.
///
/// ```typ
/// #show form.checkbox: it => {
///   // Selects all widgets where the checkbox is checked.
///   show form.widget.where(checked: true): ...
///   // Selects the widget where the checkbox is unchecked
///   // and is being hovered with the cursor.
///   show form.widget.where(checked: false, state: "hovered"): ...
///   it
/// }
/// ```
///
/// Due to how widgets are displayed in PDFs, the content inside of a widget
/// is @block.clip[clipped].
/// When writing show and show-set rules, you might have to add
/// enough inset and outset to avoid cutting off the content of the widget.
#[elem(name = "widget", since = "unreleased", Construct)]
pub struct FormWidget {
    /// Whether the form field is checked.
    ///
    /// This is `{none}` if the form field does not have a checked state.
    /// Only @form.checkbox[checkboxes] and @form.radio[radio buttons] have
    /// checked states.
    pub checked: Option<bool>,
    /// The interactive state the form field is in.
    ///
    /// This is `{none}` if the form field only has the normal state.
    pub state: Option<WidgetState>,

    /// The form field this widget belongs to.
    /// This is a bit of a hack and should be superseded by a proper ancestry
    /// mechanism.
    #[ghost]
    #[internal]
    pub parent: Option<Content>,

    /// The cap height of the font used for variable text.
    /// No text is drawn by Typst with this font, but PDF readers will use it,
    /// so Typst needs to be aware of its height to size elements accordingly.
    #[ghost]
    #[internal]
    pub font_cap_height: Option<Em>,
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

/// Constants and helper functions to style form fields.
mod styles {
    use crate::foundations::{Smart, Styles};
    use crate::layout::{Em, Sides};
    use crate::text::TextElem;
    use crate::visualize::{Color, Luma, ProcessColor, RectElem, Rgb, Stroke};

    /// The thickness of the default stroke of form fields, in points.
    pub const STROKE_THICKNESS: Em = Em::new(0.05);

    /// Background color of disabled field.
    pub const DISABLED_FILL: Color =
        Color::Process(ProcessColor::Luma(Luma::new(0.95, 0.7)));
    /// Stroke color of disabled field.
    pub const DISABLED_STROKE_COLOR: Color =
        Color::Process(ProcessColor::Luma(Luma::new(0.0, 0.5)));
    /// Text color of disabled field.
    pub const DISABLED_TEXT_FILL: Color =
        Color::Process(ProcessColor::Luma(Luma::new(0.15, 0.7)));

    /// Background color of a selected option of a listbox.
    pub const LISTBOX_SELECTED_FILL: Color =
        Color::Process(ProcessColor::Rgb(Rgb::new(0.0, 0.47843137, 1.0, 1.0)));
    /// Background color of a selected option of a disabled listbox.
    pub const LISTBOX_SELECTED_DISABLED_FILL: Color =
        Color::Process(ProcessColor::Luma(Luma::new(0.15, 0.21)));

    /// Get the stroke for a field, depending whether it is disabled.
    pub fn field_stroke(disabled: bool) -> Stroke {
        if disabled {
            Stroke::from_pair(DISABLED_STROKE_COLOR, STROKE_THICKNESS.into())
        } else {
            Stroke::from_thickness(STROKE_THICKNESS)
        }
    }

    /// Set the background and text color for a disabled field.
    pub fn set_disabled_styles(styles: &mut Styles) {
        styles.set(RectElem::fill, Some(DISABLED_FILL.into()));
        styles.set(TextElem::fill, DISABLED_TEXT_FILL.into());
    }

    /// Set the stroke and accompanying outset/inset for an inner stroke on a
    /// [`RectElem`].
    pub fn set_inner_stroke(styles: &mut Styles, disabled: bool) {
        styles
            .set(RectElem::outset, Sides::splat(Some((STROKE_THICKNESS / -2.0).into())));
        styles.set(
            RectElem::stroke,
            Smart::Custom(Sides::splat(Some(Some(field_stroke(disabled))))),
        );
        styles.set(RectElem::inset, Sides::splat(Some(STROKE_THICKNESS.into())));
    }

    /// Get a [`Sides`] with the same value on all sides if `all` is true,
    /// or only on the bottom otherwise.
    /// If `all` is false, all sides expect bottom are set to `value_rest`.
    pub fn sides_all_or_bottom<T: Clone>(all: bool, value: T, value_rest: T) -> Sides<T> {
        if all {
            Sides::splat(value)
        } else {
            Sides::new(value_rest.clone(), value_rest.clone(), value_rest, value)
        }
    }
}
