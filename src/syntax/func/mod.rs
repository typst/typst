//! Primitives for argument parsing in library functions.

use std::iter::FromIterator;
use crate::diagnostic::{Diagnostic, Diagnostics};
use super::expr::{Expr, Ident, Tuple, Object, Pair};
use super::span::{Span, Spanned};

pub_use_mod!(maps);
pub_use_mod!(keys);
pub_use_mod!(values);

/// An invocation of a function.
#[derive(Debug, Clone, PartialEq)]
pub struct FuncCall<'s> {
    pub header: FuncHeader,
    /// The body as a raw string containing what's inside of the brackets.
    pub body: Option<Spanned<&'s str>>,
}

/// The parsed header of a function (everything in the first set of brackets).
#[derive(Debug, Clone, PartialEq)]
pub struct FuncHeader {
    pub name: Spanned<Ident>,
    pub args: FuncArgs,
}

/// The positional and keyword arguments passed to a function.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct FuncArgs {
    /// The positional arguments.
    pub pos: Tuple,
    /// They keyword arguments.
    pub key: Object,
}

impl FuncArgs {
    /// Create new empty function arguments.
    pub fn new() -> FuncArgs {
        FuncArgs {
            pos: Tuple::new(),
            key: Object::new(),
        }
    }

    /// Add an argument.
    pub fn add(&mut self, arg: Spanned<FuncArg>) {
        match arg.v {
            FuncArg::Pos(item) => self.pos.add(Spanned::new(item, arg.span)),
            FuncArg::Key(pair) => self.key.add(Spanned::new(pair, arg.span)),
        }
    }

    /// Iterate over all arguments.
    pub fn into_iter(self) -> impl Iterator<Item=Spanned<FuncArg>> {
        let pos = self.pos.items.into_iter()
            .map(|spanned| spanned.map(|item| FuncArg::Pos(item)));

        let key = self.key.pairs.into_iter()
        .map(|spanned| spanned.map(|pair| FuncArg::Key(pair)));

        pos.chain(key)
    }
}

impl FromIterator<Spanned<FuncArg>> for FuncArgs {
    fn from_iter<I: IntoIterator<Item=Spanned<FuncArg>>>(iter: I) -> Self {
        let mut args = FuncArgs::new();
        for item in iter.into_iter() {
            args.add(item);
        }
        args
    }
}

/// Either a positional or keyword argument.
#[derive(Debug, Clone, PartialEq)]
pub enum FuncArg {
    /// A positional argument.
    Pos(Expr),
    /// A keyword argument.
    Key(Pair),
}

/// Extra methods on [`Options`](Option) used for argument parsing.
pub trait OptionExt: Sized {
    /// Add an error about a missing argument `arg` with the given span if the
    /// option is `None`.
    fn or_missing(self, diagnostics: &mut Diagnostics, span: Span, arg: &str) -> Self;
}

impl<T> OptionExt for Option<T> {
    fn or_missing(self, diagnostics: &mut Diagnostics, span: Span, arg: &str) -> Self {
        if self.is_none() {
            diagnostics.push(error!(span, "missing argument: {}", arg));
        }
        self
    }
}
