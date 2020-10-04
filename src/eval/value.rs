//! Computational values: Syntactical expressions can be evaluated into these.

use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;
use std::rc::Rc;

use super::{Args, Dict, SpannedEntry};
use crate::color::RgbaColor;
use crate::geom::Linear;
use crate::layout::{Command, LayoutContext};
use crate::syntax::{Ident, Span, SpanWith, Spanned, SynNode, SynTree};
use crate::DynFuture;

/// A computational value.
#[derive(Clone, PartialEq)]
pub enum Value {
    /// The value that indicates the absence of a meaningful value.
    None,
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
    /// A combination of an absolute length and a relative value: `20% + 5cm`.
    Linear(Linear),
    /// A color value with alpha channel: `#f79143ff`.
    Color(RgbaColor),
    /// A string: `"string"`.
    Str(String),
    /// A dictionary value: `(false, 12cm, greeting="hi")`.
    Dict(ValueDict),
    /// A content value: `{*Hi* there}`.
    Content(SynTree),
    /// An executable function.
    Func(ValueFunc),
    /// Layouting commands.
    Commands(Vec<Command>),
    /// The result of invalid operations.
    Error,
}

impl Value {
    /// The natural-language name of this value's type for use in error
    /// messages.
    pub fn ty(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Ident(_) => "ident",
            Self::Bool(_) => "bool",
            Self::Int(_) => "int",
            Self::Float(_) => "float",
            Self::Relative(_) => "relative",
            Self::Length(_) => "length",
            Self::Linear(_) => "linear",
            Self::Color(_) => "color",
            Self::Str(_) => "string",
            Self::Dict(_) => "dict",
            Self::Content(_) => "content",
            Self::Func(_) => "function",
            Self::Commands(_) => "commands",
            Self::Error => "error",
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::None
    }
}

impl Spanned<Value> {
    /// Transform this value into something layoutable.
    ///
    /// If this is already a command-value, it is simply unwrapped, otherwise
    /// the value is represented as layoutable content in a reasonable way.
    pub fn into_commands(self) -> Vec<Command> {
        match self.v {
            // Pass-through.
            Value::Commands(commands) => commands,
            Value::Content(tree) => vec![Command::LayoutSyntaxTree(tree)],

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

            // Don't print out none values.
            Value::None => vec![],

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
            Self::None => f.pad("none"),
            Self::Ident(v) => v.fmt(f),
            Self::Bool(v) => v.fmt(f),
            Self::Int(v) => v.fmt(f),
            Self::Float(v) => v.fmt(f),
            Self::Length(v) => v.fmt(f),
            Self::Relative(v) => v.fmt(f),
            Self::Linear(v) => v.fmt(f),
            Self::Color(v) => v.fmt(f),
            Self::Str(v) => v.fmt(f),
            Self::Dict(v) => v.fmt(f),
            Self::Content(v) => v.fmt(f),
            Self::Func(v) => v.fmt(f),
            Self::Commands(v) => v.fmt(f),
            Self::Error => f.pad("<error>"),
        }
    }
}

/// An wrapper around a reference-counted executable function value.
///
/// The dynamic function object is wrapped in an `Rc` to keep [`Value`]
/// clonable.
///
/// _Note_: This is needed because the compiler can't `derive(PartialEq)`
///         for `Value` when directly putting the boxed function in there,
///         see the [Rust Issue].
///
/// [`Value`]: enum.Value.html
/// [Rust Issue]: https://github.com/rust-lang/rust/issues/31740
#[derive(Clone)]
pub struct ValueFunc(pub Rc<Func>);

/// The signature of executable functions.
pub type Func = dyn Fn(Args, &mut LayoutContext) -> DynFuture<Value>;

impl ValueFunc {
    /// Create a new function value from a rust function or closure.
    pub fn new<F: 'static>(f: F) -> Self
    where
        F: Fn(Args, &mut LayoutContext) -> DynFuture<Value>,
    {
        Self(Rc::new(f))
    }
}

impl Eq for ValueFunc {}

impl PartialEq for ValueFunc {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Deref for ValueFunc {
    type Target = Func;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl Debug for ValueFunc {
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
pub type ValueDict = Dict<SpannedEntry<Value>>;
