use std::fmt::Debug;

use crate::func::prelude::*;
use super::decoration::Decoration;
use super::expr::{Expr, Ident, NamedTuple, Object, Pair, Tuple};
use super::parsing::{FuncArg, FuncArgs};
use super::span::Spanned;
use super::tokens::Token;
use super::tree::{DynamicNode, SyntaxNode};

pub fn check<T>(src: &str, exp: T, found: T, cmp_spans: bool)
where
    T: Debug + PartialEq + SpanlessEq,
{
    let cmp = if cmp_spans { PartialEq::eq } else { SpanlessEq::spanless_eq };
    if !cmp(&exp, &found) {
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

/// Compares elements by only looking at values and ignoring spans.
pub trait SpanlessEq<Rhs = Self> {
    fn spanless_eq(&self, other: &Rhs) -> bool;
}

impl SpanlessEq for SyntaxNode {
    fn spanless_eq(&self, other: &Self) -> bool {
        fn downcast<'a>(func: &'a (dyn DynamicNode + 'static)) -> &'a DebugNode {
            func.downcast::<DebugNode>().expect("not a debug node")
        }

        match (self, other) {
            (Self::Dyn(a), Self::Dyn(b)) => {
                downcast(a.as_ref()).spanless_eq(downcast(b.as_ref()))
            }
            (Self::Par(a), Self::Par(b)) => a.spanless_eq(b),
            (a, b) => a == b,
        }
    }
}

impl SpanlessEq for DebugNode {
    fn spanless_eq(&self, other: &Self) -> bool {
        self.0.spanless_eq(&other.0)
            && self.1.spanless_eq(&other.1)
    }
}

impl SpanlessEq for FuncCall {
    fn spanless_eq(&self, other: &Self) -> bool {
        self.name.spanless_eq(&other.name)
            && self.args.spanless_eq(&other.args)
    }
}

impl SpanlessEq for FuncArgs {
    fn spanless_eq(&self, other: &Self) -> bool {
        self.key.spanless_eq(&other.key)
            && self.pos.spanless_eq(&other.pos)
    }
}

impl SpanlessEq for FuncArg {
    fn spanless_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (FuncArg::Pos(a), FuncArg::Pos(b)) => a.spanless_eq(b),
            (FuncArg::Key(a), FuncArg::Key(b)) => a.spanless_eq(b),
            _ => false,
        }
    }
}

impl SpanlessEq for Expr {
    fn spanless_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Expr::Tuple(a), Expr::Tuple(b)) => a.spanless_eq(b),
            (Expr::NamedTuple(a), Expr::NamedTuple(b)) => a.spanless_eq(b),
            (Expr::Object(a), Expr::Object(b)) => a.spanless_eq(b),
            (Expr::Neg(a), Expr::Neg(b)) => a.spanless_eq(&b),
            (Expr::Add(a1, a2), Expr::Add(b1, b2)) => a1.spanless_eq(&b1) && a2.spanless_eq(&b2),
            (Expr::Sub(a1, a2), Expr::Sub(b1, b2)) => a1.spanless_eq(&b1) && a2.spanless_eq(&b2),
            (Expr::Mul(a1, a2), Expr::Mul(b1, b2)) => a1.spanless_eq(&b1) && a2.spanless_eq(&b2),
            (Expr::Div(a1, a2), Expr::Div(b1, b2)) => a1.spanless_eq(&b1) && a2.spanless_eq(&b2),
            (a, b) => a == b,
        }
    }
}

impl SpanlessEq for Tuple {
    fn spanless_eq(&self, other: &Self) -> bool {
        self.0.spanless_eq(&other.0)
    }
}

impl SpanlessEq for NamedTuple {
    fn spanless_eq(&self, other: &Self) -> bool {
        self.name.v == other.name.v
            && self.tuple.v.spanless_eq(&other.tuple.v)
    }
}

impl SpanlessEq for Object {
    fn spanless_eq(&self, other: &Self) -> bool {
        self.0.spanless_eq(&other.0)
    }
}

impl SpanlessEq for Pair {
    fn spanless_eq(&self, other: &Self) -> bool {
        self.key.spanless_eq(&other.key) && self.value.spanless_eq(&other.value)
    }
}

impl<T: SpanlessEq> SpanlessEq for Vec<T> {
    fn spanless_eq(&self, other: &Self) -> bool {
        self.len() == other.len()
            && self.iter().zip(other).all(|(x, y)| x.spanless_eq(&y))
    }
}

impl<T: SpanlessEq> SpanlessEq for Spanned<T> {
    fn spanless_eq(&self, other: &Self) -> bool {
        self.v.spanless_eq(&other.v)
    }
}

impl<T: SpanlessEq> SpanlessEq for Box<T> {
    fn spanless_eq(&self, other: &Self) -> bool {
        (&**self).spanless_eq(&**other)
    }
}

impl<T: SpanlessEq> SpanlessEq for Option<T> {
    fn spanless_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Some(a), Some(b)) => a.spanless_eq(b),
            (None, None) => true,
            _ => false,
        }
    }
}

macro_rules! impl_through_partial_eq {
    ($type:ty) => {
        impl SpanlessEq for $type {
            fn spanless_eq(&self, other: &$type) -> bool {
                self == other
            }
        }
    };
}

impl_through_partial_eq!(Token<'_>);
impl_through_partial_eq!(Ident);

// Implement for string and decoration to be able to compare feedback.
impl_through_partial_eq!(String);
impl_through_partial_eq!(Decoration);
