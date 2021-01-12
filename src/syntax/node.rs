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
                    // Format bracket calls appropriately.
                    pretty_bracket_call(call, p, false)
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
            p.push_str("#");
        }
        self.contents.pretty(p);
    }
}

/// A raw block with optional syntax highlighting: `` `raw` ``.
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
        // TODO: Technically, we should handle backticks in the lines
        // by wrapping with more backticks and possibly adding space
        // before the first or after the last line.
        p.join(&self.lines, "\n", |line, p| p.push_str(line));
        p.push_str("`");
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::test_pretty;

    #[test]
    fn test_pretty_print_bracket_calls() {
        // Top-level call expression formatted as bracket call.
        test_pretty("[v]", "[v]");

        // Blocks are preserved.
        test_pretty("{v()}", "{v()}");
        test_pretty("{[[v]]}", "{[[v]]}");
    }

    #[test]
    fn test_pretty_print_nodes() {
        // Basic text and markup.
        test_pretty(r"*Hi_\", r"*Hi_\");

        // Whitespace.
        test_pretty("  ", " ");
        test_pretty("\n\n\n", "\n\n");

        // Heading and raw.
        test_pretty("# Ok", "# Ok");
        test_pretty("``\none\ntwo\n``", "`one\ntwo`");
        test_pretty("`lang one\ntwo`", "`lang one\ntwo`");
    }
}
