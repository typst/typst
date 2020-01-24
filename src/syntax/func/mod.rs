use crate::error::{Error, Errors};
use super::expr::{Expr, Ident, Tuple, Object, Pair};
use super::span::{Span, Spanned};

pub mod maps;
pub mod keys;
pub mod values;


#[derive(Debug, Clone, PartialEq)]
pub struct FuncHeader {
    pub name: Spanned<Ident>,
    pub args: FuncArgs,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncArgs {
    pub pos: Tuple,
    pub key: Object,
}

impl FuncArgs {
    pub fn new() -> FuncArgs {
        FuncArgs {
            pos: Tuple::new(),
            key: Object::new(),
        }
    }

    /// Add an argument.
    pub fn add(&mut self, arg: FuncArg) {
        match arg {
            FuncArg::Pos(item) => self.add_pos(item),
            FuncArg::Key(pair) => self.add_key_pair(pair),
        }
    }

    /// Add a positional argument.
    pub fn add_pos(&mut self, item: Spanned<Expr>) {
        self.pos.add(item);
    }

    /// Add a keyword argument.
    pub fn add_key(&mut self, key: Spanned<Ident>, value: Spanned<Expr>) {
        self.key.add(key, value);
    }

    /// Add a keyword argument from an existing pair.
    pub fn add_key_pair(&mut self, pair: Pair) {
        self.key.add_pair(pair);
    }

    pub fn into_iter(self) -> impl Iterator<Item=FuncArg> {
        self.pos.items.into_iter().map(|item| FuncArg::Pos(item))
            .chain(self.key.pairs.into_iter().map(|pair| FuncArg::Key(pair)))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FuncArg {
    Pos(Spanned<Expr>),
    Key(Pair),
}

impl FuncArg {
    /// The span or the value or combined span of key and value.
    pub fn span(&self) -> Span {
        match self {
            FuncArg::Pos(item) => item.span,
            FuncArg::Key(Pair { key, value }) => Span::merge(key.span, value.span),
        }
    }
}

pub trait OptionExt: Sized {
    fn or_missing(self, errors: &mut Errors, span: Span, what: &str) -> Self;
}

impl<T> OptionExt for Option<T> {
    fn or_missing(self, errors: &mut Errors, span: Span, what: &str) -> Self {
        if self.is_none() {
            errors.push(err!(span; "missing argument: {}", what));
        }
        self
    }
}
