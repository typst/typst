//! Value types for extracting function arguments.

use fontdock::{FontStyle, FontWeight, FontWidth};

use crate::layout::prelude::*;
use crate::length::{Length, ScaleLength};
use crate::paper::Paper;
use crate::Feedback;
use super::expr::*;
use super::span::Spanned;

/// Value types are used to extract values from functions, tuples and
/// objects. They represent the value part of an argument.
///
/// # Example
/// ```typst
/// [func: 12pt, key="these are both values"]
///        ^^^^      ^^^^^^^^^^^^^^^^^^^^^^^
/// ```
pub trait Value: Sized {
    /// Try to parse this value from an expression.
    ///
    /// Returns `None` and generates an appropriate error if the expression is
    /// not valid for this value type
    fn parse(expr: Spanned<Expr>, f: &mut Feedback) -> Option<Self>;
}

impl<V: Value> Value for Spanned<V> {
    fn parse(expr: Spanned<Expr>, f: &mut Feedback) -> Option<Self> {
        let span = expr.span;
        V::parse(expr, f).map(|v| Spanned { v, span })
    }
}

macro_rules! match_value {
    ($type:ty, $name:expr, $($p:pat => $r:expr),* $(,)?) => {
        impl Value for $type {
            fn parse(expr: Spanned<Expr>, f: &mut Feedback) -> Option<Self> {
                #[allow(unreachable_patterns)]
                match expr.v {
                    $($p => Some($r)),*,
                    other => {
                        error!(
                            @f, expr.span,
                            "expected {}, found {}", $name, other.name()
                        );
                        None
                    }
                }
            }
        }
    };
}

match_value!(Expr,   "expression", e => e);
match_value!(Ident,  "identifier", Expr::Ident(i)  => i);
match_value!(String, "string",     Expr::Str(s)    => s);
match_value!(bool,   "bool",       Expr::Bool(b)   => b);
match_value!(f64,    "number",     Expr::Number(n) => n);
match_value!(Length, "length",     Expr::Length(l) => l);
match_value!(Tuple,  "tuple",      Expr::Tuple(t)  => t);
match_value!(Object, "object",     Expr::Object(o) => o);
match_value!(ScaleLength, "number or length",
    Expr::Length(length)    => ScaleLength::Absolute(length),
    Expr::Number(scale) => ScaleLength::Scaled(scale),
);

/// A value type that matches identifiers and strings and implements
/// `Into<String>`.
pub struct StringLike(pub String);

impl From<StringLike> for String {
    fn from(like: StringLike) -> String {
        like.0
    }
}

match_value!(StringLike, "identifier or string",
    Expr::Ident(Ident(s)) => StringLike(s),
    Expr::Str(s) => StringLike(s),
);

macro_rules! ident_value {
    ($type:ty, $name:expr, $parse:expr) => {
        impl Value for $type {
            fn parse(expr: Spanned<Expr>, f: &mut Feedback) -> Option<Self> {
                if let Expr::Ident(ident) = expr.v {
                    let val = $parse(ident.as_str());
                    if val.is_none() {
                        error!(@f, expr.span, "invalid {}", $name);
                    }
                    val
                } else {
                    error!(
                        @f, expr.span,
                        "expected {}, found {}", $name, expr.v.name()
                    );
                    None
                }
            }
        }
    };
}

ident_value!(Dir, "direction", |s| match s {
    "ltr" => Some(LTR),
    "rtl" => Some(RTL),
    "ttb" => Some(TTB),
    "btt" => Some(BTT),
    _ => None,
});

ident_value!(SpecAlign, "alignment", |s| match s {
    "left" => Some(Self::Left),
    "right" => Some(Self::Right),
    "top" => Some(Self::Top),
    "bottom" => Some(Self::Bottom),
    "center" => Some(Self::Center),
    _ => None,
});

ident_value!(FontStyle, "font style", FontStyle::from_name);
ident_value!(Paper, "paper", Paper::from_name);

impl Value for FontWeight {
    fn parse(expr: Spanned<Expr>, f: &mut Feedback) -> Option<Self> {
        match expr.v {
            Expr::Number(weight) => {
                const MIN: u16 = 100;
                const MAX: u16 = 900;

                Some(Self(if weight < MIN as f64 {
                    error!(@f, expr.span, "the minimum font weight is {}", MIN);
                    MIN
                } else if weight > MAX as f64 {
                    error!(@f, expr.span, "the maximum font weight is {}", MAX);
                    MAX
                } else {
                    weight.round() as u16
                }))
            }
            Expr::Ident(ident) => {
                let weight = Self::from_name(ident.as_str());
                if weight.is_none() {
                    error!(@f, expr.span, "invalid font weight");
                }
                weight
            }
            other => {
                error!(
                    @f, expr.span,
                    "expected font weight (name or number), found {}",
                    other.name(),
                );
                None
            }
        }
    }
}

impl Value for FontWidth {
    fn parse(expr: Spanned<Expr>, f: &mut Feedback) -> Option<Self> {
        match expr.v {
            Expr::Number(width) => {
                const MIN: u16 = 1;
                const MAX: u16 = 9;

                Self::new(if width < MIN as f64 {
                    error!(@f, expr.span, "the minimum font width is {}", MIN);
                    MIN
                } else if width > MAX as f64 {
                    error!(@f, expr.span, "the maximum font width is {}", MAX);
                    MAX
                } else {
                    width.round() as u16
                })
            }
            Expr::Ident(ident) => {
                let width = Self::from_name(ident.as_str());
                if width.is_none() {
                    error!(@f, expr.span, "invalid font width");
                }
                width
            }
            other => {
                error!(
                    @f, expr.span,
                    "expected font width (name or number), found {}",
                    other.name(),
                );
                None
            }
        }
    }
}
