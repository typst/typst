use crate::layout::SpacingKind;
use crate::length::ScaleLength;
use super::*;

/// `h`: Add horizontal spacing.
///
/// # Positional arguments
/// - The spacing (length or relative to font size).
pub fn h(call: FuncCall, _: &ParseState) -> Pass<SyntaxNode> {
    spacing(call, Horizontal)
}

/// `v`: Add vertical spacing.
///
/// # Positional arguments
/// - The spacing (length or relative to font size).
pub fn v(call: FuncCall, _: &ParseState) -> Pass<SyntaxNode> {
    spacing(call, Vertical)
}

fn spacing(call: FuncCall, axis: SpecAxis) -> Pass<SyntaxNode> {
    let mut f = Feedback::new();
    let mut args = call.args;
    let node = SpacingNode {
        spacing: args.pos.expect::<ScaleLength>(&mut f)
            .map(|s| (axis, s))
            .or_missing(call.name.span, "spacing", &mut f),
    };
    drain_args(args, &mut f);
    Pass::node(node, f)
}

#[derive(Debug, Clone, PartialEq)]
struct SpacingNode {
    spacing: Option<(SpecAxis, ScaleLength)>,
}

#[async_trait(?Send)]
impl Layout for SpacingNode {
    async fn layout<'a>(&'a self, ctx: LayoutContext<'_>) -> Pass<Commands<'a>> {
        Pass::okay(if let Some((axis, spacing)) = self.spacing {
            let axis = axis.to_generic(ctx.axes);
            let spacing = spacing.raw_scaled(ctx.style.text.font_size());
            vec![AddSpacing(spacing, SpacingKind::Hard, axis)]
        } else {
            vec![]
        })
    }
}
