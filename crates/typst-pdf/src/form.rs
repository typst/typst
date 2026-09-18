use std::collections::HashMap;

use az::SaturatingAs;
use ecow::EcoString;
use indexmap::IndexMap;
use krilla::action::{
    Action, ActionFieldFilter, ResetFormAction, SubmissionFormat, SubmitFormAction,
};
use krilla::annotation as ka;
use krilla::form as kf;
use krilla::geom as kg;
use krilla::num::NormalizedF32;
use krilla::page::Page;
use krilla::stream::Stream;
use krilla::surface::Surface;
use krilla::tagging::Identifier;
use krilla::text::StandardFont;
use rustc_hash::FxBuildHasher;
use typst_layout::PagedDocument;
use typst_library::diag::{
    At, ExpectInternal, SourceResult, bail, panic_internal, warning,
};
use typst_library::foundations::{Content, IntoValue, NativeElement, Repr, StyleChain};
use typst_library::introspection::{Introspector, Location};
use typst_library::layout::{Abs, FixedAlignment, Frame, Sides, Transform};
use typst_library::model::{
    ContentType, FieldAppearanceKind, FieldWidget, FontType, FormButton, FormCheckbox,
    FormChoice, FormElem, FormLabel, FormRadioGroup, FormSignature, FormTextbox,
    HttpMethod, TextboxKind, WidgetAction, form_fields_selector, gen_field_name,
};
use typst_library::visualize::Paint;
use typst_syntax::Span;

use crate::convert::{FrameContext, GlobalContext, handle_frame};
use crate::tags::{self, AnnotationId};
use crate::util::{RectExt, bounding_box};

/// An interactive form field in a PDF document.
#[derive(Debug)]
pub(crate) struct Field {
    /// The full name of the field (i.e., possibly including periods `.`).
    name: EcoString,
    /// The span of the element defining this field.
    span: Span,
    /// The underlying krilla field.
    krilla_field: kf::FieldKind,
}

impl Field {
    pub(crate) fn new(
        document: &PagedDocument,
        field: Content,
        name_generator: impl FnOnce() -> EcoString,
    ) -> SourceResult<Self> {
        let styles = StyleChain::default();
        let span = field.span();

        let get_alt_name = || {
            field
                .label()
                .map(|label| {
                    let all_labels =
                        document.introspector().query(&FormLabel::ELEM.where_(
                            [(FormLabel::target.index(), label.into_value())].into(),
                        ));
                    // TODO: should we only allow one label?

                    let text: Vec<_> =
                        all_labels.iter().map(|content| content.plain_text()).collect();
                    text.join("\n")
                })
                .filter(|s| !s.trim().is_empty())
        };

        let (name, krilla_field) = if let Some(button) = field.to_packed::<FormButton>() {
            let name = button.name.get_cloned(styles).unwrap_or_else(name_generator);
            let partial_name = name.rsplit('.').next().unwrap().to_string();

            let alt_name = Some(button.body.plain_text().to_string())
                .filter(|s| !s.trim().is_empty());
            let disabled = button.disabled.get(styles);

            let krilla_field = kf::FormField::push_button(partial_name)
                .with_location(Some(span.into_raw()))
                .with_alt_name(alt_name)
                .with_read_only(disabled);

            (name, krilla_field.into())
        } else if let Some(checkbox) = field.to_packed::<FormCheckbox>() {
            let name = checkbox.name.get_cloned(styles).unwrap_or_else(name_generator);
            let partial_name = name.rsplit('.').next().unwrap().to_string();

            let checked = checkbox.checked.get(styles);
            let required = checkbox.required.get(styles);
            let read_only = checkbox.disabled.get(styles);

            let krilla_field = kf::FormField::checkbox(partial_name, checked)
                .with_location(Some(span.into_raw()))
                .with_alt_name(get_alt_name())
                .with_default_checked(Some(checked))
                .with_required(required)
                .with_read_only(read_only);

            (name, krilla_field.into())
        } else if let Some(radio_group) = field.to_packed::<FormRadioGroup>() {
            let name = radio_group.name.get_cloned(styles).unwrap_or_else(name_generator);
            let partial_name = name.rsplit('.').next().unwrap().to_string();

            let selected = radio_group
                .resolve_selected_late(document.introspector().as_ref())?
                .map(|button| {
                    button
                        .resolved_value
                        .as_ref()
                        .map(|s| s.to_string())
                        .expect_internal("radio button's resolved value is unset")
                        .at(span)
                })
                .transpose()?;
            let required = radio_group.required.get(styles);
            let read_only = radio_group.disabled.get(styles);

            let krilla_field = kf::FormField::radio(partial_name, selected.clone())
                .with_location(Some(span.into_raw()))
                .with_alt_name(
                    radio_group
                        .title
                        .get_ref(styles)
                        .as_ref()
                        .map(|elem| elem.plain_text().to_string()),
                )
                .with_default_value(selected)
                .with_required(required)
                .with_read_only(read_only);

            (name, krilla_field.into())
        } else if let Some(textbox) = field.to_packed::<FormTextbox>() {
            let name = textbox.name.get_cloned(styles).unwrap_or_else(name_generator);
            let partial_name = name.rsplit('.').next().unwrap().to_string();

            let kind = textbox.kind.get(styles);
            let value = textbox.value.get_ref(styles).as_ref().map(Into::into);
            let multiline = kind == TextboxKind::MultiLine;
            let required = textbox.required.get(styles);
            let read_only = textbox.disabled.get(styles);
            let max_length =
                textbox.max_length.get(styles).map(|i| i.saturating_as::<i32>());
            let fixed_width = kind == TextboxKind::FixedWidth;
            let spellcheck = textbox.spellcheck.get(styles).unwrap_or(true);

            let krilla_field = kf::FormField::text(partial_name)
                .with_location(Some(span.into_raw()))
                .with_alt_name(get_alt_name())
                .with_value(value.clone())
                .with_default_value(value)
                .with_mutliline(multiline)
                .with_required(required)
                .with_read_only(read_only)
                .with_max_length(max_length)
                .with_comb(fixed_width)
                .with_spellcheck(spellcheck);

            (name, krilla_field.into())
        } else if let Some(choice_field) = field.to_packed::<FormChoice>() {
            let name =
                choice_field.name.get_cloned(styles).unwrap_or_else(name_generator);
            let partial_name = name.rsplit('.').next().unwrap().to_string();

            let multiple = choice_field.multiple.get(styles);
            let value = choice_field
                .value_indices
                .as_ref()
                .expect_internal("synthesized `value_indices` not set")
                .at(span)?;
            let required = choice_field.required.get(styles);
            let read_only = choice_field.disabled.get(styles);

            let options = choice_field.options.get_ref(styles);
            let options = options
                .0
                .iter()
                .map(|option| {
                    kf::kind::ChoiceOption::new(
                        option.mapping_name.to_string(),
                        option.display_name.as_ref().map(|s| s.to_string()).custom(),
                    )
                })
                .collect();

            let krilla_field = if multiple {
                kf::FormField::listbox(partial_name)
                    .with_location(Some(span.into_raw()))
                    .with_alt_name(get_alt_name())
                    .with_value(value.to_vec())
                    .with_default_value(value.to_vec())
                    .with_options(options)
                    .with_multiple_options(true)
                    .with_required(required)
                    .with_read_only(read_only)
                    .into()
            } else {
                let value = value.first().copied();
                let krilla_field = kf::FormField::combobox(partial_name)
                    .with_location(Some(span.into_raw()))
                    .with_alt_name(get_alt_name())
                    .with_value(value)
                    .with_default_value(value)
                    .with_options(options)
                    .with_required(required)
                    .with_read_only(read_only);
                krilla_field.into()
            };

            (name, krilla_field)
        } else if let Some(signature_field) = field.to_packed::<FormSignature>() {
            let name =
                signature_field.name.get_cloned(styles).unwrap_or_else(name_generator);
            let partial_name = name.rsplit('.').next().unwrap().to_string();

            let krilla_field = kf::FormField::signature(partial_name)
                .with_location(Some(span.into_raw()))
                .with_alt_name(get_alt_name());

            (name, krilla_field.into())
        } else {
            panic_internal("unknown field type").at(field.span())?
        };

        Ok(Self { name, span, krilla_field })
    }

    pub(crate) fn insert_annotation(
        &mut self,
        page: &mut Page,
        annotation: WidgetAnnotation,
        action: Option<Action>,
    ) -> SourceResult<Identifier> {
        let span = annotation.span;
        let identifier = match &mut self.krilla_field {
            kf::FieldKind::PushButton(form_field) => {
                let body = annotation
                    .appearance
                    .single()
                    .expect_internal("push button does not have single appearance")
                    .at(span)?;
                let widget = form_field
                    .new_widget(annotation.bbox, body.normal)
                    .with_rollover_appearance(body.hover)
                    .with_down_appearance(body.down);

                insert_annotation(span, page, form_field, widget, action)?
            }
            kf::FieldKind::Checkbox(form_field) => {
                let (body_off, body_on) = annotation
                    .appearance
                    .dual()
                    .expect_internal("checkbox does not have dual appearance")
                    .at(span)?;
                let widget = form_field
                    .new_widget(annotation.bbox, body_off.normal, body_on.normal)
                    .with_off_rollover_appearance(body_off.hover)
                    .with_off_down_appearance(body_off.down)
                    .with_on_rollover_appearance(body_on.hover)
                    .with_on_down_appearance(body_on.down);

                insert_annotation(span, page, form_field, widget, action)?
            }
            kf::FieldKind::Radio(form_field) => {
                let value = annotation
                    .value
                    .expect_internal("radio button has no value")
                    .at(span)?
                    .to_string();
                let (body_off, body_on) = annotation
                    .appearance
                    .dual()
                    .expect_internal("radio button does not have dual appearance")
                    .at(span)?;
                let widget = form_field
                    .new_widget(annotation.bbox, value, body_off.normal, body_on.normal)
                    .with_off_rollover_appearance(body_off.hover)
                    .with_off_down_appearance(body_off.down)
                    .with_on_rollover_appearance(body_on.hover)
                    .with_on_down_appearance(body_on.down);

                insert_annotation(span, page, form_field, widget, action)?
            }
            kf::FieldKind::Text(form_field) => {
                if let Some(ap) = annotation.get_krilla_variable_appearance()? {
                    form_field.set_appearance(ap);
                }
                if let Some(alignment) = annotation.get_krilla_text_alignment() {
                    form_field.set_text_alignment(alignment);
                }
                let body = annotation
                    .appearance
                    .single()
                    .expect_internal("textbox does not have single appearance")
                    .at(span)?;
                let widget = form_field.new_widget(annotation.bbox, body.normal);

                insert_annotation(span, page, form_field, widget, action)?
            }
            kf::FieldKind::ListBox(form_field) => {
                if let Some(ap) = annotation.get_krilla_variable_appearance()? {
                    form_field.set_appearance(ap);
                }
                if let Some(alignment) = annotation.get_krilla_text_alignment() {
                    form_field.set_text_alignment(alignment);
                }
                let body = annotation
                    .appearance
                    .single()
                    .expect_internal("listbox does not have single appearance")
                    .at(span)?;
                let widget = form_field.new_widget(annotation.bbox, body.normal);

                insert_annotation(span, page, form_field, widget, action)?
            }
            kf::FieldKind::ComboBox(form_field) => {
                if let Some(ap) = annotation.get_krilla_variable_appearance()? {
                    form_field.set_appearance(ap);
                }
                if let Some(alignment) = annotation.get_krilla_text_alignment() {
                    form_field.set_text_alignment(alignment);
                }
                let body = annotation
                    .appearance
                    .single()
                    .expect_internal("combobox does not have single appearance")
                    .at(span)?;
                let widget = form_field.new_widget(annotation.bbox, body.normal);

                insert_annotation(span, page, form_field, widget, action)?
            }
            kf::FieldKind::Signature(form_field) => {
                let body = annotation
                    .appearance
                    .single()
                    .expect_internal("signature field does not have single appearance")
                    .at(span)?;
                let widget = form_field.new_widget(annotation.bbox, body.normal);

                insert_annotation(span, page, form_field, widget, action)?
            }
        };

        Ok(identifier)
    }
}

fn insert_annotation<FT, A>(
    span: Span,
    page: &mut Page,
    form_field: &mut kf::FormField<FT>,
    mut widget: ka::WidgetAnnotation<A>,
    action: Option<Action>,
) -> SourceResult<Identifier>
where
    ka::WidgetAnnotation<A>: Into<ka::WidgetAnnotationKind>,
{
    widget.set_action_mouse_press(action);

    let annotation = ka::Annotation::from(widget).with_location(Some(span.into_raw()));
    Ok(page.add_widget_annotation(form_field, annotation))
}

/// A widget annotation containing appearance streams.
#[derive(Debug)]
pub(crate) struct WidgetAnnotation {
    /// The annotation id of this annotation in the tag tree, if any.
    pub annotation_id: Option<AnnotationId>,
    /// The location of the field this widget belongs to.
    ///
    /// Multiple widgets can belong to the same field, (e.g., radio buttons).
    pub field_location: Location,
    /// The span of the element defining this widget.
    span: Span,
    /// The bounding box of this widget on the page.
    bbox: kg::Rect,
    /// The value of this annotation, if any. Used for radio buttons.
    value: Option<EcoString>,
    /// The appearance streams of this annotation.
    appearance: FieldAppearanceKind<Stream>,
    /// The action to perform when this widget is clicked, if any.
    action: Option<WidgetAction>,
}

impl WidgetAnnotation {
    /// Converts the [`WidgetAction`] of this annotation to a krilla [`Action`].
    ///
    /// When converting, the appropriate field filters are applied to ensure the
    /// action only targets the fields in the same form as this widget.
    pub fn get_krilla_action(
        &self,
        gc: &mut GlobalContext,
    ) -> SourceResult<Option<Action>> {
        let get_fields = |form_location| -> SourceResult<Vec<String>> {
            Ok(gc
                .acro_form
                .get_field_iter_of_form(form_location)
                .expect_internal("cannot find form with given location")
                .at(self.span)?
                .filter(|field| {
                    !matches!(field.krilla_field, kf::FieldKind::PushButton(_))
                })
                .map(|field| field.name.to_string())
                .collect())
        };
        self.action
            .as_ref()
            .map(|action| match action {
                WidgetAction::Submit { form, target } => {
                    let fields = get_fields(*form)?;

                    if target.v.method == HttpMethod::Get {
                        bail!(
                            target.span,
                            "method `\"get\"` is not supported in PDF export"
                        )
                    }

                    let content_type = target.v.content_type.unwrap_or(ContentType::Fdf);
                    let format = match content_type {
                        ContentType::Fdf => SubmissionFormat::Fdf,
                        ContentType::UrlEncoded => {
                            SubmissionFormat::UrlEncoded(krilla::action::HttpMethod::Post)
                        }
                        ContentType::Xfdf => SubmissionFormat::Xfdf,
                        ContentType::Pdf => SubmissionFormat::Pdf,
                        ContentType::Multipart | ContentType::Plain => {
                            bail!(
                                target.span,
                                "content-type `{}` is not supported in PDF export",
                                content_type.into_value().repr()
                            )
                        }
                    };

                    if matches!(
                        content_type,
                        ContentType::UrlEncoded | ContentType::Xfdf | ContentType::Pdf
                    ) {
                        gc.warnings.push(warning!(
                            target.span,
                            "content-type `{}` has poor support among PDF readers", content_type.into_value().repr();
                            hint: "prefer the `\"fdf\"` content-type whenever possible"
                        ));
                    }

                    Ok(SubmitFormAction::new(target.v.url.to_string())
                        .with_fields(ActionFieldFilter::Include(fields))
                        .with_format(format)
                        .into())
                }
                WidgetAction::Reset { form } => {
                    let fields = get_fields(*form)?;
                    Ok(ResetFormAction::new(ActionFieldFilter::Include(fields)).into())
                }
            })
            .transpose()
    }

    /// Converts the [`typst_library::model::VariableText`] of this annotation
    /// to a krilla [`kf::variable_text::VariableAppearance`] if this annotation
    /// has variable text.
    pub fn get_krilla_variable_appearance(
        &self,
    ) -> SourceResult<Option<kf::variable_text::VariableAppearance>> {
        self.appearance.variable_text().map(|variable_text| {
            let (color, opacity) = match &variable_text.fill_color {
                Paint::Solid(color) => crate::paint::convert_solid(color),
                _ => bail!(
                    self.span, "text of a form field can only be filled with a solid color";
                    hint: "remove the gradient or pattern"
                ),
            };
            let font = get_standard_font(variable_text.font_type, variable_text.font_bold, variable_text.font_italic);
            Ok(kf::variable_text::VariableAppearance {
                font: kf::variable_text::FormFont::Standard(font),
                font_size: variable_text.font_size.to_pt_f32(),
                paint: Some(color.into()),
                opacity: NormalizedF32::new(opacity as f32 / 255.0).unwrap(),
            })
        }).transpose()
    }

    /// Converts the [`typst_library::model::VariableText`] of this annotation
    /// to a krilla [`kf::variable_text::TextAlignment`] if this annotation has
    /// variable text.
    pub fn get_krilla_text_alignment(&self) -> Option<kf::variable_text::TextAlignment> {
        self.appearance.variable_text().map(|variable_text| {
            match variable_text.alignment {
                FixedAlignment::Start => kf::variable_text::TextAlignment::Left,
                FixedAlignment::Center => kf::variable_text::TextAlignment::Center,
                FixedAlignment::End => kf::variable_text::TextAlignment::Right,
            }
        })
    }
}

pub(crate) fn handle_field_widget(
    gc: &mut GlobalContext,
    fc: &mut FrameContext,
    surface: &mut Surface,
    widget: &FieldWidget,
) -> SourceResult<()> {
    let annot_bbox = bounding_box(fc, widget.size);

    fc.push();
    // Translate the entire frame so that the annotation's (0, 0) is the
    // frame's (0, 0). This allows de-duplicating the various widgets (XObject).
    let translate = Transform::translate(-annot_bbox.min.x, -annot_bbox.min.y);
    fc.state_mut().post_concat(translate);
    fc.state_mut().container_post_concat(translate);

    let is_variable_text = widget.appearance.variable_text().is_some();
    if is_variable_text && !fc.state().transform().is_only_translate() {
        bail!(
            widget.span, "textboxes and choice fields cannot be scaled, skewed, or rotated when exporting to PDF";
            hint: "remove any surrounding `scale`, `skew`, or `rotate` elements"
        )
    }

    let annot_bbox = annot_bbox.to_krilla().unwrap();
    // We want the stream bbox to be anchored at (0, 0), so it can be
    // de-duplicated among widgets.
    let stream_bbox =
        kg::Rect::from_xywh(0.0, 0.0, annot_bbox.width(), annot_bbox.height()).unwrap();

    let annotation_id = if tags::disabled(gc) {
        if let Some(outer_span) = gc.tags.in_widget {
            bail!(
                widget.span, "form fields cannot be placed inside other form fields";
                hint[outer_span]: "the outer form field is here";
            );
        }
        if gc.tags.in_tiling
            && let Some(accessibility) = gc.options.validators().accessibility()
        {
            let validator = accessibility.as_str();
            bail!(
                widget.span,
                "{validator} error: PDF artifacts may not contain form fields";
                hint: "a form field was used within a tiling";
            );
        }

        None
    } else {
        let (group_id, field_span) = gc
            .tags
            .tree
            .parent_form_field()
            .expect_internal("expected form field ancestor in logical tree")
            .at(widget.span)?;

        if gc.tags.tree.parent_artifact().is_some() {
            if let Some(accessibility) = gc.options.validators().accessibility() {
                let validator = accessibility.as_str();
                bail!(
                    field_span,
                    "{validator} error: PDF artifacts may not contain form fields";
                );
            }

            None
        } else {
            let annot_id = gc.tags.annotations.reserve();
            let group = gc.tags.tree.groups.get_mut(group_id);
            group.push_annotation(annot_id);
            Some(annot_id)
        }
    };

    gc.tags.in_widget = Some(widget.span);
    let appearance = widget.appearance.try_map(|frame| {
        create_field_stream(gc, fc, surface, frame, is_variable_text, stream_bbox)
    })?;
    gc.tags.in_widget = None;

    let annotation = WidgetAnnotation {
        annotation_id,
        span: widget.span,
        field_location: widget.field_location,
        bbox: annot_bbox,
        value: widget.value.clone(),
        appearance,
        action: widget.action.clone(),
    };

    fc.pop();
    fc.push_widget_annotation(annotation);

    Ok(())
}

/// Create a krilla [`Stream`] from a [`Frame`].
fn create_field_stream(
    gc: &mut GlobalContext,
    fc: &mut FrameContext,
    surface: &mut Surface,
    frame: &Frame,
    is_variable_text: bool,
    bbox: kg::Rect,
) -> SourceResult<Stream> {
    let mut builder = surface.stream_builder();
    let mut surface = builder.surface_with_bbox(bbox);

    if is_variable_text {
        surface.start_variable_text();
    }

    handle_frame(fc, frame, Sides::splat(Abs::zero()), None, &mut surface, gc)?;

    if is_variable_text {
        surface.end_variable_text();
    }

    surface.finish();
    Ok(builder.finish())
}

/// Get the appropriate standard font based on font type (sans, serif or mono),
/// bold, and italic.
fn get_standard_font(
    font_type: FontType,
    is_bold: bool,
    is_italic: bool,
) -> StandardFont {
    match (font_type, is_bold, is_italic) {
        (FontType::Sans, false, false) => StandardFont::Helvetica,
        (FontType::Sans, false, true) => StandardFont::HelveticaOblique,
        (FontType::Sans, true, false) => StandardFont::HelveticaBold,
        (FontType::Sans, true, true) => StandardFont::HelveticaBoldOblique,
        (FontType::Serif, false, false) => StandardFont::TimesRoman,
        (FontType::Serif, false, true) => StandardFont::TimesItalic,
        (FontType::Serif, true, false) => StandardFont::TimesBold,
        (FontType::Serif, true, true) => StandardFont::TimesBoldItalic,
        (FontType::Mono, false, false) => StandardFont::Courier,
        (FontType::Mono, false, true) => StandardFont::CourierOblique,
        (FontType::Mono, true, false) => StandardFont::CourierBold,
        (FontType::Mono, true, true) => StandardFont::CourierBoldOblique,
    }
}

type FieldGroup = IndexMap<EcoString, Node, FxBuildHasher>;

// Allow unbalanced enum under the assumption there are way more Leafs than
// Groups under normal use.
#[allow(clippy::large_enum_variant)]
enum Node {
    Group(FieldGroup),
    Leaf(Field),
}

impl Node {
    fn get_first_leaf(&self) -> Option<&Field> {
        match self {
            Self::Group(fields) => fields.values().find_map(Self::get_first_leaf),
            Self::Leaf(field) => Some(field),
        }
    }
}

/// Build the AcroForm's tree structure for the fields present in the document.
/// The flat field list is converted into a tree structure by splitting the
/// field names by the period character (`.`), as required by PDF.
pub(crate) fn build_field_tree(gc: &mut GlobalContext) -> SourceResult<kf::FieldTree> {
    fn insert_into_tree(
        fields: &mut FieldGroup,
        path: &str,
        field: Field,
    ) -> SourceResult<()> {
        let span = field.span;
        if let Some((path_segment, remainder)) = path.split_once('.') {
            let node = fields
                .entry(path_segment.into())
                .or_insert_with(|| Node::Group(FieldGroup::default()));
            match node {
                Node::Group(group) => {
                    insert_into_tree(group, remainder, field)?;
                }
                Node::Leaf(shorter_field) => {
                    let shorter_path = shorter_field.name.repr();
                    let longer_path = field.name.repr();
                    bail!(
                        shorter_field.span,
                        "form field name `{shorter_path}` conflicts with `{longer_path}`";
                        hint: "change the names so that `{shorter_path}` isn't fully contained by `{longer_path}`";
                        hint: "form fields use dot-separated paths to define nested fields";
                        hint[span]: "the field `{longer_path}` is here";
                    )
                }
            }
        } else {
            let name = field.name.repr();
            if let Some(conflicting_node) = fields.insert(path.into(), Node::Leaf(field))
            {
                match &conflicting_node {
                    Node::Group(_) => {
                        let longer_field = conflicting_node.get_first_leaf().unwrap();
                        let longer_path = longer_field.name.repr();
                        bail!(
                            span,
                            "form field name `{name}` conflicts with `{longer_path}`";
                            hint: "change the names so that `{name}` isn't fully contained by `{longer_path}`";
                            hint: "form fields use dot-separated paths to define nested fields";
                            hint[longer_field.span]: "the field `{longer_path}` is here";
                        )
                    }
                    Node::Leaf(conflicting_field) => {
                        bail!(
                            span, "two distinct form fields are named `{name}`";
                            hint: "change the name of one of the fields";
                            hint[conflicting_field.span]: "the other field is here";
                        )
                    }
                }
            }
        }

        Ok(())
    }

    fn to_krilla_tree(group: FieldGroup) -> Vec<kf::Node> {
        group
            .into_iter()
            .map(|(name, node)| match node {
                Node::Group(group) => {
                    let fields = to_krilla_tree(group);
                    kf::Node::Group(kf::FieldGroup { name: name.to_string(), fields })
                }
                Node::Leaf(field) => kf::Node::Leaf(field.krilla_field),
            })
            .collect()
    }

    let mut fields = FieldGroup::default();
    for field in std::mem::take(&mut gc.acro_form.fields).into_values() {
        let name = field.name.clone();
        insert_into_tree(&mut fields, &name, field)?;
    }

    Ok(kf::FieldTree { fields: to_krilla_tree(fields) })
}

#[derive(Debug, Default)]
pub(crate) struct AcroForm {
    /// Interactive form fields.
    fields: IndexMap<Location, Field, FxBuildHasher>,
    /// Mapping from a form's location to the index range in the `fields` index
    /// map.
    form_to_fields: HashMap<Location, (usize, usize)>,
}

impl AcroForm {
    pub(crate) fn collect_from_document(document: &PagedDocument) -> SourceResult<Self> {
        let introspector = document.introspector();
        let mut acro_form = AcroForm::default();

        let forms = introspector.query(&FormElem::ELEM.select());
        for form in forms {
            let form_location = form.location().unwrap();
            let fields = introspector
                .query(&form_fields_selector().clone().within(form_location.into()));

            let index_start = acro_form.fields.len();

            for field in fields {
                let id = acro_form.fields.len();
                let location = field.location().unwrap();
                let name_generator = || gen_field_name(id as u64 + 1);
                let field = Field::new(document, field, name_generator)?;

                acro_form.fields.insert(location, field);
            }

            // Store index of the first and last field (+1) to belong to this
            // form.
            acro_form
                .form_to_fields
                .insert(form_location, (index_start, acro_form.fields.len()));
        }
        Ok(acro_form)
    }

    pub(crate) fn get_field_mut(&mut self, location: Location) -> Option<&mut Field> {
        self.fields.get_mut(&location)
    }

    pub(crate) fn get_field_iter_of_form(
        &self,
        form_location: Location,
    ) -> Option<impl Iterator<Item = &Field>> {
        let (from, to) = *self.form_to_fields.get(&form_location)?;
        Some(self.fields.as_slice()[from..to].values())
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}
