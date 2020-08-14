use super::*;

/// `val`: Ignores all arguments and layouts its body flatly.
///
/// This is also the fallback function, which is used when a function name
/// cannot be resolved.
pub fn val(call: FuncCall, _: &ParseState) -> Pass<SyntaxNode> {
    let node = ValNode {
        body: call.body.map(|s| s.v),
    };
    Pass::node(node, Feedback::new())
}

#[derive(Debug, Clone, PartialEq)]
struct ValNode {
    body: Option<SyntaxTree>,
}

#[async_trait(?Send)]
impl Layout for ValNode {
    async fn layout<'a>(&'a self, _: LayoutContext<'_>) -> Pass<Commands<'a>> {
        Pass::okay(match &self.body {
            Some(tree) => vec![LayoutSyntaxTree(tree)],
            None => vec![],
        })
    }
}
