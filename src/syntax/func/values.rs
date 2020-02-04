//! Value types for extracting function arguments.

use std::fmt::{self, Display, Formatter};
use toddle::query::{FontStyle, FontWeight};

use crate::layout::prelude::*;
use crate::size::{Size, ScaleSize};
use crate::style::Paper;
use super::*;

use self::AlignmentValue::*;


/// Value types are used to extract the values of positional and keyword
/// arguments from [`Tuples`](crate::syntax::expr::Tuple) and
/// [`Objects`](crate::syntax::expr::Object). They represent the value part of
/// an argument.
/// ```typst
/// [func: value, key=value]
///        ^^^^^      ^^^^^
/// ```
///
/// # Example implementation
/// An implementation for `bool` might look as follows:
/// ```
/// # use typstc::err;
/// # use typstc::error::Error;
/// # use typstc::syntax::expr::Expr;
/// # use typstc::syntax::func::Value;
/// # use typstc::syntax::span::Spanned;
/// # struct Bool; /*
/// impl Value for bool {
/// # */ impl Value for Bool {
///     fn parse(expr: Spanned<Expr>) -> Result<Self, Error> {
///         match expr.v {
///             # /*
///             Expr::Bool(b) => Ok(b),
///             # */ Expr::Bool(_) => Ok(Bool),
///             other => Err(err!("expected bool, found {}", other.name())),
///         }
///     }
/// }
/// ```
pub trait Value: Sized {
    /// Parse an expression into this value or return an error if the expression
    /// is valid for this value type.
    fn parse(expr: Spanned<Expr>) -> Result<Self, Error>;
}

impl<V: Value> Value for Spanned<V> {
    fn parse(expr: Spanned<Expr>) -> Result<Self, Error> {
        let span = expr.span;
        V::parse(expr).map(|v| Spanned { v, span })
    }
}

/// Implements [`Value`] for types that just need to match on expressions.
macro_rules! value {
    ($type:ty, $name:expr, $($p:pat => $r:expr),* $(,)?) => {
        impl Value for $type {
            fn parse(expr: Spanned<Expr>) -> Result<Self, Error> {
                #[allow(unreachable_patterns)]
                match expr.v {
                    $($p => Ok($r)),*,
                    other => Err(err!("expected {}, found {}",
                                      $name, other.name())),
                }
            }
        }
    };
}

value!(Expr,   "expression", e => e);

value!(Ident,  "identifier", Expr::Ident(i)  => i);
value!(String, "string",     Expr::Str(s)    => s);
value!(f64,    "number",     Expr::Number(n) => n);
value!(bool,   "bool",       Expr::Bool(b)   => b);
value!(Size,   "size",       Expr::Size(s)   => s);
value!(Tuple,  "tuple",      Expr::Tuple(t)  => t);
value!(Object, "object",     Expr::Object(o) => o);

value!(ScaleSize, "number or size",
    Expr::Size(size)    => ScaleSize::Absolute(size),
    Expr::Number(scale) => ScaleSize::Scaled(scale as f32),
);

/// A value type that matches [`Expr::Ident`] and [`Expr::Str`] and implements
/// `Into<String>`.
pub struct StringLike(pub String);

value!(StringLike, "identifier or string",
    Expr::Ident(Ident(s)) => StringLike(s),
    Expr::Str(s) => StringLike(s),
);

impl From<StringLike> for String {
    fn from(like: StringLike) -> String {
        like.0
    }
}

/// A value type that matches the identifier `default` or a value type `V` and
/// implements `Into<Option>` yielding `Option::Some(V)` for a value and
/// `Option::None` for `default`.
///
/// # Example
/// ```
/// # use typstc::syntax::func::{FuncArgs, Defaultable};
/// # use typstc::size::Size;
/// # let mut args = FuncArgs::new();
/// # let mut errors = vec![];
/// args.key.get::<Defaultable<Size>>(&mut errors, "size");
/// ```
/// This will yield.
/// ```typst
/// [func: size=default] => None
/// [func: size=2cm]     => Some(Size::cm(2.0))
/// ```
pub struct Defaultable<V>(pub Option<V>);

impl<V: Value> Value for Defaultable<V> {
    fn parse(expr: Spanned<Expr>) -> Result<Self, Error> {
        Ok(Defaultable(match expr.v {
            Expr::Ident(ident) if ident.as_str() == "default" => None,
            _ => Some(V::parse(expr)?)
        }))
    }
}

impl<V> From<Defaultable<V>> for Option<V> {
    fn from(defaultable: Defaultable<V>) -> Option<V> {
        defaultable.0
    }
}

impl Value for FontStyle {
    fn parse(expr: Spanned<Expr>) -> Result<Self, Error> {
        FontStyle::from_name(Ident::parse(expr)?.as_str())
            .ok_or_else(|| err!("invalid font style"))
    }
}

/// The additional boolean specifies whether a number was clamped into the range
/// 100 - 900 to make it a valid font weight.
impl Value for (FontWeight, bool) {
    fn parse(expr: Spanned<Expr>) -> Result<Self, Error> {
        match expr.v {
            Expr::Number(weight) => {
                let weight = weight.round();

                if weight >= 100.0 && weight <= 900.0 {
                    Ok((FontWeight(weight as i16), false))
                } else {
                    let clamped = weight.min(900.0).max(100.0) as i16;
                    Ok((FontWeight(clamped), true))
                }
            }
            Expr::Ident(id) => {
                FontWeight::from_name(id.as_str())
                    .ok_or_else(|| err!("invalid font weight"))
                    .map(|weight| (weight, false))
            }
            other => Err(err!("expected identifier or number, \
                               found {}", other.name())),
        }
    }
}

impl Value for Paper {
    fn parse(expr: Spanned<Expr>) -> Result<Self, Error> {
        Paper::from_name(Ident::parse(expr)?.as_str())
            .ok_or_else(|| err!("invalid paper type"))
    }
}

impl Value for Direction {
    fn parse(expr: Spanned<Expr>) -> Result<Self, Error> {
        Ok(match Ident::parse(expr)?.as_str() {
            "left-to-right" | "ltr" | "LTR" => LeftToRight,
            "right-to-left" | "rtl" | "RTL" => RightToLeft,
            "top-to-bottom" | "ttb" | "TTB" => TopToBottom,
            "bottom-to-top" | "btt" | "BTT" => BottomToTop,
            _ => return Err(err!("invalid direction"))
        })
    }
}

/// A value type that matches identifiers that are valid alignments like
/// `origin` or `right`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[allow(missing_docs)]
pub enum AlignmentValue {
    /// A generic alignment.
    Align(Alignment),
    Left,
    Top,
    Right,
    Bottom,
}

impl AlignmentValue {
    /// The specific axis this alignment corresponds to. `None` if the alignment
    /// is generic.
    pub fn axis(self) -> Option<SpecificAxis> {
        match self {
            Left | Right => Some(Horizontal),
            Top | Bottom => Some(Vertical),
            Align(_) => None,
        }
    }

    /// The generic version of this alignment on the given axis in the given
    /// system of layouting axes.
    ///
    /// Returns `None` if the alignment is invalid for the given axis.
    pub fn to_generic(self, axes: LayoutAxes, axis: GenericAxis) -> Option<Alignment> {
        let specific = axis.to_specific(axes);
        let positive = axes.get(axis).is_positive();

        // The alignment matching the origin of the positive coordinate direction.
        let start = if positive { Origin } else { End };

        match (self, specific) {
            (Align(alignment), _) => Some(alignment),
            (Left, Horizontal) | (Top, Vertical) => Some(start),
            (Right, Horizontal) | (Bottom, Vertical) => Some(start.inv()),
            _ => None
        }
    }

    /// The specific version of this alignment on the given axis in the given
    /// system of layouting axes.
    pub fn to_specific(self, axes: LayoutAxes, axis: GenericAxis) -> AlignmentValue {
        let direction = axes.get(axis);
        if let Align(alignment) = self {
            match (direction, alignment) {
                (LeftToRight, Origin) | (RightToLeft, End) => Left,
                (LeftToRight, End) | (RightToLeft, Origin) => Right,
                (TopToBottom, Origin) | (BottomToTop, End) => Top,
                (TopToBottom, End) | (BottomToTop, Origin) => Bottom,
                (_, Center) => self,
            }
        } else {
            self
        }
    }
}

impl Value for AlignmentValue {
    fn parse(expr: Spanned<Expr>) -> Result<Self, Error> {
        Ok(match Ident::parse(expr)?.as_str() {
            "origin" => Align(Origin),
            "center" => Align(Center),
            "end"    => Align(End),
            "left"   => Left,
            "top"    => Top,
            "right"  => Right,
            "bottom" => Bottom,
            _ => return Err(err!("invalid alignment"))
        })
    }
}

impl Display for AlignmentValue {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Align(Origin) => write!(f, "origin"),
            Align(Center) => write!(f, "center"),
            Align(End) => write!(f, "end"),
            Left => write!(f, "left"),
            Top => write!(f, "top"),
            Right => write!(f, "right"),
            Bottom => write!(f, "bottom"),
        }
    }
}
