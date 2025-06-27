use codex::styling::MathVariant;

use crate::foundations::{func, Cast, Content};
use crate::math::EquationElem;

/// Bold font style in math.
///
/// ```example
/// $ bold(A) := B^+ $
/// ```
#[func(keywords = ["mathbf"])]
pub fn bold(
    /// The content to style.
    body: Content,
) -> Content {
    body.styled(EquationElem::set_bold(true))
}

/// Upright (non-italic) font style in math.
///
/// ```example
/// $ upright(A) != A $
/// ```
#[func(keywords = ["mathup"])]
pub fn upright(
    /// The content to style.
    body: Content,
) -> Content {
    body.styled(EquationElem::set_italic(Some(false)))
}

/// Italic font style in math.
///
/// For roman letters and greek lowercase letters, this is already the default.
#[func(keywords = ["mathit"])]
pub fn italic(
    /// The content to style.
    body: Content,
) -> Content {
    body.styled(EquationElem::set_italic(Some(true)))
}

/// Serif (roman) font style in math.
///
/// This is already the default.
#[func(keywords = ["mathrm"])]
pub fn serif(
    /// The content to style.
    body: Content,
) -> Content {
    body.styled(EquationElem::set_variant(Some(MathVariant::Plain)))
}

/// Sans-serif font style in math.
///
/// ```example
/// $ sans(A B C) $
/// ```
#[func(title = "Sans Serif", keywords = ["mathsf"])]
pub fn sans(
    /// The content to style.
    body: Content,
) -> Content {
    body.styled(EquationElem::set_variant(Some(MathVariant::SansSerif)))
}

/// Script (roundhand) font style in math.
///
/// ```example
/// $ scr(S) $
/// ```
///
/// This is the default calligraphic/script style for most math fonts. See
/// [`cal`]($math.cal) for more on how to get the other style (chancery).
///
/// For the default math font, the chancery style is the default. But the
/// roundhand style is available through the `ss01` feature. Therefore, you
/// could define your own version of `\mathscr` like this:
///
/// ```example
/// #let scr(it) = text(
///   features: ("ss01",),
///   box($cal(it)$),
/// )
///
/// We establish $cal(P) != scr(P)$.
/// ```
///
/// (The box is not conceptually necessary, but unfortunately currently needed
/// due to limitations in Typst's text style handling in math.)
#[func(title = "Script", keywords = ["mathscr", "roundhand"])]
pub fn scr(
    /// The content to style.
    body: Content,
) -> Content {
    body.styled(EquationElem::set_variant(Some(MathVariant::Roundhand)))
}

/// Calligraphic (chancery) font style in math.
///
/// ```example
/// Let $cal(P)$ be the set of ...
/// ```
///
/// Very few math fonts currently support differentiating `cal` and `scr`. Some
/// fonts support switching between the styles via
/// [font features]($text.features).
#[func(title = "Calligraphic", keywords = ["mathcal", "chancery"])]
pub fn cal(
    /// The content to style.
    body: Content,
) -> Content {
    body.styled(EquationElem::set_variant(Some(MathVariant::Chancery)))
}

/// Fraktur font style in math.
///
/// ```example
/// $ frak(P) $
/// ```
#[func(title = "Fraktur", keywords = ["mathfrak"])]
pub fn frak(
    /// The content to style.
    body: Content,
) -> Content {
    body.styled(EquationElem::set_variant(Some(MathVariant::Fraktur)))
}

/// Monospace font style in math.
///
/// ```example
/// $ mono(x + y = z) $
/// ```
#[func(title = "Monospace", keywords = ["mathtt"])]
pub fn mono(
    /// The content to style.
    body: Content,
) -> Content {
    body.styled(EquationElem::set_variant(Some(MathVariant::Monospace)))
}

/// Blackboard bold (double-struck) font style in math.
///
/// For uppercase latin letters, blackboard bold is additionally available
/// through [symbols]($category/symbols/sym) of the form `NN` and `RR`.
///
/// ```example
/// $ bb(b) $
/// $ bb(N) = NN $
/// $ f: NN -> RR $
/// ```
#[func(title = "Blackboard Bold", keywords = ["mathbb"])]
pub fn bb(
    /// The content to style.
    body: Content,
) -> Content {
    body.styled(EquationElem::set_variant(Some(MathVariant::DoubleStruck)))
}

/// Forced display style in math.
///
/// This is the normal size for block equations.
///
/// ```example
/// $sum_i x_i/2 = display(sum_i x_i/2)$
/// ```
#[func(title = "Display Size", keywords = ["displaystyle"])]
pub fn display(
    /// The content to size.
    body: Content,
    /// Whether to impose a height restriction for exponents, like regular sub-
    /// and superscripts do.
    #[named]
    #[default(false)]
    cramped: bool,
) -> Content {
    body.styled(EquationElem::set_size(MathSize::Display))
        .styled(EquationElem::set_cramped(cramped))
}

/// Forced inline (text) style in math.
///
/// This is the normal size for inline equations.
///
/// ```example
/// $ sum_i x_i/2
///     = inline(sum_i x_i/2) $
/// ```
#[func(title = "Inline Size", keywords = ["textstyle"])]
pub fn inline(
    /// The content to size.
    body: Content,
    /// Whether to impose a height restriction for exponents, like regular sub-
    /// and superscripts do.
    #[named]
    #[default(false)]
    cramped: bool,
) -> Content {
    body.styled(EquationElem::set_size(MathSize::Text))
        .styled(EquationElem::set_cramped(cramped))
}

/// Forced script style in math.
///
/// This is the smaller size used in powers or sub- or superscripts.
///
/// ```example
/// $sum_i x_i/2 = script(sum_i x_i/2)$
/// ```
#[func(title = "Script Size", keywords = ["scriptstyle"])]
pub fn script(
    /// The content to size.
    body: Content,
    /// Whether to impose a height restriction for exponents, like regular sub-
    /// and superscripts do.
    #[named]
    #[default(true)]
    cramped: bool,
) -> Content {
    body.styled(EquationElem::set_size(MathSize::Script))
        .styled(EquationElem::set_cramped(cramped))
}

/// Forced second script style in math.
///
/// This is the smallest size, used in second-level sub- and superscripts
/// (script of the script).
///
/// ```example
/// $sum_i x_i/2 = sscript(sum_i x_i/2)$
/// ```
#[func(title = "Script-Script Size", keywords = ["scriptscriptstyle"])]
pub fn sscript(
    /// The content to size.
    body: Content,
    /// Whether to impose a height restriction for exponents, like regular sub-
    /// and superscripts do.
    #[named]
    #[default(true)]
    cramped: bool,
) -> Content {
    body.styled(EquationElem::set_size(MathSize::ScriptScript))
        .styled(EquationElem::set_cramped(cramped))
}

/// The size of elements in an equation.
///
/// See the TeXbook p. 141.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Cast, Hash)]
pub enum MathSize {
    /// Second-level sub- and superscripts.
    ScriptScript,
    /// Sub- and superscripts.
    Script,
    /// Math in text.
    Text,
    /// Math on its own line.
    Display,
}
