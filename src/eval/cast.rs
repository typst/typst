use std::num::NonZeroUsize;

use super::{Regex, Value};
use crate::diag::{with_alternative, StrResult};
use crate::geom::{Corners, Dir, Paint, Sides};
use crate::model::{Content, Group, Layout, LayoutNode, Pattern};
use crate::syntax::Spanned;

/// Cast from a value to a specific type.
pub trait Cast<V = Value>: Sized {
    /// Check whether the value is castable to `Self`.
    fn is(value: &V) -> bool;

    /// Try to cast the value into an instance of `Self`.
    fn cast(value: V) -> StrResult<Self>;
}

/// Implement traits for dynamic types.
macro_rules! dynamic {
    ($type:ty: $name:literal, $($tts:tt)*) => {
        impl $crate::eval::Type for $type {
            const TYPE_NAME: &'static str = $name;
        }

        castable! {
            $type,
            Expected: <Self as $crate::eval::Type>::TYPE_NAME,
            $($tts)*
            @this: Self => this.clone(),
        }

        impl From<$type> for $crate::eval::Value {
            fn from(v: $type) -> Self {
                $crate::eval::Value::Dyn($crate::eval::Dynamic::new(v))
            }
        }
    };
}

/// Make a type castable from a value.
macro_rules! castable {
    ($type:ty: $inner:ty) => {
        impl $crate::eval::Cast<$crate::eval::Value> for $type {
            fn is(value: &$crate::eval::Value) -> bool {
                <$inner>::is(value)
            }

            fn cast(value: $crate::eval::Value) -> $crate::diag::StrResult<Self> {
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
        impl $crate::eval::Cast<$crate::eval::Value> for $type {
            fn is(value: &$crate::eval::Value) -> bool {
                #[allow(unused_variables)]
                match value {
                    $($pattern => true,)*
                    $crate::eval::Value::Dyn(dynamic) => {
                        false $(|| dynamic.is::<$dyn_type>())*
                    }
                    _ => false,
                }
            }

            fn cast(value: $crate::eval::Value) -> $crate::diag::StrResult<Self> {
                let found = match value {
                    $($pattern => return Ok($out),)*
                    $crate::eval::Value::Dyn(dynamic) => {
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

dynamic! {
    Dir: "direction",
}

dynamic! {
    Regex: "regular expression",
}

dynamic! {
    Group: "group",
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
    String,
    Expected: "string",
    Value::Str(string) => string.into(),
}

castable! {
    LayoutNode,
    Expected: "content",
    Value::None => Self::default(),
    Value::Str(text) => Content::Text(text.into()).pack(),
    Value::Content(content) => content.pack(),
}

castable! {
    Pattern,
    Expected: "function, string or regular expression",
    Value::Func(func) => Self::Node(func.node()?),
    Value::Str(text) => Self::text(&text),
    @regex: Regex => Self::Regex(regex.clone()),
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
