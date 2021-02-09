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
            Self::Strong => p.push('*'),
            Self::Emph => p.push('_'),
            Self::Space => p.push(' '),
            Self::Linebreak => p.push_str(r"\"),
            Self::Parbreak => p.push_str("\n\n"),
            Self::Text(text) => p.push_str(&text),
            Self::Heading(heading) => heading.pretty(p),
            Self::Raw(raw) => raw.pretty(p),
            Self::Expr(expr) => {
                if let Expr::Call(call) = expr {
                    // Format function templates appropriately.
                    call.pretty_bracketed(p, false)
                } else {
                    expr.pretty(p);
                }
            }
        }
    }
}

/// A section heading: `= Introduction`.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeHeading {
    /// The section depth (numer of equals signs minus 1).
    pub level: usize,
    /// The contents of the heading.
    pub contents: Tree,
}

impl Pretty for NodeHeading {
    fn pretty(&self, p: &mut Printer) {
        for _ in 0 ..= self.level {
            p.push('=');
        }
        self.contents.pretty(p);
    }
}

/// A raw block with optional syntax highlighting: `` `raw` ``.
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
pub struct NodeRaw {
    /// An optional identifier specifying the language to syntax-highlight in.
    pub lang: Option<Ident>,
    /// The lines of raw text, determined as the raw string between the
    /// backticks trimmed according to the above rules and split at newlines.
    pub lines: Vec<String>,
    /// Whether the element is block-level, that is, it has 3+ backticks
    /// and contains at least one newline.
    pub block: bool,
}

impl Pretty for NodeRaw {
    fn pretty(&self, p: &mut Printer) {
        // Find out how many backticks we need.
        let mut backticks = 1;

        // Language tag and block-level are only possible with 3+ backticks.
        if self.lang.is_some() || self.block {
            backticks = 3;
        }

        // More backticks may be required if there are lots of consecutive
        // backticks in the lines.
        let mut count;
        for line in &self.lines {
            count = 0;
            for c in line.chars() {
                if c == '`' {
                    count += 1;
                    backticks = backticks.max(3).max(count + 1);
                } else {
                    count = 0;
                }
            }
        }

        // Starting backticks.
        for _ in 0 .. backticks {
            p.push('`');
        }

        // Language tag.
        if let Some(lang) = &self.lang {
            lang.pretty(p);
        }

        // Start untrimming.
        if self.block {
            p.push('\n');
        } else if backticks >= 3 {
            p.push(' ');
        }

        // The lines.
        p.join(&self.lines, "\n", |line, p| p.push_str(line));

        // End untrimming.
        if self.block {
            p.push('\n');
        } else if self.lines.last().map_or(false, |line| line.trim_end().ends_with('`')) {
            p.push(' ');
        }

        // Ending backticks.
        for _ in 0 .. backticks {
            p.push('`');
        }
    }
}
