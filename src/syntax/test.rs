use super::func::FuncHeader;
use super::expr::{Expr, Tuple, Object};
use super::*;


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

    layout(self, ctx, errors) { vec![] }
}

/// Compares elements by only looking at values and ignoring spans.
pub trait SpanlessEq<Rhs=Self> {
    fn spanless_eq(&self, other: &Rhs) -> bool;
}

impl SpanlessEq for Vec<Spanned<Token<'_>>> {
    fn spanless_eq(&self, other: &Vec<Spanned<Token>>) -> bool {
        self.len() == other.len()
        && self.iter().zip(other).all(|(x, y)| x.v == y.v)
    }
}

impl SpanlessEq for SyntaxModel {
    fn spanless_eq(&self, other: &SyntaxModel) -> bool {
        fn downcast<'a>(func: &'a (dyn Model + 'static)) -> &'a DebugFn {
            func.downcast::<DebugFn>().expect("not a debug fn")
        }

        self.nodes.len() == other.nodes.len()
        && self.nodes.iter().zip(&other.nodes).all(|(x, y)| match (&x.v, &y.v) {
            (Node::Model(a), Node::Model(b)) => {
                downcast(a.as_ref()).spanless_eq(downcast(b.as_ref()))
            }
            (a, b) => a == b,
        })
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
            (Expr::Object(a), Expr::Object(b)) => a.spanless_eq(b),
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

impl SpanlessEq for Object {
    fn spanless_eq(&self, other: &Object) -> bool {
        self.pairs.len() == other.pairs.len()
        && self.pairs.iter().zip(&other.pairs)
            .all(|(x, y)| x.key.v == y.key.v && x.value.v.spanless_eq(&y.value.v))
    }
}
