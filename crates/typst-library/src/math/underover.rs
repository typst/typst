use crate::foundations::{Content, elem};
use crate::math::Mathy;

/// A horizontal line under content.
///
/// ```example
/// $ underline(1 + 2 + ... + 5) $
/// ```
#[elem(Mathy)]
pub struct UnderlineElem {
    /// The content above the line.
    #[required]
    pub body: Content,
}

/// A horizontal line over content.
///
/// ```example
/// $ overline(1 + 2 + ... + 5) $
/// ```
#[elem(Mathy)]
pub struct OverlineElem {
    /// The content below the line.
    #[required]
    pub body: Content,
}

/// A horizontal brace under content, with an optional annotation below.
///
/// ```example
/// $ underbrace(0 + 1 + dots.c + n, n + 1 "numbers") $
/// ```
#[elem(Mathy)]
pub struct UnderbraceElem {
    /// The content above the brace.
    #[required]
    pub body: Content,

    /// The optional content below the brace.
    #[positional]
    pub annotation: Option<Content>,
}

/// A horizontal brace over content, with an optional annotation above.
///
/// ```example
/// $ overbrace(0 + 1 + dots.c + n, n + 1 "numbers") $
/// ```
#[elem(Mathy)]
pub struct OverbraceElem {
    /// The content below the brace.
    #[required]
    pub body: Content,

    /// The optional content above the brace.
    #[positional]
    pub annotation: Option<Content>,
}

/// A horizontal bracket under content, with an optional annotation below.
///
/// ```example
/// $ underbracket(0 + 1 + dots.c + n, n + 1 "numbers") $
/// ```
#[elem(Mathy)]
pub struct UnderbracketElem {
    /// The content above the bracket.
    #[required]
    pub body: Content,

    /// The optional content below the bracket.
    #[positional]
    pub annotation: Option<Content>,
}

/// A horizontal bracket over content, with an optional annotation above.
///
/// ```example
/// $ overbracket(0 + 1 + dots.c + n, n + 1 "numbers") $
/// ```
#[elem(Mathy)]
pub struct OverbracketElem {
    /// The content below the bracket.
    #[required]
    pub body: Content,

    /// The optional content above the bracket.
    #[positional]
    pub annotation: Option<Content>,
}

/// A horizontal parenthesis under content, with an optional annotation below.
///
/// ```example
/// $ underparen(0 + 1 + dots.c + n, n + 1 "numbers") $
/// ```
#[elem(Mathy)]
pub struct UnderparenElem {
    /// The content above the parenthesis.
    #[required]
    pub body: Content,

    /// The optional content below the parenthesis.
    #[positional]
    pub annotation: Option<Content>,
}

/// A horizontal parenthesis over content, with an optional annotation above.
///
/// ```example
/// $ overparen(0 + 1 + dots.c + n, n + 1 "numbers") $
/// ```
#[elem(Mathy)]
pub struct OverparenElem {
    /// The content below the parenthesis.
    #[required]
    pub body: Content,

    /// The optional content above the parenthesis.
    #[positional]
    pub annotation: Option<Content>,
}

/// A horizontal tortoise shell bracket under content, with an optional
/// annotation below.
///
/// ```example
/// $ undershell(0 + 1 + dots.c + n, n + 1 "numbers") $
/// ```
#[elem(Mathy)]
pub struct UndershellElem {
    /// The content above the tortoise shell bracket.
    #[required]
    pub body: Content,

    /// The optional content below the tortoise shell bracket.
    #[positional]
    pub annotation: Option<Content>,
}

/// A horizontal tortoise shell bracket over content, with an optional
/// annotation above.
///
/// ```example
/// $ overshell(0 + 1 + dots.c + n, n + 1 "numbers") $
/// ```
#[elem(Mathy)]
pub struct OvershellElem {
    /// The content below the tortoise shell bracket.
    #[required]
    pub body: Content,

    /// The optional content above the tortoise shell bracket.
    #[positional]
    pub annotation: Option<Content>,
}

/// A marker to distinguish under- and overlines.
pub enum Position {
    Under,
    Over,
}
