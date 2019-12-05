//! Spans map elements to the part of source code they originate from.

use std::fmt::{self, Display, Formatter};

/// Annotates a value with the part of the source code it corresponds to.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Spanned<T> {
    pub v: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(v: T, span: Span) -> Spanned<T> {
        Spanned { v, span }
    }

    pub fn value(self) -> T {
        self.v
    }

    pub fn map<F, U>(self, f: F) -> Spanned<U> where F: FnOnce(T) -> U {
        Spanned::new(f(self.v), self.span)
    }
}

impl<T> Display for Spanned<T> where T: std::fmt::Debug {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "({:?}:{})", self.v, self.span)
    }
}

debug_display!(Spanned; T where T: std::fmt::Debug);

/// Describes a slice of source code.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Span {
        Span { start, end }
    }

    pub fn merge(a: Span, b: Span) -> Span {
        Span {
            start: a.start.min(b.start),
            end: a.end.max(b.end),
        }
    }

    pub fn at(index: usize) -> Span {
        Span { start: index, end: index + 1 }
    }

    pub fn pair(&self) -> (usize, usize) {
        (self.start, self.end)
    }

    pub fn expand(&mut self, other: Span) {
        *self = Span::merge(*self, other)
    }
}

impl Display for Span {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "[{}, {}]", self.start, self.end)
    }
}

debug_display!(Span);
