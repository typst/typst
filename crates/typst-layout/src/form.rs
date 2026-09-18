use typst_library::diag::{At, ExpectInternal, SourceResult, bail};
use typst_library::engine::Engine;
use typst_library::foundations::{
    FromValue, IntoValue, NativeElement, Packed, StyleChain,
};
use typst_library::introspection::{Locator, SplitLocator};
use typst_library::layout::{
    BaselinePos, BlockElem, BoxElem, Em, Frame, FrameItem, Length, Point, Region, Rel,
};
use typst_library::model::{
    ButtonAction, FieldAppearanceBody, FieldAppearanceKind, FieldWidget, FormButton,
    FormCheckbox, FormChoice, FormCursor, FormElem, FormRadioButton, FormTextbox,
    FormWidget, WidgetAction,
};
use typst_library::text::TextElem;
use typst_library::visualize::Curve;

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
    let Some(form) = styles.get_ref(FormElem::form) else {
        bail!(span, "button must be placed inside a `#form[...]` element");
    };

    let action = elem.action.get(styles).map(|action| {
        let form_location = form.location().unwrap();
        match action {
            ButtonAction::Submit => {
                let Some(target) = form.target.get_cloned(styles) else {
                    bail!(
                        span, "button has submit action, but no submission target is defined for its form";
                        hint: "set the `target` property of the surrounding `#form` element";
                    );
                };
                Ok(WidgetAction::Submit { form: form_location, target })
            },
            ButtonAction::Reset => Ok(WidgetAction::Reset { form: form_location}),
        }
    }).transpose()?;

    let mut locator = locator.split();
    let frame = layout_widget(engine, None, Some(false), &mut locator, styles, region)?;
    let frame_hover =
        layout_widget(engine, None, Some(true), &mut locator, styles, region)?;

    // Button's size is dictated by the contents of the base state.
    let size = frame.size();

    let widget = FieldWidget {
        span,
        field_location: elem.location().unwrap(),
        size,
        action,
        value: None,
        appearance: FieldAppearanceKind::Single {
            variable_text: false,
            body: FieldAppearanceBody {
                normal: frame,
                hover: Some(frame_hover).filter(|f| !f.is_empty()),
            },
        },
    };

    let mut frame = Frame::soft(size);
    frame.push(Point::zero(), FrameItem::FormField(widget));
    frame.clip(Curve::rect(frame.size()));

    Ok(frame)
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
    let mut locator = locator.split();
    let frame_off =
        layout_widget(engine, Some(false), Some(false), &mut locator, styles, region)?;
    let frame_off_hover =
        layout_widget(engine, Some(false), Some(true), &mut locator, styles, region)?;
    let frame_on =
        layout_widget(engine, Some(true), Some(false), &mut locator, styles, region)?;
    let frame_on_hover =
        layout_widget(engine, Some(true), Some(true), &mut locator, styles, region)?;

    let checked = elem.checked.get(styles);
    let widget = FieldWidget {
        span: elem.span(),
        field_location: elem.location().unwrap(),
        size: region.size,
        action: None,
        value: None,
        appearance: FieldAppearanceKind::Dual {
            active: checked,
            body_off: FieldAppearanceBody {
                normal: frame_off,
                hover: Some(frame_off_hover).filter(|f| !f.is_empty()),
            },
            body_on: FieldAppearanceBody {
                normal: frame_on,
                hover: Some(frame_on_hover).filter(|f| !f.is_empty()),
            },
        },
    };

    let mut frame = Frame::soft(region.size);
    frame.push(Point::zero(), FrameItem::FormField(widget));
    frame.clip(Curve::rect(frame.size()));

    Ok(frame)
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
    let Some(group) = styles.get_ref(FormRadioButton::parent) else {
        bail!(
            span, "a radio button must appear inside a radio group";
            hint: "try surrounding this button with `#form.radio-group[...]`";
        )
    };

    let mut locator = locator.split();
    let frame_off =
        layout_widget(engine, Some(false), Some(false), &mut locator, styles, region)?;
    let frame_off_hover =
        layout_widget(engine, Some(false), Some(true), &mut locator, styles, region)?;
    let frame_on =
        layout_widget(engine, Some(true), Some(false), &mut locator, styles, region)?;
    let frame_on_hover =
        layout_widget(engine, Some(true), Some(true), &mut locator, styles, region)?;

    let value = elem
        .resolved_value
        .clone()
        .expect_internal("radio button's resolved value is unset")
        .at(span)?;
    let selected = group.selected.get_ref(styles) == &elem.label();
    let widget = FieldWidget {
        span,
        field_location: group.location().unwrap(),
        size: region.size,
        action: None,
        value: Some(value),
        appearance: FieldAppearanceKind::Dual {
            active: selected,
            body_off: FieldAppearanceBody {
                normal: frame_off,
                hover: Some(frame_off_hover).filter(|f| !f.is_empty()),
            },
            body_on: FieldAppearanceBody {
                normal: frame_on,
                hover: Some(frame_on_hover).filter(|f| !f.is_empty()),
            },
        },
    };

    let mut frame = Frame::soft(region.size);
    frame.push(Point::zero(), FrameItem::FormField(widget));
    frame.clip(Curve::rect(frame.size()));

    Ok(frame)
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
    let mut locator = locator.split();
    layout_widget(engine, None, None, &mut locator, styles, region)
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
    let span = elem.span();

    let options = elem.options.get_ref(styles);
    let value = elem.value.get_ref(styles);
    let multiple = elem.multiple.get(styles);

    FormChoice::validate(span, multiple, options, value)?;

    let mut locator = locator.split();
    let inner_frame = layout_widget(engine, None, None, &mut locator, styles, region)?;

    let widget = FieldWidget {
        span,
        field_location: elem.location().unwrap(),
        size: region.size,
        action: None,
        value: None,
        appearance: FieldAppearanceKind::Single {
            variable_text: true,
            body: FieldAppearanceBody { normal: inner_frame, hover: None },
        },
    };

    let mut frame = Frame::soft(region.size);
    frame.push(Point::zero(), FrameItem::FormField(widget));
    frame.clip(Curve::rect(frame.size()));

    Ok(frame)
}

/// Layout a form cursor (i.e., the variable text).
#[typst_macros::time(span = elem.span())]
pub fn layout_cursor(
    elem: &Packed<FormCursor>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let textbox = styles
        .get_ref(FormWidget::parent)
        .as_ref()
        .and_then(|elem| elem.to_packed::<FormTextbox>())
        .expect_internal("form cursor must have a textbox parent")
        .at(elem.span())?;

    let body = TextElem::new(textbox.value.get_cloned(styles).unwrap_or_default()).pack();
    let frame = crate::layout_frame(engine, &body, locator, styles, region)?;

    let widget = FieldWidget {
        span: textbox.span(),
        field_location: textbox.location().unwrap(),
        size: region.size,
        action: None,
        value: None,
        appearance: FieldAppearanceKind::Single {
            variable_text: true,
            body: FieldAppearanceBody { normal: frame, hover: None },
        },
    };

    let mut frame = Frame::soft(region.size);
    frame.push(Point::zero(), FrameItem::FormField(widget));
    frame.clip(Curve::rect(frame.size()));

    Ok(frame)
}

/// Similar to [`BlockElem::single_layouter`], but wraps the [`BlockElem`]
/// in a [`BoxElem`] so that form fields can be placed inline.
pub fn single_layouter<T: NativeElement>(
    captured: Packed<T>,
    f: fn(
        content: &Packed<T>,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        region: Region,
    ) -> SourceResult<Frame>,
) -> BoxElem {
    let inner = BlockElem::single_layouter(captured, f).pack();

    BoxElem::new()
        .with_baseline(
            // TODO: impl From<...> for BaselinePos
            BaselinePos::from_value(Rel::from(Length::from(Em::new(0.2))).into_value())
                .unwrap(),
        )
        .with_body(Some(inner))
}

/// Layout a form field as a [`FormWidget`] with the given interactive state (i.e.,
/// checked/hovered).
fn layout_widget(
    engine: &mut Engine,
    checked: Option<bool>,
    hovered: Option<bool>,
    locator: &mut SplitLocator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let content = FormWidget::new().with_checked(checked).with_hovered(hovered).pack();

    crate::layout_frame(
        engine,
        &content,
        locator.next(&(checked, hovered)),
        styles,
        region,
    )
}
