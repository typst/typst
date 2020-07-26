use std::fmt::Debug;

use super::func::FuncHeader;
use super::span::Spanned;
use super::expr::{Expr, Tuple, NamedTuple, Object};
use super::*;

/// Check whether the expected and found results are the same.
pub fn check<T>(src: &str, exp: T, found: T, cmp_spans: bool)
where T: Debug + PartialEq + SpanlessEq {
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
    (($sl:tt:$sc:tt, $el:tt:$ec:tt, $v:expr)) => ({
        use $crate::syntax::span::{Position, Span, Spanned};
        Spanned {
            span: Span::new(
                Position::new($sl, $sc),
                Position::new($el, $ec)
            ),
            v: $v
        }
    });

    ($v:expr) => {
        $crate::syntax::span::Spanned::zero($v)
    };
}

function! {
    /// Most functions in the tests are parsed into the debug function for easy
    /// inspection of arguments and body.
    #[derive(Debug, Clone, PartialEq)]
    pub struct DebugFn {
        pub header: FuncHeader,
        pub body: Option<SyntaxModel>,
    }

    parse(header, body, ctx, f) {
        let cloned = header.clone();
        header.args.pos.items.clear();
        header.args.key.pairs.clear();
        DebugFn {
            header: cloned,
            body: body!(opt: body, ctx, f),
        }
    }

    layout(self, ctx, f) { vec![] }
}

/// Compares elements by only looking at values and ignoring spans.
pub trait SpanlessEq<Rhs=Self> {
    fn spanless_eq(&self, other: &Rhs) -> bool;
}

impl SpanlessEq for SyntaxModel {
    fn spanless_eq(&self, other: &SyntaxModel) -> bool {
        self.nodes.spanless_eq(&other.nodes)
    }
}

impl SpanlessEq for Node {
    fn spanless_eq(&self, other: &Node) -> bool {
        fn downcast<'a>(func: &'a (dyn Model + 'static)) -> &'a DebugFn {
            func.downcast::<DebugFn>().expect("not a debug fn")
        }

        match (self, other) {
            (Node::Model(a), Node::Model(b)) => {
                downcast(a.as_ref()).spanless_eq(downcast(b.as_ref()))
            }
            (a, b) => a == b,
        }
    }
}

impl SpanlessEq for DebugFn {
    fn spanless_eq(&self, other: &DebugFn) -> bool {
        self.header.name.v == other.header.name.v
        && self.header.args.pos.spanless_eq(&other.header.args.pos)
        && self.header.args.key.spanless_eq(&other.header.args.key)
    }
}

impl SpanlessEq for Expr {
    fn spanless_eq(&self, other: &Expr) -> bool {
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
    fn spanless_eq(&self, other: &Tuple) -> bool {
        self.items.len() == other.items.len()
        && self.items.iter().zip(&other.items)
            .all(|(x, y)| x.v.spanless_eq(&y.v))
    }
}

impl SpanlessEq for NamedTuple {
    fn spanless_eq(&self, other: &NamedTuple) -> bool {
        self.name.v == other.name.v
        && self.tuple.v.spanless_eq(&other.tuple.v)
    }
}

impl SpanlessEq for Object {
    fn spanless_eq(&self, other: &Object) -> bool {
        self.pairs.len() == other.pairs.len()
        && self.pairs.iter().zip(&other.pairs)
            .all(|(x, y)| x.v.key.v == y.v.key.v && x.v.value.v.spanless_eq(&y.v.value.v))
    }
}

impl<T: SpanlessEq> SpanlessEq for Vec<T> {
    fn spanless_eq(&self, other: &Vec<T>) -> bool {
        self.len() == other.len()
        && self.iter().zip(other).all(|(x, y)| x.spanless_eq(&y))
    }
}

impl<T: SpanlessEq> SpanlessEq for Spanned<T> {
    fn spanless_eq(&self, other: &Spanned<T>) -> bool {
        self.v.spanless_eq(&other.v)
    }
}

impl<T: SpanlessEq> SpanlessEq for Box<T> {
    fn spanless_eq(&self, other: &Box<T>) -> bool {
        (&**self).spanless_eq(&**other)
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

// Implement for string and decoration to be able to compare feedback.
impl_through_partial_eq!(String);
impl_through_partial_eq!(Decoration);
