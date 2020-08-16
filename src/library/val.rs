use super::*;

/// `val`: Ignores all arguments and layouts its body flatly.
///
/// This is also the fallback function, which is used when a function name
/// cannot be resolved.
pub fn val(call: FuncCall, _: &ParseState) -> Pass<SyntaxNode> {
    let mut args = call.args;
    let node = ValNode {
        content: args.take::<SyntaxTree>(),
    };
    Pass::node(node, Feedback::new())
}

#[derive(Debug, Clone, PartialEq)]
struct ValNode {
    content: Option<SyntaxTree>,
}

#[async_trait(?Send)]
impl Layout for ValNode {
    async fn layout<'a>(&'a self, _: LayoutContext<'_>) -> Pass<Commands<'a>> {
        Pass::okay(match &self.content {
            Some(tree) => vec![LayoutSyntaxTree(tree)],
            None => vec![],
        })
    }
}
