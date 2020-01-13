//! Tokenization and parsing of source code.

use std::fmt::{self, Display, Formatter};
use unicode_xid::UnicodeXID;

use crate::func::LayoutFunc;
use crate::size::{Size, ScaleSize};


pub type ParseResult<T> = crate::TypesetResult<T>;

pub_use_mod!(color);
pub_use_mod!(expr);
pub_use_mod!(tokens);
pub_use_mod!(parsing);
pub_use_mod!(span);
