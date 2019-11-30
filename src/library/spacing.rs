use crate::func::prelude::*;

/// `line.break`, `n`: Ends the current line.
#[derive(Debug, PartialEq)]
pub struct LineBreak;

function! {
    data: LineBreak,
    parse: plain,
    layout(_, _) { Ok(vec![FinishLine]) }
}

/// `paragraph.break`: Ends the current paragraph.
///
/// This has the same effect as two subsequent newlines.
#[derive(Debug, PartialEq)]
pub struct ParagraphBreak;

function! {
    data: ParagraphBreak,
    parse: plain,
    layout(_, _) { Ok(vec![BreakParagraph]) }
}

macro_rules! space_func {
    ($ident:ident, $doc:expr, $var:ident => $command:expr) => (
        #[doc = $doc]
        #[derive(Debug, PartialEq)]
        pub struct $ident(Spacing);

        function! {
            data: $ident,

            parse(args, body, _ctx) {
                parse!(forbidden: body);

                let arg = args.get_pos::<ArgExpr>()?;
                let spacing = match arg.val {
                    Expression::Size(s) => Spacing::Absolute(*s),
                    Expression::Num(f) => Spacing::Relative(*f as f32),
                    _ => perr!("invalid spacing, expected size or number"),
                };

                Ok($ident(spacing))
            }

            layout(this, ctx) {
                let $var = match this.0 {
                    Spacing::Absolute(s) => s,
                    Spacing::Relative(f) => f * ctx.style.text.font_size,
                };

                Ok(vec![$command])
            }
        }
    );
}

/// Absolute or font-relative spacing.
#[derive(Debug, PartialEq)]
enum Spacing {
    Absolute(Size),
    Relative(f32),
}

// FIXME: h != primary and v != secondary.
space_func!(HorizontalSpace, "📖 `h`: Adds horizontal whitespace.",
    space => AddPrimarySpace(space));

space_func!(VerticalSpace, "📑 `v`: Adds vertical whitespace.",
    space => AddSecondarySpace(space));
