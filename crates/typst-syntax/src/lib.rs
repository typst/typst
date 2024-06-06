//! Parser and syntax tree for Typst.

pub mod ast;
pub mod package;

mod file;
mod highlight;
mod kind;
mod lexer;
mod node;
mod parser;
mod path;
mod reparser;
mod set;
mod source;
mod span;

pub use self::file::FileId;
pub use self::highlight::{highlight, highlight_html, Tag};
pub use self::kind::SyntaxKind;
pub use self::lexer::{
    is_id_continue, is_id_start, is_ident, is_newline, is_valid_label_literal_id,
    link_prefix, split_newlines,
};
pub use self::node::{LinkedChildren, LinkedNode, Side, SyntaxError, SyntaxNode};
pub use self::parser::{parse, parse_code, parse_math};
pub use self::path::VirtualPath;
pub use self::source::Source;
pub use self::span::{Span, Spanned};

use self::lexer::{LexMode, Lexer};
use self::parser::{reparse_block, reparse_markup};
