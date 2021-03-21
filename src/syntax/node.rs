use std::rc::Rc;

use super::*;

/// A syntax node, encompassing a single logical entity of parsed source code.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Plain text.
    Text(String),
    /// Whitespace containing less than two newlines.
    Space,
    /// A forced line break: `\`.
    Linebreak(Span),
    /// A paragraph break: Two or more newlines.
    Parbreak(Span),
    /// Strong text was enabled / disabled: `*`.
    Strong(Span),
    /// Emphasized text was enabled / disabled: `_`.
    Emph(Span),
    /// A section heading: `= Introduction`.
    Heading(HeadingNode),
    /// A raw block with optional syntax highlighting: `` `...` ``.
    Raw(RawNode),
    /// An expression.
    Expr(Expr),
}

impl Node {
    // The names of the corresponding library functions.
    pub const LINEBREAK: &'static str = "linebreak";
    pub const PARBREAK: &'static str = "parbreak";
    pub const STRONG: &'static str = "strong";
    pub const EMPH: &'static str = "emph";
    pub const HEADING: &'static str = "heading";
    pub const RAW: &'static str = "raw";

    /// Desugar markup into a function call.
    pub fn desugar(&self) -> Option<CallExpr> {
        match *self {
            Node::Text(_) => None,
            Node::Space => None,
            Node::Linebreak(span) => Some(call(span, Self::LINEBREAK)),
            Node::Parbreak(span) => Some(call(span, Self::PARBREAK)),
            Node::Strong(span) => Some(call(span, Self::STRONG)),
            Node::Emph(span) => Some(call(span, Self::EMPH)),
            Self::Heading(ref heading) => Some(heading.desugar()),
            Self::Raw(ref raw) => Some(raw.desugar()),
            Node::Expr(_) => None,
        }
    }
}

/// A section heading: `= Introduction`.
#[derive(Debug, Clone, PartialEq)]
pub struct HeadingNode {
    /// The source code location.
    pub span: Span,
    /// The section depth (numer of equals signs).
    pub level: usize,
    /// The contents of the heading.
    pub contents: Rc<Tree>,
}

impl HeadingNode {
    pub const LEVEL: &'static str = "level";
    pub const BODY: &'static str = "body";

    /// Desugar into a function call.
    pub fn desugar(&self) -> CallExpr {
        let Self { span, level, ref contents } = *self;
        let mut call = call(span, Node::HEADING);
        call.args.items.push(CallArg::Named(Named {
            name: ident(span, Self::LEVEL),
            expr: Expr::Int(span, level as i64),
        }));
        call.args.items.push(CallArg::Pos(Expr::Template(TemplateExpr {
            span,
            tree: Rc::clone(&contents),
        })));
        call
    }
}

/// A raw block with optional syntax highlighting: `` `...` ``.
///
/// Raw blocks start with 1 or 3+ backticks and end with the same number of
/// backticks.
///
/// When using at least three backticks, an optional language tag may follow
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
///   is surrounded by at least three backticks.
///   ````typst
///   ```rust println!("hello!")```;
///   ````
/// - Blocks can span multiple lines.
///   ````typst
///   ```rust
///   loop {
///      find_yak().shave();
///   }
///   ```
///   ````
/// - Start with a space to omit the language tag (the space will be trimmed
///   from the output).
///   `````typst
///   ```` This has no leading space.````
///   `````
/// - Use more backticks to allow backticks in the raw text.
///   `````typst
///   ```` This contains ```backticks```.````
///   `````
///
/// # Trimming
/// If we would always render the raw text between the backticks exactly as
/// given, some things would become cumbersome/impossible to write:
/// - Typical multiline code blocks (like in the example above) would have an
///   additional newline before and after the code.
/// - Multi-line blocks would need to start with a space since a word would be
///   interpreted as a language tag.
/// - Text ending with a backtick would be impossible since the backtick would
///   be interpreted as belonging to the closing backticks.
///
/// To fix these problems, we sometimes trim a bit of space from blocks with 3+
/// backticks:
/// - At the start, we trim a single space or a sequence of whitespace followed
///   by a newline.
/// - At the end, we trim
///   - a single space if the raw text ends with a backtick followed only by
///     whitespace,
///   - a newline followed by a sequence of whitespace.
///
/// You can thus produce a single backtick without surrounding spaces with the
/// sequence ```` ``` ` ``` ````.
///
/// Note that with these rules you can always force leading or trailing
/// whitespace simply by adding more spaces.
#[derive(Debug, Clone, PartialEq)]
pub struct RawNode {
    /// The source code location.
    pub span: Span,
    /// An optional identifier specifying the language to syntax-highlight in.
    pub lang: Option<Ident>,
    /// The raw text, determined as the raw string between the backticks trimmed
    /// according to the above rules.
    pub text: String,
    /// Whether the element is block-level, that is, it has 3+ backticks
    /// and contains at least one newline.
    pub block: bool,
}

impl RawNode {
    pub const LANG: &'static str = "lang";
    pub const BLOCK: &'static str = "block";
    pub const TEXT: &'static str = "text";

    /// Desugar into a function call.
    pub fn desugar(&self) -> CallExpr {
        let Self { span, ref lang, ref text, block } = *self;
        let mut call = call(span, Node::RAW);
        if let Some(lang) = lang {
            call.args.items.push(CallArg::Named(Named {
                name: ident(span, Self::LANG),
                expr: Expr::Str(span, lang.string.clone()),
            }));
        }
        call.args.items.push(CallArg::Named(Named {
            name: ident(span, Self::BLOCK),
            expr: Expr::Bool(span, block),
        }));
        call.args.items.push(CallArg::Pos(Expr::Str(span, text.clone())));
        call
    }
}

fn call(span: Span, name: &str) -> CallExpr {
    CallExpr {
        span,
        callee: Box::new(Expr::Ident(Ident { span, string: name.into() })),
        args: CallArgs { span, items: vec![] },
    }
}

fn ident(span: Span, string: &str) -> Ident {
    Ident { span, string: string.into() }
}
