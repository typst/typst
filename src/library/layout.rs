use super::*;
use crate::layout::{GridNode, PadNode, StackChild, StackNode, TrackSizing};
use crate::paper::{Paper, PaperClass};

/// `page`: Configure pages.
pub fn page(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let paper = args.eat::<Spanned<String>>(ctx).and_then(|name| {
        Paper::from_name(&name.v).or_else(|| {
            ctx.diag(error!(name.span, "invalid paper name"));
            None
        })
    });

    let width = args.named(ctx, "width");
    let height = args.named(ctx, "height");
    let margins = args.named(ctx, "margins");
    let left = args.named(ctx, "left");
    let top = args.named(ctx, "top");
    let right = args.named(ctx, "right");
    let bottom = args.named(ctx, "bottom");
    let flip = args.named(ctx, "flip");
    let body = args.eat::<TemplateValue>(ctx);
    let span = args.span;

    Value::template("page", move |ctx| {
        let snapshot = ctx.state.clone();

        if let Some(paper) = paper {
            ctx.state.page.class = paper.class;
            ctx.state.page.size = paper.size();
        }

        if let Some(width) = width {
            ctx.state.page.class = PaperClass::Custom;
            ctx.state.page.size.width = width;
        }

        if let Some(height) = height {
            ctx.state.page.class = PaperClass::Custom;
            ctx.state.page.size.height = height;
        }

        if let Some(margins) = margins {
            ctx.state.page.margins = Sides::splat(Some(margins));
        }

        if let Some(left) = left {
            ctx.state.page.margins.left = Some(left);
        }

        if let Some(top) = top {
            ctx.state.page.margins.top = Some(top);
        }

        if let Some(right) = right {
            ctx.state.page.margins.right = Some(right);
        }

        if let Some(bottom) = bottom {
            ctx.state.page.margins.bottom = Some(bottom);
        }

        if flip.unwrap_or(false) {
            let page = &mut ctx.state.page;
            std::mem::swap(&mut page.size.width, &mut page.size.height);
        }

        ctx.pagebreak(false, true, span);

        if let Some(body) = &body {
            // TODO: Restrict body to a single page?
            body.exec(ctx);
            ctx.state = snapshot;
            ctx.pagebreak(true, false, span);
        }
    })
}

/// `pagebreak`: Start a new page.
pub fn pagebreak(_: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let span = args.span;
    Value::template("pagebreak", move |ctx| {
        ctx.pagebreak(true, true, span);
    })
}

/// `h`: Horizontal spacing.
pub fn h(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    spacing_impl("h", ctx, args, GenAxis::Cross)
}

/// `v`: Vertical spacing.
pub fn v(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    spacing_impl("v", ctx, args, GenAxis::Main)
}

fn spacing_impl(
    name: &str,
    ctx: &mut EvalContext,
    args: &mut FuncArgs,
    axis: GenAxis,
) -> Value {
    let spacing: Option<Linear> = args.expect(ctx, "spacing");
    Value::template(name, move |ctx| {
        if let Some(linear) = spacing {
            // TODO: Should this really always be font-size relative?
            let amount = linear.resolve(ctx.state.font.size);
            ctx.push_spacing(axis, amount);
        }
    })
}

/// `align`: Configure the alignment along the layouting axes.
pub fn align(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let first = args.eat::<AlignValue>(ctx);
    let second = args.eat::<AlignValue>(ctx);
    let mut horizontal = args.named::<AlignValue>(ctx, "horizontal");
    let mut vertical = args.named::<AlignValue>(ctx, "vertical");
    let body = args.eat::<TemplateValue>(ctx);

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

    Value::template("align", move |ctx| {
        let snapshot = ctx.state.clone();

        if let Some(horizontal) = horizontal {
            ctx.state.aligns.cross = horizontal.to_align(ctx.state.lang.dir);
        }

        if let Some(vertical) = vertical {
            ctx.state.aligns.main = vertical.to_align(Dir::TTB);
            if ctx.state.aligns.main != snapshot.aligns.main {
                ctx.parbreak();
            }
        }

        if let Some(body) = &body {
            body.exec(ctx);
            ctx.state = snapshot;
        }
    })
}

/// An alignment specifier passed to `align`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub(super) enum AlignValue {
    Start,
    Center,
    End,
    Left,
    Right,
    Top,
    Bottom,
}

impl AlignValue {
    fn axis(self) -> Option<SpecAxis> {
        match self {
            Self::Start => None,
            Self::Center => None,
            Self::End => None,
            Self::Left => Some(SpecAxis::Horizontal),
            Self::Right => Some(SpecAxis::Horizontal),
            Self::Top => Some(SpecAxis::Vertical),
            Self::Bottom => Some(SpecAxis::Vertical),
        }
    }

    fn to_align(self, dir: Dir) -> Align {
        let side = |is_at_positive_start| {
            if dir.is_positive() == is_at_positive_start {
                Align::Start
            } else {
                Align::End
            }
        };

        match self {
            Self::Start => Align::Start,
            Self::Center => Align::Center,
            Self::End => Align::End,
            Self::Left => side(true),
            Self::Right => side(false),
            Self::Top => side(true),
            Self::Bottom => side(false),
        }
    }
}

impl Display for AlignValue {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Start => "start",
            Self::Center => "center",
            Self::End => "end",
            Self::Left => "left",
            Self::Right => "right",
            Self::Top => "top",
            Self::Bottom => "bottom",
        })
    }
}

castable! {
    AlignValue: "alignment",
}

/// `pad`: Pad content at the sides.
pub fn pad(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let all = args.eat(ctx);
    let left = args.named(ctx, "left");
    let top = args.named(ctx, "top");
    let right = args.named(ctx, "right");
    let bottom = args.named(ctx, "bottom");
    let body = args.expect::<TemplateValue>(ctx, "body").unwrap_or_default();

    let padding = Sides::new(
        left.or(all).unwrap_or_default(),
        top.or(all).unwrap_or_default(),
        right.or(all).unwrap_or_default(),
        bottom.or(all).unwrap_or_default(),
    );

    Value::template("pad", move |ctx| {
        let child = ctx.exec_template_stack(&body).into();
        ctx.push_into_stack(PadNode { padding, child });
    })
}

/// `stack`: Stack children along an axis.
pub fn stack(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let dir = args.named::<Dir>(ctx, "dir").unwrap_or(Dir::TTB);
    let children = args.all::<TemplateValue>(ctx);

    Value::template("stack", move |ctx| {
        let children = children
            .iter()
            .map(|child| {
                let child = ctx.exec_template_stack(child).into();
                StackChild::Any(child, ctx.state.aligns)
            })
            .collect();

        ctx.push_into_stack(StackNode {
            dirs: Gen::new(ctx.state.lang.dir, dir),
            aspect: None,
            children,
        });
    })
}

/// `grid`: Arrange children into a grid.
pub fn grid(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let columns = args.named::<Tracks>(ctx, "columns").unwrap_or_default();
    let rows = args.named::<Tracks>(ctx, "rows").unwrap_or_default();
    let gutter = args
        .named::<Linear>(ctx, "gutter")
        .map(|v| vec![TrackSizing::Linear(v)])
        .unwrap_or_default();
    let gutter_columns = args.named::<Tracks>(ctx, "gutter-columns");
    let gutter_rows = args.named::<Tracks>(ctx, "gutter-rows");
    let column_dir = args.named(ctx, "column-dir");
    let row_dir = args.named(ctx, "row-dir");
    let children = args.all::<TemplateValue>(ctx);

    Value::template("grid", move |ctx| {
        let children = children
            .iter()
            .map(|child| ctx.exec_template_stack(child).into())
            .collect();

        let cross_dir = column_dir.unwrap_or(ctx.state.lang.dir);
        let main_dir = row_dir.unwrap_or(cross_dir.axis().other().dir(true));

        ctx.push_into_stack(GridNode {
            dirs: Gen::new(cross_dir, main_dir),
            tracks: Gen::new(columns.clone(), rows.clone()),
            gutter: Gen::new(
                gutter_columns.as_ref().unwrap_or(&gutter).clone(),
                gutter_rows.as_ref().unwrap_or(&gutter).clone(),
            ),
            children,
        })
    })
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
