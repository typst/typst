//! Syntax trees, parsing and tokenization.

pub mod decoration;
pub mod expr;
pub mod parsing;
pub mod scope;
pub mod span;
pub mod tokens;
pub mod tree;

#[cfg(test)]
mod tests {
    use std::fmt::Debug;
    use crate::func::prelude::*;
    use super::span;

    /// Assert that expected and found are equal, printing both and panicking
    /// and the source of their test case if they aren't.
    ///
    /// When `cmp_spans` is false, spans are ignored.
    pub fn check<T>(src: &str, exp: T, found: T, cmp_spans: bool)
    where
        T: Debug + PartialEq,
    {
        span::set_cmp(cmp_spans);
        let equal = exp == found;
        span::set_cmp(true);

        if !equal {
            println!("source:   {:?}", src);
            if cmp_spans {
                println!("expected: {:#?}", exp);
                println!("found:    {:#?}", found);
            } else {
                println!("expected: {:?}", exp);
                println!("found:    {:?}", found);
            }
            panic!("test failed");
        }
    }

    pub fn s<T>(sl: usize, sc: usize, el: usize, ec: usize, v: T) -> Spanned<T> {
        Spanned::new(v, Span::new(Pos::new(sl, sc), Pos::new(el, ec)))
    }

    // Enables tests to optionally specify spans.
    impl<T> From<T> for Spanned<T> {
        fn from(t: T) -> Self {
            Spanned::zero(t)
        }
    }

    pub fn debug_func(call: FuncCall, _: &ParseState) -> Pass<SyntaxNode> {
        Pass::node(DebugNode(call), Feedback::new())
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct DebugNode(pub FuncCall);

    #[async_trait(?Send)]
    impl Layout for DebugNode {
        async fn layout<'a>(&'a self, _: LayoutContext<'_>) -> Pass<Commands<'a>> {
            unimplemented!()
        }
    }
}
