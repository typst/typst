use typst_library::diag::{SourceResult, bail};
use typst_library::engine::Engine;
use typst_library::foundations::{
    FromValue, IntoValue, NativeElement, Packed, Smart, StyleChain, SymbolElem,
};
use typst_library::introspection::Locator;
use typst_library::layout::{
    Abs, AlignElem, BaselinePos, BlockBody, BlockElem, BoxElem, Em, Frame, FrameItem,
    GroupItem, HAlignment, Length, Point, Ratio, Region, Rel, Sides, Sizing, StackChild,
    StackElem, VAlignment,
};
use typst_library::model::{
    ButtonAction, FieldAppearanceKind, FieldWidget, FormButtonElem, FormCheckboxElem,
    FormChoiceElem, FormElem, FormRadioElem, FormRadioGroupElem, FormTextboxElem,
    WidgetAction,
};
use typst_library::text::TextElem;
use typst_library::visualize::{CircleElem, Color, Curve, RectElem};

/// Layout the button.
#[typst_macros::time(span = elem.span())]
pub fn layout_button(
    elem: &Packed<FormButtonElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let span = elem.span();
    let Some(form) = styles.get_ref(FormElem::form) else {
        bail!(span, "button must be placed inside a `#form[...]` element");
    };

    // TODO: use introspection in typst_pdf for this
    let action = elem.action.get(styles).map(|action| {
        match action {
            ButtonAction::Submit => {
                let Some(target) = form.target.clone() else {
                    bail!(
                        span, "button has submit action, but no submission target is defined for its form";
                        hint: "set the `target` property of the surrounding `#form` element";
                    );
                };
                Ok(WidgetAction::Submit { form: form.location, target })
            },
            ButtonAction::Reset => Ok(WidgetAction::Reset { form: form.location }),
        }
    }).transpose()?;

    let inner_frame = crate::layout_frame(engine, &elem.body, locator, styles, region)?;

    let widget = FieldWidget {
        field_location: elem.location().unwrap(),
        size: region.size,
        action,
        value: None,
        appearance: FieldAppearanceKind::Single {
            variable_text: false,
            body: inner_frame,
        },
    };

    let mut frame = Frame::soft(region.size);
    frame.push(Point::zero(), FrameItem::FieldAppearance(widget));
    frame.clip(Curve::rect(frame.size()));

    Ok(frame)
}

/// Layout the checkbox.
#[typst_macros::time(span = elem.span())]
pub fn layout_checkbox(
    elem: &Packed<FormCheckboxElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let inner_width = Smart::Custom(Ratio::one().into());
    let inner_height = Sizing::Rel(Ratio::one().into());
    let off_content = {
        RectElem::new()
            .with_width(inner_width)
            .with_height(inner_height)
            .with_outset(Sides::splat(Some(Abs::pt(-0.5).into())))
            .pack()
    };
    let on_content = {
        let checkmark = AlignElem::new(SymbolElem::packed("✓"))
            .with_alignment(HAlignment::Center + VAlignment::Horizon)
            .pack();
        RectElem::new()
            .with_width(inner_width)
            .with_height(inner_height)
            .with_outset(Sides::splat(Some(Abs::pt(-0.5).into())))
            .with_inset(Sides::splat(Some(Rel::zero())))
            .with_body(Some(checkmark))
            .pack()
    };
    let mut locator = locator.split();
    let off_frame =
        crate::layout_frame(engine, &off_content, locator.next(&()), styles, region)?;
    let on_frame =
        crate::layout_frame(engine, &on_content, locator.next(&()), styles, region)?;

    let checked = elem.checked.get(styles);
    let widget = FieldWidget {
        field_location: elem.location().unwrap(),
        size: region.size,
        action: None,
        value: None,
        appearance: FieldAppearanceKind::Dual {
            active: checked,
            body_off: off_frame,
            body_on: on_frame,
        },
    };

    let mut frame = Frame::soft(region.size);
    frame.push(Point::zero(), FrameItem::FieldAppearance(widget));
    frame.clip(Curve::rect(frame.size()));

    Ok(frame)
}

/// Layout the radio button.
#[typst_macros::time(span = elem.span())]
pub fn layout_radio_button(
    elem: &Packed<FormRadioElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let span = elem.span();
    let Some(group) = styles.get_ref(FormRadioGroupElem::radio_group) else {
        bail!(
            span, "a radio button must appear inside a radio group";
            hint: "try surrounding this button with `#form.radio-group[...]`";
        )
    };

    let inner_width = Smart::Custom(Ratio::one().into());
    let inner_height = Sizing::Rel(Ratio::one().into());
    let off_content = {
        CircleElem::new()
            .with_width(inner_width)
            .with_height(inner_height)
            .with_outset(Sides::splat(Some(Abs::pt(-0.5).into())))
            .pack()
    };
    let on_content = {
        let inner_circle = AlignElem::new(
            CircleElem::new()
                .with_width(inner_width)
                .with_height(inner_height)
                .with_fill(Some(Color::BLACK.into()))
                .pack(),
        )
        .with_alignment(HAlignment::Center + VAlignment::Horizon)
        .pack();
        CircleElem::new()
            .with_width(inner_width)
            .with_height(inner_height)
            .with_outset(Sides::splat(Some(Abs::pt(-0.5).into())))
            .with_inset(Sides::splat(Some(Rel::zero())))
            .with_body(Some(inner_circle))
            .pack()
    };
    let mut locator = locator.split();
    let off_frame =
        crate::layout_frame(engine, &off_content, locator.next(&()), styles, region)?;
    let on_frame =
        crate::layout_frame(engine, &on_content, locator.next(&()), styles, region)?;

    let value = &elem.value;
    let selected = group.selected.as_ref().is_some_and(|s| s == value);
    let widget = FieldWidget {
        field_location: group.location,
        size: region.size,
        action: None,
        value: Some(value.clone()),
        appearance: FieldAppearanceKind::Dual {
            active: selected,
            body_off: off_frame,
            body_on: on_frame,
        },
    };

    let mut frame = Frame::soft(region.size);
    frame.push(Point::zero(), FrameItem::FieldAppearance(widget));
    frame.clip(Curve::rect(frame.size()));

    Ok(frame)
}

/// Layout the textbox.
#[typst_macros::time(span = elem.span())]
pub fn layout_textbox(
    elem: &Packed<FormTextboxElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let inner_width = Smart::Custom(Ratio::one().into());
    let inner_height = Sizing::Rel(Ratio::one().into());

    let background = RectElem::new()
        .with_width(inner_width)
        .with_height(inner_height)
        .with_outset(Sides::splat(Some(Abs::pt(-0.5).into())))
        .pack();
    let foreground = BlockElem::new()
        .with_width(inner_width)
        .with_height(inner_height)
        .with_outset(Sides::splat(Some(Abs::pt(-0.5).into())))
        .with_inset(Sides::splat(Some(Em::new(0.1).into())))
        .with_body(
            elem.value
                .get_cloned(styles)
                .map(|v| BlockBody::Content(TextElem::new(v).pack())),
        )
        .pack();

    let mut locator = locator.split();
    let background_frame =
        crate::layout_frame(engine, &background, locator.next(&()), styles, region)?;
    let foreground_frame =
        crate::layout_frame(engine, &foreground, locator.next(&()), styles, region)?;

    let widget = FieldWidget {
        field_location: elem.location().unwrap(),
        size: region.size,
        action: None,
        value: None,
        appearance: FieldAppearanceKind::Single {
            variable_text: true,
            body: foreground_frame,
        },
    };

    let mut frame = Frame::soft(region.size);
    frame.push(Point::zero(), FrameItem::FieldAppearance(widget));
    frame.clip(Curve::rect(frame.size()));
    frame.push(Point::zero(), FrameItem::Group(GroupItem::new(background_frame)));

    Ok(frame)
}

/// Layout the choice field.
#[typst_macros::time(span = elem.span())]
pub fn layout_choice(
    elem: &Packed<FormChoiceElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let span = elem.span();

    let inner_width = Smart::Custom(Ratio::one().into());

    let options = elem.options.get_ref(styles);
    let value = elem.value.get_ref(styles);
    let multiple = elem.multiple.get(styles);

    FormChoiceElem::validate(span, multiple, options, value)?;

    let body = if multiple {
        let children = options.0.iter().map(|option| {
            let name = option.display_or_mapping_name();
            let fill =
                value.0.contains(&option.mapping_name).then_some(Color::BLUE.into());

            BlockElem::new()
                .with_width(inner_width)
                .with_breakable(false)
                .with_inset(Sides::splat(Some(Em::new(0.1).into())))
                .with_fill(fill)
                .with_body(Some(BlockBody::Content(TextElem::new(name).pack())))
                .pack()
        });

        StackElem::new(children.map(StackChild::Block).collect()).pack()
    } else {
        let value = value
            .0
            .first()
            .and_then(|value| {
                options.0.iter().find_map(|option| {
                    (&option.mapping_name == value)
                        .then(|| option.display_or_mapping_name())
                })
            })
            .unwrap_or_else(|| {
                options
                    .0
                    .first()
                    .map(|option| option.display_or_mapping_name())
                    .unwrap_or_default()
            });

        TextElem::new(value).pack()
    };

    let inner_frame = crate::layout_frame(engine, &body, locator, styles, region)?;

    let widget = FieldWidget {
        field_location: elem.location().unwrap(),
        size: region.size,
        action: None,
        value: None,
        appearance: FieldAppearanceKind::Single {
            variable_text: true,
            body: inner_frame,
        },
    };

    let mut frame = Frame::soft(region.size);
    frame.push(Point::zero(), FrameItem::FieldAppearance(widget));
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
