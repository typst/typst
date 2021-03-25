use super::*;

/// `align`: Configure the alignment along the layouting axes.
///
/// # Positional parameters
/// - Alignments: variadic, of type `alignment`.
/// - Body: optional, of type `template`.
///
/// # Named parameters
/// - Horizontal alignment: `horizontal`, of type `alignment`.
/// - Vertical alignment: `vertical`, of type `alignment`.
///
/// # Return value
/// A template that changes the alignment along the layouting axes. The effect
/// is scoped to the body if present.
///
/// # Relevant types and constants
/// - Type `alignment`
///   - `start`
///   - `center`
///   - `end`
///   - `left`
///   - `right`
///   - `top`
///   - `bottom`
pub fn align(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let first = args.find::<AlignValue>(ctx);
    let second = args.find::<AlignValue>(ctx);
    let mut horizontal = args.get::<AlignValue>(ctx, "horizontal");
    let mut vertical = args.get::<AlignValue>(ctx, "vertical");
    let body = args.find::<TemplateValue>(ctx);

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
                ctx.push_linebreak();
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

typify! {
    AlignValue: "alignment",
}
