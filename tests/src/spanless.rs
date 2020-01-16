use super::*;


/// Compares elements by only looking at values and ignoring spans.
pub trait SpanlessEq<T> {
    fn spanless_eq(&self, other: &T) -> bool;
}

impl SpanlessEq<Vec<Spanned<Token<'_>>>> for Vec<Spanned<Token<'_>>> {
    fn spanless_eq(&self, other: &Vec<Spanned<Token>>) -> bool {
        self.len() == other.len()
        && self.iter().zip(other).all(|(x, y)| x.v == y.v)
    }
}

impl SpanlessEq<SyntaxTree> for SyntaxTree {
    fn spanless_eq(&self, other: &SyntaxTree) -> bool {
        fn downcast(func: &FuncCall) -> &DebugFn {
            func.0.downcast::<DebugFn>().expect("not a debug fn")
        }

        self.nodes.len() == other.nodes.len()
        && self.nodes.iter().zip(&other.nodes).all(|(x, y)| match (&x.v, &y.v) {
            (Node::Func(a), Node::Func(b)) => downcast(a).spanless_eq(downcast(b)),
            (a, b) => a == b,
        })
    }
}

impl SpanlessEq<DebugFn> for DebugFn {
    fn spanless_eq(&self, other: &DebugFn) -> bool {
        self.header.name.v == other.header.name.v
        && self.header.args.positional.spanless_eq(&other.header.args.positional)
        && self.header.args.keyword.spanless_eq(&other.header.args.keyword)
    }
}

impl SpanlessEq<Expression> for Expression {
    fn spanless_eq(&self, other: &Expression) -> bool {
        match (self, other) {
            (Expression::Tuple(a), Expression::Tuple(b)) => a.spanless_eq(b),
            (Expression::Object(a), Expression::Object(b)) => a.spanless_eq(b),
            (a, b) => a == b,
        }
    }
}

impl SpanlessEq<Tuple> for Tuple {
    fn spanless_eq(&self, other: &Tuple) -> bool {
        self.items.len() == other.items.len()
        && self.items.iter().zip(&other.items)
            .all(|(x, y)| x.v.spanless_eq(&y.v))
    }
}

impl SpanlessEq<Object> for Object {
    fn spanless_eq(&self, other: &Object) -> bool {
        self.pairs.len() == other.pairs.len()
        && self.pairs.iter().zip(&other.pairs)
            .all(|(x, y)| x.key.v == y.key.v && x.value.v.spanless_eq(&y.value.v))
    }
}
