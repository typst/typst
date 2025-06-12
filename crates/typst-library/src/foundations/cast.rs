#[rustfmt::skip]
#[doc(inline)]
pub use typst_macros::{cast, Cast};

use std::borrow::Cow;
use std::fmt::Write;
use std::hash::Hash;
use std::ops::Add;

use ecow::eco_format;
use smallvec::SmallVec;
use typst_syntax::{Span, Spanned, SyntaxMode};
use unicode_math_class::MathClass;

use crate::diag::{At, HintedStrResult, HintedString, SourceResult, StrResult};
use crate::foundations::{
    array, repr, Fold, NativeElement, Packed, Repr, Str, Type, Value,
};

/// Determine details of a type.
///
/// Type casting works as follows:
/// - [`Reflect for T`](Reflect) describes the possible Typst values for `T`
///   (for documentation and autocomplete).
/// - [`IntoValue for T`](IntoValue) is for conversion from `T -> Value`
///   (infallible)
/// - [`FromValue for T`](FromValue) is for conversion from `Value -> T`
///   (fallible).
///
/// We can't use `TryFrom<Value>` due to conflicting impls. We could use
/// `From<T> for Value`, but that inverses the impl and leads to tons of
/// `.into()` all over the place that become hard to decipher.
pub trait Reflect {
    /// Describe what can be cast into this value.
    fn input() -> CastInfo;

    /// Describe what this value can be cast into.
    fn output() -> CastInfo;

    /// Whether the given value can be converted to `T`.
    ///
    /// This exists for performance. The check could also be done through the
    /// [`CastInfo`], but it would be much more expensive (heap allocation +
    /// dynamic checks instead of optimized machine code for each type).
    fn castable(value: &Value) -> bool;

    /// Produce an error message for an unacceptable value type.
    ///
    /// ```ignore
    /// assert_eq!(
    ///   <i64 as Reflect>::error(&Value::None),
    ///   "expected integer, found none",
    /// );
    /// ```
    fn error(found: &Value) -> HintedString {
        Self::input().error(found)
    }
}

impl Reflect for Value {
    fn input() -> CastInfo {
        CastInfo::Any
    }

    fn output() -> CastInfo {
        CastInfo::Any
    }

    fn castable(_: &Value) -> bool {
        true
    }
}

impl<T: Reflect> Reflect for Spanned<T> {
    fn input() -> CastInfo {
        T::input()
    }

    fn output() -> CastInfo {
        T::output()
    }

    fn castable(value: &Value) -> bool {
        T::castable(value)
    }
}

impl<T: NativeElement + Reflect> Reflect for Packed<T> {
    fn input() -> CastInfo {
        T::input()
    }

    fn output() -> CastInfo {
        T::output()
    }

    fn castable(value: &Value) -> bool {
        T::castable(value)
    }
}

impl<T: Reflect> Reflect for StrResult<T> {
    fn input() -> CastInfo {
        T::input()
    }

    fn output() -> CastInfo {
        T::output()
    }

    fn castable(value: &Value) -> bool {
        T::castable(value)
    }
}

impl<T: Reflect> Reflect for HintedStrResult<T> {
    fn input() -> CastInfo {
        T::input()
    }

    fn output() -> CastInfo {
        T::output()
    }

    fn castable(value: &Value) -> bool {
        T::castable(value)
    }
}

impl<T: Reflect> Reflect for SourceResult<T> {
    fn input() -> CastInfo {
        T::input()
    }

    fn output() -> CastInfo {
        T::output()
    }

    fn castable(value: &Value) -> bool {
        T::castable(value)
    }
}

impl<T: Reflect> Reflect for &T {
    fn input() -> CastInfo {
        T::input()
    }

    fn output() -> CastInfo {
        T::output()
    }

    fn castable(value: &Value) -> bool {
        T::castable(value)
    }
}

impl<T: Reflect> Reflect for &mut T {
    fn input() -> CastInfo {
        T::input()
    }

    fn output() -> CastInfo {
        T::output()
    }

    fn castable(value: &Value) -> bool {
        T::castable(value)
    }
}

/// Cast a Rust type into a Typst [`Value`].
///
/// See also: [`Reflect`].
pub trait IntoValue {
    /// Cast this type into a value.
    fn into_value(self) -> Value;
}

impl IntoValue for Value {
    fn into_value(self) -> Value {
        self
    }
}

impl IntoValue for (&Str, &Value) {
    fn into_value(self) -> Value {
        Value::Array(array![self.0.clone(), self.1.clone()])
    }
}

impl<T: IntoValue + Clone> IntoValue for Cow<'_, T> {
    fn into_value(self) -> Value {
        self.into_owned().into_value()
    }
}

impl<T: NativeElement + IntoValue> IntoValue for Packed<T> {
    fn into_value(self) -> Value {
        Value::Content(self.pack())
    }
}

impl<T: IntoValue> IntoValue for Spanned<T> {
    fn into_value(self) -> Value {
        self.v.into_value()
    }
}

/// Cast a Rust type or result into a [`SourceResult<Value>`].
///
/// Converts `T`, [`StrResult<T>`], or [`SourceResult<T>`] into
/// [`SourceResult<Value>`] by `Ok`-wrapping or adding span information.
pub trait IntoResult {
    /// Cast this type into a value.
    fn into_result(self, span: Span) -> SourceResult<Value>;
}

impl<T: IntoValue> IntoResult for T {
    fn into_result(self, _: Span) -> SourceResult<Value> {
        Ok(self.into_value())
    }
}

impl<T: IntoValue> IntoResult for StrResult<T> {
    fn into_result(self, span: Span) -> SourceResult<Value> {
        self.map(IntoValue::into_value).at(span)
    }
}

impl<T: IntoValue> IntoResult for HintedStrResult<T> {
    fn into_result(self, span: Span) -> SourceResult<Value> {
        self.map(IntoValue::into_value).at(span)
    }
}

impl<T: IntoValue> IntoResult for SourceResult<T> {
    fn into_result(self, _: Span) -> SourceResult<Value> {
        self.map(IntoValue::into_value)
    }
}

impl<T: IntoValue> IntoValue for fn() -> T {
    fn into_value(self) -> Value {
        self().into_value()
    }
}

/// Try to cast a Typst [`Value`] into a Rust type.
///
/// See also: [`Reflect`].
pub trait FromValue<V = Value>: Sized + Reflect {
    /// Try to cast the value into an instance of `Self`.
    fn from_value(value: V) -> HintedStrResult<Self>;
}

impl FromValue for Value {
    fn from_value(value: Value) -> HintedStrResult<Self> {
        Ok(value)
    }
}

impl<T: NativeElement + FromValue> FromValue for Packed<T> {
    fn from_value(mut value: Value) -> HintedStrResult<Self> {
        if let Value::Content(content) = value {
            match content.into_packed::<T>() {
                Ok(packed) => return Ok(packed),
                Err(content) => value = Value::Content(content),
            }
        }
        let val = T::from_value(value)?;
        Ok(Packed::new(val))
    }
}

impl<T: FromValue> FromValue<Spanned<Value>> for T {
    fn from_value(value: Spanned<Value>) -> HintedStrResult<Self> {
        T::from_value(value.v)
    }
}

impl<T: FromValue> FromValue<Spanned<Value>> for Spanned<T> {
    fn from_value(value: Spanned<Value>) -> HintedStrResult<Self> {
        let span = value.span;
        T::from_value(value.v).map(|t| Spanned::new(t, span))
    }
}

/// Describes a possible value for a cast.
#[derive(Debug, Clone, PartialEq, Hash, PartialOrd)]
pub enum CastInfo {
    /// Any value is okay.
    Any,
    /// A specific value, plus short documentation for that value.
    Value(Value, &'static str),
    /// Any value of a type.
    Type(Type),
    /// Multiple alternatives.
    Union(Vec<Self>),
}

impl CastInfo {
    /// Produce an error message describing what was expected and what was
    /// found.
    pub fn error(&self, found: &Value) -> HintedString {
        let mut matching_type = false;
        let mut parts = vec![];

        self.walk(|info| match info {
            CastInfo::Any => parts.push("anything".into()),
            CastInfo::Value(value, _) => {
                parts.push(value.repr());
                if value.ty() == found.ty() {
                    matching_type = true;
                }
            }
            CastInfo::Type(ty) => parts.push(eco_format!("{ty}")),
            CastInfo::Union(_) => {}
        });

        let mut msg = String::from("expected ");
        if parts.is_empty() {
            msg.push_str(" nothing");
        }

        msg.push_str(&repr::separated_list(&parts, "or"));

        if !matching_type {
            msg.push_str(", found ");
            write!(msg, "{}", found.ty()).unwrap();
        }

        let mut msg: HintedString = msg.into();

        if let Value::Int(i) = found {
            if !matching_type && parts.iter().any(|p| p == "length") {
                msg.hint(eco_format!("a length needs a unit - did you mean {i}pt?"));
            }
        } else if let Value::Str(s) = found {
            if !matching_type && parts.iter().any(|p| p == "label") {
                if typst_syntax::is_valid_label_literal_id(s) {
                    msg.hint(eco_format!(
                        "use `<{s}>` or `label({})` to create a label",
                        s.repr()
                    ));
                } else {
                    msg.hint(eco_format!("use `label({})` to create a label", s.repr()));
                }
            }
        } else if let Value::Decimal(_) = found {
            if !matching_type && parts.iter().any(|p| p == "float") {
                msg.hint(eco_format!(
                    "if loss of precision is acceptable, explicitly cast the \
                     decimal to a float with `float(value)`"
                ));
            }
        }

        msg
    }

    /// Walk all contained non-union infos.
    pub fn walk<F>(&self, mut f: F)
    where
        F: FnMut(&Self),
    {
        fn inner<F>(info: &CastInfo, f: &mut F)
        where
            F: FnMut(&CastInfo),
        {
            if let CastInfo::Union(infos) = info {
                for child in infos {
                    inner(child, f);
                }
            } else {
                f(info);
            }
        }

        inner(self, &mut f)
    }
}

impl Add for CastInfo {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self::Union(match (self, rhs) {
            (Self::Union(mut lhs), Self::Union(rhs)) => {
                for cast in rhs {
                    if !lhs.contains(&cast) {
                        lhs.push(cast);
                    }
                }
                lhs
            }
            (Self::Union(mut lhs), rhs) => {
                if !lhs.contains(&rhs) {
                    lhs.push(rhs);
                }
                lhs
            }
            (lhs, Self::Union(mut rhs)) => {
                if !rhs.contains(&lhs) {
                    rhs.insert(0, lhs);
                }
                rhs
            }
            (lhs, rhs) => vec![lhs, rhs],
        })
    }
}

/// A container for an argument.
pub trait Container {
    /// The contained type.
    type Inner;
}

impl<T> Container for Option<T> {
    type Inner = T;
}

impl<T> Container for Vec<T> {
    type Inner = T;
}

impl<T, const N: usize> Container for SmallVec<[T; N]> {
    type Inner = T;
}

/// An uninhabitable type.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Never {}

impl Reflect for Never {
    fn input() -> CastInfo {
        CastInfo::Union(vec![])
    }

    fn output() -> CastInfo {
        CastInfo::Union(vec![])
    }

    fn castable(_: &Value) -> bool {
        false
    }
}

impl IntoValue for Never {
    fn into_value(self) -> Value {
        match self {}
    }
}

impl FromValue for Never {
    fn from_value(value: Value) -> HintedStrResult<Self> {
        Err(Self::error(&value))
    }
}

cast! {
    SyntaxMode,
    self => IntoValue::into_value(match self {
        SyntaxMode::Markup => "markup",
        SyntaxMode::Math => "math",
        SyntaxMode::Code => "code",
    }),
    /// Evaluate as markup, as in a Typst file.
    "markup" => SyntaxMode::Markup,
    /// Evaluate as math, as in an equation.
    "math" => SyntaxMode::Math,
    /// Evaluate as code, as after a hash.
    "code" => SyntaxMode::Code,
}

cast! {
    MathClass,
    self => IntoValue::into_value(match self {
        MathClass::Normal => "normal",
        MathClass::Alphabetic => "alphabetic",
        MathClass::Binary => "binary",
        MathClass::Closing => "closing",
        MathClass::Diacritic => "diacritic",
        MathClass::Fence => "fence",
        MathClass::GlyphPart => "glyph-part",
        MathClass::Large => "large",
        MathClass::Opening => "opening",
        MathClass::Punctuation => "punctuation",
        MathClass::Relation => "relation",
        MathClass::Space => "space",
        MathClass::Unary => "unary",
        MathClass::Vary => "vary",
        MathClass::Special => "special",
    }),
    /// The default class for non-special things.
    "normal" => MathClass::Normal,
    /// Punctuation, e.g. a comma.
    "punctuation" => MathClass::Punctuation,
    /// An opening delimiter, e.g. `(`.
    "opening" => MathClass::Opening,
    /// A closing delimiter, e.g. `)`.
    "closing" => MathClass::Closing,
    /// A delimiter that is the same on both sides, e.g. `|`.
    "fence" => MathClass::Fence,
    /// A large operator like `sum`.
    "large" => MathClass::Large,
    /// A relation like `=` or `prec`.
    "relation" => MathClass::Relation,
    /// A unary operator like `not`.
    "unary" => MathClass::Unary,
    /// A binary operator like `times`.
    "binary" => MathClass::Binary,
    /// An operator that can be both unary or binary like `+`.
    "vary" => MathClass::Vary,
}

/// A type that contains a user-visible source portion and something that is
/// derived from it, but not user-visible.
///
/// An example usage would be `source` being a `DataSource` and `derived` a
/// TextMate theme parsed from it. With `Derived`, we can store both parts in
/// the `RawElem::theme` field and get automatic nice `Reflect` and `IntoValue`
/// impls.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Derived<S, D> {
    /// The source portion.
    pub source: S,
    /// The derived portion.
    pub derived: D,
}

impl<S, D> Derived<S, D> {
    /// Create a new instance from the `source` and the `derived` data.
    pub fn new(source: S, derived: D) -> Self {
        Self { source, derived }
    }
}

impl<S: Reflect, D> Reflect for Derived<S, D> {
    fn input() -> CastInfo {
        S::input()
    }

    fn output() -> CastInfo {
        S::output()
    }

    fn castable(value: &Value) -> bool {
        S::castable(value)
    }

    fn error(found: &Value) -> HintedString {
        S::error(found)
    }
}

impl<S: IntoValue, D> IntoValue for Derived<S, D> {
    fn into_value(self) -> Value {
        self.source.into_value()
    }
}

impl<S: Fold, D: Fold> Fold for Derived<S, D> {
    fn fold(self, outer: Self) -> Self {
        Self {
            source: self.source.fold(outer.source),
            derived: self.derived.fold(outer.derived),
        }
    }
}
