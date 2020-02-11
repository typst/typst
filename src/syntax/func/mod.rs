//! Primitives for argument parsing in library functions.

use std::iter::FromIterator;
use crate::error::{Error, Errors};
use super::expr::{Expr, Ident, Tuple, Object, Pair};
use super::span::{Span, Spanned};

pub_use_mod!(maps);
pub_use_mod!(keys);
pub_use_mod!(values);


/// The parsed header of a function.
#[derive(Debug, Clone, PartialEq)]
pub struct FuncHeader {
    /// The function name, that is:
    /// ```typst
    /// [box: w=5cm]
    ///  ^^^
    /// ```
    pub name: Spanned<Ident>,
    /// The arguments passed to the function.
    pub args: FuncArgs,
}

/// The positional and keyword arguments passed to a function.
#[derive(Debug, Clone, PartialEq)]
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
    pub fn add(&mut self, arg: FuncArg) {
        match arg {
            FuncArg::Pos(item) => self.pos.add(item),
            FuncArg::Key(pair) => self.key.add(pair),
        }
    }

    /// Iterate over all arguments.
    pub fn into_iter(self) -> impl Iterator<Item=FuncArg> {
        self.pos.items.into_iter().map(|item| FuncArg::Pos(item))
            .chain(self.key.pairs.into_iter().map(|pair| FuncArg::Key(pair)))
    }
}

impl FromIterator<FuncArg> for FuncArgs {
    fn from_iter<I: IntoIterator<Item=FuncArg>>(iter: I) -> Self {
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
    Pos(Spanned<Expr>),
    /// A keyword argument.
    Key(Pair),
}

impl FuncArg {
    /// The full span of this argument.
    ///
    /// In case of a positional argument this is just the span of the expression
    /// and in case of a keyword argument the combined span of key and value.
    pub fn span(&self) -> Span {
        match self {
            FuncArg::Pos(item) => item.span,
            FuncArg::Key(Pair { key, value }) => Span::merge(key.span, value.span),
        }
    }
}

/// Extra methods on [`Options`](Option) used for argument parsing.
pub trait OptionExt: Sized {
    /// Add an error about a missing argument `arg` with the given span if the
    /// option is `None`.
    fn or_missing(self, errors: &mut Errors, span: Span, arg: &str) -> Self;
}

impl<T> OptionExt for Option<T> {
    fn or_missing(self, errors: &mut Errors, span: Span, arg: &str) -> Self {
        if self.is_none() {
            errors.push(err!(span; "missing argument: {}", arg));
        }
        self
    }
}
