//! Syntax trees, parsing and tokenization.

pub mod decoration;
pub mod parsing;
pub mod span;
pub mod tokens;
pub mod tree;

use std::fmt::{self, Debug, Formatter};
use tokens::is_identifier;

/// An identifier as defined by unicode with a few extra permissible characters.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Ident(pub String);

impl Ident {
    /// Create a new identifier from a string checking that it is a valid.
    pub fn new(ident: impl AsRef<str> + Into<String>) -> Option<Self> {
        if is_identifier(ident.as_ref()) {
            Some(Self(ident.into()))
        } else {
            None
        }
    }

    /// Return a reference to the underlying string.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Debug for Ident {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "`{}`", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::span;
    use crate::prelude::*;
    use std::fmt::Debug;

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
}
