use super::prelude::*;

/// `align`: Configure the alignment along the layouting axes.
pub fn align(ctx: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let first = args.find::<Align>();
    let second = args.find::<Align>();
    let body = args.find::<Template>();

    let mut horizontal = args.named("horizontal")?;
    let mut vertical = args.named("vertical")?;

    for value in first.into_iter().chain(second) {
        match value.axis() {
            Some(SpecAxis::Horizontal) | None if horizontal.is_none() => {
                horizontal = Some(value);
            }
            Some(SpecAxis::Vertical) | None if vertical.is_none() => {
                vertical = Some(value);
            }
            _ => {}
        }
    }

    let realign = |template: &mut Template| {
        template.modify(move |style| {
            if let Some(horizontal) = horizontal {
                style.aligns.inline = horizontal;
            }

            if let Some(vertical) = vertical {
                style.aligns.block = vertical;
            }
        });

        if vertical.is_some() {
            template.parbreak();
        }
    };

    Ok(if let Some(body) = body {
        let mut template = Template::new();
        template.save();
        realign(&mut template);
        template += body;
        template.restore();
        Value::Template(template)
    } else {
        realign(&mut ctx.template);
        Value::None
    })
}
