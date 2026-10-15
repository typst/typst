use ecow::EcoString;
use ecow::eco_format;
use ecow::eco_vec;
use krilla::action::Action;
use krilla::action::ResetFormAction;
use krilla::form as kf;
use krilla::geom as kg;
use krilla::page::Page;
use krilla::stream::Stream;
use krilla::surface::Surface;
use krilla::tagging::Identifier;
use rustc_hash::FxHashMap;
use std::iter::Peekable;
use typst_layout::PagedDocument;
use typst_library::diag::{At, ExpectInternal, SourceResult, bail, panic_internal};
use typst_library::foundations::{Content, NativeElement, Repr, Selector, StyleChain};
use typst_library::introspection::{Introspector, Location};
use typst_library::layout::Frame;
use typst_library::layout::{Abs, Point, Sides, Size};
use typst_library::model::{
    FieldAppearanceKind, FieldWidget, FormButtonElem, FormCheckboxElem, FormChoiceElem,
    FormElem, FormRadioGroupElem, FormTextboxElem, WidgetAction,
};
use typst_syntax::Span;

use crate::convert::{FrameContext, GlobalContext, handle_frame};
use crate::tags::{self, AnnotationId};
use crate::util::PointExt;

#[derive(Debug)]
pub(crate) struct Field {
    /// The full name of the field (i.e., possibly including periods `.`).
    name: EcoString,
    form_location: Location,
    body: Content,
    /// The underlying krilla field.
    krilla_field: kf::FieldKind,
}

impl Field {
    pub(crate) fn new(
        form_location: Location,
        field: Content,
        name_generator: impl FnOnce() -> EcoString,
    ) -> SourceResult<Self> {
        let styles = StyleChain::default();
        let (name, krilla_field) = if let Some(button) =
            field.to_packed::<FormButtonElem>()
        {
            let name = button.name.get_cloned(styles).unwrap_or_else(name_generator);
            let partial_name = name.rsplit('.').next().unwrap().to_string();

            let krilla_field = kf::FormField::push_button(partial_name);

            (name, krilla_field.into())
        } else if let Some(checkbox) = field.to_packed::<FormCheckboxElem>() {
            let name = checkbox.name.get_cloned(styles).unwrap_or_else(name_generator);
            let partial_name = name.rsplit('.').next().unwrap().to_string();

            let checked = checkbox.checked.get(styles);
            let required = checkbox.required.get(styles);
            let read_only = checkbox.read_only.get(styles);

            let krilla_field = kf::FormField::checkbox(partial_name, checked)
                .with_required(required)
                .with_read_only(read_only);

            (name, krilla_field.into())
        } else if let Some(radio_group) = field.to_packed::<FormRadioGroupElem>() {
            let name = radio_group.name.get_cloned(styles).unwrap_or_else(name_generator);
            let partial_name = name.rsplit('.').next().unwrap().to_string();

            let selected = radio_group.selected.get_ref(styles);
            let required = radio_group.required.get(styles);
            let read_only = radio_group.read_only.get(styles);

            let krilla_field = kf::FormField::radio(
                partial_name,
                selected.as_ref().map(|s| s.to_string()),
            )
            .with_required(required)
            .with_read_only(read_only);

            (name, krilla_field.into())
        } else if let Some(textbox) = field.to_packed::<FormTextboxElem>() {
            let name = textbox.name.get_cloned(styles).unwrap_or_else(name_generator);
            let partial_name = name.rsplit('.').next().unwrap().to_string();

            let multiline = textbox.multiline.get(styles);
            let required = textbox.required.get(styles);
            let read_only = textbox.read_only.get(styles);
            // TODO: checked cast?
            let max_length = textbox.max_length.get(styles).map(|i| i as i32);
            let fixed_width = textbox.fixed_width.get(styles);
            let value = textbox.value.get_ref(styles);

            let mut krilla_field = kf::FormField::text(partial_name)
                .with_mutliline(multiline)
                .with_required(required)
                .with_read_only(read_only)
                .with_max_length(max_length)
                .with_comb(fixed_width);
            // TODO spellchecker

            if let Some(value) = value {
                krilla_field.set_value(value.to_string());
            }

            (name, krilla_field.into())
        } else if let Some(choice_field) = field.to_packed::<FormChoiceElem>() {
            let name =
                choice_field.name.get_cloned(styles).unwrap_or_else(name_generator);
            let partial_name = name.rsplit('.').next().unwrap().to_string();

            let multiple = choice_field.multiple.get(styles);
            let value = &choice_field.value.get_ref(styles).0;
            let required = choice_field.required.get(styles);
            let read_only = choice_field.read_only.get(styles);

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
                let value = value.iter().map(|s| s.to_string()).collect();
                kf::FormField::listbox(partial_name)
                    .with_value(value)
                    .with_options(options)
                    .with_multiple_options(true)
                    .with_required(required)
                    .with_read_only(read_only)
                    .into()
            } else {
                let mut krilla_field = kf::FormField::combobox(partial_name)
                    .with_options(options)
                    .with_required(required)
                    .with_read_only(read_only);
                if let Some(value) = value.first() {
                    krilla_field.set_value(value.to_string());
                }
                krilla_field.into()
            };

            (name, krilla_field)
        } else {
            // FIXME: Possibly `return panic_internal()` when it uses never type (MSRV 1.100)
            panic_internal("unknown field type").at(field.span())?;
            unreachable!();
        };

        Ok(Self { name, form_location, body: field, krilla_field })
    }

    pub(crate) fn insert_annotation(
        &mut self,
        page: &mut Page,
        annotation: WidgetAnnotation,
        action: Option<Action>,
    ) -> SourceResult<Identifier> {
        let span = self.body.span();
        let identifier = match &mut self.krilla_field {
            kf::FieldKind::PushButton(form_field) => {
                let stream = annotation
                    .appearance
                    .single()
                    .expect_internal("push button does not have single appearance")
                    .at(span)?;
                let mut widget = form_field.new_widget(annotation.bbox, stream);
                if let Some(action) = action {
                    widget.set_action_mouse_press(action);
                }
                page.add_widget_annotation(form_field, widget.into())
            }
            kf::FieldKind::Checkbox(form_field) => {
                let (stream_off, stream_on) = annotation
                    .appearance
                    .dual()
                    .expect_internal("checkbox does not have dual appearance")
                    .at(span)?;
                let widget =
                    form_field.new_widget(annotation.bbox, stream_off, stream_on);

                page.add_widget_annotation(form_field, widget.into())
            }
            kf::FieldKind::Radio(form_field) => {
                let value = annotation
                    .value
                    .expect_internal("radio button has no value")
                    .at(span)?
                    .to_string();
                let (stream_off, stream_on) = annotation
                    .appearance
                    .dual()
                    .expect_internal("radio button does not have dual appearance")
                    .at(span)?;
                let widget =
                    form_field.new_widget(annotation.bbox, value, stream_off, stream_on);

                page.add_widget_annotation(form_field, widget.into())
            }
            kf::FieldKind::Text(form_field) => {
                let stream = annotation
                    .appearance
                    .single()
                    .expect_internal("textbox does not have single appearance")
                    .at(span)?;
                let widget = form_field.new_widget(annotation.bbox, stream);

                page.add_widget_annotation(form_field, widget.into())
            }
            kf::FieldKind::ListBox(form_field) => {
                let stream = annotation
                    .appearance
                    .single()
                    .expect_internal("listbox does not have single appearance")
                    .at(span)?;
                let widget = form_field.new_widget(annotation.bbox, stream);

                page.add_widget_annotation(form_field, widget.into())
            }
            kf::FieldKind::ComboBox(form_field) => {
                let stream = annotation
                    .appearance
                    .single()
                    .expect_internal("combobox does not have single appearance")
                    .at(span)?;
                let widget = form_field.new_widget(annotation.bbox, stream);

                page.add_widget_annotation(form_field, widget.into())
            }
        };

        Ok(identifier)
    }
}

/// A widget annotation containing appearance streams.
#[derive(Debug)]
pub(crate) struct WidgetAnnotation {
    pub annotation_id: Option<AnnotationId>,
    pub field_location: Location,
    bbox: kg::Rect,
    /// The value of this annotation, if any. Used for radio buttons.
    value: Option<EcoString>,
    appearance: FieldAppearanceKind<Stream>,
    /// The action to perform when this widget is clicked, if any.
    action: Option<WidgetAction>,
}

impl WidgetAnnotation {
    pub fn get_krilla_action(&self, gc: &mut GlobalContext) -> Option<Action> {
        self.action.as_ref().map(|action| match action {
            WidgetAction::Submit { .. } => todo!(),
            WidgetAction::Reset { form } => {
                let fields = gc
                    .acro_form
                    .fields
                    .iter()
                    .filter(|field| field.form_location == *form)
                    .filter(|field| {
                        !matches!(field.krilla_field, kf::FieldKind::PushButton(_))
                    })
                    .map(|filter| filter.name.to_string())
                    .collect();
                ResetFormAction::Include(fields).into()
            }
        })
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

    let appearance = widget.appearance.convert(|frame| {
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
        field_location: widget.field_location,
        bbox: annot_bbox,
        value: widget.value.clone(),
        appearance,
        action: widget.action.clone(),
    };

    fc.push_widget_annotation(annotation);

    Ok(())
}

pub(crate) fn create_field_stream(
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

// TODO: copied from link.rs, maybe create some kind of utils?
/// Compute the bounding box of the transformed rectangle for this frame.
fn bounding_box(fc: &FrameContext, size: Size) -> kg::Rect {
    let pos = Point::zero();
    let points = [
        pos + Point::with_y(size.y),
        pos + size.to_point(),
        pos + Point::with_x(size.x),
        pos,
    ];

    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for point in points {
        let p = point.transform(fc.state().transform()).to_krilla();
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }

    kg::Rect::from_ltrb(min_x, min_y, max_x, max_y).unwrap()
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
        let span = field.body.span();
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
                    hint: "this can happen if you have a field named `{name}` and another named `{name}.<something else>`"
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
                        field.body.span(), "there are two distinct form fields named `{name}`";
                        hint: "this can happen if you have a field named `{name}` and another named `{name}.<something else>`"
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
    for field in std::mem::take(&mut gc.acro_form.fields) {
        let name = field.name.clone();
        let path = name.split('.').peekable();
        insert_into_tree(&mut fields, path, field)?;
    }

    Ok(kf::FieldTree { fields: to_krilla_tree(fields) })
}

#[derive(Debug, Default)]
pub(crate) struct AcroForm {
    /// Interactive form fields.
    fields: Vec<Field>,
    /// Mapping between a location and its field name.
    // Note: two typst fields can have the same name and therefore correspond to the same field in
    // PDF, resulting in a many-to-one relationship.
    location_to_fields: FxHashMap<Location, usize>,
    // /// Mapping between a form and all the non-pushbutton fields it contains.
    // // TODO: should we use introspection instead?
    // form_fields: FxHashMap<Location, Vec<EcoString>>,
}

impl AcroForm {
    pub(crate) fn collect_from_document(
        typst_document: &PagedDocument,
    ) -> SourceResult<Self> {
        let introspector = typst_document.introspector();
        let mut seen_names = FxHashMap::default();
        let mut unnamed_field_counter: u32 = 0;

        let mut acro_form = AcroForm::default();

        let forms = introspector.query(&FormElem::ELEM.select());
        for form in forms {
            let form_location = form.location().unwrap();
            let fields = introspector.query(
                &Selector::Or(eco_vec![
                    FormButtonElem::ELEM.select(),
                    FormCheckboxElem::ELEM.select(),
                    FormRadioGroupElem::ELEM.select(),
                    FormTextboxElem::ELEM.select(),
                    FormChoiceElem::ELEM.select(),
                ])
                .within(form_location.into()),
            );

            for field in fields {
                let name_generator = || {
                    unnamed_field_counter += 1;
                    eco_format!("typst.field-{}", unnamed_field_counter)
                };
                let field = Field::new(form_location, field, name_generator)?;
                let id = acro_form.fields.len();
                let prev = seen_names.insert(field.name.clone(), id);
                if let Some(prev_id) = prev {
                    bail!(
                        field.body.span(), "two fields share the same name ({})", field.name.repr();
                        hint: "change the name of one of the fields";
                        hint[acro_form.fields[prev_id].body.span()]: "the other field is here";
                    );
                }

                acro_form
                    .location_to_fields
                    .insert(field.body.location().unwrap(), id);
                acro_form.fields.push(field);
            }
        }
        Ok(acro_form)
    }

    pub(crate) fn get_field_mut(&mut self, location: Location) -> Option<&mut Field> {
        let id = self.location_to_fields.get(&location)?;
        self.fields.get_mut(*id)
    }
}
