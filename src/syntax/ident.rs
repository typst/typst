//! Unicode identifiers.

use std::ops::Deref;

use unicode_xid::UnicodeXID;

/// An identifier as defined by unicode with a few extra permissible characters.
///
/// This is defined as in the [Unicode Standard], but allows additionally
/// `-` and `_` as starting and continuing characters.
///
/// [Unicode Standard]: http://www.unicode.org/reports/tr31/
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Ident(pub String);

impl Ident {
    /// Create a new identifier from a string checking that it is a valid.
    pub fn new(ident: impl AsRef<str> + Into<String>) -> Option<Self> {
        if is_ident(ident.as_ref()) {
            Some(Self(ident.into()))
        } else {
            None
        }
    }

    /// Return a reference to the underlying string.
    pub fn as_str(&self) -> &str {
        self
    }
}

impl AsRef<str> for Ident {
    fn as_ref(&self) -> &str {
        self
    }
}

impl Deref for Ident {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

/// Whether the string is a valid identifier.
pub fn is_ident(string: &str) -> bool {
    let mut chars = string.chars();
    chars
        .next()
        .map_or(false, |c| is_id_start(c) && chars.all(is_id_continue))
}

/// Whether the character can start an identifier.
pub fn is_id_start(c: char) -> bool {
    c.is_xid_start() || c == '_'
}

/// Whether the character can continue an identifier.
pub fn is_id_continue(c: char) -> bool {
    c.is_xid_continue() || c == '_' || c == '-'
}
