//! Computational values: Syntactical expressions can be evaluated into these.

use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;
use std::rc::Rc;

use fontdock::{FontStretch, FontStyle, FontWeight};

use super::dict::{Dict, SpannedEntry};
use crate::color::RgbaColor;
use crate::geom::Linear;
use crate::layout::{Command, Commands, Dir, LayoutContext, SpecAlign};
use crate::paper::Paper;
use crate::syntax::{Ident, Span, SpanWith, Spanned, SynNode, SynTree};
use crate::{DynFuture, Feedback};

/// A computational value.
#[derive(Clone, PartialEq)]
pub enum Value {
    /// The result of invalid operations.
    Error,
    /// An identifier: `ident`.
    Ident(Ident),
    /// A boolean: `true, false`.
    Bool(bool),
    /// An integer: `120`.
    Int(i64),
    /// A floating-point number: `1.2, 200%`.
    Float(f64),
    /// A length: `2cm, 5.2in`.
    Length(f64),
    /// A relative value: `50%`.
    ///
    /// _Note_: `50%` is represented as `0.5` here, but as `50.0` in the
    /// corresponding [literal].
    ///
    /// [literal]: ../syntax/ast/enum.Lit.html#variant.Percent
    Relative(f64),
    /// A combination of an absolute length and a relative value.
    Linear(Linear),
    /// A color value with alpha channel: `#f79143ff`.
    Color(RgbaColor),
    /// A string: `"string"`.
    Str(String),
    /// A dictionary value: `(false, 12cm, greeting="hi")`.
    Dict(DictValue),
    /// A syntax tree containing typesetting content.
    Tree(SynTree),
    /// An executable function.
    Func(FuncValue),
    /// Layouting commands.
    Commands(Commands),
}

impl Value {
    /// The natural-language name of this value for use in error messages.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Ident(_) => "identifier",
            Self::Bool(_) => "bool",
            Self::Int(_) => "integer",
            Self::Float(_) => "float",
            Self::Relative(_) => "relative",
            Self::Length(_) => "length",
            Self::Linear(_) => "linear",
            Self::Color(_) => "color",
            Self::Str(_) => "string",
            Self::Dict(_) => "dict",
            Self::Tree(_) => "syntax tree",
            Self::Func(_) => "function",
            Self::Commands(_) => "commands",
        }
    }
}

impl Spanned<Value> {
    /// Transform this value into something layoutable.
    ///
    /// If this is already a command-value, it is simply unwrapped, otherwise
    /// the value is represented as layoutable content in a reasonable way.
    pub fn into_commands(self) -> Commands {
        match self.v {
            // Forward to each entry, separated with spaces.
            Value::Dict(dict) => {
                let mut commands = vec![];
                let mut end = None;
                for entry in dict.into_values() {
                    if let Some(last_end) = end {
                        let span = Span::new(last_end, entry.key_span.start);
                        let tree = vec![SynNode::Space.span_with(span)];
                        commands.push(Command::LayoutSyntaxTree(tree));
                    }

                    end = Some(entry.value.span.end);
                    commands.extend(entry.value.into_commands());
                }
                commands
            }

            Value::Tree(tree) => vec![Command::LayoutSyntaxTree(tree)],
            Value::Commands(commands) => commands,

            // Format with debug.
            val => {
                let fmt = format!("{:?}", val);
                let tree = vec![SynNode::Text(fmt).span_with(self.span)];
                vec![Command::LayoutSyntaxTree(tree)]
            }
        }
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Error => f.pad("<error>"),
            Self::Ident(i) => i.fmt(f),
            Self::Bool(b) => b.fmt(f),
            Self::Int(i) => i.fmt(f),
            Self::Float(n) => n.fmt(f),
            Self::Length(l) => l.fmt(f),
            Self::Relative(r) => r.fmt(f),
            Self::Linear(l) => l.fmt(f),
            Self::Color(c) => c.fmt(f),
            Self::Str(s) => s.fmt(f),
            Self::Dict(d) => d.fmt(f),
            Self::Tree(t) => t.fmt(f),
            Self::Func(c) => c.fmt(f),
            Self::Commands(c) => c.fmt(f),
        }
    }
}

/// An executable function value.
///
/// The first argument is a dictionary containing the arguments passed to the
/// function. The function may be asynchronous (as such it returns a dynamic
/// future) and it may emit diagnostics, which are contained in the returned
/// `Pass`. In the end, the function must evaluate to [`Value`]. A typical
/// typesetting function will return a `Commands` value which will instruct the
/// layouting engine to do what the function pleases.
///
/// The dynamic function object is wrapped in an `Rc` to keep [`Value`]
/// clonable.
///
/// [`Value`]: enum.Value.html
#[derive(Clone)]
pub struct FuncValue(pub Rc<FuncType>);

/// The signature of executable functions.
type FuncType = dyn Fn(DictValue, &mut LayoutContext) -> DynFuture<Value>;

impl FuncValue {
    /// Create a new function value from a rust function or closure.
    pub fn new<F: 'static>(f: F) -> Self
    where
        F: Fn(DictValue, &mut LayoutContext) -> DynFuture<Value>,
    {
        Self(Rc::new(f))
    }
}

impl Eq for FuncValue {}

impl PartialEq for FuncValue {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Deref for FuncValue {
    type Target = FuncType;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl Debug for FuncValue {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("<function>")
    }
}

/// A dictionary of values.
///
/// # Example
/// ```typst
/// (false, 12cm, greeting="hi")
/// ```
pub type DictValue = Dict<SpannedEntry<Value>>;

impl DictValue {
    /// Retrieve and remove the matching value with the lowest number key,
    /// skipping and ignoring all non-matching entries with lower keys.
    pub fn take<T: TryFromValue>(&mut self) -> Option<T> {
        for (&key, entry) in self.nums() {
            let expr = entry.value.as_ref();
            if let Some(val) = T::try_from_value(expr, &mut Feedback::new()) {
                self.remove(key);
                return Some(val);
            }
        }
        None
    }

    /// Retrieve and remove the matching value with the lowest number key,
    /// removing and generating errors for all non-matching entries with lower
    /// keys.
    ///
    /// Generates an error at `err_span` when no matching value was found.
    pub fn expect<T: TryFromValue>(
        &mut self,
        name: &str,
        span: Span,
        f: &mut Feedback,
    ) -> Option<T> {
        while let Some((num, _)) = self.first() {
            let entry = self.remove(num).unwrap();
            if let Some(val) = T::try_from_value(entry.value.as_ref(), f) {
                return Some(val);
            }
        }
        error!(@f, span, "missing argument: {}", name);
        None
    }

    /// Retrieve and remove a matching value associated with the given key if
    /// there is any.
    ///
    /// Generates an error if the key exists but the value does not match.
    pub fn take_key<T>(&mut self, key: &str, f: &mut Feedback) -> Option<T>
    where
        T: TryFromValue,
    {
        self.remove(key).and_then(|entry| {
            let expr = entry.value.as_ref();
            T::try_from_value(expr, f)
        })
    }

    /// Retrieve and remove all matching pairs with number keys, skipping and
    /// ignoring non-matching entries.
    ///
    /// The pairs are returned in order of increasing keys.
    pub fn take_all_num<'a, T>(&'a mut self) -> impl Iterator<Item = (u64, T)> + 'a
    where
        T: TryFromValue,
    {
        let mut skip = 0;
        std::iter::from_fn(move || {
            for (&key, entry) in self.nums().skip(skip) {
                let expr = entry.value.as_ref();
                if let Some(val) = T::try_from_value(expr, &mut Feedback::new()) {
                    self.remove(key);
                    return Some((key, val));
                }
                skip += 1;
            }

            None
        })
    }


    /// Retrieve and remove all matching values with number keys, skipping and
    /// ignoring non-matching entries.
    ///
    /// The values are returned in order of increasing keys.
    pub fn take_all_num_vals<'a, T: 'a>(&'a mut self) -> impl Iterator<Item = T> + 'a
    where
        T: TryFromValue,
    {
        self.take_all_num::<T>().map(|(_, v)| v)
    }

    /// Retrieve and remove all matching pairs with string keys, skipping and
    /// ignoring non-matching entries.
    ///
    /// The pairs are returned in order of increasing keys.
    pub fn take_all_str<'a, T>(&'a mut self) -> impl Iterator<Item = (String, T)> + 'a
    where
        T: TryFromValue,
    {
        let mut skip = 0;
        std::iter::from_fn(move || {
            for (key, entry) in self.strs().skip(skip) {
                let expr = entry.value.as_ref();
                if let Some(val) = T::try_from_value(expr, &mut Feedback::new()) {
                    let key = key.clone();
                    self.remove(&key);
                    return Some((key, val));
                }
                skip += 1;
            }

            None
        })
    }

    /// Generated `"unexpected argument"` errors for all remaining entries.
    pub fn unexpected(&self, f: &mut Feedback) {
        for entry in self.values() {
            error!(@f, entry.key_span.join(entry.value.span), "unexpected argument");
        }
    }
}

/// A trait for converting values into more specific types.
pub trait TryFromValue: Sized {
    // This trait takes references because we don't want to move the value
    // out of its origin in case this returns `None`. This solution is not
    // perfect because we need to do some cloning in the impls for this trait,
    // but we haven't got a better solution, for now.

    /// Try to convert a value to this type.
    ///
    /// Returns `None` and generates an appropriate error if the value is not
    /// valid for this type.
    fn try_from_value(value: Spanned<&Value>, f: &mut Feedback) -> Option<Self>;
}

macro_rules! impl_match {
    ($type:ty, $name:expr, $($p:pat => $r:expr),* $(,)?) => {
        impl TryFromValue for $type {
            fn try_from_value(value: Spanned<&Value>, f: &mut Feedback) -> Option<Self> {
                #[allow(unreachable_patterns)]
                match value.v {
                    $($p => Some($r)),*,
                    other => {
                        error!(
                            @f, value.span,
                            "expected {}, found {}", $name, other.name()
                        );
                        None
                    }
                }
            }
        }
    };
}

macro_rules! impl_ident {
    ($type:ty, $name:expr, $parse:expr) => {
        impl TryFromValue for $type {
            fn try_from_value(value: Spanned<&Value>, f: &mut Feedback) -> Option<Self> {
                if let Value::Ident(ident) = value.v {
                    let val = $parse(ident);
                    if val.is_none() {
                        error!(@f, value.span, "invalid {}", $name);
                    }
                    val
                } else {
                    error!(
                        @f, value.span,
                        "expected {}, found {}", $name, value.v.name()
                    );
                    None
                }
            }
        }
    };
}

impl<T: TryFromValue> TryFromValue for Spanned<T> {
    fn try_from_value(value: Spanned<&Value>, f: &mut Feedback) -> Option<Self> {
        let span = value.span;
        T::try_from_value(value, f).map(|v| Spanned { v, span })
    }
}

impl_match!(Value, "value", v => v.clone());
impl_match!(Ident, "identifier", Value::Ident(i) => i.clone());
impl_match!(bool, "bool", &Value::Bool(b) => b);
impl_match!(i64, "integer", &Value::Int(i) => i);
impl_match!(f64, "float",
    &Value::Int(i) => i as f64,
    &Value::Float(f) => f,
);
impl_match!(Abs, "length", &Value::Length(l) => Abs(l));
impl_match!(Rel, "relative", &Value::Relative(r) => Rel(r));
impl_match!(Linear, "linear",
    &Value::Linear(l) => l,
    &Value::Length(l) => Linear::abs(l),
    &Value::Relative(r) => Linear::rel(r),
);
impl_match!(String, "string", Value::Str(s) => s.clone());
impl_match!(SynTree, "tree", Value::Tree(t) => t.clone());
impl_match!(DictValue, "dict", Value::Dict(t) => t.clone());
impl_match!(FuncValue, "function", Value::Func(f) => f.clone());

/// A value type that matches [length] values.
///
/// [length]: enum.Value.html#variant.Length
pub struct Abs(pub f64);

impl From<Abs> for f64 {
    fn from(abs: Abs) -> f64 {
        abs.0
    }
}

/// A value type that matches [relative] values.
///
/// [relative]: enum.Value.html#variant.Relative
pub struct Rel(pub f64);

impl From<Rel> for f64 {
    fn from(rel: Rel) -> f64 {
        rel.0
    }
}

/// A value type that matches [identifier] and [string] values.
///
/// [identifier]: enum.Value.html#variant.Ident
/// [string]: enum.Value.html#variant.Str
pub struct StringLike(pub String);

impl From<StringLike> for String {
    fn from(like: StringLike) -> String {
        like.0
    }
}

impl Deref for StringLike {
    type Target = str;

    fn deref(&self) -> &str {
        self.0.as_str()
    }
}

impl_match!(StringLike, "identifier or string",
    Value::Ident(Ident(s)) => StringLike(s.clone()),
    Value::Str(s) => StringLike(s.clone()),
);

impl_ident!(Dir, "direction", |s| match s {
    "ltr" => Some(Self::LTR),
    "rtl" => Some(Self::RTL),
    "ttb" => Some(Self::TTB),
    "btt" => Some(Self::BTT),
    _ => None,
});

impl_ident!(SpecAlign, "alignment", |s| match s {
    "left" => Some(Self::Left),
    "right" => Some(Self::Right),
    "top" => Some(Self::Top),
    "bottom" => Some(Self::Bottom),
    "center" => Some(Self::Center),
    _ => None,
});

impl_ident!(FontStyle, "font style", Self::from_str);
impl_ident!(FontStretch, "font stretch", Self::from_str);
impl_ident!(Paper, "paper", Self::from_name);

impl TryFromValue for FontWeight {
    fn try_from_value(value: Spanned<&Value>, f: &mut Feedback) -> Option<Self> {
        match value.v {
            &Value::Int(weight) => {
                const MIN: i64 = 100;
                const MAX: i64 = 900;
                let weight = if weight < MIN {
                    error!(@f, value.span, "the minimum font weight is {}", MIN);
                    MIN
                } else if weight > MAX {
                    error!(@f, value.span, "the maximum font weight is {}", MAX);
                    MAX
                } else {
                    weight
                };
                Self::from_number(weight as u16)
            }
            Value::Ident(ident) => {
                let weight = Self::from_str(ident);
                if weight.is_none() {
                    error!(@f, value.span, "invalid font weight");
                }
                weight
            }
            other => {
                error!(
                    @f, value.span,
                    "expected font weight (name or integer), found {}",
                    other.name(),
                );
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(value: Value) -> SpannedEntry<Value> {
        SpannedEntry::val(Spanned::zero(value))
    }

    #[test]
    fn test_dict_take_removes_correct_entry() {
        let mut dict = Dict::new();
        dict.insert(1, entry(Value::Bool(false)));
        dict.insert(2, entry(Value::Str("hi".to_string())));
        assert_eq!(dict.take::<String>(), Some("hi".to_string()));
        assert_eq!(dict.len(), 1);
        assert_eq!(dict.take::<bool>(), Some(false));
        assert!(dict.is_empty());
    }

    #[test]
    fn test_dict_expect_errors_about_previous_entries() {
        let mut f = Feedback::new();
        let mut dict = Dict::new();
        dict.insert(1, entry(Value::Bool(false)));
        dict.insert(3, entry(Value::Str("hi".to_string())));
        dict.insert(5, entry(Value::Bool(true)));
        assert_eq!(
            dict.expect::<String>("", Span::ZERO, &mut f),
            Some("hi".to_string())
        );
        assert_eq!(f.diagnostics, [error!(
            Span::ZERO,
            "expected string, found bool"
        )]);
        assert_eq!(dict.len(), 1);
    }

    #[test]
    fn test_dict_take_with_key_removes_the_entry() {
        let mut f = Feedback::new();
        let mut dict = Dict::new();
        dict.insert(1, entry(Value::Bool(false)));
        dict.insert("hi", entry(Value::Bool(true)));
        assert_eq!(dict.take::<bool>(), Some(false));
        assert_eq!(dict.take_key::<f64>("hi", &mut f), None);
        assert_eq!(f.diagnostics, [error!(
            Span::ZERO,
            "expected float, found bool"
        )]);
        assert!(dict.is_empty());
    }

    #[test]
    fn test_dict_take_all_removes_the_correct_entries() {
        let mut dict = Dict::new();
        dict.insert(1, entry(Value::Bool(false)));
        dict.insert(3, entry(Value::Float(0.0)));
        dict.insert(7, entry(Value::Bool(true)));
        assert_eq!(dict.take_all_num::<bool>().collect::<Vec<_>>(), [
            (1, false),
            (7, true)
        ],);
        assert_eq!(dict.len(), 1);
        assert_eq!(dict[3].value.v, Value::Float(0.0));
    }
}
