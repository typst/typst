//! Parser and syntax tree for Typst.

pub mod ast;
pub mod package;

mod file;
mod highlight;
mod kind;
mod lexer;
mod lines;
mod node;
mod parser;
mod path;
mod reparser;
mod set;
mod source;
mod span;
mod version;

pub use self::file::FileId;
pub use self::highlight::{Tag, highlight, highlight_html};
pub use self::kind::SyntaxKind;
pub use self::lexer::{
    is_id_continue, is_id_start, is_ident, is_newline, is_valid_label_literal_id,
    link_prefix, split_newlines,
};
pub use self::lines::Lines;
pub use self::node::{LinkedChildren, LinkedNode, Side, SyntaxError, SyntaxNode};
pub use self::parser::{parse, parse_code, parse_math};
pub use self::path::VirtualPath;
pub use self::source::Source;
pub use self::span::{Span, Spanned};
pub use self::version::TypstVersion;

use self::lexer::Lexer;
use self::parser::{reparse_block, reparse_markup};

/// The syntax mode of a portion of Typst code.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum SyntaxMode {
    /// Text and markup, as in the top level.
    Markup,
    /// Math atoms, operators, etc., as in equations.
    Math,
    /// Keywords, literals and operators, as after hashes.
    Code,
}
