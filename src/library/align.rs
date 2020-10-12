use crate::prelude::*;
use std::fmt::{self, Display, Formatter};

/// `align`: Align content along the layouting axes.
///
/// # Positional arguments
/// - At most two of `left`, `right`, `top`, `bottom`, `center`.
///
/// When `center` is used as a positional argument, it is automatically inferred
/// which axis it should apply to depending on further arguments, defaulting
/// to the axis, text is set along.
///
/// # Keyword arguments
/// - `horizontal`: Any of `left`, `right` or `center`.
/// - `vertical`: Any of `top`, `bottom` or `center`.
///
/// There may not be two alignment specifications for the same axis.
pub fn align(mut args: Args, ctx: &mut EvalContext) -> Value {
    let snapshot = ctx.state.clone();

    let body = args.find::<SynTree>();
    let first = args.get::<_, Spanned<AlignArg>>(ctx, 0);
    let second = args.get::<_, Spanned<AlignArg>>(ctx, 1);
    let hor = args.get::<_, Spanned<AlignArg>>(ctx, "horizontal");
    let ver = args.get::<_, Spanned<AlignArg>>(ctx, "vertical");
    args.done(ctx);

    let iter = first
        .into_iter()
        .chain(second.into_iter())
        .map(|align| (align.v.axis(), align))
        .chain(hor.into_iter().map(|align| (Some(SpecAxis::Horizontal), align)))
        .chain(ver.into_iter().map(|align| (Some(SpecAxis::Vertical), align)));

    let aligns = dedup_aligns(ctx, iter);
    if aligns.main != ctx.state.aligns.main {
        ctx.end_par_group();
        ctx.start_par_group();
    }

    ctx.state.aligns = aligns;

    if let Some(body) = body {
        body.eval(ctx);
        ctx.state = snapshot;
    }

    Value::None
}

/// Deduplicate alignments and deduce to which axes they apply.
fn dedup_aligns(
    ctx: &mut EvalContext,
    iter: impl Iterator<Item = (Option<SpecAxis>, Spanned<AlignArg>)>,
) -> Gen<Align> {
    let mut aligns = ctx.state.aligns;
    let mut had = Gen::new(false, false);
    let mut had_center = false;

    for (axis, Spanned { v: align, span }) in iter {
        // Check whether we know which axis this alignment belongs to.
        if let Some(axis) = axis {
            // We know the axis.
            let gen_axis = axis.switch(ctx.state.dirs);
            let gen_align = align.switch(ctx.state.dirs);

            if align.axis().map_or(false, |a| a != axis) {
                ctx.diag(error!(
                    span,
                    "invalid alignment `{}` for {} axis", align, axis,
                ));
            } else if had.get(gen_axis) {
                ctx.diag(error!(span, "duplicate alignment for {} axis", axis));
            } else {
                *aligns.get_mut(gen_axis) = gen_align;
                *had.get_mut(gen_axis) = true;
            }
        } else {
            // We don't know the axis: This has to be a `center` alignment for a
            // positional argument.
            debug_assert_eq!(align, AlignArg::Center);

            if had.main && had.cross {
                ctx.diag(error!(span, "duplicate alignment"));
            } else if had_center {
                // Both this and the previous one are unspecified `center`
                // alignments. Both axes should be centered.
                aligns = Gen::new(Align::Center, Align::Center);
                had = Gen::new(true, true);
            } else {
                had_center = true;
            }
        }

        // If we we know one alignment, we can handle the unspecified `center`
        // alignment.
        if had_center && (had.main || had.cross) {
            if had.main {
                aligns.cross = Align::Center;
                had.cross = true;
            } else {
                aligns.main = Align::Center;
                had.main = true;
            }
            had_center = false;
        }
    }

    // If center has not been flushed by now, it is the only argument and then
    // we default to applying it to the cross axis.
    if had_center {
        aligns.cross = Align::Center;
    }

    aligns
}

/// An alignment argument.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
enum AlignArg {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}

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

    fn switch(self, dirs: Gen<Dir>) -> Self::Other {
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

convert_ident!(AlignArg, "alignment", |v| match v {
    "left" => Some(Self::Left),
    "right" => Some(Self::Right),
    "top" => Some(Self::Top),
    "bottom" => Some(Self::Bottom),
    "center" => Some(Self::Center),
    _ => None,
});

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
