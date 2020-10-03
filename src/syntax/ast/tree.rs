//! The syntax tree.

use crate::syntax::{Expr, Ident, SpanVec, Spanned};

/// A collection of nodes which form a tree together with the nodes' children.
pub type SynTree = SpanVec<SynNode>;

/// A syntax node, which encompasses a single logical entity of parsed source
/// code.
#[derive(Debug, Clone, PartialEq)]
pub enum SynNode {
    /// Whitespace containing less than two newlines.
    Space,
    /// Plain text.
    Text(String),

    /// A forced line break.
    Linebreak,
    /// A paragraph break.
    Parbreak,
    /// Emphasized text was enabled / disabled.
    Emph,
    /// Strong text was enabled / disabled.
    Strong,

    /// A section heading.
    Heading(NodeHeading),
    /// An optionally syntax-highlighted raw block.
    Raw(NodeRaw),

    /// An expression.
    Expr(Expr),
}

/// A section heading.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeHeading {
    /// The section depth (how many hashtags minus 1).
    pub level: Spanned<u8>,
    /// The contents of the heading.
    pub contents: SynTree,
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
