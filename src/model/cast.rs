use std::num::NonZeroUsize;
use std::ops::Add;
use std::str::FromStr;

use super::{
    castable, Array, Content, Dict, Func, Label, Regex, Selector, Str, Transform, Value,
};
use crate::diag::StrResult;
use crate::doc::{Destination, Lang, Location, Region};
use crate::font::{FontStretch, FontStyle, FontWeight};
use crate::geom::{
    Axes, Color, Corners, Dir, GenAlign, Get, Length, Paint, PartialStroke, Point, Ratio,
    Rel, Sides, Smart,
};
use crate::syntax::Spanned;
use crate::util::EcoString;

/// Cast from a value to a specific type.
pub trait Cast<V = Value>: Sized {
    /// Check whether the value is castable to `Self`.
    fn is(value: &V) -> bool;

    /// Try to cast the value into an instance of `Self`.
    fn cast(value: V) -> StrResult<Self>;

    /// Describe the acceptable values.
    fn describe() -> CastInfo;

    /// Produce an error for an inacceptable value.
    fn error(value: Value) -> StrResult<Self> {
        Err(Self::describe().error(&value))
    }
}

/// Describes a possible value for a cast.
#[derive(Debug, Clone, Hash)]
pub enum CastInfo {
    /// Any value is okay.
    Any,
    /// A specific value, plus short documentation for that value.
    Value(Value, &'static str),
    /// Any value of a type.
    Type(&'static str),
    /// Multiple alternatives.
    Union(Vec<Self>),
}

impl CastInfo {
    /// Produce an error message describing what was expected and what was
    /// found.
    pub fn error(&self, found: &Value) -> EcoString {
        fn accumulate(
            info: &CastInfo,
            found: &Value,
            parts: &mut Vec<EcoString>,
            matching_type: &mut bool,
        ) {
            match info {
                CastInfo::Any => parts.push("anything".into()),
                CastInfo::Value(value, _) => {
                    parts.push(value.repr().into());
                    if value.type_name() == found.type_name() {
                        *matching_type = true;
                    }
                }
                CastInfo::Type(ty) => parts.push((*ty).into()),
                CastInfo::Union(options) => {
                    for option in options {
                        accumulate(option, found, parts, matching_type);
                    }
                }
            }
        }

        let mut matching_type = false;
        let mut parts = vec![];
        accumulate(self, found, &mut parts, &mut matching_type);

        let mut msg = String::from("expected ");
        if parts.is_empty() {
            msg.push_str(" nothing");
        }

        crate::diag::comma_list(&mut msg, &parts, "or");

        if !matching_type {
            msg.push_str(", found ");
            msg.push_str(found.type_name());
        }

        msg.into()
    }
}

impl Add for CastInfo {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self::Union(match (self, rhs) {
            (Self::Union(mut lhs), Self::Union(rhs)) => {
                lhs.extend(rhs);
                lhs
            }
            (Self::Union(mut lhs), rhs) => {
                lhs.push(rhs);
                lhs
            }
            (lhs, Self::Union(mut rhs)) => {
                rhs.insert(0, lhs);
                rhs
            }
            (lhs, rhs) => vec![lhs, rhs],
        })
    }
}

impl Cast for Value {
    fn is(_: &Value) -> bool {
        true
    }

    fn cast(value: Value) -> StrResult<Self> {
        Ok(value)
    }

    fn describe() -> CastInfo {
        CastInfo::Any
    }
}

impl<T: Cast> Cast<Spanned<Value>> for T {
    fn is(value: &Spanned<Value>) -> bool {
        T::is(&value.v)
    }

    fn cast(value: Spanned<Value>) -> StrResult<Self> {
        T::cast(value.v)
    }

    fn describe() -> CastInfo {
        T::describe()
    }
}

impl<T: Cast> Cast<Spanned<Value>> for Spanned<T> {
    fn is(value: &Spanned<Value>) -> bool {
        T::is(&value.v)
    }

    fn cast(value: Spanned<Value>) -> StrResult<Self> {
        let span = value.span;
        T::cast(value.v).map(|t| Spanned::new(t, span))
    }

    fn describe() -> CastInfo {
        T::describe()
    }
}

castable! {
    Dir: "direction",
}

castable! {
    GenAlign: "alignment",
}

castable! {
    Regex: "regular expression",
}

castable! {
    Selector: "selector",
    text: EcoString => Self::text(&text),
    label: Label => Self::Label(label),
    func: Func => func.select(None)?,
    regex: Regex => Self::Regex(regex),
}

castable! {
    Axes<GenAlign>: "2d alignment",
}

castable! {
    PartialStroke: "stroke",
    thickness: Length => Self {
        paint: Smart::Auto,
        thickness: Smart::Custom(thickness),
    },
    color: Color => Self {
        paint: Smart::Custom(color.into()),
        thickness: Smart::Auto,
    },
}

castable! {
    u32,
    int: i64 => int.try_into().map_err(|_| {
        if int < 0 {
            "number must be at least zero"
        } else {
            "number too large"
        }
    })?,
}

castable! {
    usize,
    int: i64 => int.try_into().map_err(|_| {
        if int < 0 {
            "number must be at least zero"
        } else {
            "number too large"
        }
    })?,
}

castable! {
    NonZeroUsize,
    int: i64 => int
        .try_into()
        .and_then(|int: usize| int.try_into())
        .map_err(|_| if int <= 0 {
            "number must be positive"
        } else {
            "number too large"
        })?,
}

castable! {
    Paint,
    color: Color => Self::Solid(color),
}

castable! {
    EcoString,
    string: Str => string.into(),
}

castable! {
    String,
    string: Str => string.into(),
}

castable! {
    Transform,
    content: Content => Self::Content(content),
    func: Func => {
        if func.argc().map_or(false, |count| count != 1) {
            Err("function must have exactly one parameter")?
        }
        Self::Func(func)
    },
}

castable! {
    Axes<Option<GenAlign>>,
    align: GenAlign => {
        let mut aligns = Axes::default();
        aligns.set(align.axis(), Some(align));
        aligns
    },
    aligns: Axes<GenAlign> => aligns.map(Some),
}

castable! {
    Axes<Rel<Length>>,
    array: Array => {
        let mut iter = array.into_iter();
        match (iter.next(), iter.next(), iter.next()) {
            (Some(a), Some(b), None) => Axes::new(a.cast()?, b.cast()?),
            _ => Err("point array must contain exactly two entries")?,
        }
    },
}

castable! {
    Location,
    mut dict: Dict => {
        let page = dict.take("page")?.cast()?;
        let x: Length = dict.take("x")?.cast()?;
        let y: Length = dict.take("y")?.cast()?;
        dict.finish(&["page", "x", "y"])?;
        Self { page, pos: Point::new(x.abs, y.abs) }
    },
}

castable! {
    Destination,
    loc: Location => Self::Internal(loc),
    string: EcoString => Self::Url(string),
}

castable! {
    FontStyle,
    /// The default, typically upright style.
    "normal" => Self::Normal,
    /// A cursive style with custom letterform.
    "italic" => Self::Italic,
    /// Just a slanted version of the normal style.
    "oblique" => Self::Oblique,
}

castable! {
    FontWeight,
    v: i64 => Self::from_number(v.clamp(0, u16::MAX as i64) as u16),
    /// Thin weight (100).
    "thin" => Self::THIN,
    /// Extra light weight (200).
    "extralight" => Self::EXTRALIGHT,
    /// Light weight (300).
    "light" => Self::LIGHT,
    /// Regular weight (400).
    "regular" => Self::REGULAR,
    /// Medium weight (500).
    "medium" => Self::MEDIUM,
    /// Semibold weight (600).
    "semibold" => Self::SEMIBOLD,
    /// Bold weight (700).
    "bold" => Self::BOLD,
    /// Extrabold weight (800).
    "extrabold" => Self::EXTRABOLD,
    /// Black weight (900).
    "black" => Self::BLACK,
}

castable! {
    FontStretch,
    v: Ratio => Self::from_ratio(v.get() as f32),
}

castable! {
    Lang,
    string: EcoString => Self::from_str(&string)?,
}

castable! {
    Region,
    string: EcoString => Self::from_str(&string)?,
}

/// Castable from [`Value::None`].
pub struct NoneValue;

impl Cast for NoneValue {
    fn is(value: &Value) -> bool {
        matches!(value, Value::None)
    }

    fn cast(value: Value) -> StrResult<Self> {
        match value {
            Value::None => Ok(Self),
            _ => <Self as Cast>::error(value),
        }
    }

    fn describe() -> CastInfo {
        CastInfo::Type("none")
    }
}

impl<T: Cast> Cast for Option<T> {
    fn is(value: &Value) -> bool {
        matches!(value, Value::None) || T::is(value)
    }

    fn cast(value: Value) -> StrResult<Self> {
        match value {
            Value::None => Ok(None),
            v if T::is(&v) => Ok(Some(T::cast(v)?)),
            _ => <Self as Cast>::error(value),
        }
    }

    fn describe() -> CastInfo {
        T::describe() + CastInfo::Type("none")
    }
}

/// Castable from [`Value::Auto`].
pub struct AutoValue;

impl Cast for AutoValue {
    fn is(value: &Value) -> bool {
        matches!(value, Value::Auto)
    }

    fn cast(value: Value) -> StrResult<Self> {
        match value {
            Value::Auto => Ok(Self),
            _ => <Self as Cast>::error(value),
        }
    }

    fn describe() -> CastInfo {
        CastInfo::Type("auto")
    }
}

impl<T: Cast> Cast for Smart<T> {
    fn is(value: &Value) -> bool {
        matches!(value, Value::Auto) || T::is(value)
    }

    fn cast(value: Value) -> StrResult<Self> {
        match value {
            Value::Auto => Ok(Self::Auto),
            v if T::is(&v) => Ok(Self::Custom(T::cast(v)?)),
            _ => <Self as Cast>::error(value),
        }
    }

    fn describe() -> CastInfo {
        T::describe() + CastInfo::Type("auto")
    }
}

impl<T> Cast for Sides<T>
where
    T: Cast + Default + Copy,
{
    fn is(value: &Value) -> bool {
        matches!(value, Value::Dict(_)) || T::is(value)
    }

    fn cast(mut value: Value) -> StrResult<Self> {
        if let Value::Dict(dict) = &mut value {
            let mut take = |key| dict.take(key).ok().map(T::cast).transpose();

            let rest = take("rest")?;
            let x = take("x")?.or(rest);
            let y = take("y")?.or(rest);
            let sides = Sides {
                left: take("left")?.or(x),
                top: take("top")?.or(y),
                right: take("right")?.or(x),
                bottom: take("bottom")?.or(y),
            };

            dict.finish(&["left", "top", "right", "bottom", "x", "y", "rest"])?;

            Ok(sides.map(Option::unwrap_or_default))
        } else if T::is(&value) {
            Ok(Self::splat(T::cast(value)?))
        } else {
            <Self as Cast>::error(value)
        }
    }

    fn describe() -> CastInfo {
        T::describe() + CastInfo::Type("dictionary")
    }
}

impl<T> Cast for Corners<T>
where
    T: Cast + Default + Copy,
{
    fn is(value: &Value) -> bool {
        matches!(value, Value::Dict(_)) || T::is(value)
    }

    fn cast(mut value: Value) -> StrResult<Self> {
        if let Value::Dict(dict) = &mut value {
            let mut take = |key| dict.take(key).ok().map(T::cast).transpose();

            let rest = take("rest")?;
            let left = take("left")?.or(rest);
            let top = take("top")?.or(rest);
            let right = take("right")?.or(rest);
            let bottom = take("bottom")?.or(rest);
            let corners = Corners {
                top_left: take("top-left")?.or(top).or(left),
                top_right: take("top-right")?.or(top).or(right),
                bottom_right: take("bottom-right")?.or(bottom).or(right),
                bottom_left: take("bottom-left")?.or(bottom).or(left),
            };

            dict.finish(&[
                "top-left",
                "top-right",
                "bottom-right",
                "bottom-left",
                "left",
                "top",
                "right",
                "bottom",
                "rest",
            ])?;

            Ok(corners.map(Option::unwrap_or_default))
        } else if T::is(&value) {
            Ok(Self::splat(T::cast(value)?))
        } else {
            <Self as Cast>::error(value)
        }
    }

    fn describe() -> CastInfo {
        T::describe() + CastInfo::Type("dictionary")
    }
}
