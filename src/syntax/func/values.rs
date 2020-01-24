use std::marker::PhantomData;
use toddle::query::{FontStyle, FontWeight};

use crate::layout::prelude::*;
use crate::size::ScaleSize;
use crate::style::Paper;
use super::*;

use AlignmentValue::*;


pub trait Value {
    type Output;

    fn parse(expr: Spanned<Expr>) -> Result<Self::Output, Error>;
}

impl<V: Value> Value for Spanned<V> {
    type Output = Spanned<V::Output>;

    fn parse(expr: Spanned<Expr>) -> Result<Self::Output, Error> {
        let span = expr.span;
        V::parse(expr).map(|v| Spanned { v, span })
    }
}

macro_rules! value {
    ($type:ty, $output:ty, $name:expr, $($p:pat => $r:expr),* $(,)?) => {
        impl Value for $type {
            type Output = $output;

            fn parse(expr: Spanned<Expr>) -> Result<Self::Output, Error> {
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

value!(Expr,   Self, "expression", e => e);

value!(Ident,  Self, "identifier", Expr::Ident(i)  => i);
value!(String, Self, "string",     Expr::Str(s)    => s);
value!(f64,    Self, "number",     Expr::Number(n) => n);
value!(bool,   Self, "bool",       Expr::Bool(b)   => b);
value!(Size,   Self, "size",       Expr::Size(s)   => s);
value!(Tuple,  Self, "tuple",      Expr::Tuple(t)  => t);
value!(Object, Self, "object",     Expr::Object(o) => o);

value!(ScaleSize, Self, "number or size",
    Expr::Size(size)    => ScaleSize::Absolute(size),
    Expr::Number(scale) => ScaleSize::Scaled(scale as f32),
);

pub struct StringLike;

value!(StringLike, String, "identifier or string",
    Expr::Ident(Ident(s)) => s,
    Expr::Str(s) => s,
);

pub struct Defaultable<T>(PhantomData<T>);

impl<T: Value> Value for Defaultable<T> {
    type Output = Option<T::Output>;

    fn parse(expr: Spanned<Expr>) -> Result<Self::Output, Error> {
        match expr.v {
            Expr::Ident(ident) if ident.as_str() == "default" => Ok(None),
            _ => T::parse(expr).map(Some)
        }
    }
}

impl Value for Direction {
    type Output = Self;

    fn parse(expr: Spanned<Expr>) -> Result<Self::Output, Error> {
        Ok(match Ident::parse(expr)?.as_str() {
            "left-to-right" | "ltr" | "LTR" => Direction::LeftToRight,
            "right-to-left" | "rtl" | "RTL" => Direction::RightToLeft,
            "top-to-bottom" | "ttb" | "TTB" => Direction::TopToBottom,
            "bottom-to-top" | "btt" | "BTT" => Direction::BottomToTop,
            other => return Err(err!("invalid direction"))
        })
    }
}

impl Value for FontStyle {
    type Output = Self;

    fn parse(expr: Spanned<Expr>) -> Result<Self::Output, Error> {
        FontStyle::from_str(Ident::parse(expr)?.as_str())
            .ok_or_else(|| err!("invalid font style"))
    }
}

impl Value for FontWeight {
    type Output = (Self, bool);

    fn parse(expr: Spanned<Expr>) -> Result<Self::Output, Error> {
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
                FontWeight::from_str(id.as_str())
                    .ok_or_else(|| err!("invalid font weight"))
                    .map(|weight| (weight, false))
            }
            other => Err(err!("expected identifier or number, \
                               found {}", other.name())),
        }
    }
}

impl Value for Paper {
    type Output = Self;

    fn parse(expr: Spanned<Expr>) -> Result<Self::Output, Error> {
        Paper::from_str(Ident::parse(expr)?.as_str())
            .ok_or_else(|| err!("invalid paper type"))
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum AlignmentValue {
    Align(Alignment),
    Left,
    Top,
    Right,
    Bottom,
}

impl AlignmentValue {
    /// The generic axis this alignment corresponds to in the given system of
    /// layouting axes. `None` if the alignment is generic.
    pub fn axis(self, axes: LayoutAxes) -> Option<GenericAxis> {
        match self {
            Left | Right => Some(Horizontal.to_generic(axes)),
            Top | Bottom => Some(Vertical.to_generic(axes)),
            Align(_) => None,
        }
    }

    /// The generic version of this alignment in the given system of layouting
    /// axes.
    ///
    /// Returns `None` if the alignment is invalid for the given axis.
    pub fn to_generic(self, axes: LayoutAxes, axis: GenericAxis) -> Option<Alignment> {
        let specific = axis.to_specific(axes);
        let start = match axes.get(axis).is_positive() {
            true => Origin,
            false => End,
        };

        match (self, specific) {
            (Align(alignment), _) => Some(alignment),
            (Left, Horizontal) | (Top, Vertical) => Some(start),
            (Right, Horizontal) | (Bottom, Vertical) => Some(start.inv()),
            _ => None
        }
    }

    /// The specific version of this alignment in the given system of layouting
    /// axes.
    pub fn to_specific(self, axes: LayoutAxes, axis: SpecificAxis) -> AlignmentValue {
        let direction = axes.get_specific(axis);
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
    type Output = Self;

    fn parse(expr: Spanned<Expr>) -> Result<Self::Output, Error> {
        Ok(match Ident::parse(expr)?.as_str() {
            "origin" => Align(Origin),
            "center" => Align(Center),
            "end"    => Align(End),
            "left"   => Left,
            "top"    => Top,
            "right"  => Right,
            "bottom" => Bottom,
            other => return Err(err!("invalid alignment"))
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
