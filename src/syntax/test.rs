use std::fmt::Debug;

use crate::func::prelude::*;
use super::tree::SyntaxNode;
use super::span;

pub fn check<T>(src: &str, exp: T, found: T, cmp_spans: bool)
where
    T: Debug + PartialEq,
{
    span::set_cmp(cmp_spans);
    let equal = exp == found;
    span::set_cmp(true);

    if !equal {
        println!("source:   {:?}", src);
        println!("expected: {:#?}", exp);
        println!("found:    {:#?}", found);
        panic!("test failed");
    }
}

/// Create a vector of optionally spanned expressions from a list description.
///
/// # Examples
/// ```
/// // With spans
/// spanned![(0:0, 0:5, "hello"), (0:5, 0:3, "world")]
///
/// // Without spans: Implicit zero spans.
/// spanned!["hello", "world"]
/// ```
macro_rules! span_vec {
    ($(($sl:tt:$sc:tt, $el:tt:$ec:tt, $v:expr)),* $(,)?) => {
        (vec![$(span_item!(($sl:$sc, $el:$ec, $v))),*], true)
    };

    ($($v:expr),* $(,)?) => {
        (vec![$(span_item!($v)),*], false)
    };
}

macro_rules! span_item {
    (($sl:tt:$sc:tt, $el:tt:$ec:tt, $v:expr)) => {{
        use $crate::syntax::span::{Pos, Span, Spanned};
        Spanned {
            span: Span::new(
                Pos::new($sl, $sc),
                Pos::new($el, $ec)
            ),
            v: $v
        }
    }};

    ($v:expr) => {
        $crate::syntax::span::Spanned::zero($v)
    };
}

pub fn debug_func(mut call: FuncCall, _: &ParseState) -> Pass<SyntaxNode> {
    let tree = call.args.pos.get::<SyntaxTree>();
    Pass::node(DebugNode(call, tree), Feedback::new())
}

#[derive(Debug, Clone, PartialEq)]
pub struct DebugNode(pub FuncCall, pub Option<SyntaxTree>);

#[async_trait(?Send)]
impl Layout for DebugNode {
    async fn layout<'a>(&'a self, _: LayoutContext<'_>) -> Pass<Commands<'a>> {
        unimplemented!()
    }
}
