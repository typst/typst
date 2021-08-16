use super::*;
use crate::layout::{FixedNode, GridNode, PadNode, StackChild, StackNode, TrackSizing};
use crate::paper::{Paper, PaperClass};

/// `page`: Configure pages.
pub fn page(_: &mut EvalContext, args: &mut Arguments) -> TypResult<Value> {
    let paper = match args.eat::<Spanned<Str>>() {
        Some(name) => match Paper::from_name(&name.v) {
            None => bail!(name.span, "invalid paper name"),
            paper => paper,
        },
        None => None,
    };

    let width = args.named("width")?;
    let height = args.named("height")?;
    let margins = args.named("margins")?;
    let left = args.named("left")?;
    let top = args.named("top")?;
    let right = args.named("right")?;
    let bottom = args.named("bottom")?;
    let flip = args.named("flip")?;
    let body = args.expect::<Template>("body")?;

    Ok(Value::template(move |ctx| {
        let snapshot = ctx.state.clone();
        let state = ctx.state.page_mut();

        if let Some(paper) = paper {
            state.class = paper.class();
            state.size = paper.size();
        }

        if let Some(width) = width {
            state.class = PaperClass::Custom;
            state.size.width = width;
        }

        if let Some(height) = height {
            state.class = PaperClass::Custom;
            state.size.height = height;
        }

        if let Some(margins) = margins {
            state.margins = Sides::splat(Some(margins));
        }

        if let Some(left) = left {
            state.margins.left = Some(left);
        }

        if let Some(top) = top {
            state.margins.top = Some(top);
        }

        if let Some(right) = right {
            state.margins.right = Some(right);
        }

        if let Some(bottom) = bottom {
            state.margins.bottom = Some(bottom);
        }

        if flip.unwrap_or(false) {
            std::mem::swap(&mut state.size.width, &mut state.size.height);
        }

        ctx.pagebreak(false, true);
        body.exec(ctx);

        ctx.state = snapshot;
        ctx.pagebreak(true, false);
    }))
}

/// `pagebreak`: Start a new page.
pub fn pagebreak(_: &mut EvalContext, _: &mut Arguments) -> TypResult<Value> {
    Ok(Value::template(move |ctx| ctx.pagebreak(true, true)))
}

/// `h`: Horizontal spacing.
pub fn h(_: &mut EvalContext, args: &mut Arguments) -> TypResult<Value> {
    spacing_impl(args, GenAxis::Cross)
}

/// `v`: Vertical spacing.
pub fn v(_: &mut EvalContext, args: &mut Arguments) -> TypResult<Value> {
    spacing_impl(args, GenAxis::Main)
}

fn spacing_impl(args: &mut Arguments, axis: GenAxis) -> TypResult<Value> {
    let spacing = args.expect::<Linear>("spacing")?;
    Ok(Value::template(move |ctx| {
        // TODO: Should this really always be font-size relative?
        let amount = spacing.resolve(ctx.state.font.size);
        ctx.push_spacing(axis, amount);
    }))
}

/// `align`: Configure the alignment along the layouting axes.
pub fn align(_: &mut EvalContext, args: &mut Arguments) -> TypResult<Value> {
    let mut horizontal = args.named("horizontal")?;
    let mut vertical = args.named("vertical")?;
    let first = args.eat::<Align>();
    let second = args.eat::<Align>();
    let body = args.expect::<Template>("body")?;

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

    Ok(Value::template(move |ctx| {
        if let Some(horizontal) = horizontal {
            ctx.state.aligns.cross = horizontal;
        }

        if let Some(vertical) = vertical {
            ctx.state.aligns.main = vertical;
            ctx.parbreak();
        }

        body.exec(ctx);
    }))
}

/// `box`: Place content in a rectangular box.
pub fn boxed(_: &mut EvalContext, args: &mut Arguments) -> TypResult<Value> {
    let width = args.named("width")?;
    let height = args.named("height")?;
    let body = args.eat().unwrap_or_default();
    Ok(Value::template(move |ctx| {
        let child = ctx.exec_template_stack(&body).into();
        ctx.push_into_par(FixedNode { width, height, child });
    }))
}

/// `block`: Place content in a block.
pub fn block(_: &mut EvalContext, args: &mut Arguments) -> TypResult<Value> {
    let body = args.expect("body")?;
    Ok(Value::template(move |ctx| {
        let block = ctx.exec_template_stack(&body);
        ctx.push_into_stack(block);
    }))
}

/// `pad`: Pad content at the sides.
pub fn pad(_: &mut EvalContext, args: &mut Arguments) -> TypResult<Value> {
    let all = args.eat();
    let left = args.named("left")?;
    let top = args.named("top")?;
    let right = args.named("right")?;
    let bottom = args.named("bottom")?;
    let body = args.expect("body")?;

    let padding = Sides::new(
        left.or(all).unwrap_or_default(),
        top.or(all).unwrap_or_default(),
        right.or(all).unwrap_or_default(),
        bottom.or(all).unwrap_or_default(),
    );

    Ok(Value::template(move |ctx| {
        let child = ctx.exec_template_stack(&body).into();
        ctx.push_into_stack(PadNode { padding, child });
    }))
}

/// `stack`: Stack children along an axis.
pub fn stack(_: &mut EvalContext, args: &mut Arguments) -> TypResult<Value> {
    let dir = args.named("dir")?;
    let children: Vec<_> = args.all().collect();

    Ok(Value::template(move |ctx| {
        let children = children
            .iter()
            .map(|child| {
                let child = ctx.exec_template_stack(child).into();
                StackChild::Any(child, ctx.state.aligns)
            })
            .collect();

        let mut dirs = Gen::new(None, dir).unwrap_or(ctx.state.dirs);

        // If the directions become aligned, fix up the cross direction since
        // that's the one that is not user-defined.
        if dirs.main.axis() == dirs.cross.axis() {
            dirs.cross = ctx.state.dirs.main;
        }

        ctx.push_into_stack(StackNode { dirs, aspect: None, children });
    }))
}

/// `grid`: Arrange children into a grid.
pub fn grid(_: &mut EvalContext, args: &mut Arguments) -> TypResult<Value> {
    let columns = args.named("columns")?.unwrap_or_default();
    let rows = args.named("rows")?.unwrap_or_default();

    let gutter_columns = args.named("gutter-columns")?;
    let gutter_rows = args.named("gutter-rows")?;
    let default = args
        .named("gutter")?
        .map(|v| vec![TrackSizing::Linear(v)])
        .unwrap_or_default();

    let column_dir = args.named("column-dir")?;
    let row_dir = args.named("row-dir")?;

    let children: Vec<_> = args.all().collect();

    let tracks = Gen::new(columns, rows);
    let gutter = Gen::new(
        gutter_columns.unwrap_or_else(|| default.clone()),
        gutter_rows.unwrap_or(default),
    );

    Ok(Value::template(move |ctx| {
        let children = children
            .iter()
            .map(|child| ctx.exec_template_stack(child).into())
            .collect();

        let mut dirs = Gen::new(column_dir, row_dir).unwrap_or(ctx.state.dirs);

        // If the directions become aligned, try to fix up the direction which
        // is not user-defined.
        if dirs.main.axis() == dirs.cross.axis() {
            let target = if column_dir.is_some() {
                &mut dirs.main
            } else {
                &mut dirs.cross
            };

            *target = if target.axis() == ctx.state.dirs.cross.axis() {
                ctx.state.dirs.main
            } else {
                ctx.state.dirs.cross
            };
        }

        ctx.push_into_stack(GridNode {
            dirs,
            tracks: tracks.clone(),
            gutter: gutter.clone(),
            children,
        })
    }))
}

/// Defines size of rows and columns in a grid.
type Tracks = Vec<TrackSizing>;

castable! {
    Tracks: "array of `auto`s, linears, and fractionals",
    Value::Int(count) => vec![TrackSizing::Auto; count.max(0) as usize],
    Value::Array(values) => values
        .into_iter()
        .filter_map(|v| v.cast().ok())
        .collect(),
}

castable! {
    TrackSizing: "`auto`, linear, or fractional",
    Value::Auto => TrackSizing::Auto,
    Value::Length(v) => TrackSizing::Linear(v.into()),
    Value::Relative(v) => TrackSizing::Linear(v.into()),
    Value::Linear(v) => TrackSizing::Linear(v),
    Value::Fractional(v) => TrackSizing::Fractional(v),
}
