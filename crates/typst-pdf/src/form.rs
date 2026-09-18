use std::iter::Peekable;

use az::SaturatingAs;
use ecow::{EcoString, EcoVec};
use krilla::action::{
    Action, ActionFieldFilter, ResetFormAction, SubmissionFormat, SubmitFormAction,
};
use krilla::annotation as ka;
use krilla::form as kf;
use krilla::geom as kg;
use krilla::page::Page;
use krilla::stream::Stream;
use krilla::surface::Surface;
use krilla::tagging::Identifier;
use rustc_hash::FxHashMap;
use typst_layout::PagedDocument;
use typst_library::diag::{
    At, ExpectInternal, SourceDiagnostic, SourceResult, bail, panic_internal, warning,
};
use typst_library::foundations::{Content, IntoValue, NativeElement, Repr, StyleChain};
use typst_library::introspection::{Introspector, Location};
use typst_library::layout::{Abs, Frame, Sides, Size};
use typst_library::model::{
    ContentType, FieldAppearanceKind, FieldWidget, FormButton, FormCheckbox, FormChoice,
    FormElem, FormLabel, FormRadioGroup, FormTextbox, HttpMethod, WidgetAction,
    form_fields_selector, gen_field_name,
};
use typst_syntax::Span;

use crate::convert::{FrameContext, GlobalContext, handle_frame};
use crate::tags::{self, AnnotationId};
use crate::util::bounding_box;

/// An interactive form field in a PDF document.
#[derive(Debug)]
pub(crate) struct Field {
    /// The full name of the field (i.e., possibly including periods `.`).
    name: EcoString,
    /// The location of the form this fields belongs to.
    form_location: Location,
    /// The span of the element defining this field.
    span: Span,
    /// The underlying krilla field.
    krilla_field: kf::FieldKind,
}

impl Field {
    pub(crate) fn new(
        document: &PagedDocument,
        form_location: Location,
        field: Content,
        name_generator: impl FnOnce() -> EcoString,
    ) -> SourceResult<Self> {
        let styles = StyleChain::default();
        let span = field.span();

        let get_alt_text = || {
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
                .unwrap_or_default()
        };

        let (name, krilla_field) = if let Some(button) = field.to_packed::<FormButton>() {
            let name = button.name.get_cloned(styles).unwrap_or_else(name_generator);
            let partial_name = name.rsplit('.').next().unwrap().to_string();

            let krilla_field = kf::FormField::push_button(partial_name)
                .with_location(Some(span.into_raw()))
                .with_alt_name(button.body.plain_text().to_string());

            (name, krilla_field.into())
        } else if let Some(checkbox) = field.to_packed::<FormCheckbox>() {
            let name = checkbox.name.get_cloned(styles).unwrap_or_else(name_generator);
            let partial_name = name.rsplit('.').next().unwrap().to_string();

            let checked = checkbox.checked.get(styles);
            let required = checkbox.required.get(styles);
            let read_only = checkbox.disabled.get(styles);

            let krilla_field = kf::FormField::checkbox(partial_name, checked)
                .with_location(Some(span.into_raw()))
                .with_alt_name(get_alt_text())
                .with_default_checked(checked)
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
                        .map(|elem| elem.plain_text().to_string())
                        .unwrap_or_else(get_alt_text),
                )
                .with_default_value(selected)
                .with_required(required)
                .with_read_only(read_only);

            (name, krilla_field.into())
        } else if let Some(textbox) = field.to_packed::<FormTextbox>() {
            let name = textbox.name.get_cloned(styles).unwrap_or_else(name_generator);
            let partial_name = name.rsplit('.').next().unwrap().to_string();

            let multiline = textbox.multiline.get(styles);
            let required = textbox.required.get(styles);
            let read_only = textbox.disabled.get(styles);
            let max_length =
                textbox.max_length.get(styles).map(|i| i.saturating_as::<i32>());
            let fixed_width = textbox.fixed_width.get(styles);
            let spellcheck = textbox.spellcheck.get(styles).unwrap_or(true);
            let value = textbox.value.get_ref(styles);

            let mut krilla_field = kf::FormField::text(partial_name)
                .with_location(Some(span.into_raw()))
                .with_alt_name(get_alt_text())
                .with_mutliline(multiline)
                .with_required(required)
                .with_read_only(read_only)
                .with_max_length(max_length)
                .with_comb(fixed_width)
                .with_spellcheck(spellcheck);

            if let Some(value) = value {
                krilla_field.set_value(value.to_string());
                krilla_field.set_default_value(value.to_string());
            }

            (name, krilla_field.into())
        } else if let Some(choice_field) = field.to_packed::<FormChoice>() {
            let name =
                choice_field.name.get_cloned(styles).unwrap_or_else(name_generator);
            let partial_name = name.rsplit('.').next().unwrap().to_string();

            let multiple = choice_field.multiple.get(styles);
            let value = choice_field
                .value_indexes
                .as_ref()
                .expect_internal("synthesized `value_indexes` not set")
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
                    .with_alt_name(get_alt_text())
                    .with_value(value.to_vec())
                    .with_default_value(value.to_vec())
                    .with_options(options)
                    .with_multiple_options(true)
                    .with_required(required)
                    .with_read_only(read_only)
                    .into()
            } else {
                let mut krilla_field = kf::FormField::combobox(partial_name)
                    .with_location(Some(span.into_raw()))
                    .with_alt_name(get_alt_text())
                    .with_options(options)
                    .with_required(required)
                    .with_read_only(read_only);
                if let Some(&value) = value.first() {
                    krilla_field.set_value(value);
                    krilla_field.set_default_value(value);
                }
                krilla_field.into()
            };

            (name, krilla_field)
        } else {
            panic_internal("unknown field type").at(field.span())?
        };

        Ok(Self { name, form_location, span, krilla_field })
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
                let mut widget = form_field.new_widget(annotation.bbox, body.normal);
                if let Some(rollover) = body.hover {
                    widget.set_rollover_appearance(rollover);
                }
                if let Some(down) = body.down {
                    widget.set_down_appearance(down);
                }

                Self::insert_annotation_inner(span, page, form_field, widget, action)?
            }
            kf::FieldKind::Checkbox(form_field) => {
                let (body_off, body_on) = annotation
                    .appearance
                    .dual()
                    .expect_internal("checkbox does not have dual appearance")
                    .at(span)?;
                let mut widget = form_field.new_widget(
                    annotation.bbox,
                    body_off.normal,
                    body_on.normal,
                );
                if let Some(rollover) = body_off.hover {
                    widget.set_off_rollover_appearance(rollover);
                }
                if let Some(down) = body_off.down {
                    widget.set_off_down_appearance(down);
                }
                if let Some(rollover) = body_on.hover {
                    widget.set_on_rollover_appearance(rollover);
                }
                if let Some(down) = body_on.down {
                    widget.set_on_down_appearance(down);
                }

                Self::insert_annotation_inner(span, page, form_field, widget, action)?
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
                let mut widget = form_field.new_widget(
                    annotation.bbox,
                    value,
                    body_off.normal,
                    body_on.normal,
                );
                if let Some(rollover) = body_off.hover {
                    widget.set_off_rollover_appearance(rollover);
                }
                if let Some(down) = body_off.down {
                    widget.set_off_down_appearance(down);
                }
                if let Some(rollover) = body_on.hover {
                    widget.set_on_rollover_appearance(rollover);
                }
                if let Some(down) = body_on.down {
                    widget.set_on_down_appearance(down);
                }

                Self::insert_annotation_inner(span, page, form_field, widget, action)?
            }
            kf::FieldKind::Text(form_field) => {
                let body = annotation
                    .appearance
                    .single()
                    .expect_internal("textbox does not have single appearance")
                    .at(span)?;
                let widget = form_field.new_widget(annotation.bbox, body.normal);

                Self::insert_annotation_inner(span, page, form_field, widget, action)?
            }
            kf::FieldKind::ListBox(form_field) => {
                let body = annotation
                    .appearance
                    .single()
                    .expect_internal("listbox does not have single appearance")
                    .at(span)?;
                let widget = form_field.new_widget(annotation.bbox, body.normal);

                Self::insert_annotation_inner(span, page, form_field, widget, action)?
            }
            kf::FieldKind::ComboBox(form_field) => {
                let body = annotation
                    .appearance
                    .single()
                    .expect_internal("combobox does not have single appearance")
                    .at(span)?;
                let widget = form_field.new_widget(annotation.bbox, body.normal);

                Self::insert_annotation_inner(span, page, form_field, widget, action)?
            }
        };

        Ok(identifier)
    }

    fn insert_annotation_inner<FT, A>(
        span: Span,
        page: &mut Page,
        form_field: &mut kf::FormField<FT>,
        mut widget: ka::WidgetAnnotation<A>,
        action: Option<Action>,
    ) -> SourceResult<Identifier>
    where
        ka::WidgetAnnotation<A>: Into<ka::WidgetAnnotationKind>,
    {
        if let Some(action) = action {
            widget.set_action_mouse_press(action);
        }

        let annotation =
            ka::Annotation::from(widget).with_location(Some(span.into_raw()));
        Ok(page.add_widget_annotation(form_field, annotation))
    }
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
        warnings: &mut EcoVec<SourceDiagnostic>,
    ) -> SourceResult<Option<Action>> {
        let get_fields = |form_location| {
            gc.acro_form
                .fields
                .values()
                .filter(|field| field.form_location == form_location)
                .filter(|field| {
                    !matches!(field.krilla_field, kf::FieldKind::PushButton(_))
                })
                .map(|filter| filter.name.to_string())
                .collect::<Vec<_>>()
        };
        self.action
            .as_ref()
            .map(|action| match action {
                WidgetAction::Submit { form, target } => {
                    let fields = get_fields(*form);

                    if target.method == HttpMethod::Get {
                        bail!(
                            self.span,
                            "method `\"GET\"` is not supported in PDF export"
                        )
                    }

                    let content_type = target.content_type.unwrap_or(ContentType::Fdf);
                    let format = match content_type {
                        ContentType::Fdf => SubmissionFormat::Fdf,
                        ContentType::UrlEncoded => {
                            SubmissionFormat::UrlEncoded(krilla::action::HttpMethod::Post)
                        }
                        ContentType::Xfdf => SubmissionFormat::Xfdf,
                        ContentType::Pdf => SubmissionFormat::Pdf,
                        ContentType::Multipart | ContentType::Plain => {
                            bail!(
                                self.span,
                                "content-type `{}` is not supported in PDF export",
                                content_type.into_value().repr()
                            )
                        }
                    };

                    if matches!(
                        content_type,
                        ContentType::UrlEncoded | ContentType::Xfdf | ContentType::Pdf
                    ) {
                        warnings.push(warning!(
                            self.span,
                            "content-type `{}` has poor support among PDF readers", content_type.into_value().repr();
                            hint: "prefer the `\"fdf\"` content-type whenever possible"
                        ));
                    }

                    Ok(SubmitFormAction::new(target.url.to_string())
                        .with_fields(ActionFieldFilter::Include(fields))
                        .with_format(format)
                        .into())
                }
                WidgetAction::Reset { form } => {
                    let fields = get_fields(*form);
                    Ok(ResetFormAction::new(ActionFieldFilter::Include(fields)).into())
                }
            })
            .transpose()
    }
}

pub(crate) fn handle_field_appearance(
    fc: &mut FrameContext,
    gc: &mut GlobalContext,
    surface: &mut Surface,
    widget: &FieldWidget,
) -> SourceResult<()> {
    let annot_bbox = bounding_box(fc, widget.size);
    let stream_bbox =
        kg::Rect::from_xywh(0.0, 0.0, annot_bbox.width(), annot_bbox.height()).unwrap();

    let annotation_id = if tags::disabled(gc) {
        if gc.tags.in_tiling
            && let Some(accessibility) = gc.options.validators().accessibility()
        {
            let validator = accessibility.as_str();
            bail!(
                Span::detached(),
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
            .at(Span::detached())?;

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

    let appearance = widget.appearance.try_map(|frame| {
        create_field_stream(
            gc,
            surface,
            frame,
            widget.appearance.is_variable_text(),
            widget.size,
            stream_bbox,
        )
    })?;

    let annotation = WidgetAnnotation {
        annotation_id,
        span: widget.span,
        field_location: widget.field_location,
        bbox: annot_bbox,
        value: widget.value.clone(),
        appearance,
        action: widget.action.clone(),
    };

    fc.push_widget_annotation(annotation);

    Ok(())
}

/// Create a krilla [`Stream`] from a [`Frame`].
fn create_field_stream(
    gc: &mut GlobalContext,
    surface: &mut Surface,
    frame: &Frame,
    is_variable_text: bool,
    size: Size,
    bbox: kg::Rect,
) -> SourceResult<Stream> {
    let mut builder = surface.stream_builder();
    let mut surface = builder.surface_with_bbox(bbox);

    if is_variable_text {
        surface.start_variable_text();
    }

    let mut fc = FrameContext::new(None, size);
    handle_frame(&mut fc, frame, Sides::splat(Abs::zero()), None, &mut surface, gc)?;

    if is_variable_text {
        surface.end_variable_text();
    }

    surface.finish();
    Ok(builder.finish())
}

type FieldGroup = FxHashMap<EcoString, Node>;

// TODO: is it worth optimizing allocations by using indexes and SmallVec instead?; krilla uses this
// layout anyway already
// Allow unbalanced enum under the assumption there are way more Leafs than Groups
// under normal use.
#[allow(clippy::large_enum_variant)]
enum Node {
    Group(FieldGroup),
    Leaf(Field),
}

/// Build the AcroForm's tree structure for the fields present in the document.
/// The flat field list is converted into a tree structure by splitting the field names by the
/// period character (`.`), as required by PDF.
pub(crate) fn build_field_tree(gc: &mut GlobalContext) -> SourceResult<kf::FieldTree> {
    fn insert_into_tree<'a, I>(
        fields: &mut FieldGroup,
        mut path: Peekable<I>,
        field: Field,
    ) -> SourceResult<()>
    where
        I: Iterator<Item = &'a str>,
    {
        let span = field.span;
        let path_segment = path
            .next()
            .expect_internal("there is always one path segment when splitting")
            .at(span)?;
        let is_leaf = path.peek().is_none();

        if is_leaf {
            let name = field.name.clone();
            if fields.insert(path_segment.into(), Node::Leaf(field)).is_some() {
                bail!(
                    span, "there are two distinct form fields named `{name}`";
                    hint: "this can happen if you have a field named `{name}` \
                           and another named `{name}.something-else`"
                )
            }
        } else {
            let node = fields
                .entry(path_segment.into())
                .or_insert_with(|| Node::Group(FieldGroup::default()));
            match node {
                Node::Group(group) => {
                    insert_into_tree(group, path, field)?;
                }
                Node::Leaf(field) => {
                    let name = &field.name;
                    bail!(
                        field.span, "there are two distinct form fields named `{name}`";
                        hint: "this can happen if you have a field named `{name}` \
                               and another named `{name}.something-else`"
                    )
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
        let path = name.split('.').peekable();
        insert_into_tree(&mut fields, path, field)?;
    }

    Ok(kf::FieldTree { fields: to_krilla_tree(fields) })
}

#[derive(Debug, Default)]
pub(crate) struct AcroForm {
    /// Interactive form fields.
    fields: FxHashMap<Location, Field>,
}

impl AcroForm {
    pub(crate) fn collect_from_document(document: &PagedDocument) -> SourceResult<Self> {
        let introspector = document.introspector();
        let mut seen_names = FxHashMap::default();

        let mut acro_form = AcroForm::default();

        let forms = introspector.query(&FormElem::ELEM.select());
        for form in forms {
            let form_location = form.location().unwrap();
            let fields = introspector
                .query(&form_fields_selector().clone().within(form_location.into()));

            for field in fields {
                let id = acro_form.fields.len();
                let location = field.location().unwrap();
                let name_generator = || gen_field_name(id as u64 + 1);
                let field = Field::new(document, form_location, field, name_generator)?;

                let prev = seen_names.insert(field.name.clone(), location);
                if let Some(prev_id) = prev {
                    bail!(
                        field.span, "two fields share the same name ({})", field.name.repr();
                        hint: "change the name of one of the fields";
                        hint[acro_form.fields[&prev_id].span]: "the other field is here";
                    );
                }

                acro_form.fields.insert(location, field);
            }
        }
        Ok(acro_form)
    }

    pub(crate) fn get_field_mut(&mut self, location: Location) -> Option<&mut Field> {
        self.fields.get_mut(&location)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}
