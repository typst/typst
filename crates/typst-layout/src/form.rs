use std::sync::Arc;

use comemo::Tracked;
use ecow::EcoString;
use typst_library::World;
use typst_library::diag::{At, ExpectInternal, SourceResult, bail};
use typst_library::engine::Engine;
use typst_library::foundations::{NativeElement, Packed, Resolve, StyleChain};
use typst_library::introspection::{Location, Locator, LocatorLink};
use typst_library::layout::{Abs, AlignElem, Em, Frame, FrameItem, Point, Region, Size};
use typst_library::model::{
    ButtonAction, FieldAppearanceBody, FieldAppearanceKind, FieldWidget, FontType,
    FormButton, FormCheckbox, FormChoice, FormElem, FormRadioButton, FormRadioGroup,
    FormSignature, FormTextbox, FormWidget, TextboxKind, VariableText, WidgetAction,
    WidgetState,
};
use typst_library::text::{
    FontFlags, FontStyle, FontWeight, TextElem, families, variant,
};
use typst_library::visualize::{Curve, Geometry, RectElem};
use typst_syntax::Span;

/// Layout the button.
#[typst_macros::time(span = elem.span())]
pub fn layout_button(
    elem: &Packed<FormButton>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let span = elem.span();
    let form = FormElem::try_from_styles(elem, &styles)?;

    let action = elem.action.get(styles).map(|action| {
        let form_location = form.location().unwrap();
        match action {
            ButtonAction::Submit => {
                let Some(target) = form.target.get_cloned(styles).transpose() else {
                    bail!(
                        span, "button has a submit action, but no submission target is defined for its form";
                        hint[form.span()]: "set the `target` property of the surrounding `#form` element";
                    );
                };
                Ok(WidgetAction::Submit { form: form_location, target })
            },
            ButtonAction::Reset => Ok(WidgetAction::Reset { form: form_location}),
        }
    }).transpose()?;

    let link = LocatorLink::measure(elem.location().unwrap(), span);
    let body =
        layout_appearance_body(engine, span, None, Some(locator), &link, styles, region)?;

    // Button's size and baseline are dictated by the contents of the normal state.
    let size = region.expand.select(region.size, body.normal.size());
    let baseline = body.normal.baseline();

    let widget = FieldWidget {
        span,
        field_location: elem.location().unwrap(),
        size,
        action,
        value: None,
        appearance: Arc::new(FieldAppearanceKind::Single { variable_text: None, body }),
    };

    Ok(create_widget_frame(size, widget, baseline))
}

/// Layout the checkbox.
#[typst_macros::time(span = elem.span())]
pub fn layout_checkbox(
    elem: &Packed<FormCheckbox>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let location = elem.location().unwrap();
    layout_dual_appearance(
        engine,
        locator,
        styles,
        region,
        None,
        elem.checked.get(styles),
        location,
        location,
        elem.span(),
    )
}

/// Layout the radio button.
#[typst_macros::time(span = elem.span())]
pub fn layout_radio_button(
    elem: &Packed<FormRadioButton>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let span = elem.span();
    let group = FormRadioGroup::try_from_styles(elem, &styles)?;

    let value = elem
        .resolved_value
        .clone()
        .expect_internal("radio button's resolved value is unset")
        .at(span)?;
    let selected_label = group.selected.get_ref(styles);
    let selected = selected_label.is_some() && selected_label == &elem.label();

    layout_dual_appearance(
        engine,
        locator,
        styles,
        region,
        Some(value),
        selected,
        elem.location().unwrap(),
        group.location().unwrap(),
        span,
    )
}

/// Layout the textbox.
#[typst_macros::time(span = elem.span())]
pub fn layout_textbox(
    elem: &Packed<FormTextbox>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let span = elem.span();
    let mut frame = layout_single_appearance(
        engine,
        locator,
        styles,
        region,
        true,
        elem.location().unwrap(),
        span,
    )?;

    // Only draw fixed-width grid if rect stroke is uniform (the default of
    // FormTextbox's show-set rule).
    if elem.kind.get(styles) == TextboxKind::FixedWidth
        && let Some(stroke) = styles
            .get_cloned(RectElem::stroke)
            .custom()
            .filter(|sides| sides.is_uniform())
            .and_then(|sides| sides.left)
            .flatten()
    {
        let stroke = stroke.resolve(styles).unwrap_or_default();
        let max_length = elem.max_length.get(styles).unwrap_or_default().max(1);
        let step_width = frame.size().x / max_length as f64;
        let height = frame.size().y;

        frame.prepend_multiple((1..max_length).map(|i| {
            let start = Point::new(step_width * i as f64, Abs::zero());
            let delta = Point::new(Abs::zero(), height);
            let line = Geometry::Line(delta).stroked(stroke.clone());

            let item = FrameItem::Shape(line, span);
            (start, item)
        }));
    }

    Ok(frame)
}

/// Layout the choice field.
#[typst_macros::time(span = elem.span())]
pub fn layout_choice(
    elem: &Packed<FormChoice>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    layout_single_appearance(
        engine,
        locator,
        styles,
        region,
        true,
        elem.location().unwrap(),
        elem.span(),
    )
}

/// Layout the signature field.
#[typst_macros::time(span = elem.span())]
pub fn layout_signature(
    elem: &Packed<FormSignature>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    layout_single_appearance(
        engine,
        locator,
        styles,
        region,
        false,
        elem.location().unwrap(),
        elem.span(),
    )
}

// Layout a single field appearance, without any hover or down states.
fn layout_single_appearance(
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
    has_variable_text: bool,
    location: Location,
    span: Span,
) -> SourceResult<Frame> {
    let variable_text =
        has_variable_text.then(|| get_variable_text(&engine.world, styles));
    let cap_height = variable_text
        .as_ref()
        .map(|vt| get_cap_height(vt.font_type, vt.font_bold, vt.font_italic));

    let inner_frame =
        layout_widget(engine, span, None, None, locator, styles, region, cap_height)?;
    let size = region.expand.select(region.size, inner_frame.size());
    let baseline = inner_frame.baseline();

    let widget = FieldWidget {
        span,
        field_location: location,
        size,
        action: None,
        value: None,
        appearance: Arc::new(FieldAppearanceKind::Single {
            variable_text,
            body: FieldAppearanceBody::new(inner_frame),
        }),
    };

    Ok(create_widget_frame(size, widget, baseline))
}

// Layout a dual field appearance, with hover and down states.
#[allow(clippy::too_many_arguments)]
fn layout_dual_appearance(
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
    value: Option<EcoString>,
    active: bool,
    widget_location: Location,
    field_location: Location,
    span: Span,
) -> SourceResult<Frame> {
    let link = LocatorLink::measure(widget_location, span);
    let body_off = layout_appearance_body(
        engine,
        span,
        Some(false),
        Some(locator),
        &link,
        styles,
        region,
    )?;
    let body_on =
        layout_appearance_body(engine, span, Some(true), None, &link, styles, region)?;

    // Frame size and baseline are dictated by the contents of the off normal state.
    // Note: region.expand is always true for checkboxes/radio, but this is here for correctness.
    let size = region.expand.select(region.size, body_off.normal.size());
    let baseline = body_off.normal.baseline();

    let widget = FieldWidget {
        span,
        field_location,
        size,
        action: None,
        value,
        appearance: Arc::new(FieldAppearanceKind::Dual { active, body_off, body_on }),
    };

    Ok(create_widget_frame(size, widget, baseline))
}

/// Layout a form field as a [`FormWidget`] with the given interactive state
/// (i.e., checked/hovered).
#[allow(clippy::too_many_arguments)]
fn layout_widget(
    engine: &mut Engine,
    span: Span,
    checked: Option<bool>,
    state: Option<WidgetState>,
    locator: Locator,
    styles: StyleChain,
    region: Region,
    font_cap_height: Option<Em>,
) -> SourceResult<Frame> {
    let content = FormWidget::new()
        .with_checked(checked)
        .with_state(state)
        .pack()
        .spanned(span)
        .set(FormWidget::font_cap_height, font_cap_height);
    crate::layout_frame(engine, &content, locator, styles, region)
}

/// Layout a field appearance body with all three states.
///
/// If `locator` is [`Some`], it is used for the normal state.
/// Otherwise, the measurement locator is used just like for the other states.
fn layout_appearance_body(
    engine: &mut Engine,
    span: Span,
    checked: Option<bool>,
    locator: Option<Locator>,
    link: &LocatorLink,
    styles: StyleChain,
    region: Region,
) -> SourceResult<FieldAppearanceBody<Frame>> {
    let normal = layout_widget(
        engine,
        span,
        checked,
        Some(WidgetState::Normal),
        locator.unwrap_or_else(|| Locator::link(link)),
        styles,
        region,
        None,
    )?;
    let hover = layout_widget(
        engine,
        span,
        checked,
        Some(WidgetState::Hovered),
        Locator::link(link),
        styles,
        region,
        None,
    )?;
    let down = layout_widget(
        engine,
        span,
        checked,
        Some(WidgetState::Down),
        Locator::link(link),
        styles,
        region,
        None,
    )?;

    // In Pdfium, we must either have only normal, or all three.
    // If we only have, for example, normal and hover, pdfium will ignore the
    // hover appearance.
    let emit_interactive_appearances = hover != normal || down != normal;

    Ok(FieldAppearanceBody {
        normal,
        hover: emit_interactive_appearances.then_some(hover),
        down: emit_interactive_appearances.then_some(down),
    })
}

/// Create a clipped soft frame that contains a single form field widget.
fn create_widget_frame(size: Size, widget: FieldWidget, baseline: Abs) -> Frame {
    let mut frame = Frame::soft(size);
    frame.push(Point::zero(), FrameItem::FormField(widget));
    frame.clip(Curve::rect(frame.size()));
    frame.set_baseline(baseline);
    frame
}

fn get_variable_text<'a>(
    world: &Tracked<'a, dyn World + 'a>,
    styles: StyleChain,
) -> VariableText {
    // TODO: we might want to look through recipes for FormWidget in the
    // StyleChain so that `#show form.widget: set align(center)` also works.
    // Currently we have to do `#show form.textbox: set align(center)` which
    // aligns the whole textbox and not just the text.

    // Detect whether the current font is sans-serif, serif, or monospace,
    // along with whether bold and/or italic is set.
    let variant = variant(styles);
    let font_type = families(styles)
        .find_map(|family| {
            world
                .book()
                .select(family.as_str(), variant)
                .and_then(|id| world.book().info(id))
                .map(|info| info.flags)
        })
        .map_or(FontType::Serif, |flags| {
            if flags.contains(FontFlags::MONOSPACE) {
                FontType::Mono
            } else if flags.contains(FontFlags::SERIF) {
                FontType::Serif
            } else {
                FontType::Sans
            }
        });
    let fill_paint = styles.get_cloned(TextElem::fill);
    let font_size = styles.resolve(TextElem::size);
    let alignment = styles.resolve(AlignElem::alignment).x;

    VariableText {
        font_type,
        font_bold: variant.weight >= FontWeight::SEMIBOLD,
        font_italic: matches!(variant.style, FontStyle::Italic | FontStyle::Oblique),
        font_size,
        fill_color: fill_paint,
        alignment,
    }
}

/// Get the `CapHeight` of the PDF standard font that matches the given
/// characteristics, i.e., one of Helvetica, Times New Roman, or Courier,
/// and their variants.
fn get_cap_height(font_type: FontType, is_bold: bool, is_italic: bool) -> Em {
    let em = match (font_type, is_bold, is_italic) {
        (FontType::Sans, _, _) => 0.718,
        (FontType::Serif, false, false) => 0.662,
        (FontType::Serif, false, true) => 0.653,
        (FontType::Serif, true, false) => 0.676,
        (FontType::Serif, true, true) => 0.669,
        (FontType::Mono, _, _) => 0.562,
    };
    Em::new(em)
}
