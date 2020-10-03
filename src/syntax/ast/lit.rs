//! Literals.

use crate::color::RgbaColor;
use crate::eval::{DictKey, DictValue, SpannedEntry, Value};
use crate::layout::LayoutContext;
use crate::length::Length;
use crate::syntax::{Expr, Ident, SpanWith, Spanned, SynTree};
use crate::{DynFuture, Feedback};

/// A literal.
#[derive(Debug, Clone, PartialEq)]
pub enum Lit {
    /// A identifier literal: `left`.
    Ident(Ident),
    /// A boolean literal: `true`, `false`.
    Bool(bool),
    /// An integer literal: `120`.
    Int(i64),
    /// A floating-point literal: `1.2`, `10e-4`.
    Float(f64),
    /// A length literal: `12pt`, `3cm`.
    Length(Length),
    /// A percent literal: `50%`.
    ///
    /// _Note_: `50%` is represented as `50.0` here, but as `0.5` in the
    /// corresponding [value].
    ///
    /// [value]: ../../eval/enum.Value.html#variant.Relative
    Percent(f64),
    /// A color literal: `#ffccee`.
    Color(RgbaColor),
    /// A string literal: `"hello!"`.
    Str(String),
    /// A dictionary literal: `(false, 12cm, greeting = "hi")`.
    Dict(LitDict),
    /// A content literal: `{*Hello* there!}`.
    Content(SynTree),
}

impl Lit {
    /// Evaluate the dictionary literal to a dictionary value.
    pub async fn eval(&self, ctx: &LayoutContext<'_>, f: &mut Feedback) -> Value {
        match *self {
            Lit::Ident(ref i) => Value::Ident(i.clone()),
            Lit::Bool(b) => Value::Bool(b),
            Lit::Int(i) => Value::Int(i),
            Lit::Float(f) => Value::Float(f),
            Lit::Length(l) => Value::Length(l.as_raw()),
            Lit::Percent(p) => Value::Relative(p / 100.0),
            Lit::Color(c) => Value::Color(c),
            Lit::Str(ref s) => Value::Str(s.clone()),
            Lit::Dict(ref d) => Value::Dict(d.eval(ctx, f).await),
            Lit::Content(ref c) => Value::Tree(c.clone()),
        }
    }
}

/// A dictionary literal: `(false, 12cm, greeting = "hi")`.
#[derive(Debug, Clone, PartialEq)]
pub struct LitDict(pub Vec<LitDictEntry>);

impl LitDict {
    /// Create an empty dict literal.
    pub fn new() -> Self {
        Self(vec![])
    }

    /// Evaluate the dictionary literal to a dictionary value.
    pub fn eval<'a>(
        &'a self,
        ctx: &'a LayoutContext<'a>,
        f: &'a mut Feedback,
    ) -> DynFuture<'a, DictValue> {
        Box::pin(async move {
            let mut dict = DictValue::new();

            for entry in &self.0 {
                let val = entry.expr.v.eval(ctx, f).await;
                let spanned = val.span_with(entry.expr.span);
                if let Some(key) = &entry.key {
                    dict.insert(&key.v, SpannedEntry::new(key.span, spanned));
                } else {
                    dict.push(SpannedEntry::val(spanned));
                }
            }

            dict
        })
    }
}

/// An entry in a dictionary literal: `false` or `greeting = "hi"`.
#[derive(Debug, Clone, PartialEq)]
pub struct LitDictEntry {
    /// The key of the entry if there was one: `greeting`.
    pub key: Option<Spanned<DictKey>>,
    /// The value of the entry: `"hi"`.
    pub expr: Spanned<Expr>,
}
