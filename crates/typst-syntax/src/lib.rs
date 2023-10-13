//! Parser and syntax tree for Typst.

pub mod ast;

mod file;
mod highlight;
mod kind;
mod lexer;
mod node;
mod parser;
mod reparser;
mod source;
mod span;

pub use self::file::{FileId, PackageSpec, PackageVersion, VirtualPath};
pub use self::highlight::{highlight, highlight_html, Tag};
pub use self::kind::SyntaxKind;
pub use self::lexer::{is_id_continue, is_id_start, is_ident, is_newline};
pub use self::node::{LinkedChildren, LinkedNode, SyntaxError, SyntaxNode};
pub use self::parser::{parse, parse_code, parse_math};
pub use self::source::Source;
pub use self::span::{Span, Spanned};

use self::lexer::{split_newlines, LexMode, Lexer};
use self::parser::{reparse_block, reparse_markup};
