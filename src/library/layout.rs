use std::fmt::{self, Display, Formatter};

use crate::eval::Softness;
use crate::geom::{Length, Linear};
use crate::layout::{Expansion, Fixed, Spacing, Stack};
use crate::paper::{Paper, PaperClass};
use crate::prelude::*;

/// `align`: Align content along the layouting axes.
///
/// # Positional arguments
/// - At most two of `left`, `right`, `top`, `bottom`, `center`.
///
/// When `center` is used as a positional argument, it is automatically inferred
/// which axis it should apply to depending on further arguments, defaulting
/// to the cross axis.
///
/// # Keyword arguments
/// - `horizontal`: Any of `left`, `right` or `center`.
/// - `vertical`: Any of `top`, `bottom` or `center`.
pub fn align(mut args: Args, ctx: &mut EvalContext) -> Value {
    let snapshot = ctx.state.clone();
    let body = args.find::<SynTree>();
    let first = args.get::<_, Spanned<AlignArg>>(ctx, 0);
    let second = args.get::<_, Spanned<AlignArg>>(ctx, 1);
    let hor = args.get::<_, Spanned<AlignArg>>(ctx, "horizontal");
    let ver = args.get::<_, Spanned<AlignArg>>(ctx, "vertical");
    args.done(ctx);

    let prev_main = ctx.state.align.main;
    let mut had = Gen::uniform(false);
    let mut had_center = false;

    for (axis, Spanned { v: arg, span }) in first
        .into_iter()
        .chain(second.into_iter())
        .map(|arg| (arg.v.axis(), arg))
        .chain(hor.into_iter().map(|arg| (Some(SpecAxis::Horizontal), arg)))
        .chain(ver.into_iter().map(|arg| (Some(SpecAxis::Vertical), arg)))
    {
        // Check whether we know which axis this alignment belongs to.
        if let Some(axis) = axis {
            // We know the axis.
            let gen_axis = axis.switch(ctx.state.flow);
            let gen_align = arg.switch(ctx.state.flow);

            if arg.axis().map_or(false, |a| a != axis) {
                ctx.diag(error!(
                    span,
                    "invalid alignment `{}` for {} axis", arg, axis,
                ));
            } else if had.get(gen_axis) {
                ctx.diag(error!(span, "duplicate alignment for {} axis", axis));
            } else {
                *ctx.state.align.get_mut(gen_axis) = gen_align;
                *had.get_mut(gen_axis) = true;
            }
        } else {
            // We don't know the axis: This has to be a `center` alignment for a
            // positional argument.
            debug_assert_eq!(arg, AlignArg::Center);

            if had.main && had.cross {
                ctx.diag(error!(span, "duplicate alignment"));
            } else if had_center {
                // Both this and the previous one are unspecified `center`
                // alignments. Both axes should be centered.
                ctx.state.align.main = Align::Center;
                ctx.state.align.cross = Align::Center;
                had = Gen::uniform(true);
            } else {
                had_center = true;
            }
        }

        // If we we know the other alignment, we can handle the unspecified
        // `center` alignment.
        if had_center && (had.main || had.cross) {
            if had.main {
                ctx.state.align.cross = Align::Center;
                had.cross = true;
            } else {
                ctx.state.align.main = Align::Center;
                had.main = true;
            }
            had_center = false;
        }
    }

    // If `had_center` wasn't flushed by now, it's the only argument and then we
    // default to applying it to the cross axis.
    if had_center {
        ctx.state.align.cross = Align::Center;
    }

    if ctx.state.align.main != prev_main {
        ctx.end_par_group();
        ctx.start_par_group();
    }

    if let Some(body) = body {
        body.eval(ctx);
        ctx.state = snapshot;
    }

    Value::None
}

/// An argument to `[align]`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
enum AlignArg {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}

convert_ident!(AlignArg, "alignment", |v| match v {
    "left" => Some(Self::Left),
    "right" => Some(Self::Right),
    "top" => Some(Self::Top),
    "bottom" => Some(Self::Bottom),
    "center" => Some(Self::Center),
    _ => None,
});

impl AlignArg {
    /// The specific axis this alignment refers to.
    ///
    /// Returns `None` if this is `Center` since the axis is unknown.
    pub fn axis(self) -> Option<SpecAxis> {
        match self {
            Self::Left => Some(SpecAxis::Horizontal),
            Self::Right => Some(SpecAxis::Horizontal),
            Self::Top => Some(SpecAxis::Vertical),
            Self::Bottom => Some(SpecAxis::Vertical),
            Self::Center => None,
        }
    }
}

impl Switch for AlignArg {
    type Other = Align;

    fn switch(self, flow: Flow) -> Self::Other {
        let get = |dir: Dir, at_positive_start| {
            if dir.is_positive() == at_positive_start {
                Align::Start
            } else {
                Align::End
            }
        };

        let flow = flow.switch(flow);
        match self {
            Self::Left => get(flow.horizontal, true),
            Self::Right => get(flow.horizontal, false),
            Self::Top => get(flow.vertical, true),
            Self::Bottom => get(flow.vertical, false),
            Self::Center => Align::Center,
        }
    }
}

impl Display for AlignArg {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::Center => "center",
        })
    }
}

/// `box`: Layout content into a box.
///
/// # Keyword arguments
/// - `width`: The width of the box (length or relative to parent's width).
/// - `height`: The height of the box (length or relative to parent's height).
pub fn boxed(mut args: Args, ctx: &mut EvalContext) -> Value {
    let snapshot = ctx.state.clone();
    let body = args.find::<SynTree>().unwrap_or_default();
    let width = args.get::<_, Linear>(ctx, "width");
    let height = args.get::<_, Linear>(ctx, "height");
    let main = args.get::<_, Spanned<Dir>>(ctx, "main-dir");
    let cross = args.get::<_, Spanned<Dir>>(ctx, "cross-dir");
    ctx.set_flow(Gen::new(main, cross));
    args.done(ctx);

    let flow = ctx.state.flow;
    let align = ctx.state.align;

    ctx.start_content_group();
    body.eval(ctx);
    let children = ctx.end_content_group();

    ctx.push(Fixed {
        width,
        height,
        child: LayoutNode::dynamic(Stack {
            flow,
            align,
            expansion: Spec::new(
                Expansion::fill_if(width.is_some()),
                Expansion::fill_if(height.is_some()),
            )
            .switch(flow),
            children,
        }),
    });

    ctx.state = snapshot;
    Value::None
}

/// `h`: Add horizontal spacing.
///
/// # Positional arguments
/// - The spacing (length or relative to font size).
pub fn h(args: Args, ctx: &mut EvalContext) -> Value {
    spacing(args, ctx, SpecAxis::Horizontal)
}

/// `v`: Add vertical spacing.
///
/// # Positional arguments
/// - The spacing (length or relative to font size).
pub fn v(args: Args, ctx: &mut EvalContext) -> Value {
    spacing(args, ctx, SpecAxis::Vertical)
}

/// Apply spacing along a specific axis.
fn spacing(mut args: Args, ctx: &mut EvalContext, axis: SpecAxis) -> Value {
    let spacing = args.need::<_, Linear>(ctx, 0, "spacing");
    args.done(ctx);

    if let Some(linear) = spacing {
        let amount = linear.resolve(ctx.state.font.font_size());
        let spacing = Spacing { amount, softness: Softness::Hard };
        if ctx.state.flow.main.axis() == axis {
            ctx.end_par_group();
            ctx.push(spacing);
            ctx.start_par_group();
        } else {
            ctx.push(spacing);
        }
    }

    Value::None
}

/// `page`: Configure pages.
///
/// # Positional arguments
/// - The name of a paper, e.g. `a4` (optional).
///
/// # Keyword arguments
/// - `width`: The width of pages (length).
/// - `height`: The height of pages (length).
/// - `margins`: The margins for all sides (length or relative to side lengths).
/// - `left`: The left margin (length or relative to width).
/// - `right`: The right margin (length or relative to width).
/// - `top`: The top margin (length or relative to height).
/// - `bottom`: The bottom margin (length or relative to height).
/// - `flip`: Flips custom or paper-defined width and height (boolean).
pub fn page(mut args: Args, ctx: &mut EvalContext) -> Value {
    let snapshot = ctx.state.clone();
    let body = args.find::<SynTree>();

    if let Some(paper) = args.get::<_, Paper>(ctx, 0) {
        ctx.state.page.class = paper.class;
        ctx.state.page.size = paper.size();
    }

    if let Some(width) = args.get::<_, Length>(ctx, "width") {
        ctx.state.page.class = PaperClass::Custom;
        ctx.state.page.size.width = width;
    }

    if let Some(height) = args.get::<_, Length>(ctx, "height") {
        ctx.state.page.class = PaperClass::Custom;
        ctx.state.page.size.height = height;
    }

    if let Some(margins) = args.get::<_, Linear>(ctx, "margins") {
        ctx.state.page.margins = Sides::uniform(Some(margins));
    }

    if let Some(left) = args.get::<_, Linear>(ctx, "left") {
        ctx.state.page.margins.left = Some(left);
    }

    if let Some(top) = args.get::<_, Linear>(ctx, "top") {
        ctx.state.page.margins.top = Some(top);
    }

    if let Some(right) = args.get::<_, Linear>(ctx, "right") {
        ctx.state.page.margins.right = Some(right);
    }

    if let Some(bottom) = args.get::<_, Linear>(ctx, "bottom") {
        ctx.state.page.margins.bottom = Some(bottom);
    }

    if args.get::<_, bool>(ctx, "flip").unwrap_or(false) {
        let size = &mut ctx.state.page.size;
        std::mem::swap(&mut size.width, &mut size.height);
    }

    let main = args.get::<_, Spanned<Dir>>(ctx, "main-dir");
    let cross = args.get::<_, Spanned<Dir>>(ctx, "cross-dir");
    ctx.set_flow(Gen::new(main, cross));

    args.done(ctx);

    let mut softness = ctx.end_page_group(|_| false);

    if let Some(body) = body {
        // TODO: Restrict body to a single page?
        ctx.start_page_group(Softness::Hard);
        body.eval(ctx);
        ctx.end_page_group(|s| s == Softness::Hard);
        ctx.state = snapshot;
        softness = Softness::Soft;
    }

    ctx.start_page_group(softness);

    Value::None
}

/// `pagebreak`: Start a new page.
pub fn pagebreak(args: Args, ctx: &mut EvalContext) -> Value {
    args.done(ctx);
    ctx.end_page_group(|_| true);
    ctx.start_page_group(Softness::Hard);
    Value::None
}
