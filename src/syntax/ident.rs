use std::borrow::Borrow;
use std::ops::Deref;

use unicode_xid::UnicodeXID;

use super::{NodeKind, RedTicket, Span, TypedNode};
use crate::util::EcoString;

/// An unicode identifier with a few extra permissible characters.
///
/// In addition to what is specified in the [Unicode Standard][uax31], we allow:
/// - `_` as a starting character,
/// - `_` and `-` as continuing characters.
///
/// [uax31]: http://www.unicode.org/reports/tr31/
#[derive(Debug, Clone, PartialEq)]
pub struct Ident {
    /// The source code location.
    pub span: Span,
    /// The identifier string.
    pub string: EcoString,
}

impl Ident {
    /// Create a new identifier from a string checking that it is a valid.
    pub fn new(
        string: impl AsRef<str> + Into<EcoString>,
        span: impl Into<Span>,
    ) -> Option<Self> {
        if is_ident(string.as_ref()) {
            Some(Self { span: span.into(), string: string.into() })
        } else {
            None
        }
    }

    /// Return a reference to the underlying string.
    pub fn as_str(&self) -> &str {
        self
    }
}

impl Deref for Ident {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.string.as_str()
    }
}

impl AsRef<str> for Ident {
    fn as_ref(&self) -> &str {
        self
    }
}

impl Borrow<str> for Ident {
    fn borrow(&self) -> &str {
        self
    }
}

impl From<&Ident> for EcoString {
    fn from(ident: &Ident) -> Self {
        ident.string.clone()
    }
}

impl TypedNode for Ident {
    fn cast_from(node: RedTicket) -> Option<Self> {
        if let NodeKind::Ident(i) = node.kind() {
            Some(Ident::new(i, node.own().span()).unwrap())
        } else {
            None
        }
    }
}

/// Whether a string is a valid identifier.
pub fn is_ident(string: &str) -> bool {
    let mut chars = string.chars();
    chars
        .next()
        .map_or(false, |c| is_id_start(c) && chars.all(is_id_continue))
}

/// Whether a character can start an identifier.
pub fn is_id_start(c: char) -> bool {
    c.is_xid_start() || c == '_'
}

/// Whether a character can continue an identifier.
pub fn is_id_continue(c: char) -> bool {
    c.is_xid_continue() || c == '_' || c == '-'
}
