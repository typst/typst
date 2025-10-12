use codex::styling::MathVariant;
use typst_utils::LazyHash;

use crate::foundations::{Cast, Content, Style, StyleChain, func};
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
    body.set(EquationElem::bold, true)
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
    body.set(EquationElem::italic, Some(false))
}

/// Italic font style in math.
///
/// For roman letters and greek lowercase letters, this is already the default.
#[func(keywords = ["mathit"])]
pub fn italic(
    /// The content to style.
    body: Content,
) -> Content {
    body.set(EquationElem::italic, Some(true))
}

/// Serif (roman) font style in math.
///
/// This is already the default.
#[func(keywords = ["mathrm"])]
pub fn serif(
    /// The content to style.
    body: Content,
) -> Content {
    body.set(EquationElem::variant, Some(MathVariant::Plain))
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
    body.set(EquationElem::variant, Some(MathVariant::SansSerif))
}

/// Calligraphic (chancery) font style in math.
///
/// ```example
/// Let $cal(P)$ be the set of ...
/// ```
///
/// This is the default calligraphic/script style for most math fonts. See
/// [`scr`]($math.scr) for more on how to get the other style (roundhand).
#[func(title = "Calligraphic", keywords = ["mathcal", "chancery"])]
pub fn cal(
    /// The content to style.
    body: Content,
) -> Content {
    body.set(EquationElem::variant, Some(MathVariant::Chancery))
}

/// Script (roundhand) font style in math.
///
/// ```example
/// $scr(L)$ is not the set of linear
/// maps $cal(L)$.
/// ```
///
/// There are two ways that fonts can support differentiating `cal` and `scr`.
/// The first is using Unicode variation sequences. This works out of the box
/// in Typst, however only a few math fonts currently support this.
///
/// The other way is using [font features]($text.features). For example, the
/// roundhand style might be available in a font through the
/// _[stylistic set]($text.stylistic-set) 1_ (`ss01`) feature. To use it in
/// Typst, you could then define your own version of `scr` like in the example
/// below.
///
/// ```example:"Recreation using stylistic set 1"
/// #let scr(it) = text(
///   stylistic-set: 1,
///   $cal(it)$,
/// )
///
/// We establish $cal(P) != scr(P)$.
/// ```
#[func(title = "Script Style", keywords = ["mathscr", "roundhand"])]
pub fn scr(
    /// The content to style.
    body: Content,
) -> Content {
    body.set(EquationElem::variant, Some(MathVariant::Roundhand))
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
    body.set(EquationElem::variant, Some(MathVariant::Fraktur))
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
    body.set(EquationElem::variant, Some(MathVariant::Monospace))
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
    body.set(EquationElem::variant, Some(MathVariant::DoubleStruck))
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
    body.set(EquationElem::size, MathSize::Display)
        .set(EquationElem::cramped, cramped)
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
    body.set(EquationElem::size, MathSize::Text)
        .set(EquationElem::cramped, cramped)
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
    body.set(EquationElem::size, MathSize::Script)
        .set(EquationElem::cramped, cramped)
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
    body.set(EquationElem::size, MathSize::ScriptScript)
        .set(EquationElem::cramped, cramped)
}

/// The size of elements in an equation.
///
/// See the TeXbook p. 141.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Cast)]
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

/// Styles something as cramped.
pub fn style_cramped() -> LazyHash<Style> {
    EquationElem::cramped.set(true).wrap()
}

/// The style for subscripts in the current style.
pub fn style_for_subscript(styles: StyleChain) -> [LazyHash<Style>; 2] {
    [style_for_superscript(styles), EquationElem::cramped.set(true).wrap()]
}

/// The style for superscripts in the current style.
pub fn style_for_superscript(styles: StyleChain) -> LazyHash<Style> {
    EquationElem::size
        .set(match styles.get(EquationElem::size) {
            MathSize::Display | MathSize::Text => MathSize::Script,
            MathSize::Script | MathSize::ScriptScript => MathSize::ScriptScript,
        })
        .wrap()
}

/// The style for numerators in the current style.
pub fn style_for_numerator(styles: StyleChain) -> LazyHash<Style> {
    EquationElem::size
        .set(match styles.get(EquationElem::size) {
            MathSize::Display => MathSize::Text,
            MathSize::Text => MathSize::Script,
            MathSize::Script | MathSize::ScriptScript => MathSize::ScriptScript,
        })
        .wrap()
}

/// The style for denominators in the current style.
pub fn style_for_denominator(styles: StyleChain) -> [LazyHash<Style>; 2] {
    [style_for_numerator(styles), EquationElem::cramped.set(true).wrap()]
}
