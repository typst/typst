//! Literals.

use super::*;
use crate::color::RgbaColor;
use crate::eval::{DictKey, SpannedEntry, Value, ValueDict};
use crate::layout::LayoutContext;
use crate::length::Length;
use crate::DynFuture;

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
    pub async fn eval(&self, ctx: &mut LayoutContext) -> Value {
        match *self {
            Lit::Ident(ref v) => Value::Ident(v.clone()),
            Lit::Bool(v) => Value::Bool(v),
            Lit::Int(v) => Value::Int(v),
            Lit::Float(v) => Value::Float(v),
            Lit::Length(v) => Value::Length(v.as_raw()),
            Lit::Percent(v) => Value::Relative(v / 100.0),
            Lit::Color(v) => Value::Color(v),
            Lit::Str(ref v) => Value::Str(v.clone()),
            Lit::Dict(ref v) => Value::Dict(v.eval(ctx).await),
            Lit::Content(ref v) => Value::Content(v.clone()),
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
    pub fn eval<'a>(&'a self, ctx: &'a mut LayoutContext) -> DynFuture<'a, ValueDict> {
        Box::pin(async move {
            let mut dict = ValueDict::new();

            for entry in &self.0 {
                let val = entry.expr.v.eval(ctx).await;
                let spanned = val.span_with(entry.expr.span);
                if let Some(key) = &entry.key {
                    dict.insert(&key.v, SpannedEntry::new(key.span, spanned));
                } else {
                    dict.push(SpannedEntry::value(spanned));
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
