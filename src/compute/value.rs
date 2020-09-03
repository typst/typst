//! Computational values: Syntactical expressions can be evaluated into these.

use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;
use std::rc::Rc;

use fontdock::{FontStyle, FontWeight, FontWidth};

use super::table::{SpannedEntry, Table};
use crate::color::RgbaColor;
use crate::layout::{Command, Commands, Dir, LayoutContext, SpecAlign};
use crate::length::{Length, ScaleLength};
use crate::paper::Paper;
use crate::syntax::span::{Span, Spanned};
use crate::syntax::tree::{Ident, SyntaxNode, SyntaxTree};
use crate::{DynFuture, Feedback, Pass};

/// A computational value.
#[derive(Clone)]
pub enum Value {
    /// An identifier: `ident`.
    Ident(Ident),
    /// A string: `"string"`.
    Str(String),
    /// A boolean: `true, false`.
    Bool(bool),
    /// A number: `1.2, 200%`.
    Number(f64),
    /// A length: `2cm, 5.2in`.
    Length(Length),
    /// A color value with alpha channel: `#f79143ff`.
    Color(RgbaColor),
    /// A table value: `(false, 12cm, greeting="hi")`.
    Table(TableValue),
    /// A syntax tree containing typesetting content.
    Tree(SyntaxTree),
    /// An executable function.
    Func(FuncValue),
    /// Layouting commands.
    Commands(Commands),
}

impl Value {
    /// A natural-language name of the type of this expression, e.g.
    /// "identifier".
    pub fn name(&self) -> &'static str {
        use Value::*;
        match self {
            Ident(_) => "identifier",
            Str(_) => "string",
            Bool(_) => "bool",
            Number(_) => "number",
            Length(_) => "length",
            Color(_) => "color",
            Table(_) => "table",
            Tree(_) => "syntax tree",
            Func(_) => "function",
            Commands(_) => "commands",
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
            Value::Commands(commands) => commands,
            Value::Tree(tree) => vec![Command::LayoutSyntaxTree(tree)],

            // Forward to each entry, separated with spaces.
            Value::Table(table) => {
                let mut commands = vec![];
                let mut end = None;
                for entry in table.into_values() {
                    if let Some(last_end) = end {
                        let span = Span::new(last_end, entry.key.start);
                        commands.push(Command::LayoutSyntaxTree(vec![Spanned::new(
                            SyntaxNode::Spacing,
                            span,
                        )]));
                    }

                    end = Some(entry.val.span.end);
                    commands.extend(entry.val.into_commands());
                }
                commands
            }

            // Format with debug.
            val => vec![Command::LayoutSyntaxTree(vec![Spanned::new(
                SyntaxNode::Text(format!("{:?}", val)),
                self.span,
            )])],
        }
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use Value::*;
        match self {
            Ident(i) => i.fmt(f),
            Str(s) => s.fmt(f),
            Bool(b) => b.fmt(f),
            Number(n) => n.fmt(f),
            Length(s) => s.fmt(f),
            Color(c) => c.fmt(f),
            Table(t) => t.fmt(f),
            Tree(t) => t.fmt(f),
            Func(_) => f.pad("<function>"),
            Commands(c) => c.fmt(f),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        use Value::*;
        match (self, other) {
            (Ident(a), Ident(b)) => a == b,
            (Str(a), Str(b)) => a == b,
            (Bool(a), Bool(b)) => a == b,
            (Number(a), Number(b)) => a == b,
            (Length(a), Length(b)) => a == b,
            (Color(a), Color(b)) => a == b,
            (Table(a), Table(b)) => a == b,
            (Tree(a), Tree(b)) => a == b,
            (Func(a), Func(b)) => Rc::ptr_eq(a, b),
            (Commands(a), Commands(b)) => a == b,
            _ => false,
        }
    }
}

/// An executable function value.
///
/// The first argument is a table containing the arguments passed to the
/// function. The function may be asynchronous (as such it returns a dynamic
/// future) and it may emit diagnostics, which are contained in the returned
/// `Pass`. In the end, the function must evaluate to `Value`. Your typical
/// typesetting function will return a `Commands` value which will instruct the
/// layouting engine to do what the function pleases.
///
/// The dynamic function object is wrapped in an `Rc` to keep `Value` clonable.
pub type FuncValue =
    Rc<dyn Fn(Span, TableValue, LayoutContext<'_>) -> DynFuture<Pass<Value>>>;

/// A table of values.
///
/// # Example
/// ```typst
/// (false, 12cm, greeting="hi")
/// ```
pub type TableValue = Table<SpannedEntry<Value>>;

impl TableValue {
    /// Retrieve and remove the matching value with the lowest number key,
    /// skipping and ignoring all non-matching entries with lower keys.
    pub fn take<T: TryFromValue>(&mut self) -> Option<T> {
        for (&key, entry) in self.nums() {
            let expr = entry.val.as_ref();
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
            if let Some(val) = T::try_from_value(entry.val.as_ref(), f) {
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
            let expr = entry.val.as_ref();
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
                let expr = entry.val.as_ref();
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
                let expr = entry.val.as_ref();
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
            let span = Span::merge(entry.key, entry.val.span);
            error!(@f, span, "unexpected argument");
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
                    let val = $parse(ident.as_str());
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
impl_match!(String, "string", Value::Str(s) => s.clone());
impl_match!(bool, "bool", &Value::Bool(b) => b);
impl_match!(f64, "number", &Value::Number(n) => n);
impl_match!(Length, "length", &Value::Length(l) => l);
impl_match!(SyntaxTree, "tree", Value::Tree(t) => t.clone());
impl_match!(TableValue, "table", Value::Table(t) => t.clone());
impl_match!(FuncValue, "function", Value::Func(f) => f.clone());
impl_match!(ScaleLength, "number or length",
    &Value::Length(length) => ScaleLength::Absolute(length),
    &Value::Number(scale) => ScaleLength::Scaled(scale),
);

/// A value type that matches identifiers and strings and implements
/// `Into<String>`.
pub struct StringLike(pub String);

impl Deref for StringLike {
    type Target = str;

    fn deref(&self) -> &str {
        self.0.as_str()
    }
}

impl From<StringLike> for String {
    fn from(like: StringLike) -> String {
        like.0
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

impl_ident!(FontStyle, "font style", FontStyle::from_name);
impl_ident!(Paper, "paper", Paper::from_name);

impl TryFromValue for FontWeight {
    fn try_from_value(value: Spanned<&Value>, f: &mut Feedback) -> Option<Self> {
        match value.v {
            &Value::Number(weight) => {
                const MIN: u16 = 100;
                const MAX: u16 = 900;

                Some(Self(if weight < MIN as f64 {
                    error!(@f, value.span, "the minimum font weight is {}", MIN);
                    MIN
                } else if weight > MAX as f64 {
                    error!(@f, value.span, "the maximum font weight is {}", MAX);
                    MAX
                } else {
                    weight.round() as u16
                }))
            }
            Value::Ident(ident) => {
                let weight = Self::from_name(ident.as_str());
                if weight.is_none() {
                    error!(@f, value.span, "invalid font weight");
                }
                weight
            }
            other => {
                error!(
                    @f, value.span,
                    "expected font weight (name or number), found {}",
                    other.name(),
                );
                None
            }
        }
    }
}

impl TryFromValue for FontWidth {
    fn try_from_value(value: Spanned<&Value>, f: &mut Feedback) -> Option<Self> {
        match value.v {
            &Value::Number(width) => {
                const MIN: u16 = 1;
                const MAX: u16 = 9;

                Self::new(if width < MIN as f64 {
                    error!(@f, value.span, "the minimum font width is {}", MIN);
                    MIN
                } else if width > MAX as f64 {
                    error!(@f, value.span, "the maximum font width is {}", MAX);
                    MAX
                } else {
                    width.round() as u16
                })
            }
            Value::Ident(ident) => {
                let width = Self::from_name(ident.as_str());
                if width.is_none() {
                    error!(@f, value.span, "invalid font width");
                }
                width
            }
            other => {
                error!(
                    @f, value.span,
                    "expected font width (name or number), found {}",
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
    fn test_table_take_removes_correct_entry() {
        let mut table = Table::new();
        table.insert(1, entry(Value::Bool(false)));
        table.insert(2, entry(Value::Str("hi".to_string())));
        assert_eq!(table.take::<String>(), Some("hi".to_string()));
        assert_eq!(table.len(), 1);
        assert_eq!(table.take::<bool>(), Some(false));
        assert!(table.is_empty());
    }

    #[test]
    fn test_table_expect_errors_about_previous_entries() {
        let mut f = Feedback::new();
        let mut table = Table::new();
        table.insert(1, entry(Value::Bool(false)));
        table.insert(3, entry(Value::Str("hi".to_string())));
        table.insert(5, entry(Value::Bool(true)));
        assert_eq!(
            table.expect::<String>("", Span::ZERO, &mut f),
            Some("hi".to_string())
        );
        assert_eq!(f.diagnostics, [error!(
            Span::ZERO,
            "expected string, found bool"
        )]);
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn test_table_take_with_key_removes_the_entry() {
        let mut f = Feedback::new();
        let mut table = Table::new();
        table.insert(1, entry(Value::Bool(false)));
        table.insert("hi", entry(Value::Bool(true)));
        assert_eq!(table.take::<bool>(), Some(false));
        assert_eq!(table.take_key::<f64>("hi", &mut f), None);
        assert_eq!(f.diagnostics, [error!(
            Span::ZERO,
            "expected number, found bool"
        )]);
        assert!(table.is_empty());
    }

    #[test]
    fn test_table_take_all_removes_the_correct_entries() {
        let mut table = Table::new();
        table.insert(1, entry(Value::Bool(false)));
        table.insert(3, entry(Value::Number(0.0)));
        table.insert(7, entry(Value::Bool(true)));
        assert_eq!(table.take_all_num::<bool>().collect::<Vec<_>>(), [
            (1, false),
            (7, true)
        ],);
        assert_eq!(table.len(), 1);
        assert_eq!(table[3].val.v, Value::Number(0.0));
    }
}
