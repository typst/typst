use ecow::EcoString;
use krilla::action::Action;
use krilla::action::ResetFormAction;
use krilla::form as kf;
use krilla::geom as kg;
use krilla::page::Page;
use krilla::stream::Stream;
use krilla::surface::Surface;
use krilla::tagging::Identifier;
use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;
use std::iter::Peekable;
use typst_library::diag::{At, ExpectInternal, SourceResult, bail};
use typst_library::introspection::Location;
use typst_library::layout::{Abs, Frame, Point, Sides, Size};
use typst_library::model::WidgetAction;
use typst_library::model::{
    FieldAppearance, FieldAppearanceKind, FormField, FormFieldKind,
};
use typst_syntax::Span;

use crate::convert::{FrameContext, GlobalContext, handle_frame};
use crate::tags::{self, AnnotationId};
use crate::util::PointExt;

pub(crate) struct Field {
    pub name: EcoString,
    pub span: Span,
    pub kind: FormFieldKind,
    pub krilla_field: kf::FieldKind,
}

impl Field {
    pub(crate) fn new(name: EcoString, span: Span, field_kind: FormFieldKind) -> Self {
        let partial_name = name.rsplit('.').next().unwrap().to_string();
        let krilla_field = match &field_kind {
            FormFieldKind::Button => kf::FormField::push_button(partial_name).into(),
            FormFieldKind::Checkbox(checkbox_field) => {
                kf::FormField::checkbox(partial_name, checkbox_field.checked)
                    .with_required(checkbox_field.required)
                    .with_read_only(checkbox_field.read_only)
                    .into()
            }
            FormFieldKind::Radio(radio_field) => kf::FormField::radio(
                partial_name,
                radio_field.selected.as_ref().map(|s| s.to_string()),
            )
            .with_required(radio_field.required)
            .with_read_only(radio_field.read_only)
            .into(),
            FormFieldKind::Text(text_field) => {
                let mut field = kf::FormField::text(partial_name)
                    .with_mutliline(text_field.multiline)
                    .with_required(text_field.required)
                    .with_read_only(text_field.read_only)
                    // TODO: checked cast?
                    .with_max_length(text_field.max_length.map(|i| i as i32));
                // TODO spellchecker

                if let Some(value) = &text_field.value {
                    field.set_value(value.to_string());
                }
                field.into()
            }
            FormFieldKind::Choice(choice_field) => {
                let options = choice_field
                    .options
                    .iter()
                    .map(|option| {
                        kf::kind::ChoiceOption::new(
                            option.mapping_name.to_string(),
                            option.display_name.as_ref().map(|s| s.to_string()),
                        )
                    })
                    .collect();
                if choice_field.multiple {
                    let value =
                        choice_field.value.iter().map(|s| s.to_string()).collect();
                    kf::FormField::listbox(partial_name)
                        .with_value(value)
                        .with_options(options)
                        .with_multiple_options(true)
                        .with_required(choice_field.required)
                        .with_read_only(choice_field.read_only)
                        .into()
                } else {
                    let mut field = kf::FormField::combobox(partial_name)
                        .with_options(options)
                        .with_required(choice_field.required)
                        .with_read_only(choice_field.read_only);
                    if let Some(value) = choice_field.value.first() {
                        field.set_value(value.to_string());
                    }
                    field.into()
                }
            }
        };

        Self { name, span, kind: field_kind, krilla_field }
    }

    pub(crate) fn insert_annotation(
        &mut self,
        page: &mut Page,
        annotation: WidgetAnnotation,
        action: Option<Action>,
    ) -> Identifier {
        match &mut self.krilla_field {
            kf::FieldKind::PushButton(form_field) => {
                let mut widget = form_field.new_widget(
                    annotation.bbox,
                    annotation.off_stream.expect("button has no appearance"),
                );
                if let Some(action) = action {
                    widget.set_action_mouse_press(action);
                }
                page.add_widget_annotation(form_field, widget.into())
            }
            kf::FieldKind::Checkbox(form_field) => {
                let widget = form_field.new_widget(
                    annotation.bbox,
                    annotation.off_stream.expect("checkbox has no off appearance"),
                    annotation.on_stream.expect("checkbox has no on appearance"),
                );

                page.add_widget_annotation(form_field, widget.into())
            }
            kf::FieldKind::Radio(form_field) => {
                let widget = form_field.new_widget(
                    annotation.bbox,
                    annotation.value.expect("radio button has no value").to_string(),
                    annotation.off_stream.expect("radio button has no off appearance"),
                    annotation.on_stream.expect("radio button has no on appearance"),
                );

                page.add_widget_annotation(form_field, widget.into())
            }
            kf::FieldKind::Text(form_field) => {
                let widget = form_field.new_widget(
                    annotation.bbox,
                    annotation.off_stream.expect("text has no appearance"),
                );

                page.add_widget_annotation(form_field, widget.into())
            }
            kf::FieldKind::ListBox(form_field) => {
                let widget = form_field.new_widget(
                    annotation.bbox,
                    annotation.off_stream.expect("listbox has no appearance"),
                );

                page.add_widget_annotation(form_field, widget.into())
            }
            kf::FieldKind::ComboBox(form_field) => {
                let widget = form_field.new_widget(
                    annotation.bbox,
                    annotation.off_stream.expect("combobox has no appearance"),
                );

                page.add_widget_annotation(form_field, widget.into())
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct WidgetAnnotation {
    pub annotation_id: Option<AnnotationId>,
    pub field_location: Location,
    pub bbox: kg::Rect,
    pub value: Option<EcoString>,
    pub on_stream: Option<Stream>,
    pub off_stream: Option<Stream>,
    pub action: Option<WidgetAction>,
}

impl WidgetAnnotation {
    pub fn new(field_location: Location, bbox: kg::Rect) -> Self {
        Self {
            annotation_id: None,
            field_location,
            bbox,
            value: None,
            on_stream: None,
            off_stream: None,
            action: None,
        }
    }

    pub fn get_krilla_action(&self, gc: &mut GlobalContext) -> Option<Action> {
        self.action.as_ref().map(|action| match action {
            WidgetAction::Submit { .. } => todo!(),
            WidgetAction::Reset { form } => {
                let fields = gc
                    .form_fields
                    .get(form)
                    .map(|v| v.iter().map(|s| s.to_string()).collect())
                    .unwrap_or_default();
                ResetFormAction::Include(fields).into()
            }
        })
    }
}

pub(crate) fn handle_field_appearance(
    fc: &mut FrameContext,
    gc: &mut GlobalContext,
    surface: &mut Surface,
    appearance: &FieldAppearance,
    body: &Frame,
) -> SourceResult<()> {
    let rect = bounding_box(fc, body.size());

    let stream = {
        let is_variable_text = appearance.kind == FieldAppearanceKind::VariableText;
        let mut builder = surface.stream_builder();
        let bbox = kg::Rect::from_xywh(0.0, 0.0, rect.width(), rect.height()).unwrap();
        let mut surface = builder.surface_with_bbox(bbox);

        if is_variable_text {
            surface.start_variable_text();
        }

        let mut fc = FrameContext::new(None, body.size());
        handle_frame(&mut fc, body, Sides::splat(Abs::zero()), None, &mut surface, gc)?;

        if is_variable_text {
            surface.end_variable_text();
        }

        surface.finish();
        builder.finish()
    };

    let tag_group = if tags::disabled(gc) {
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
        let (group_id, form_field) = gc
            .tags
            .tree
            .parent_form_field()
            .expect_internal("expected form field ancestor in logical tree")
            .at(Span::detached())?;

        if gc.tags.tree.parent_artifact().is_some() {
            if let Some(accessibility) = gc.options.validators().accessibility() {
                let validator = accessibility.as_str();
                bail!(
                    form_field.span(),
                    "{validator} error: PDF artifacts may not contain form fields";
                );
            }

            None
        } else {
            Some((group_id, form_field))
        }
    };

    let widget = fc.get_widget_annotation_mut(
        appearance.widget_location,
        appearance.field_location,
        rect,
    );
    match appearance.kind {
        FieldAppearanceKind::Single
        | FieldAppearanceKind::VariableText
        | FieldAppearanceKind::Off(_) => {
            debug_assert!(widget.off_stream.is_none());
            widget.off_stream = Some(stream);

            if let Some((group_id, _)) = tag_group {
                let annot_id = gc.tags.annotations.reserve();
                widget.annotation_id = Some(annot_id);
                let group = gc.tags.tree.groups.get_mut(group_id);
                group.push_annotation(annot_id);
            }
        }
        FieldAppearanceKind::On(_) => {
            debug_assert!(widget.on_stream.is_none());
            widget.on_stream = Some(stream);
        }
    }
    widget.action = appearance.action.clone();
    if appearance.value.is_some() {
        debug_assert!(widget.value.is_none() || widget.value == appearance.value);
        widget.value = appearance.value.clone();
    }

    Ok(())
}

pub(crate) fn handle_form_field(
    gc: &mut GlobalContext,
    form_field: &FormField,
) -> SourceResult<()> {
    let name = form_field.name.clone().unwrap_or_else(|| {
        EcoString::from(format!("typst.field-{}", gc.get_next_field_number()))
    });
    gc.location_to_fields.insert(form_field.location, name.clone());
    gc.form_fields.entry(form_field.form).or_default().push(name.clone());
    match gc.fields.entry(name.clone()) {
        Entry::Occupied(existing) => {
            if form_field.kind != existing.get().kind {
                bail!(
                    form_field.span, "two fields share the same name with conflicting properties";
                    hint: "change the name of one of the fields";
                    hint: "alternatively, ensure the properties of both are the same";
                    hint[existing.get().span]: "the other field is here";
                );
            }
        }
        Entry::Vacant(vacant_entry) => {
            vacant_entry.insert(Field::new(
                name,
                form_field.span,
                form_field.kind.clone(),
            ));
        }
    }

    Ok(())
}

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

#[allow(clippy::large_enum_variant)]
enum Node {
    Group(FieldGroup),
    Leaf(Field),
}

pub(crate) fn build_field_tree(gc: &mut GlobalContext) -> SourceResult<kf::FieldTree> {
    fn insert_into_tree<'a, I>(
        fields: &mut FieldGroup,
        mut path: Peekable<I>,
        field: Field,
    ) -> SourceResult<()>
    where
        I: Iterator<Item = &'a str>,
    {
        let path_segment =
            path.next().expect("there is always one path segment when splitting");
        let is_leaf = path.peek().is_none();

        if is_leaf {
            let name = field.name.clone();
            let span = field.span;
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
                        field.span, "there are two distinct form fields named `{name}`";
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
    for field in std::mem::take(&mut gc.fields).into_values() {
        let name = field.name.clone();
        let path = name.split('.').peekable();
        insert_into_tree(&mut fields, path, field)?;
    }

    Ok(kf::FieldTree { fields: to_krilla_tree(fields) })
}
