use super::*;

/// A syntax node, encompassing a single logical entity of parsed source code.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Strong text was enabled / disabled.
    Strong,
    /// Emphasized text was enabled / disabled.
    Emph,
    /// Whitespace containing less than two newlines.
    Space,
    /// A forced line break.
    Linebreak,
    /// A paragraph break.
    Parbreak,
    /// Plain text.
    Text(String),
    /// A section heading.
    Heading(NodeHeading),
    /// An optionally syntax-highlighted raw block.
    Raw(NodeRaw),
    /// An expression.
    Expr(Expr),
}

impl Pretty for Node {
    fn pretty(&self, p: &mut Printer) {
        match self {
            Self::Strong => p.push_str("*"),
            Self::Emph => p.push_str("_"),
            Self::Space => p.push_str(" "),
            Self::Linebreak => p.push_str(r"\"),
            Self::Parbreak => p.push_str("\n\n"),
            Self::Text(text) => p.push_str(&text),
            Self::Heading(heading) => heading.pretty(p),
            Self::Raw(raw) => raw.pretty(p),
            Self::Expr(expr) => {
                if let Expr::Call(call) = expr {
                    // Format function templates appropriately.
                    pretty_func_template(call, p, false)
                } else {
                    expr.pretty(p);
                }
            }
        }
    }
}

/// A section heading: `# Introduction`.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeHeading {
    /// The section depth (numer of hashtags minus 1, capped at 5).
    pub level: Spanned<u8>,
    /// The contents of the heading.
    pub contents: Tree,
}

impl Pretty for NodeHeading {
    fn pretty(&self, p: &mut Printer) {
        for _ in 0 ..= self.level.v {
            p.push_str("=");
        }
        self.contents.pretty(p);
    }
}

/// A raw block with optional syntax highlighting: `` `raw` ``.
///
/// Raw blocks start with 1 or 3+ backticks and end with the same number of
/// backticks. If you want to include a sequence of backticks in a raw block,
/// simply surround the block with more backticks.
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
///   - Blocks can span multiple lines.
///   ````typst
///   ```rust
///   loop {
///      find_yak().shave();
///   }
///   ```
///   ````
///   - Start with a space to omit the language tag (the space will be trimmed
///     from the output) and use more backticks to allow backticks in the raw
///     text.
///   `````typst
///   ```` This contains ```backticks``` and has no leading & trailing spaces. ````
///   `````
///
///   # Trimming
///   If we would always render the raw text between the backticks exactly as
///   given, a few things would become problematic or even impossible:
///   - Typical multiline code blocks (like in the example above) would have an
///     additional newline before and after the code.
///   - The first word of text wrapped in more than three backticks would always
///     be interpreted as a language tag which means that text without leading
///     space would be impossible.
///   - A single backtick without surrounding spaces could not exist as raw text
///     since it would be interpreted as belonging to the opening or closing
///     backticks.
///
///   To fix these problems, we trim blocks with 3+ backticks as follows:
///   - A single space or a sequence of whitespace followed by a newline at the start.
///   - A single space or a newline followed by a sequence of whitespace at the end.
///
///   With these rules, a single raw backtick can be produced by the sequence
///   ```` ``` ` ``` ````, ```` ``` unhighlighted text ``` ```` has no
///   surrounding spaces and multiline code blocks don't have extra empty lines.
///   Note that you can always force leading or trailing whitespace simply by
///   adding more spaces.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeRaw {
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

impl Pretty for NodeRaw {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("`");
        if let Some(lang) = &self.lang {
            p.push_str(&lang);
            p.push_str(" ");
        }
        // TODO: Technically, we should handle backticks in the lines by
        // wrapping with more backticks, and we should add space before the
        // first and/or after the last line if necessary.
        p.join(&self.lines, "\n", |line, p| p.push_str(line));
        p.push_str("`");
    }
}
