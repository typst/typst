use std::any::{Any, TypeId};
use std::cmp::Ordering;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use ecow::{eco_format, EcoString};
use serde::de::value::{MapAccessDeserializer, SeqAccessDeserializer};
use serde::de::{Error, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use typst_syntax::{ast, Span};
use typst_utils::ArcExt;

use crate::diag::{DeprecationSink, HintedStrResult, HintedString, StrResult};
use crate::foundations::{
    fields, ops, repr, Args, Array, AutoValue, Bytes, CastInfo, Content, Datetime,
    Decimal, Dict, Duration, Fold, FromValue, Func, IntoValue, Label, Module,
    NativeElement, NativeType, NoneValue, Reflect, Repr, Resolve, Scope, Str, Styles,
    Symbol, SymbolElem, Type, Version,
};
use crate::layout::{Abs, Angle, Em, Fr, Length, Ratio, Rel};
use crate::text::{RawContent, RawElem, TextElem};
use crate::visualize::{Color, Gradient, Tiling};

/// A computational value.
#[derive(Default, Clone)]
pub enum Value {
    /// The value that indicates the absence of a meaningful value.
    #[default]
    None,
    /// A value that indicates some smart default behaviour.
    Auto,
    /// A boolean: `true, false`.
    Bool(bool),
    /// An integer: `120`.
    Int(i64),
    /// A floating-point number: `1.2`, `10e-4`.
    Float(f64),
    /// A length: `12pt`, `3cm`, `1.5em`, `1em - 2pt`.
    Length(Length),
    /// An angle: `1.5rad`, `90deg`.
    Angle(Angle),
    /// A ratio: `50%`.
    Ratio(Ratio),
    /// A relative length, combination of a ratio and a length: `20% + 5cm`.
    Relative(Rel<Length>),
    /// A fraction: `1fr`.
    Fraction(Fr),
    /// A color value: `#f79143ff`.
    Color(Color),
    /// A gradient value: `gradient.linear(...)`.
    Gradient(Gradient),
    /// A tiling fill: `tiling(...)`.
    Tiling(Tiling),
    /// A symbol: `arrow.l`.
    Symbol(Symbol),
    /// A version.
    Version(Version),
    /// A string: `"string"`.
    Str(Str),
    /// Raw bytes.
    Bytes(Bytes),
    /// A label: `<intro>`.
    Label(Label),
    /// A datetime
    Datetime(Datetime),
    /// A decimal value: `decimal("123.4500")`
    Decimal(Decimal),
    /// A duration
    Duration(Duration),
    /// A content value: `[*Hi* there]`.
    Content(Content),
    // Content styles.
    Styles(Styles),
    /// An array of values: `(1, "hi", 12cm)`.
    Array(Array),
    /// A dictionary value: `(a: 1, b: "hi")`.
    Dict(Dict),
    /// An executable function.
    Func(Func),
    /// Captured arguments to a function.
    Args(Args),
    /// A type.
    Type(Type),
    /// A module.
    Module(Module),
    /// A dynamic value.
    Dyn(Dynamic),
}

impl Value {
    /// Create a new dynamic value.
    pub fn dynamic<T>(any: T) -> Self
    where
        T: Debug + Repr + NativeType + PartialEq + Hash + Sync + Send + 'static,
    {
        Self::Dyn(Dynamic::new(any))
    }

    /// Create a numeric value from a number with a unit.
    pub fn numeric(pair: (f64, ast::Unit)) -> Self {
        let (v, unit) = pair;
        match unit {
            ast::Unit::Pt => Abs::pt(v).into_value(),
            ast::Unit::Mm => Abs::mm(v).into_value(),
            ast::Unit::Cm => Abs::cm(v).into_value(),
            ast::Unit::In => Abs::inches(v).into_value(),
            ast::Unit::Rad => Angle::rad(v).into_value(),
            ast::Unit::Deg => Angle::deg(v).into_value(),
            ast::Unit::Em => Em::new(v).into_value(),
            ast::Unit::Fr => Fr::new(v).into_value(),
            ast::Unit::Percent => Ratio::new(v / 100.0).into_value(),
        }
    }

    /// The type of this value.
    pub fn ty(&self) -> Type {
        match self {
            Self::None => Type::of::<NoneValue>(),
            Self::Auto => Type::of::<AutoValue>(),
            Self::Bool(_) => Type::of::<bool>(),
            Self::Int(_) => Type::of::<i64>(),
            Self::Float(_) => Type::of::<f64>(),
            Self::Length(_) => Type::of::<Length>(),
            Self::Angle(_) => Type::of::<Angle>(),
            Self::Ratio(_) => Type::of::<Ratio>(),
            Self::Relative(_) => Type::of::<Rel<Length>>(),
            Self::Fraction(_) => Type::of::<Fr>(),
            Self::Color(_) => Type::of::<Color>(),
            Self::Gradient(_) => Type::of::<Gradient>(),
            Self::Tiling(_) => Type::of::<Tiling>(),
            Self::Symbol(_) => Type::of::<Symbol>(),
            Self::Version(_) => Type::of::<Version>(),
            Self::Str(_) => Type::of::<Str>(),
            Self::Bytes(_) => Type::of::<Bytes>(),
            Self::Label(_) => Type::of::<Label>(),
            Self::Datetime(_) => Type::of::<Datetime>(),
            Self::Decimal(_) => Type::of::<Decimal>(),
            Self::Duration(_) => Type::of::<Duration>(),
            Self::Content(_) => Type::of::<Content>(),
            Self::Styles(_) => Type::of::<Styles>(),
            Self::Array(_) => Type::of::<Array>(),
            Self::Dict(_) => Type::of::<Dict>(),
            Self::Func(_) => Type::of::<Func>(),
            Self::Args(_) => Type::of::<Args>(),
            Self::Type(_) => Type::of::<Type>(),
            Self::Module(_) => Type::of::<Module>(),
            Self::Dyn(v) => v.ty(),
        }
    }

    /// Try to cast the value into a specific type.
    pub fn cast<T: FromValue>(self) -> HintedStrResult<T> {
        T::from_value(self)
    }

    /// Try to access a field on the value.
    pub fn field(&self, field: &str, sink: impl DeprecationSink) -> StrResult<Value> {
        match self {
            Self::Symbol(symbol) => symbol.clone().modified(field).map(Self::Symbol),
            Self::Version(version) => version.component(field).map(Self::Int),
            Self::Dict(dict) => dict.get(field).cloned(),
            Self::Content(content) => content.field_by_name(field),
            Self::Type(ty) => ty.field(field, sink).cloned(),
            Self::Func(func) => func.field(field, sink).cloned(),
            Self::Module(module) => module.field(field, sink).cloned(),
            _ => fields::field(self, field),
        }
    }

    /// The associated scope, if this is a function, type, or module.
    pub fn scope(&self) -> Option<&Scope> {
        match self {
            Self::Func(func) => func.scope(),
            Self::Type(ty) => Some(ty.scope()),
            Self::Module(module) => Some(module.scope()),
            _ => None,
        }
    }

    /// Try to extract documentation for the value.
    pub fn docs(&self) -> Option<&'static str> {
        match self {
            Self::Func(func) => func.docs(),
            Self::Type(ty) => Some(ty.docs()),
            _ => None,
        }
    }

    /// Return the display representation of the value.
    pub fn display(self) -> Content {
        match self {
            Self::None => Content::empty(),
            Self::Int(v) => TextElem::packed(repr::format_int_with_base(v, 10)),
            Self::Float(v) => TextElem::packed(repr::display_float(v)),
            Self::Decimal(v) => TextElem::packed(eco_format!("{v}")),
            Self::Str(v) => TextElem::packed(v),
            Self::Version(v) => TextElem::packed(eco_format!("{v}")),
            Self::Symbol(v) => SymbolElem::packed(v.get()),
            Self::Content(v) => v,
            Self::Module(module) => module.content(),
            _ => RawElem::new(RawContent::Text(self.repr()))
                .with_lang(Some("typc".into()))
                .with_block(false)
                .pack(),
        }
    }

    /// Attach a span to the value, if possible.
    pub fn spanned(self, span: Span) -> Self {
        match self {
            Value::Content(v) => Value::Content(v.spanned(span)),
            Value::Func(v) => Value::Func(v.spanned(span)),
            v => v,
        }
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::None => Debug::fmt(&NoneValue, f),
            Self::Auto => Debug::fmt(&AutoValue, f),
            Self::Bool(v) => Debug::fmt(v, f),
            Self::Int(v) => Debug::fmt(v, f),
            Self::Float(v) => Debug::fmt(v, f),
            Self::Length(v) => Debug::fmt(v, f),
            Self::Angle(v) => Debug::fmt(v, f),
            Self::Ratio(v) => Debug::fmt(v, f),
            Self::Relative(v) => Debug::fmt(v, f),
            Self::Fraction(v) => Debug::fmt(v, f),
            Self::Color(v) => Debug::fmt(v, f),
            Self::Gradient(v) => Debug::fmt(v, f),
            Self::Tiling(v) => Debug::fmt(v, f),
            Self::Symbol(v) => Debug::fmt(v, f),
            Self::Version(v) => Debug::fmt(v, f),
            Self::Str(v) => Debug::fmt(v, f),
            Self::Bytes(v) => Debug::fmt(v, f),
            Self::Label(v) => Debug::fmt(v, f),
            Self::Datetime(v) => Debug::fmt(v, f),
            Self::Decimal(v) => Debug::fmt(v, f),
            Self::Duration(v) => Debug::fmt(v, f),
            Self::Content(v) => Debug::fmt(v, f),
            Self::Styles(v) => Debug::fmt(v, f),
            Self::Array(v) => Debug::fmt(v, f),
            Self::Dict(v) => Debug::fmt(v, f),
            Self::Func(v) => Debug::fmt(v, f),
            Self::Args(v) => Debug::fmt(v, f),
            Self::Type(v) => Debug::fmt(v, f),
            Self::Module(v) => Debug::fmt(v, f),
            Self::Dyn(v) => Debug::fmt(v, f),
        }
    }
}

impl Repr for Value {
    fn repr(&self) -> EcoString {
        match self {
            Self::None => NoneValue.repr(),
            Self::Auto => AutoValue.repr(),
            Self::Bool(v) => v.repr(),
            Self::Int(v) => v.repr(),
            Self::Float(v) => v.repr(),
            Self::Length(v) => v.repr(),
            Self::Angle(v) => v.repr(),
            Self::Ratio(v) => v.repr(),
            Self::Relative(v) => v.repr(),
            Self::Fraction(v) => v.repr(),
            Self::Color(v) => v.repr(),
            Self::Gradient(v) => v.repr(),
            Self::Tiling(v) => v.repr(),
            Self::Symbol(v) => v.repr(),
            Self::Version(v) => v.repr(),
            Self::Str(v) => v.repr(),
            Self::Bytes(v) => v.repr(),
            Self::Label(v) => v.repr(),
            Self::Datetime(v) => v.repr(),
            Self::Decimal(v) => v.repr(),
            Self::Duration(v) => v.repr(),
            Self::Content(v) => v.repr(),
            Self::Styles(v) => v.repr(),
            Self::Array(v) => v.repr(),
            Self::Dict(v) => v.repr(),
            Self::Func(v) => v.repr(),
            Self::Args(v) => v.repr(),
            Self::Type(v) => v.repr(),
            Self::Module(v) => v.repr(),
            Self::Dyn(v) => v.repr(),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        ops::equal(self, other)
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        ops::compare(self, other).ok()
    }
}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::None => {}
            Self::Auto => {}
            Self::Bool(v) => v.hash(state),
            Self::Int(v) => v.hash(state),
            Self::Float(v) => v.to_bits().hash(state),
            Self::Length(v) => v.hash(state),
            Self::Angle(v) => v.hash(state),
            Self::Ratio(v) => v.hash(state),
            Self::Relative(v) => v.hash(state),
            Self::Fraction(v) => v.hash(state),
            Self::Color(v) => v.hash(state),
            Self::Gradient(v) => v.hash(state),
            Self::Tiling(v) => v.hash(state),
            Self::Symbol(v) => v.hash(state),
            Self::Version(v) => v.hash(state),
            Self::Str(v) => v.hash(state),
            Self::Bytes(v) => v.hash(state),
            Self::Label(v) => v.hash(state),
            Self::Content(v) => v.hash(state),
            Self::Styles(v) => v.hash(state),
            Self::Datetime(v) => v.hash(state),
            Self::Decimal(v) => v.hash(state),
            Self::Duration(v) => v.hash(state),
            Self::Array(v) => v.hash(state),
            Self::Dict(v) => v.hash(state),
            Self::Func(v) => v.hash(state),
            Self::Args(v) => v.hash(state),
            Self::Type(v) => v.hash(state),
            Self::Module(v) => v.hash(state),
            Self::Dyn(v) => v.hash(state),
        }
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::None => NoneValue.serialize(serializer),
            Self::Bool(v) => v.serialize(serializer),
            Self::Int(v) => v.serialize(serializer),
            Self::Float(v) => v.serialize(serializer),
            Self::Str(v) => v.serialize(serializer),
            Self::Bytes(v) => v.serialize(serializer),
            Self::Symbol(v) => v.serialize(serializer),
            Self::Content(v) => v.serialize(serializer),
            Self::Array(v) => v.serialize(serializer),
            Self::Dict(v) => v.serialize(serializer),

            // Fall back to repr() for other things.
            other => serializer.serialize_str(&other.repr()),
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

/// Visitor for value deserialization.
struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a typst value")
    }

    fn visit_bool<E: Error>(self, v: bool) -> Result<Self::Value, E> {
        Ok(v.into_value())
    }

    fn visit_i8<E: Error>(self, v: i8) -> Result<Self::Value, E> {
        Ok(v.into_value())
    }

    fn visit_i16<E: Error>(self, v: i16) -> Result<Self::Value, E> {
        Ok(v.into_value())
    }

    fn visit_i32<E: Error>(self, v: i32) -> Result<Self::Value, E> {
        Ok(v.into_value())
    }

    fn visit_i64<E: Error>(self, v: i64) -> Result<Self::Value, E> {
        Ok(v.into_value())
    }

    fn visit_u8<E: Error>(self, v: u8) -> Result<Self::Value, E> {
        Ok(v.into_value())
    }

    fn visit_u16<E: Error>(self, v: u16) -> Result<Self::Value, E> {
        Ok(v.into_value())
    }

    fn visit_u32<E: Error>(self, v: u32) -> Result<Self::Value, E> {
        Ok(v.into_value())
    }

    fn visit_u64<E: Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(v.into_value())
    }

    fn visit_f32<E: Error>(self, v: f32) -> Result<Self::Value, E> {
        Ok((v as f64).into_value())
    }

    fn visit_f64<E: Error>(self, v: f64) -> Result<Self::Value, E> {
        Ok(v.into_value())
    }

    fn visit_char<E: Error>(self, v: char) -> Result<Self::Value, E> {
        Ok(v.into_value())
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(v.into_value())
    }

    fn visit_borrowed_str<E: Error>(self, v: &'de str) -> Result<Self::Value, E> {
        Ok(v.into_value())
    }

    fn visit_string<E: Error>(self, v: String) -> Result<Self::Value, E> {
        Ok(v.into_value())
    }

    fn visit_bytes<E: Error>(self, v: &[u8]) -> Result<Self::Value, E> {
        Ok(Bytes::new(v.to_vec()).into_value())
    }

    fn visit_borrowed_bytes<E: Error>(self, v: &'de [u8]) -> Result<Self::Value, E> {
        Ok(Bytes::new(v.to_vec()).into_value())
    }

    fn visit_byte_buf<E: Error>(self, v: Vec<u8>) -> Result<Self::Value, E> {
        Ok(Bytes::new(v).into_value())
    }

    fn visit_none<E: Error>(self) -> Result<Self::Value, E> {
        Ok(Value::None)
    }

    fn visit_some<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        Value::deserialize(deserializer)
    }

    fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
        Ok(Value::None)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        Ok(Array::deserialize(SeqAccessDeserializer::new(seq))?.into_value())
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        let dict = Dict::deserialize(MapAccessDeserializer::new(map))?;
        Ok(match Datetime::from_toml_dict(&dict) {
            None => dict.into_value(),
            Some(datetime) => datetime.into_value(),
        })
    }
}

/// A value that is not part of the built-in enum.
#[derive(Clone, Hash)]
#[allow(clippy::derived_hash_with_manual_eq)]
pub struct Dynamic(Arc<dyn Bounds>);

impl Dynamic {
    /// Create a new instance from any value that satisfies the required bounds.
    pub fn new<T>(any: T) -> Self
    where
        T: Debug + Repr + NativeType + PartialEq + Hash + Sync + Send + 'static,
    {
        Self(Arc::new(any))
    }

    /// Whether the wrapped type is `T`.
    pub fn is<T: 'static>(&self) -> bool {
        (*self.0).as_any().is::<T>()
    }

    /// Try to downcast to a reference to a specific type.
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        (*self.0).as_any().downcast_ref()
    }

    /// The name of the stored value's type.
    pub fn ty(&self) -> Type {
        self.0.dyn_ty()
    }
}

impl Debug for Dynamic {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Repr for Dynamic {
    fn repr(&self) -> EcoString {
        self.0.repr()
    }
}

impl PartialEq for Dynamic {
    fn eq(&self, other: &Self) -> bool {
        self.0.dyn_eq(other)
    }
}

trait Bounds: Debug + Repr + Sync + Send + 'static {
    fn as_any(&self) -> &dyn Any;
    fn dyn_eq(&self, other: &Dynamic) -> bool;
    fn dyn_ty(&self) -> Type;
    fn dyn_hash(&self, state: &mut dyn Hasher);
}

impl<T> Bounds for T
where
    T: Debug + Repr + NativeType + PartialEq + Hash + Sync + Send + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn dyn_eq(&self, other: &Dynamic) -> bool {
        let Some(other) = other.downcast::<Self>() else { return false };
        self == other
    }

    fn dyn_ty(&self) -> Type {
        Type::of::<T>()
    }

    fn dyn_hash(&self, mut state: &mut dyn Hasher) {
        // Also hash the TypeId since values with different types but
        // equal data should be different.
        TypeId::of::<Self>().hash(&mut state);
        self.hash(&mut state);
    }
}

impl Hash for dyn Bounds {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.dyn_hash(state);
    }
}

/// Implements traits for primitives (Value enum variants).
macro_rules! primitive {
    (
        $ty:ty: $name:literal, $variant:ident
        $(, $other:ident$(($binding:ident))? => $out:expr)*
    ) => {
        impl Reflect for $ty {
            fn input() -> CastInfo {
                CastInfo::Type(Type::of::<Self>())
            }

            fn output() -> CastInfo {
                CastInfo::Type(Type::of::<Self>())
            }

            fn castable(value: &Value) -> bool {
                matches!(value, Value::$variant(_)
                    $(|  primitive!(@$other $(($binding))?))*)
            }
        }

        impl IntoValue for $ty {
            fn into_value(self) -> Value {
                Value::$variant(self)
            }
        }

        impl FromValue for $ty {
            fn from_value(value: Value) -> HintedStrResult<Self> {
                match value {
                    Value::$variant(v) => Ok(v),
                    $(Value::$other$(($binding))? => Ok($out),)*
                    v => Err(<Self as Reflect>::error(&v)),
                }
            }
        }
    };

    (@$other:ident($binding:ident)) => { Value::$other(_) };
    (@$other:ident) => { Value::$other };
}

primitive! { bool: "boolean", Bool }
primitive! { i64: "integer", Int }
primitive! { f64: "float", Float, Int(v) => v as f64 }
primitive! { Length: "length", Length }
primitive! { Angle: "angle", Angle }
primitive! { Ratio: "ratio", Ratio }
primitive! { Rel<Length>:  "relative length",
    Relative,
    Length(v) => v.into(),
    Ratio(v) => v.into()
}
primitive! { Fr: "fraction", Fraction }
primitive! { Color: "color", Color }
primitive! { Gradient: "gradient", Gradient }
primitive! { Tiling: "tiling", Tiling }
primitive! { Symbol: "symbol", Symbol }
primitive! { Version: "version", Version }
primitive! {
    Str: "string",
    Str,
    Symbol(symbol) => symbol.get().into()
}
primitive! { Bytes: "bytes", Bytes }
primitive! { Label: "label", Label }
primitive! { Datetime: "datetime", Datetime }
primitive! { Decimal: "decimal", Decimal }
primitive! { Duration: "duration", Duration }
primitive! { Content: "content",
    Content,
    None => Content::empty(),
    Symbol(v) => SymbolElem::packed(v.get()),
    Str(v) => TextElem::packed(v)
}
primitive! { Styles: "styles", Styles }
primitive! { Array: "array", Array }
primitive! { Dict: "dictionary", Dict }
primitive! {
    Func: "function",
    Func,
    Type(ty) => ty.constructor()?.clone(),
    Symbol(symbol) => symbol.func()?
}
primitive! { Args: "arguments", Args }
primitive! { Type: "type", Type }
primitive! { Module: "module", Module }

impl<T: Reflect> Reflect for Arc<T> {
    fn input() -> CastInfo {
        T::input()
    }

    fn output() -> CastInfo {
        T::output()
    }

    fn castable(value: &Value) -> bool {
        T::castable(value)
    }

    fn error(found: &Value) -> HintedString {
        T::error(found)
    }
}

impl<T: Clone + IntoValue> IntoValue for Arc<T> {
    fn into_value(self) -> Value {
        Arc::take(self).into_value()
    }
}

impl<T: FromValue> FromValue for Arc<T> {
    fn from_value(value: Value) -> HintedStrResult<Self> {
        match value {
            v if T::castable(&v) => Ok(Arc::new(T::from_value(v)?)),
            _ => Err(Self::error(&value)),
        }
    }
}

impl<T: Clone + Resolve> Resolve for Arc<T> {
    type Output = Arc<T::Output>;

    fn resolve(self, styles: super::StyleChain) -> Self::Output {
        Arc::new(Arc::take(self).resolve(styles))
    }
}

impl<T: Clone + Fold> Fold for Arc<T> {
    fn fold(self, outer: Self) -> Self {
        Arc::new(Arc::take(self).fold(Arc::take(outer)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundations::{array, dict};

    #[track_caller]
    fn test(value: impl IntoValue, exp: &str) {
        assert_eq!(value.into_value().repr(), exp);
    }

    #[test]
    fn test_value_size() {
        assert!(std::mem::size_of::<Value>() <= 32);
    }

    #[test]
    fn test_value_debug() {
        // Primitives.
        test(Value::None, "none");
        test(Value::Auto, "auto");
        test(Value::None.ty(), "type(none)");
        test(Value::Auto.ty(), "type(auto)");
        test(false, "false");
        test(12i64, "12");
        test(3.24, "3.24");
        test(Abs::pt(5.5), "5.5pt");
        test(Angle::deg(90.0), "90deg");
        test(Ratio::one() / 2.0, "50%");
        test(Ratio::new(0.3) + Length::from(Abs::cm(2.0)), "30% + 56.69pt");
        test(Fr::one() * 7.55, "7.55fr");

        // Collections.
        test("hello", r#""hello""#);
        test("\n", r#""\n""#);
        test("\\", r#""\\""#);
        test("\"", r#""\"""#);
        test(array![], "()");
        test(array![Value::None], "(none,)");
        test(array![1, 2], "(1, 2)");
        test(dict![], "(:)");
        test(dict!["one" => 1], "(one: 1)");
        test(dict!["two" => false, "one" => 1], "(two: false, one: 1)");
    }
}
