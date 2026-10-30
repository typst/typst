use std::sync::Arc;

use typst_library::diag::{At, ExpectInternal, SourceResult, bail};
use typst_library::engine::Engine;
use typst_library::foundations::{NativeElement, Packed, StyleChain};
use typst_library::introspection::{Locator, LocatorLink};
use typst_library::layout::{Frame, FrameItem, Point, Region, Size};
use typst_library::model::{
    ButtonAction, FieldAppearanceBody, FieldAppearanceKind, FieldWidget, FormButton,
    FormCheckbox, FormChoice, FormCursor, FormElem, FormRadioButton, FormRadioGroup,
    FormTextbox, FormWidget, WidgetAction, WidgetState,
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
    let form = FormElem::try_from_styles(elem, &styles)?;

    let action = elem.action.get(styles).map(|action| {
        let form_location = form.location().unwrap();
        match action {
            ButtonAction::Submit => {
                let Some(target) = form.target.get_cloned(styles) else {
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
        layout_appearance_body(engine, None, Some(locator), &link, styles, region)?;

    // Button's size is dictated by the contents of the normal state.
    let size = body.normal.size();

    let widget = FieldWidget {
        span,
        field_location: elem.location().unwrap(),
        size,
        action,
        value: None,
        appearance: Arc::new(FieldAppearanceKind::Single { variable_text: false, body }),
    };

    Ok(create_widget_frame(size, widget))
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
    let link = LocatorLink::measure(elem.location().unwrap(), elem.span());
    let body_off = layout_appearance_body(
        engine,
        Some(false),
        Some(locator),
        &link,
        styles,
        region,
    )?;
    let body_on =
        layout_appearance_body(engine, Some(true), None, &link, styles, region)?;

    let checked = elem.checked.get(styles);
    let widget = FieldWidget {
        span: elem.span(),
        field_location: elem.location().unwrap(),
        size: region.size,
        action: None,
        value: None,
        appearance: Arc::new(FieldAppearanceKind::Dual {
            active: checked,
            body_off,
            body_on,
        }),
    };

    Ok(create_widget_frame(region.size, widget))
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

    let link = LocatorLink::measure(elem.location().unwrap(), span);
    let body_off = layout_appearance_body(
        engine,
        Some(false),
        Some(locator),
        &link,
        styles,
        region,
    )?;
    let body_on =
        layout_appearance_body(engine, Some(true), None, &link, styles, region)?;

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
        appearance: Arc::new(FieldAppearanceKind::Dual {
            active: selected,
            body_off,
            body_on,
        }),
    };

    Ok(create_widget_frame(region.size, widget))
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
    layout_widget(engine, None, None, locator, styles, region)
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

    let inner_frame = layout_widget(engine, None, None, locator, styles, region)?;

    let widget = FieldWidget {
        span,
        field_location: elem.location().unwrap(),
        size: region.size,
        action: None,
        value: None,
        appearance: Arc::new(FieldAppearanceKind::Single {
            variable_text: true,
            body: FieldAppearanceBody::new(inner_frame),
        }),
    };

    Ok(create_widget_frame(region.size, widget))
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
    let Some(textbox) = styles
        .get_ref(FormWidget::parent)
        .as_ref()
        .and_then(|elem| elem.to_packed::<FormTextbox>())
    else {
        bail!(
            elem.span(), "this cursor element must be inside a textbox element";
            hint: "you can use it in a show rule such as \
```
#show form.textbox: {{
   show form.widget: form.cursor()
   it
}}
```"
        )
    };

    let body = TextElem::new(textbox.value.get_cloned(styles).unwrap_or_default()).pack();
    let frame = crate::layout_frame(engine, &body, locator, styles, region)?;

    let widget = FieldWidget {
        span: textbox.span(),
        field_location: textbox.location().unwrap(),
        size: region.size,
        action: None,
        value: None,
        appearance: Arc::new(FieldAppearanceKind::Single {
            variable_text: true,
            body: FieldAppearanceBody::new(frame),
        }),
    };

    Ok(create_widget_frame(region.size, widget))
}

/// Layout a form field as a [`FormWidget`] with the given interactive state (i.e.,
/// checked/hovered).
fn layout_widget(
    engine: &mut Engine,
    checked: Option<bool>,
    state: Option<WidgetState>,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let content = FormWidget::new().with_checked(checked).with_state(state).pack();
    crate::layout_frame(engine, &content, locator, styles, region)
}

/// Layout a field appearance body with all three states.
///
/// If `locator` is [`Some`], it is used for the normal state.
/// Otherwise, the measurement locator is used just like for the other states.
fn layout_appearance_body(
    engine: &mut Engine,
    checked: Option<bool>,
    locator: Option<Locator>,
    link: &LocatorLink,
    styles: StyleChain,
    region: Region,
) -> SourceResult<FieldAppearanceBody<Frame>> {
    let normal = layout_widget(
        engine,
        checked,
        Some(WidgetState::Normal),
        locator.unwrap_or_else(|| Locator::link(link)),
        styles,
        region,
    )?;
    let hover = layout_widget(
        engine,
        checked,
        Some(WidgetState::Hovered),
        Locator::link(link),
        styles,
        region,
    )?;
    let down = layout_widget(
        engine,
        checked,
        Some(WidgetState::Down),
        Locator::link(link),
        styles,
        region,
    )?;
    Ok(FieldAppearanceBody {
        hover: Some(hover).filter(|f| f != &normal),
        down: Some(down).filter(|f| f != &normal),
        normal,
    })
}

/// Create a clipped soft frame that contains a single form field widget.
fn create_widget_frame(size: Size, widget: FieldWidget) -> Frame {
    let mut frame = Frame::soft(size);
    frame.push(Point::zero(), FrameItem::FormField(widget));
    frame.clip(Curve::rect(frame.size()));
    frame
}
