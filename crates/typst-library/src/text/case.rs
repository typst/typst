use crate::foundations::{cast, func, Cast, Content, Str};
use crate::text::TextElem;

/// Converts a string or content to lowercase.
///
/// # Example
/// ```example
/// #lower("ABC") \
/// #lower[*My Text*] \
/// #lower[already low]
/// ```
#[func(title = "Lowercase")]
pub fn lower(
    /// The text to convert to lowercase.
    text: Caseable,
) -> Caseable {
    case(text, Case::Lower)
}

/// Converts a string or content to uppercase.
///
/// # Example
/// ```example
/// #upper("abc") \
/// #upper[*my text*] \
/// #upper[ALREADY HIGH]
/// ```
#[func(title = "Uppercase")]
pub fn upper(
    /// The text to convert to uppercase.
    text: Caseable,
) -> Caseable {
    case(text, Case::Upper)
}

/// Change the case of text.
fn case(text: Caseable, case: Case) -> Caseable {
    match text {
        Caseable::Str(v) => Caseable::Str(case.apply(&v).into()),
        Caseable::Content(v) => Caseable::Content(v.set(TextElem::case, Some(case))),
    }
}

/// A value whose case can be changed.
pub enum Caseable {
    Str(Str),
    Content(Content),
}

cast! {
    Caseable,
    self => match self {
        Self::Str(v) => v.into_value(),
        Self::Content(v) => v.into_value(),
    },
    v: Str => Self::Str(v),
    v: Content => Self::Content(v),
}

/// A case transformation on text.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum Case {
    /// Everything is lowercased.
    Lower,
    /// Everything is uppercased.
    Upper,
}

impl Case {
    /// Apply the case to a string.
    pub fn apply(self, text: &str) -> String {
        match self {
            Self::Lower => text.to_lowercase(),
            Self::Upper => text.to_uppercase(),
        }
    }
}
