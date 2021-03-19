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
    Heading(HeadingNode),
    /// An optionally syntax-highlighted raw block.
    Raw(RawNode),
    /// An expression.
    Expr(Expr),
}

/// A section heading: `= Introduction`.
#[derive(Debug, Clone, PartialEq)]
pub struct HeadingNode {
    /// The section depth (numer of equals signs minus 1).
    pub level: usize,
    /// The contents of the heading.
    pub contents: Tree,
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
    /// An optional identifier specifying the language to syntax-highlight in.
    pub lang: Option<Ident>,
    /// The lines of raw text, determined as the raw string between the
    /// backticks trimmed according to the above rules and split at newlines.
    pub lines: Vec<String>,
    /// Whether the element is block-level, that is, it has 3+ backticks
    /// and contains at least one newline.
    pub block: bool,
}
