use std::fmt::{self, Display, Formatter};

use crate::layout::{Expansion, Fill, NodeFixed, NodeSpacing, NodeStack};
use crate::paper::{Paper, PaperClass};
use crate::prelude::*;
use crate::{eval::Softness, layout::NodeBackground};

/// `align`: Align content along the layouting axes.
///
/// Which axis an alignment should apply to (main or cross) is inferred from
/// either the argument itself (for anything other than `center`) or from the
/// second argument if present, defaulting to the cross axis for a single
/// `center` alignment.
///
/// # Positional arguments
/// - Alignments: variadic, of type `alignment`.
///
/// # Named arguments
/// - Horizontal alignment: `horizontal`, of type `alignment`.
/// - Vertical alignment:   `vertical`, of type `alignment`.
///
/// # Relevant types and constants
/// - Type `alignment`
///     - `left`
///     - `right`
///     - `top`
///     - `bottom`
///     - `center`
pub fn align(ctx: &mut EvalContext, args: &mut Args) -> Value {
    let snapshot = ctx.state.clone();

    let first = args.find(ctx);
    let second = args.find(ctx);
    let hor = args.get(ctx, "horizontal");
    let ver = args.get(ctx, "vertical");

    let mut had = Gen::uniform(false);
    let mut had_center = false;

    for (axis, Spanned { v: arg, span }) in first
        .into_iter()
        .chain(second.into_iter())
        .map(|arg: Spanned<Alignment>| (arg.v.axis(), arg))
        .chain(hor.into_iter().map(|arg| (Some(SpecAxis::Horizontal), arg)))
        .chain(ver.into_iter().map(|arg| (Some(SpecAxis::Vertical), arg)))
    {
        // Check whether we know which axis this alignment belongs to.
        if let Some(axis) = axis {
            // We know the axis.
            let gen_axis = axis.switch(ctx.state.dirs);
            let gen_align = arg.switch(ctx.state.dirs);

            if arg.axis().map_or(false, |a| a != axis) {
                ctx.diag(error!(span, "invalid alignment for {} axis", axis));
            } else if had.get(gen_axis) {
                ctx.diag(error!(span, "duplicate alignment for {} axis", axis));
            } else {
                *ctx.state.align.get_mut(gen_axis) = gen_align;
                *had.get_mut(gen_axis) = true;
            }
        } else {
            // We don't know the axis: This has to be a `center` alignment for a
            // positional argument.
            debug_assert_eq!(arg, Alignment::Center);

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

    if ctx.state.align.main != snapshot.align.main {
        ctx.end_par_group();
        ctx.start_par_group();
    }

    if let Some(body) = args.find::<ValueTemplate>(ctx) {
        body.eval(ctx);
        ctx.state = snapshot;
    }

    Value::None
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum Alignment {
    Left,
    Center,
    Right,
    Top,
    Bottom,
}

impl Alignment {
    /// The specific axis this alignment refers to.
    fn axis(self) -> Option<SpecAxis> {
        match self {
            Self::Left => Some(SpecAxis::Horizontal),
            Self::Right => Some(SpecAxis::Horizontal),
            Self::Top => Some(SpecAxis::Vertical),
            Self::Bottom => Some(SpecAxis::Vertical),
            Self::Center => None,
        }
    }
}

impl Switch for Alignment {
    type Other = Align;

    fn switch(self, dirs: LayoutDirs) -> Self::Other {
        let get = |dir: Dir, at_positive_start| {
            if dir.is_positive() == at_positive_start {
                Align::Start
            } else {
                Align::End
            }
        };

        let dirs = dirs.switch(dirs);
        match self {
            Self::Left => get(dirs.horizontal, true),
            Self::Right => get(dirs.horizontal, false),
            Self::Top => get(dirs.vertical, true),
            Self::Bottom => get(dirs.vertical, false),
            Self::Center => Align::Center,
        }
    }
}

impl_type! {
    Alignment: "alignment",
}

impl Display for Alignment {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
            Self::Top => "top",
            Self::Bottom => "bottom",
        })
    }
}

/// `box`: Layout content into a box.
///
/// # Named arguments
/// - Width of the box:  `width`, of type `linear` relative to parent width.
/// - Height of the box: `height`, of type `linear` relative to parent height.
/// - Main layouting direction: `main-dir`, of type `direction`.
/// - Cross layouting direction: `cross-dir`, of type `direction`.
/// - Background color of the box: `color`, of type `color`.
///
/// # Relevant types and constants
/// - Type `direction`
///     - `ltr` (left to right)
///     - `rtl` (right to left)
///     - `ttb` (top to bottom)
///     - `btt` (bottom to top)
pub fn box_(ctx: &mut EvalContext, args: &mut Args) -> Value {
    let snapshot = ctx.state.clone();

    let width = args.get(ctx, "width");
    let height = args.get(ctx, "height");
    let main = args.get(ctx, "main-dir");
    let cross = args.get(ctx, "cross-dir");
    let color = args.get(ctx, "color");

    ctx.set_dirs(Gen::new(main, cross));

    let dirs = ctx.state.dirs;
    let align = ctx.state.align;

    ctx.start_content_group();

    if let Some(body) = args.find::<ValueTemplate>(ctx) {
        body.eval(ctx);
    }

    let children = ctx.end_content_group();

    let fill_if = |c| if c { Expansion::Fill } else { Expansion::Fit };
    let expand = Spec::new(fill_if(width.is_some()), fill_if(height.is_some()));

    let fixed_node = NodeFixed {
        width,
        height,
        child: NodeStack { dirs, align, expand, children }.into(),
    };

    if let Some(color) = color {
        ctx.push(NodeBackground {
            fill: Fill::Color(color),
            child: fixed_node.into(),
        })
    } else {
        ctx.push(fixed_node);
    }

    ctx.state = snapshot;
    Value::None
}

impl_type! {
    Dir: "direction"
}

/// `h`: Add horizontal spacing.
///
/// # Positional arguments
/// - Amount of spacing: of type `linear` relative to current font size.
pub fn h(ctx: &mut EvalContext, args: &mut Args) -> Value {
    spacing(ctx, args, SpecAxis::Horizontal)
}

/// `v`: Add vertical spacing.
///
/// # Positional arguments
/// - Amount of spacing: of type `linear` relative to current font size.
pub fn v(ctx: &mut EvalContext, args: &mut Args) -> Value {
    spacing(ctx, args, SpecAxis::Vertical)
}

/// Apply spacing along a specific axis.
fn spacing(ctx: &mut EvalContext, args: &mut Args, axis: SpecAxis) -> Value {
    let spacing: Option<Linear> = args.require(ctx, "spacing");

    if let Some(linear) = spacing {
        let amount = linear.resolve(ctx.state.font.font_size());
        let spacing = NodeSpacing { amount, softness: Softness::Hard };
        if axis == ctx.state.dirs.main.axis() {
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
/// - Paper name: optional, of type `string`, see [here](crate::paper) for a
///   full list of all paper names.
///
/// # Named arguments
/// - Width of the page:     `width`, of type `length`.
/// - Height of the page:    `height`, of type `length`.
/// - Margins for all sides: `margins`, of type `linear` relative to sides.
/// - Left margin:           `left`, of type `linear` relative to width.
/// - Right margin:          `right`, of type `linear` relative to width.
/// - Top margin:            `top`, of type `linear` relative to height.
/// - Bottom margin:         `bottom`, of type `linear` relative to height.
/// - Flip width and height: `flip`, of type `bool`.
pub fn page(ctx: &mut EvalContext, args: &mut Args) -> Value {
    let snapshot = ctx.state.clone();

    if let Some(name) = args.find::<Spanned<String>>(ctx) {
        if let Some(paper) = Paper::from_name(&name.v) {
            ctx.state.page.class = paper.class;
            ctx.state.page.size = paper.size();
            ctx.state.page.expand = Spec::uniform(Expansion::Fill);
        } else {
            ctx.diag(error!(name.span, "invalid paper name"));
        }
    }

    if let Some(width) = args.get(ctx, "width") {
        ctx.state.page.class = PaperClass::Custom;
        ctx.state.page.size.width = width;
        ctx.state.page.expand.horizontal = Expansion::Fill;
    }

    if let Some(height) = args.get(ctx, "height") {
        ctx.state.page.class = PaperClass::Custom;
        ctx.state.page.size.height = height;
        ctx.state.page.expand.vertical = Expansion::Fill;
    }

    if let Some(margins) = args.get(ctx, "margins") {
        ctx.state.page.margins = Sides::uniform(Some(margins));
    }

    if let Some(left) = args.get(ctx, "left") {
        ctx.state.page.margins.left = Some(left);
    }

    if let Some(top) = args.get(ctx, "top") {
        ctx.state.page.margins.top = Some(top);
    }

    if let Some(right) = args.get(ctx, "right") {
        ctx.state.page.margins.right = Some(right);
    }

    if let Some(bottom) = args.get(ctx, "bottom") {
        ctx.state.page.margins.bottom = Some(bottom);
    }

    if args.get(ctx, "flip").unwrap_or(false) {
        let page = &mut ctx.state.page;
        std::mem::swap(&mut page.size.width, &mut page.size.height);
        std::mem::swap(&mut page.expand.horizontal, &mut page.expand.vertical);
    }

    let main = args.get(ctx, "main-dir");
    let cross = args.get(ctx, "cross-dir");

    ctx.set_dirs(Gen::new(main, cross));

    let mut softness = ctx.end_page_group(|_| false);
    if let Some(body) = args.find::<ValueTemplate>(ctx) {
        // TODO: Restrict body to a single page?
        ctx.start_page_group(Softness::Hard);
        body.eval(ctx);
        ctx.end_page_group(|s| s == Softness::Hard);
        softness = Softness::Soft;
        ctx.state = snapshot;
    }

    ctx.start_page_group(softness);

    Value::None
}

/// `pagebreak`: Start a new page.
pub fn pagebreak(ctx: &mut EvalContext, _: &mut Args) -> Value {
    ctx.end_page_group(|_| true);
    ctx.start_page_group(Softness::Hard);
    Value::None
}
