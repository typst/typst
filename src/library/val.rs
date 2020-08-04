use super::*;

/// `val`: Ignores all arguments and layouts its body flatly.
///
/// This is also the fallback function, which is used when a function name
/// cannot be resolved.
pub fn val(call: FuncCall, state: &ParseState) -> Pass<SyntaxNode> {
    let mut f = Feedback::new();
    let node = ValNode {
        body: parse_body_maybe(call.body, state, &mut f),
    };
    Pass::node(node, f)
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
