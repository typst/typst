//! Syntax definition, parsing, and highlighting.

pub mod ast;

mod kind;
mod lexer;
mod node;
mod parser;
mod reparser;
mod source;
mod span;

pub use self::kind::SyntaxKind;
pub use self::lexer::{is_ident, is_newline};
pub use self::node::{ErrorPos, LinkedChildren, LinkedNode, SyntaxNode};
pub use self::parser::{parse, parse_code};
pub use self::source::{Source, SourceId};
pub use self::span::{Span, Spanned};

pub(crate) use self::lexer::{is_id_continue, is_id_start};

use self::lexer::{split_newlines, LexMode, Lexer};
use self::parser::{reparse_block, reparse_markup};
