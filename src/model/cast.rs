use std::num::NonZeroUsize;
use std::str::FromStr;

use super::{Content, Regex, Selector, Transform, Value};
use crate::diag::{with_alternative, StrResult};
use crate::doc::{Destination, Lang, Location, Region};
use crate::font::{FontStretch, FontStyle, FontWeight};
use crate::geom::{
    Axes, Corners, Dir, GenAlign, Get, Length, Paint, PartialStroke, Point, Rel, Sides,
};
use crate::syntax::Spanned;
use crate::util::EcoString;

/// Cast from a value to a specific type.
pub trait Cast<V = Value>: Sized {
    /// Check whether the value is castable to `Self`.
    fn is(value: &V) -> bool;

    /// Try to cast the value into an instance of `Self`.
    fn cast(value: V) -> StrResult<Self>;
}

/// Implement traits for dynamic types.
#[macro_export]
#[doc(hidden)]
macro_rules! __dynamic {
    ($type:ty: $name:literal, $($tts:tt)*) => {
        impl $crate::model::Type for $type {
            const TYPE_NAME: &'static str = $name;
        }

        castable! {
            $type,
            Expected: <Self as $crate::model::Type>::TYPE_NAME,
            $($tts)*
            @this: Self => this.clone(),
        }

        impl From<$type> for $crate::model::Value {
            fn from(v: $type) -> Self {
                $crate::model::Value::Dyn($crate::model::Dynamic::new(v))
            }
        }
    };
}

#[doc(inline)]
pub use crate::__dynamic as dynamic;

/// Make a type castable from a value.
#[macro_export]
#[doc(hidden)]
macro_rules! __castable {
    ($type:ty: $inner:ty) => {
        impl $crate::model::Cast<$crate::model::Value> for $type {
            fn is(value: &$crate::model::Value) -> bool {
                <$inner>::is(value)
            }

            fn cast(value: $crate::model::Value) -> $crate::diag::StrResult<Self> {
                <$inner>::cast(value).map(Self)
            }
        }
    };

    (
        $type:ty,
        Expected: $expected:expr,
        $($pattern:pat => $out:expr,)*
        $(@$dyn_in:ident: $dyn_type:ty => $dyn_out:expr,)*
    ) => {
        #[allow(unreachable_patterns)]
        impl $crate::model::Cast<$crate::model::Value> for $type {
            fn is(value: &$crate::model::Value) -> bool {
                #[allow(unused_variables)]
                match value {
                    $($pattern => true,)*
                    $crate::model::Value::Dyn(dynamic) => {
                        false $(|| dynamic.is::<$dyn_type>())*
                    }
                    _ => false,
                }
            }

            fn cast(value: $crate::model::Value) -> $crate::diag::StrResult<Self> {
                let found = match value {
                    $($pattern => return Ok($out),)*
                    $crate::model::Value::Dyn(dynamic) => {
                        $(if let Some($dyn_in) = dynamic.downcast::<$dyn_type>() {
                            return Ok($dyn_out);
                        })*
                        dynamic.type_name()
                    }
                    v => v.type_name(),
                };

                Err(format!("expected {}, found {}", $expected, found))
            }
        }
    };
}

#[doc(inline)]
pub use crate::__castable as castable;

impl Cast for Value {
    fn is(_: &Value) -> bool {
        true
    }

    fn cast(value: Value) -> StrResult<Self> {
        Ok(value)
    }
}

impl<T: Cast> Cast<Spanned<Value>> for T {
    fn is(value: &Spanned<Value>) -> bool {
        T::is(&value.v)
    }

    fn cast(value: Spanned<Value>) -> StrResult<Self> {
        T::cast(value.v)
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
}

castable! {
    usize,
    Expected: "non-negative integer",
    Value::Int(int) => int.try_into().map_err(|_| {
        if int < 0 {
            "must be at least zero"
        } else {
            "number too large"
        }
    })?,
}

castable! {
    NonZeroUsize,
    Expected: "positive integer",
    Value::Int(int) => int
        .try_into()
        .and_then(|int: usize| int.try_into())
        .map_err(|_| if int <= 0 {
            "must be positive"
        } else {
            "number too large"
        })?,
}

castable! {
    Paint,
    Expected: "color",
    Value::Color(color) => Paint::Solid(color),
}

castable! {
    EcoString,
    Expected: "string",
    Value::Str(str) => str.into(),
}

castable! {
    String,
    Expected: "string",
    Value::Str(string) => string.into(),
}

dynamic! {
    Regex: "regular expression",
}

dynamic! {
    Selector: "selector",
    Value::Str(text) => Self::text(&text),
    Value::Label(label) => Self::Label(label),
    Value::Func(func) => func.select(None)?,
    @regex: Regex => Self::Regex(regex.clone()),
}

castable! {
    Transform,
    Expected: "content or function",
    Value::None => Self::Content(Content::empty()),
    Value::Str(text) => Self::Content(item!(text)(text.into())),
    Value::Content(content) => Self::Content(content),
    Value::Func(func) => {
        if func.argc().map_or(false, |count| count != 1) {
            Err("function must have exactly one parameter")?
        }
        Self::Func(func)
    },
}

dynamic! {
    Dir: "direction",
}

dynamic! {
    GenAlign: "alignment",
}

dynamic! {
    Axes<GenAlign>: "2d alignment",
}

castable! {
    Axes<Option<GenAlign>>,
    Expected: "1d or 2d alignment",
    @align: GenAlign => {
        let mut aligns = Axes::default();
        aligns.set(align.axis(), Some(*align));
        aligns
    },
    @aligns: Axes<GenAlign> => aligns.map(Some),
}

dynamic! {
    PartialStroke: "stroke",
    Value::Length(thickness) => Self {
        paint: Smart::Auto,
        thickness: Smart::Custom(thickness),
    },
    Value::Color(color) => Self {
        paint: Smart::Custom(color.into()),
        thickness: Smart::Auto,
    },
}

castable! {
    Axes<Rel<Length>>,
    Expected: "array of two relative lengths",
    Value::Array(array) => {
        let mut iter = array.into_iter();
        match (iter.next(), iter.next(), iter.next()) {
            (Some(a), Some(b), None) => Axes::new(a.cast()?, b.cast()?),
            _ => Err("point array must contain exactly two entries")?,
        }
    },
}

castable! {
    Destination,
    Expected: "string or dictionary with `page`, `x`, and `y` keys",
    Value::Str(string) => Self::Url(string.into()),
    Value::Dict(dict) => {
        let page = dict.get("page")?.clone().cast()?;
        let x: Length = dict.get("x")?.clone().cast()?;
        let y: Length = dict.get("y")?.clone().cast()?;
        Self::Internal(Location { page, pos: Point::new(x.abs, y.abs) })
    },
}

castable! {
    FontStyle,
    Expected: "string",
    Value::Str(string) => match string.as_str() {
        "normal" => Self::Normal,
        "italic" => Self::Italic,
        "oblique" => Self::Oblique,
        _ => Err(r#"expected "normal", "italic" or "oblique""#)?,
    },
}

castable! {
    FontWeight,
    Expected: "integer or string",
    Value::Int(v) => Value::Int(v)
        .cast::<usize>()?
        .try_into()
        .map_or(Self::BLACK, Self::from_number),
    Value::Str(string) => match string.as_str() {
        "thin" => Self::THIN,
        "extralight" => Self::EXTRALIGHT,
        "light" => Self::LIGHT,
        "regular" => Self::REGULAR,
        "medium" => Self::MEDIUM,
        "semibold" => Self::SEMIBOLD,
        "bold" => Self::BOLD,
        "extrabold" => Self::EXTRABOLD,
        "black" => Self::BLACK,
        _ => Err("unknown font weight")?,
    },
}

castable! {
    FontStretch,
    Expected: "ratio",
    Value::Ratio(v) => Self::from_ratio(v.get() as f32),
}

castable! {
    Lang,
    Expected: "string",
    Value::Str(string) => Self::from_str(&string)?,
}

castable! {
    Region,
    Expected: "string",
    Value::Str(string) => Self::from_str(&string)?,
}

impl<T: Cast> Cast for Option<T> {
    fn is(value: &Value) -> bool {
        matches!(value, Value::None) || T::is(value)
    }

    fn cast(value: Value) -> StrResult<Self> {
        match value {
            Value::None => Ok(None),
            v => T::cast(v).map(Some).map_err(|msg| with_alternative(msg, "none")),
        }
    }
}

/// A value that can be automatically determined.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Smart<T> {
    /// The value should be determined smartly based on the circumstances.
    Auto,
    /// A specific value.
    Custom(T),
}

impl<T> Smart<T> {
    /// Map the contained custom value with `f`.
    pub fn map<F, U>(self, f: F) -> Smart<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Self::Auto => Smart::Auto,
            Self::Custom(x) => Smart::Custom(f(x)),
        }
    }

    /// Keeps `self` if it contains a custom value, otherwise returns `other`.
    pub fn or(self, other: Smart<T>) -> Self {
        match self {
            Self::Custom(x) => Self::Custom(x),
            Self::Auto => other,
        }
    }

    /// Returns the contained custom value or a provided default value.
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Self::Auto => default,
            Self::Custom(x) => x,
        }
    }

    /// Returns the contained custom value or computes a default value.
    pub fn unwrap_or_else<F>(self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        match self {
            Self::Auto => f(),
            Self::Custom(x) => x,
        }
    }

    /// Returns the contained custom value or the default value.
    pub fn unwrap_or_default(self) -> T
    where
        T: Default,
    {
        self.unwrap_or_else(T::default)
    }
}

impl<T> Default for Smart<T> {
    fn default() -> Self {
        Self::Auto
    }
}

impl<T: Cast> Cast for Smart<T> {
    fn is(value: &Value) -> bool {
        matches!(value, Value::Auto) || T::is(value)
    }

    fn cast(value: Value) -> StrResult<Self> {
        match value {
            Value::Auto => Ok(Self::Auto),
            v => T::cast(v)
                .map(Self::Custom)
                .map_err(|msg| with_alternative(msg, "auto")),
        }
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
            let mut take = |key| dict.take(key).map(T::cast).transpose();

            let rest = take("rest")?;
            let x = take("x")?.or(rest);
            let y = take("y")?.or(rest);
            let sides = Sides {
                left: take("left")?.or(x),
                top: take("top")?.or(y),
                right: take("right")?.or(x),
                bottom: take("bottom")?.or(y),
            };

            if let Some((key, _)) = dict.iter().next() {
                return Err(format!("unexpected key {key:?}"));
            }

            Ok(sides.map(Option::unwrap_or_default))
        } else {
            T::cast(value).map(Self::splat).map_err(|msg| {
                with_alternative(
                    msg,
                    "dictionary with any of \
                     `left`, `top`, `right`, `bottom`, \
                     `x`, `y`, or `rest` as keys",
                )
            })
        }
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
            let mut take = |key| dict.take(key).map(T::cast).transpose();

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

            if let Some((key, _)) = dict.iter().next() {
                return Err(format!("unexpected key {key:?}"));
            }

            Ok(corners.map(Option::unwrap_or_default))
        } else {
            T::cast(value).map(Self::splat).map_err(|msg| {
                with_alternative(
                    msg,
                    "dictionary with any of \
                     `top-left`, `top-right`, `bottom-right`, `bottom-left`, \
                     `left`, `top`, `right`, `bottom`, or `rest` as keys",
                )
            })
        }
    }
}
