use crate::length::ScaleLength;
use super::*;

/// `box`: Layouts its contents into a box.
///
/// # Keyword arguments
/// - `width`: The width of the box (length of relative to parent's width).
/// - `height`: The height of the box (length of relative to parent's height).
pub fn boxed(call: FuncCall, _: &ParseState) -> Pass<SyntaxNode> {
    let mut f = Feedback::new();
    let mut args = call.args;
    let node = BoxNode {
        content: args.pos.get::<SyntaxTree>().unwrap_or(SyntaxTree::new()),
        width: args.key.get::<ScaleLength>("width", &mut f),
        height: args.key.get::<ScaleLength>("height", &mut f),
    };
    drain_args(args, &mut f);
    Pass::node(node, f)
}

#[derive(Debug, Clone, PartialEq)]
struct BoxNode {
    content: SyntaxTree,
    width: Option<ScaleLength>,
    height: Option<ScaleLength>,
}

#[async_trait(?Send)]
impl Layout for BoxNode {
    async fn layout<'a>(&'a self, mut ctx: LayoutContext<'_>) -> Pass<Commands<'a>> {
        ctx.spaces.truncate(1);
        ctx.repeat = false;

        self.width.with(|v| {
            let length = v.raw_scaled(ctx.base.x);
            ctx.base.x = length;
            ctx.spaces[0].size.x = length;
            ctx.spaces[0].expansion.horizontal = true;
        });

        self.height.with(|v| {
            let length = v.raw_scaled(ctx.base.y);
            ctx.base.y = length;
            ctx.spaces[0].size.y = length;
            ctx.spaces[0].expansion.vertical = true;
        });

        layout(&self.content, ctx).await.map(|out| {
            let layout = out.into_iter().next().unwrap();
            vec![Add(layout)]
        })
    }
}
