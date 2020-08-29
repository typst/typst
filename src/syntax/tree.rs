//! The syntax tree.

use std::fmt::{self, Debug, Formatter};

use crate::color::RgbaColor;
use crate::compute::table::{SpannedEntry, Table};
use crate::compute::value::{TableValue, Value};
use crate::layout::LayoutContext;
use crate::length::Length;
use crate::{DynFuture, Feedback};
use super::decoration::Decoration;
use super::span::{Spanned, SpanVec};
use super::Ident;

/// A collection of nodes which form a tree together with the nodes' children.
pub type SyntaxTree = SpanVec<SyntaxNode>;

/// A syntax node, which encompasses a single logical entity of parsed source
/// code.
#[derive(Debug, Clone, PartialEq)]
pub enum SyntaxNode {
    /// Whitespace containing less than two newlines.
    Spacing,
    /// A forced line break.
    Linebreak,
    /// A paragraph break.
    Parbreak,
    /// Italics were enabled / disabled.
    ToggleItalic,
    /// Bolder was enabled / disabled.
    ToggleBolder,
    /// Plain text.
    Text(String),
    /// Lines of raw text.
    Raw(Vec<String>),
    /// An optionally highlighted multi-line code block.
    CodeBlock(CodeBlockExpr),
    /// A function call.
    Call(CallExpr),
}

/// An expression.
#[derive(Clone, PartialEq)]
pub enum Expr {
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
    /// A table expression: `(false, 12cm, greeting="hi")`.
    Table(TableExpr),
    /// A syntax tree containing typesetting content.
    Tree(SyntaxTree),
    /// A function call expression: `cmyk(37.7, 0, 3.9, 1.1)`.
    Call(CallExpr),
    /// An operation that negates the contained expression.
    Neg(Box<Spanned<Expr>>),
    /// An operation that adds the contained expressions.
    Add(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    /// An operation that subtracts the contained expressions.
    Sub(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    /// An operation that multiplies the contained expressions.
    Mul(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    /// An operation that divides the contained expressions.
    Div(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
}

impl Expr {
    /// A natural-language name of the type of this expression, e.g.
    /// "identifier".
    pub fn name(&self) -> &'static str {
        use Expr::*;
        match self {
            Ident(_) => "identifier",
            Str(_) => "string",
            Bool(_) => "bool",
            Number(_) => "number",
            Length(_) => "length",
            Color(_) => "color",
            Table(_) => "table",
            Tree(_) => "syntax tree",
            Call(_) => "function call",
            Neg(_) => "negation",
            Add(_, _) => "addition",
            Sub(_, _) => "subtraction",
            Mul(_, _) => "multiplication",
            Div(_, _) => "division",
        }
    }

    /// Evaluate the expression to a value.
    pub async fn eval(
        &self,
        ctx: &LayoutContext<'_>,
        f: &mut Feedback,
    ) -> Value {
        use Expr::*;
        match self {
            Ident(i) => Value::Ident(i.clone()),
            Str(s) => Value::Str(s.clone()),
            &Bool(b) => Value::Bool(b),
            &Number(n) => Value::Number(n),
            &Length(s) => Value::Length(s),
            &Color(c) => Value::Color(c),
            Table(t) => Value::Table(t.eval(ctx, f).await),
            Tree(t) => Value::Tree(t.clone()),
            Call(call) => call.eval(ctx, f).await,
            Neg(_) => todo!("eval neg"),
            Add(_, _) => todo!("eval add"),
            Sub(_, _) => todo!("eval sub"),
            Mul(_, _) => todo!("eval mul"),
            Div(_, _) => todo!("eval div"),
        }
    }
}

impl Debug for Expr {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use Expr::*;
        match self {
            Ident(i) => i.fmt(f),
            Str(s) => s.fmt(f),
            Bool(b) => b.fmt(f),
            Number(n) => n.fmt(f),
            Length(s) => s.fmt(f),
            Color(c) => c.fmt(f),
            Table(t) => t.fmt(f),
            Tree(t) => t.fmt(f),
            Call(c) => c.fmt(f),
            Neg(e) => write!(f, "-{:?}", e),
            Add(a, b) => write!(f, "({:?} + {:?})", a, b),
            Sub(a, b) => write!(f, "({:?} - {:?})", a, b),
            Mul(a, b) => write!(f, "({:?} * {:?})", a, b),
            Div(a, b) => write!(f, "({:?} / {:?})", a, b),
        }
    }
}

/// A table of expressions.
///
/// # Example
/// ```typst
/// (false, 12cm, greeting="hi")
/// ```
pub type TableExpr = Table<SpannedEntry<Expr>>;

impl TableExpr {
    /// Evaluate the table expression to a table value.
    pub fn eval<'a>(
        &'a self,
        ctx: &'a LayoutContext<'a>,
        f: &'a mut Feedback,
    ) -> DynFuture<'a, TableValue> {
        Box::pin(async move {
            let mut table = TableValue::new();

            for (key, entry) in self.iter() {
                let val = entry.val.v.eval(ctx, f).await;
                let spanned = Spanned::new(val, entry.val.span);
                let entry = SpannedEntry::new(entry.key, spanned);
                table.insert(key, entry);
            }

            table
        })
    }
}

/// An invocation of a function.
#[derive(Debug, Clone, PartialEq)]
pub struct CallExpr {
    pub name: Spanned<Ident>,
    pub args: TableExpr,
}

impl CallExpr {
    /// Evaluate the call expression to a value.
    pub async fn eval(&self, ctx: &LayoutContext<'_>, f: &mut Feedback) -> Value {
        let name = self.name.v.as_str();
        let span = self.name.span;
        let args = self.args.eval(ctx, f).await;

        if let Some(func) = ctx.scope.func(name) {
            let pass = func(span, args, ctx.clone()).await;
            f.extend(pass.feedback);
            f.decorations.push(Spanned::new(Decoration::Resolved, span));
            pass.output
        } else {
            if !name.is_empty() {
                error!(@f, span, "unknown function");
                f.decorations.push(Spanned::new(Decoration::Unresolved, span));
            }
            Value::Table(args)
        }
    }
}
/// An code block.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeBlockExpr {
    pub lang: Option<Spanned<Ident>>,
    pub raw: Vec<String>,
}
