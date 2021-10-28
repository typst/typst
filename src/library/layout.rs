use super::*;
use crate::layout::{
    GridNode, PadNode, ShapeKind, ShapeNode, StackChild, StackNode, TrackSizing,
};
use crate::style::{Paper, PaperClass};

/// `page`: Configure pages.
pub fn page(ctx: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let paper = match args.named::<Spanned<Str>>("paper")?.or_else(|| args.find()) {
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

    ctx.template.modify(move |style| {
        let page = style.page_mut();

        if let Some(paper) = paper {
            page.class = paper.class();
            page.size = paper.size();
        }

        if let Some(width) = width {
            page.class = PaperClass::Custom;
            page.size.w = width;
        }

        if let Some(height) = height {
            page.class = PaperClass::Custom;
            page.size.h = height;
        }

        if let Some(margins) = margins {
            page.margins = Sides::splat(Some(margins));
        }

        if let Some(left) = left {
            page.margins.left = Some(left);
        }

        if let Some(top) = top {
            page.margins.top = Some(top);
        }

        if let Some(right) = right {
            page.margins.right = Some(right);
        }

        if let Some(bottom) = bottom {
            page.margins.bottom = Some(bottom);
        }

        if flip.unwrap_or(false) {
            std::mem::swap(&mut page.size.w, &mut page.size.h);
        }
    });

    ctx.template.pagebreak(false);

    Ok(Value::None)
}

/// `pagebreak`: Start a new page.
pub fn pagebreak(_: &mut EvalContext, _: &mut Args) -> TypResult<Value> {
    let mut template = Template::new();
    template.pagebreak(true);
    Ok(Value::Template(template))
}

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

/// `h`: Horizontal spacing.
pub fn h(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let mut template = Template::new();
    template.spacing(GenAxis::Inline, args.expect("spacing")?);
    Ok(Value::Template(template))
}

/// `v`: Vertical spacing.
pub fn v(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let mut template = Template::new();
    template.spacing(GenAxis::Block, args.expect("spacing")?);
    Ok(Value::Template(template))
}

/// `box`: Place content in a rectangular box.
pub fn box_(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let width = args.named("width")?;
    let height = args.named("height")?;
    let fill = args.named("fill")?;
    let body: Template = args.find().unwrap_or_default();
    Ok(Value::Template(Template::from_inline(move |style| {
        ShapeNode {
            shape: ShapeKind::Rect,
            width,
            height,
            fill: fill.map(Paint::Color),
            child: Some(body.to_stack(style).pack()),
        }
    })))
}

/// `block`: Place content in a block.
pub fn block(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let body: Template = args.expect("body")?;
    Ok(Value::Template(Template::from_block(move |style| {
        body.to_stack(style)
    })))
}

/// `pad`: Pad content at the sides.
pub fn pad(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let all = args.find();
    let left = args.named("left")?;
    let top = args.named("top")?;
    let right = args.named("right")?;
    let bottom = args.named("bottom")?;
    let body: Template = args.expect("body")?;

    let padding = Sides::new(
        left.or(all).unwrap_or_default(),
        top.or(all).unwrap_or_default(),
        right.or(all).unwrap_or_default(),
        bottom.or(all).unwrap_or_default(),
    );

    Ok(Value::Template(Template::from_block(move |style| {
        PadNode {
            padding,
            child: body.to_stack(&style).pack(),
        }
    })))
}

/// `move`: Move content without affecting layout.
pub fn move_(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    #[derive(Debug, Hash)]
    struct MoveNode {
        offset: Spec<Option<Linear>>,
        child: ShapeNode,
    }

    impl InlineLevel for MoveNode {
        fn layout(&self, ctx: &mut LayoutContext, space: Length, base: Size) -> Frame {
            let offset = Point::new(
                self.offset.x.map(|x| x.resolve(base.w)).unwrap_or_default(),
                self.offset.y.map(|y| y.resolve(base.h)).unwrap_or_default(),
            );

            let mut frame = self.child.layout(ctx, space, base);
            for (point, _) in &mut frame.children {
                *point += offset;
            }

            frame
        }
    }

    let x = args.named("x")?;
    let y = args.named("y")?;
    let body: Template = args.expect("body")?;

    Ok(Value::Template(Template::from_inline(move |style| {
        MoveNode {
            offset: Spec::new(x, y),
            child: ShapeNode {
                shape: ShapeKind::Rect,
                width: None,
                height: None,
                fill: None,
                child: Some(body.to_stack(style).pack()),
            },
        }
    })))
}

/// `stack`: Stack children along an axis.
pub fn stack(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    enum Child {
        Spacing(Spacing),
        Any(Template),
    }

    castable! {
        Child: "linear, fractional or template",
        Value::Length(v) => Self::Spacing(Spacing::Linear(v.into())),
        Value::Relative(v) => Self::Spacing(Spacing::Linear(v.into())),
        Value::Linear(v) => Self::Spacing(Spacing::Linear(v)),
        Value::Fractional(v) => Self::Spacing(Spacing::Fractional(v)),
        Value::Template(v) => Self::Any(v),
    }

    let dir = args.named("dir")?.unwrap_or(Dir::TTB);
    let spacing = args.named("spacing")?;
    let list: Vec<Child> = args.all().collect();

    Ok(Value::Template(Template::from_block(move |style| {
        let mut children = vec![];
        let mut delayed = None;

        // Build the list of stack children.
        for child in &list {
            match child {
                Child::Spacing(v) => {
                    children.push(StackChild::Spacing(*v));
                    delayed = None;
                }
                Child::Any(template) => {
                    if let Some(v) = delayed {
                        children.push(StackChild::Spacing(v));
                    }

                    let node = template.to_stack(style).pack();
                    children.push(StackChild::Node(node, style.aligns.block));
                    delayed = spacing;
                }
            }
        }

        StackNode { dir, children }
    })))
}

/// `grid`: Arrange children into a grid.
pub fn grid(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    castable! {
        Vec<TrackSizing>: "integer or (auto, linear, fractional, or array thereof)",
        Value::Auto => vec![TrackSizing::Auto],
        Value::Length(v) => vec![TrackSizing::Linear(v.into())],
        Value::Relative(v) => vec![TrackSizing::Linear(v.into())],
        Value::Linear(v) => vec![TrackSizing::Linear(v)],
        Value::Fractional(v) => vec![TrackSizing::Fractional(v)],
        Value::Int(count) => vec![TrackSizing::Auto; count.max(0) as usize],
        Value::Array(values) => values
            .into_iter()
            .filter_map(|v| v.cast().ok())
            .collect(),
    }

    castable! {
        TrackSizing: "auto, linear, or fractional",
        Value::Auto => Self::Auto,
        Value::Length(v) => Self::Linear(v.into()),
        Value::Relative(v) => Self::Linear(v.into()),
        Value::Linear(v) => Self::Linear(v),
        Value::Fractional(v) => Self::Fractional(v),
    }

    let columns = args.named("columns")?.unwrap_or_default();
    let rows = args.named("rows")?.unwrap_or_default();
    let tracks = Spec::new(columns, rows);

    let base_gutter: Vec<TrackSizing> = args.named("gutter")?.unwrap_or_default();
    let column_gutter = args.named("column-gutter")?;
    let row_gutter = args.named("row-gutter")?;
    let gutter = Spec::new(
        column_gutter.unwrap_or_else(|| base_gutter.clone()),
        row_gutter.unwrap_or(base_gutter),
    );

    let children: Vec<Template> = args.all().collect();

    Ok(Value::Template(Template::from_block(move |style| {
        GridNode {
            tracks: tracks.clone(),
            gutter: gutter.clone(),
            children: children
                .iter()
                .map(|child| child.to_stack(&style).pack())
                .collect(),
        }
    })))
}
