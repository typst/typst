//! The syntax tree.

use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;

use unicode_xid::UnicodeXID;

use super::span::{SpanVec, SpanWith, Spanned};
use super::Decoration;
use crate::color::RgbaColor;
use crate::compute::table::{SpannedEntry, Table};
use crate::compute::value::{TableValue, Value};
use crate::layout::LayoutContext;
use crate::length::Length;
use crate::{DynFuture, Feedback};

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
    /// An optionally syntax-highlighted raw block.
    Raw(Raw),
    /// Section headings.
    Heading(Heading),
    /// A function call.
    Call(CallExpr),
}

/// A raw block, rendered in monospace with optional syntax highlighting.
///
/// Raw blocks start with an arbitrary number of backticks and end with the same
/// number of backticks. If you want to include a sequence of backticks in a raw
/// block, simply surround the block with more backticks.
///
/// When using at least two backticks, an optional language tag may follow
/// directly after the backticks. This tag defines which language to
/// syntax-highlight the text in. Apart from the language tag and some
/// whitespace trimming discussed below, everything inside a raw block is
/// rendered verbatim, in particular, there are no escape sequences.
///
/// # Examples
/// - Raw text is surrounded by backticks.
///   ```typst
///   `raw`
///   ```
/// - An optional language tag may follow directly at the start when the block
///   is surrounded by at least two backticks.
///   ```typst
///   ``rust println!("hello!")``;
///   ```
/// - Blocks can span multiple lines. Two backticks suffice to be able to
///   specify the language tag, but three are fine, too.
///   ```typst
///   ``rust
///   loop {
///      find_yak().shave();
///   }
///   ``
///   ```
/// - Start with a space to omit the language tag (the space will be trimmed
///   from the output) and use more backticks to allow backticks in the raw
///   text.
///   `````typst
///   ```` This contains ```backticks``` and has no leading & trailing spaces. ````
///   `````
///
///   # Trimming
///   If we would always render the raw text between the backticks exactly as
///   given, a few things would become problematic or even impossible:
///   - Typical multiline code blocks (like in the example above) would have an
///     additional newline before and after the code.
///   - Raw text wrapped in more than one backtick could not exist without
///     leading whitespace since the first word would be interpreted as a
///     language tag.
///   - A single backtick without surrounding spaces could not exist as raw text
///     since it would be interpreted as belonging to the opening or closing
///     backticks.
///
///   To fix these problems, we trim text in multi-backtick blocks as follows:
///   - We trim a single space or a sequence of whitespace followed by a newline
///     at the start.
///   - We trim a single space or a newline followed by a sequence of whitespace
///     at the end.
///
///   With these rules, a single raw backtick can be produced by the sequence
///   ``` `` ` `` ```, ``` `` unhighlighted text `` ``` has no surrounding
///   spaces and multiline code blocks don't have extra empty lines. Note that
///   you can always force leading or trailing whitespace simply by adding more
///   spaces.
#[derive(Debug, Clone, PartialEq)]
pub struct Raw {
    /// An optional identifier specifying the language to syntax-highlight in.
    pub lang: Option<Ident>,
    /// The lines of raw text, determined as the raw string between the
    /// backticks trimmed according to the above rules and split at newlines.
    pub lines: Vec<String>,
    /// Whether the element can be layouted inline.
    ///
    /// - When true, it will be layouted integrated within the surrounding
    ///   paragraph.
    /// - When false, it will be separated into its own paragraph.
    ///
    /// Single-backtick blocks are always inline-level. Multi-backtick blocks
    /// are inline-level when they contain no newlines.
    pub inline: bool,
}

/// A section heading.
#[derive(Debug, Clone, PartialEq)]
pub struct Heading {
    /// The section depth (how many hashtags minus 1).
    pub level: Spanned<u8>,
    pub tree: SyntaxTree,
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
    pub async fn eval(&self, ctx: &LayoutContext<'_>, f: &mut Feedback) -> Value {
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

/// An identifier as defined by unicode with a few extra permissible characters.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Ident(pub String);

impl Ident {
    /// Create a new identifier from a string checking that it is a valid.
    pub fn new(ident: impl AsRef<str> + Into<String>) -> Option<Self> {
        if is_ident(ident.as_ref()) {
            Some(Self(ident.into()))
        } else {
            None
        }
    }

    /// Return a reference to the underlying string.
    pub fn as_str(&self) -> &str {
        self
    }
}

impl AsRef<str> for Ident {
    fn as_ref(&self) -> &str {
        self
    }
}

impl Deref for Ident {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl Debug for Ident {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "`{}`", self.0)
    }
}

/// Whether the string is a valid identifier.
pub fn is_ident(string: &str) -> bool {
    fn is_ok(c: char) -> bool {
        c == '-' || c == '_'
    }

    let mut chars = string.chars();
    if matches!(chars.next(), Some(c) if c.is_xid_start() || is_ok(c)) {
        chars.all(|c| c.is_xid_continue() || is_ok(c))
    } else {
        false
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
                let spanned = val.span_with(entry.val.span);
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
        let name = &self.name.v;
        let span = self.name.span;
        let args = self.args.eval(ctx, f).await;

        if let Some(func) = ctx.scope.func(name) {
            let pass = func(span, args, ctx.clone()).await;
            f.extend(pass.feedback);
            f.decorations.push(Decoration::Resolved.span_with(span));
            pass.output
        } else {
            if !name.is_empty() {
                error!(@f, span, "unknown function");
                f.decorations.push(Decoration::Unresolved.span_with(span));
            }
            Value::Table(args)
        }
    }
}
