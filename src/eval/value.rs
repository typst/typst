use std::any::Any;
use std::collections::BTreeMap;
use std::fmt::{self, Debug, Display, Formatter};
use std::ops::Deref;
use std::rc::Rc;

use super::*;
use crate::color::Color;
use crate::exec::ExecContext;
use crate::geom::{Angle, Length, Linear, Relative};
use crate::syntax::Tree;

/// A computational value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// The value that indicates the absence of a meaningful value.
    None,
    /// A boolean: `true, false`.
    Bool(bool),
    /// An integer: `120`.
    Int(i64),
    /// A floating-point number: `1.2`, `10e-4`.
    Float(f64),
    /// A length: `12pt`, `3cm`.
    Length(Length),
    /// An angle:  `1.5rad`, `90deg`.
    Angle(Angle),
    /// A relative value: `50%`.
    Relative(Relative),
    /// A combination of an absolute length and a relative value: `20% + 5cm`.
    Linear(Linear),
    /// A color value: `#f79143ff`.
    Color(Color),
    /// A string: `"string"`.
    Str(String),
    /// An array value: `(1, "hi", 12cm)`.
    Array(ValueArray),
    /// A dictionary value: `(color: #f79143, pattern: dashed)`.
    Dict(ValueDict),
    /// A template value: `[*Hi* there]`.
    Template(ValueTemplate),
    /// An executable function.
    Func(ValueFunc),
    /// Arguments to a function.
    Args(ValueArgs),
    /// Any object.
    Any(ValueAny),
    /// The result of invalid operations.
    Error,
}

impl Value {
    /// Create a new template value consisting of a single dynamic node.
    pub fn template<F>(name: impl Into<String>, f: F) -> Self
    where
        F: Fn(&mut ExecContext) + 'static,
    {
        Self::Template(vec![TemplateNode::Any(TemplateAny::new(name, f))])
    }

    /// The name of the stored value's type.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Bool(_) => bool::TYPE_NAME,
            Self::Int(_) => i64::TYPE_NAME,
            Self::Float(_) => f64::TYPE_NAME,
            Self::Length(_) => Length::TYPE_NAME,
            Self::Angle(_) => Angle::TYPE_NAME,
            Self::Relative(_) => Relative::TYPE_NAME,
            Self::Linear(_) => Linear::TYPE_NAME,
            Self::Color(_) => Color::TYPE_NAME,
            Self::Str(_) => String::TYPE_NAME,
            Self::Array(_) => ValueArray::TYPE_NAME,
            Self::Dict(_) => ValueDict::TYPE_NAME,
            Self::Template(_) => ValueTemplate::TYPE_NAME,
            Self::Func(_) => ValueFunc::TYPE_NAME,
            Self::Args(_) => ValueArgs::TYPE_NAME,
            Self::Any(v) => v.type_name(),
            Self::Error => "error",
        }
    }

    /// Try to cast the value into a specific type.
    pub fn cast<T>(self) -> CastResult<T, Self>
    where
        T: Cast<Value>,
    {
        T::cast(self)
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::None
    }
}

/// An array value: `(1, "hi", 12cm)`.
pub type ValueArray = Vec<Value>;

/// A dictionary value: `(color: #f79143, pattern: dashed)`.
pub type ValueDict = BTreeMap<String, Value>;

/// A template value: `[*Hi* there]`.
pub type ValueTemplate = Vec<TemplateNode>;

/// One chunk of a template.
///
/// Evaluating a template expression creates only a single node. Adding multiple
/// templates can yield multi-node templates.
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateNode {
    /// A template that consists of a syntax tree plus already evaluated
    /// expression.
    Tree {
        /// The syntax tree of the corresponding template expression.
        tree: Rc<Tree>,
        /// The evaluated expressions for the `tree`.
        map: ExprMap,
    },
    /// A template that can implement custom behaviour.
    Any(TemplateAny),
}

/// A reference-counted dynamic template node (can implement custom behaviour).
#[derive(Clone)]
pub struct TemplateAny {
    name: String,
    f: Rc<dyn Fn(&mut ExecContext)>,
}

impl TemplateAny {
    /// Create a new dynamic template value from a rust function or closure.
    pub fn new<F>(name: impl Into<String>, f: F) -> Self
    where
        F: Fn(&mut ExecContext) + 'static,
    {
        Self { name: name.into(), f: Rc::new(f) }
    }

    /// The name of the template node.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl PartialEq for TemplateAny {
    fn eq(&self, _: &Self) -> bool {
        // TODO: Figure out what we want here.
        false
    }
}

impl Deref for TemplateAny {
    type Target = dyn Fn(&mut ExecContext);

    fn deref(&self) -> &Self::Target {
        self.f.as_ref()
    }
}

impl Debug for TemplateAny {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("TemplateAny").finish()
    }
}

/// A wrapper around a reference-counted executable function.
#[derive(Clone)]
pub struct ValueFunc {
    name: String,
    f: Rc<dyn Fn(&mut EvalContext, &mut ValueArgs) -> Value>,
}

impl ValueFunc {
    /// Create a new function value from a rust function or closure.
    pub fn new<F>(name: impl Into<String>, f: F) -> Self
    where
        F: Fn(&mut EvalContext, &mut ValueArgs) -> Value + 'static,
    {
        Self { name: name.into(), f: Rc::new(f) }
    }

    /// The name of the function.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl PartialEq for ValueFunc {
    fn eq(&self, _: &Self) -> bool {
        // TODO: Figure out what we want here.
        false
    }
}

impl Deref for ValueFunc {
    type Target = dyn Fn(&mut EvalContext, &mut ValueArgs) -> Value;

    fn deref(&self) -> &Self::Target {
        self.f.as_ref()
    }
}

impl Debug for ValueFunc {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("ValueFunc").field("name", &self.name).finish()
    }
}

/// Evaluated arguments to a function.
#[derive(Debug, Clone, PartialEq)]
pub struct ValueArgs {
    /// The span of the whole argument list.
    pub span: Span,
    /// The arguments.
    pub items: Vec<ValueArg>,
}

impl ValueArgs {
    /// Find and remove the first convertible positional argument.
    pub fn find<T>(&mut self, ctx: &mut EvalContext) -> Option<T>
    where
        T: Cast<Spanned<Value>>,
    {
        (0 .. self.items.len()).find_map(move |i| self.try_take(&mut ctx.diags, i))
    }

    /// Find and remove the first convertible positional argument, producing an
    /// error if no match was found.
    pub fn require<T>(&mut self, ctx: &mut EvalContext, what: &str) -> Option<T>
    where
        T: Cast<Spanned<Value>>,
    {
        let found = self.find(ctx);
        if found.is_none() {
            ctx.diag(error!(self.span, "missing argument: {}", what));
        }
        found
    }

    /// Filter out and remove all convertible positional arguments.
    pub fn filter<'a, T>(
        &'a mut self,
        ctx: &'a mut EvalContext,
    ) -> impl Iterator<Item = T> + 'a
    where
        T: Cast<Spanned<Value>>,
    {
        let diags = &mut ctx.diags;
        let mut i = 0;
        std::iter::from_fn(move || {
            while i < self.items.len() {
                if let Some(val) = self.try_take(diags, i) {
                    return Some(val);
                }
                i += 1;
            }
            None
        })
    }

    /// Convert and remove the value for the given named argument, producing an
    /// error if the conversion fails.
    pub fn get<T>(&mut self, ctx: &mut EvalContext, name: &str) -> Option<T>
    where
        T: Cast<Spanned<Value>>,
    {
        let index = self
            .items
            .iter()
            .position(|arg| arg.name.as_ref().map(|s| s.v.as_str()) == Some(name))?;

        let value = self.items.remove(index).value;
        self.cast(ctx, value)
    }

    /// Produce "unexpected argument" errors for all remaining arguments.
    pub fn finish(self, ctx: &mut EvalContext) {
        for arg in &self.items {
            if arg.value.v != Value::Error {
                ctx.diag(error!(arg.span(), "unexpected argument"));
            }
        }
    }

    /// Cast the value into `T`, generating an error if the conversion fails.
    fn cast<T>(&self, ctx: &mut EvalContext, value: Spanned<Value>) -> Option<T>
    where
        T: Cast<Spanned<Value>>,
    {
        let span = value.span;
        match T::cast(value) {
            CastResult::Ok(t) => Some(t),
            CastResult::Warn(t, m) => {
                ctx.diag(warning!(span, "{}", m));
                Some(t)
            }
            CastResult::Err(value) => {
                ctx.diag(error!(
                    span,
                    "expected {}, found {}",
                    T::TYPE_NAME,
                    value.v.type_name()
                ));
                None
            }
        }
    }

    /// Try to take and cast a positional argument in the i'th slot into `T`,
    /// putting it back if the conversion fails.
    fn try_take<T>(&mut self, diags: &mut DiagSet, i: usize) -> Option<T>
    where
        T: Cast<Spanned<Value>>,
    {
        let slot = &mut self.items[i];
        if slot.name.is_some() {
            return None;
        }

        let value = std::mem::replace(&mut slot.value, Spanned::zero(Value::None));
        let span = value.span;
        match T::cast(value) {
            CastResult::Ok(t) => {
                self.items.remove(i);
                Some(t)
            }
            CastResult::Warn(t, m) => {
                self.items.remove(i);
                diags.insert(warning!(span, "{}", m));
                Some(t)
            }
            CastResult::Err(value) => {
                slot.value = value;
                None
            }
        }
    }
}

/// An argument to a function call: `12` or `draw: false`.
#[derive(Debug, Clone, PartialEq)]
pub struct ValueArg {
    /// The name of the argument (`None` for positional arguments).
    pub name: Option<Spanned<String>>,
    /// The value of the argument.
    pub value: Spanned<Value>,
}

impl ValueArg {
    /// The source code location.
    pub fn span(&self) -> Span {
        match &self.name {
            Some(name) => name.span.join(self.value.span),
            None => self.value.span,
        }
    }
}

/// A wrapper around a dynamic value.
pub struct ValueAny(Box<dyn Bounds>);

impl ValueAny {
    /// Create a new instance from any value that satisifies the required bounds.
    pub fn new<T>(any: T) -> Self
    where
        T: Type + Debug + Display + Clone + PartialEq + 'static,
    {
        Self(Box::new(any))
    }

    /// Whether the wrapped type is `T`.
    pub fn is<T: 'static>(&self) -> bool {
        self.0.as_any().is::<T>()
    }

    /// Try to downcast to a specific type.
    pub fn downcast<T: 'static>(self) -> Result<T, Self> {
        if self.is::<T>() {
            Ok(*self.0.into_any().downcast().unwrap())
        } else {
            Err(self)
        }
    }

    /// Try to downcast to a reference to a specific type.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.0.as_any().downcast_ref()
    }

    /// The name of the stored value's type.
    pub fn type_name(&self) -> &'static str {
        self.0.dyn_type_name()
    }
}

impl Clone for ValueAny {
    fn clone(&self) -> Self {
        Self(self.0.dyn_clone())
    }
}

impl PartialEq for ValueAny {
    fn eq(&self, other: &Self) -> bool {
        self.0.dyn_eq(other)
    }
}

impl Debug for ValueAny {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_tuple("ValueAny").field(&self.0).finish()
    }
}

impl Display for ValueAny {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

trait Bounds: Debug + Display + 'static {
    fn as_any(&self) -> &dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    fn dyn_eq(&self, other: &ValueAny) -> bool;
    fn dyn_clone(&self) -> Box<dyn Bounds>;
    fn dyn_type_name(&self) -> &'static str;
}

impl<T> Bounds for T
where
    T: Type + Debug + Display + Clone + PartialEq + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn dyn_eq(&self, other: &ValueAny) -> bool {
        if let Some(other) = other.downcast_ref::<Self>() {
            self == other
        } else {
            false
        }
    }

    fn dyn_clone(&self) -> Box<dyn Bounds> {
        Box::new(self.clone())
    }

    fn dyn_type_name(&self) -> &'static str {
        T::TYPE_NAME
    }
}

/// Types that can be stored in values.
pub trait Type {
    /// The name of the type.
    const TYPE_NAME: &'static str;
}

impl<T> Type for Spanned<T>
where
    T: Type,
{
    const TYPE_NAME: &'static str = T::TYPE_NAME;
}

/// Cast from a value to a specific type.
pub trait Cast<V>: Type + Sized {
    /// Try to cast the value into an instance of `Self`.
    fn cast(value: V) -> CastResult<Self, V>;
}

/// The result of casting a value to a specific type.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CastResult<T, V> {
    /// The value was cast successfully.
    Ok(T),
    /// The value was cast successfully, but with a warning message.
    Warn(T, String),
    /// The value could not be cast into the specified type.
    Err(V),
}

impl<T, V> CastResult<T, V> {
    /// Access the conversion result, discarding a possibly existing warning.
    pub fn ok(self) -> Option<T> {
        match self {
            CastResult::Ok(t) | CastResult::Warn(t, _) => Some(t),
            CastResult::Err(_) => None,
        }
    }
}

impl Type for Value {
    const TYPE_NAME: &'static str = "value";
}

impl Cast<Value> for Value {
    fn cast(value: Value) -> CastResult<Self, Value> {
        CastResult::Ok(value)
    }
}

impl<T> Cast<Spanned<Value>> for T
where
    T: Cast<Value>,
{
    fn cast(value: Spanned<Value>) -> CastResult<Self, Spanned<Value>> {
        let span = value.span;
        match T::cast(value.v) {
            CastResult::Ok(t) => CastResult::Ok(t),
            CastResult::Warn(t, m) => CastResult::Warn(t, m),
            CastResult::Err(v) => CastResult::Err(Spanned::new(v, span)),
        }
    }
}

impl<T> Cast<Spanned<Value>> for Spanned<T>
where
    T: Cast<Value>,
{
    fn cast(value: Spanned<Value>) -> CastResult<Self, Spanned<Value>> {
        let span = value.span;
        match T::cast(value.v) {
            CastResult::Ok(t) => CastResult::Ok(Spanned::new(t, span)),
            CastResult::Warn(t, m) => CastResult::Warn(Spanned::new(t, span), m),
            CastResult::Err(v) => CastResult::Err(Spanned::new(v, span)),
        }
    }
}

macro_rules! primitive {
    ($type:ty:
        $type_name:literal,
        $variant:path
        $(, $pattern:pat => $out:expr)* $(,)?
    ) => {
        impl Type for $type {
            const TYPE_NAME: &'static str = $type_name;
        }

        impl From<$type> for Value {
            fn from(v: $type) -> Self {
                $variant(v)
            }
        }

        impl Cast<Value> for $type {
            fn cast(value: Value) -> CastResult<Self, Value> {
                match value {
                    $variant(v) => CastResult::Ok(v),
                    $($pattern => CastResult::Ok($out),)*
                    v => CastResult::Err(v),
                }
            }
        }
    };
}

primitive! { bool: "boolean", Value::Bool }
primitive! { i64: "integer", Value::Int }
primitive! {
    f64: "float",
    Value::Float,
    Value::Int(v) => v as f64,
}
primitive! { Length: "length", Value::Length }
primitive! { Angle: "angle", Value::Angle }
primitive! { Relative: "relative", Value::Relative }
primitive! {
    Linear: "linear",
    Value::Linear,
    Value::Length(v) => v.into(),
    Value::Relative(v) => v.into(),
}
primitive! { Color: "color", Value::Color }
primitive! { String: "string", Value::Str }
primitive! { ValueArray: "array", Value::Array }
primitive! { ValueDict: "dictionary", Value::Dict }
primitive! { ValueTemplate: "template", Value::Template }
primitive! { ValueFunc: "function", Value::Func }
primitive! { ValueArgs: "arguments", Value::Args }

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Self::Str(v.to_string())
    }
}

impl From<ValueAny> for Value {
    fn from(v: ValueAny) -> Self {
        Self::Any(v)
    }
}

/// Make a type usable as a [`Value`].
///
/// Given a type `T`, this implements the following traits:
/// - [`Type`] for `T`,
/// - [`Cast<Value>`](Cast) for `T`.
///
/// # Example
/// Make a type `FontFamily` that can be cast from a [`Value::Any`] variant
/// containing a `FontFamily` or from a string.
/// ```
/// # use typst::typify;
/// # enum FontFamily { Named(String) }
/// typify! {
///     FontFamily: "font family",
///     Value::Str(string) => Self::Named(string.to_lowercase())
/// }
/// ```
#[macro_export]
macro_rules! typify {
    ($type:ty:
        $type_name:literal
        $(, $pattern:pat => $out:expr)*
        $(, #($anyvar:ident: $anytype:ty) => $anyout:expr)*
        $(,)?
    ) => {
        impl $crate::eval::Type for $type {
            const TYPE_NAME: &'static str = $type_name;
        }

        impl $crate::eval::Cast<$crate::eval::Value> for $type {
            fn cast(
                value: $crate::eval::Value,
            ) -> $crate::eval::CastResult<Self, $crate::eval::Value> {
                use $crate::eval::*;

                #[allow(unreachable_code)]
                match value {
                    $($pattern => CastResult::Ok($out),)*
                    Value::Any(mut any) => {
                        any = match any.downcast::<Self>() {
                            Ok(t) => return CastResult::Ok(t),
                            Err(any) => any,
                        };

                        $(any = match any.downcast::<$anytype>() {
                            Ok($anyvar) => return CastResult::Ok($anyout),
                            Err(any) => any,
                        };)*

                        CastResult::Err(Value::Any(any))
                    },
                    v => CastResult::Err(v),
                }
            }
        }
    };
}
