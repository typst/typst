use typst_syntax::Spanned;

use crate::diag::bail;
use crate::foundations::{Cast, Content, Value, elem};
use crate::layout::Em;
use crate::math::{DelimiterPair, Mathy};

/// How much padding to add around each side of a fraction.
pub const FRAC_PADDING: Em = Em::new(0.1);

/// A mathematical fraction.
///
/// = Example <example>
/// ```example
/// $ 1/2 < (x+1)/2 $
/// $ ((x+1)) / 2 = frac(a, b) $
/// ```
///
/// = Syntax <syntax>
/// This function also has dedicated syntax: Use a slash to turn neighbouring
/// expressions into a fraction. Multiple atoms can be grouped into a single
/// expression using round grouping parentheses. Such parentheses are removed
/// from the output, but you can nest multiple to force them.
#[elem(title = "Fraction", since = "forever", keywords = ["over"], Mathy)]
pub struct FracElem {
    /// The fraction's numerator.
    #[required]
    pub num: Content,

    /// The fraction's denominator.
    #[required]
    pub denom: Content,

    /// How the fraction should be laid out.
    ///
    /// #example(
    ///   title: "Styles",
    ///   ```
    ///   $ frac(x, y, style: "vertical") $
    ///   $ frac(x, y, style: "skewed") $
    ///   $ frac(x, y, style: "horizontal") $
    ///   ```
    /// )
    ///
    /// #example(
    ///   title: "Setting the default",
    ///   ```
    ///   #set math.frac(style: "skewed")
    ///   $ a / b $
    ///   ```
    /// )
    ///
    /// #example(
    ///   title: "Handling of grouping parentheses",
    ///   ```
    ///   // Grouping parentheses are removed.
    ///   #set math.frac(style: "vertical")
    ///   $ (a + b) / b $
    ///
    ///   // Grouping parentheses are removed.
    ///   #set math.frac(style: "skewed")
    ///   $ (a + b) / b $
    ///
    ///   // Grouping parentheses are retained.
    ///   #set math.frac(style: "horizontal")
    ///   $ (a + b) / b $
    ///   ```
    /// )
    ///
    /// #example(
    ///   title: "Different styles in inline vs block equations",
    ///   ```
    ///   // This changes the style for inline equations only.
    ///   #show math.equation.where(block: false): set math.frac(style: "horizontal")
    ///
    ///   This $(x-y)/z = 3$ is inline math, and this is block math:
    ///   $ (x-y)/z = 3 $
    ///   ```
    /// )
    ///
    /// #example(
    ///   title: "Use LaTeX-like convention",
    ///   ```
    ///   // Change the default style.
    ///   #set math.frac(style: "horizontal")
    ///   // Define a shorthand with the original style.
    ///   #let frac = math.frac.with(style: "vertical")
    ///
    ///   $ p/q = frac(p, q) $
    ///
    ///   // The shadowed definition can still be accessed.
    ///   #assert.eq($p/q$, $std.math.frac(p, q)$)
    ///   ```
    /// )
    #[default(FracStyle::Vertical)]
    pub style: FracStyle,

    /// Whether the numerator was originally surrounded by parentheses that were
    /// stripped by the parser.
    #[internal]
    #[parse(None)]
    #[default(false)]
    pub num_deparenthesized: bool,

    /// Whether the denominator was originally surrounded by parentheses that
    /// were stripped by the parser.
    #[internal]
    #[parse(None)]
    #[default(false)]
    pub denom_deparenthesized: bool,
}

/// Fraction style
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum FracStyle {
    /// Stacked numerator and denominator with a bar.
    #[default]
    Vertical,
    /// Numerator and denominator separated by a slash.
    Skewed,
    /// Numerator and denominator placed inline and parentheses are not
    /// absorbed.
    Horizontal,
}

/// A binomial coefficient.
///
/// To calculate the value of a binomial coefficient, use @calc.binom.
///
/// This uses the same vertical and delimiter spacing as the @math.atop[`atop`]
/// element.
///
/// Can also render a multinomial coefficient by passing more than two
/// expressions.
///
/// = Example <example>
/// ```example
/// >>> #set page(width: auto)
/// $ binom(n, k) = n! / (k!(n-k)!) $
///
/// $
///   binom(n, k_1,  k_2,  dots.c, k_m)
///    = n! / (k_1 ! k_2 ! dots.c  k_m !)
/// $
/// ```
/// // FUTURE: Remove the spaces before the factorials once attachment
/// // precedence is updated.
#[elem(title = "Binomial", since = "forever", keywords = ["choose"], Mathy)]
pub struct BinomElem {
    /// The upper index, the number of items to choose from.
    #[required]
    pub upper: Content,

    /// The lower index, the number of items to choose.
    ///
    /// If multiple expressions are given, renders the multinomial coefficient
    /// by adding commas between them.
    #[required]
    #[variadic]
    #[parse(
        let values = args.all::<Spanned<Value>>()?;
        if values.is_empty() {
            // Prevents one element binomials.
            bail!(args.span, "missing argument: lower");
        }
        values.into_iter().map(|spanned| spanned.v.display()).collect()
    )]
    pub lower: Vec<Content>,
}

/// Render one expression on top of another.
///
/// To render more than two expressions, consider the @math.vec[vector] or
/// @math.mat[matrix] elements.
///
/// This uses the same vertical spacing between expressions as
/// @math.frac[fractions] and @math.binom[binomials].
///
/// = Example <example>
/// ```example
/// >>> #set page(width: auto)
/// $ atop(a, b) $
///
/// $ atop(n, k, delim: chevron) $
/// ```
#[elem(since = "unreleased", keywords = ["over"], Mathy)]
pub struct AtopElem {
    /// The upper expression.
    #[required]
    pub upper: Content,

    /// The lower expression.
    #[required]
    pub lower: Content,

    /// The delimiter to use.
    ///
    /// Can be a single character specifying the left delimiter, in which case
    /// the right delimiter is inferred. Otherwise, can be an array containing a
    /// left and a right delimiter.
    ///
    /// #example(
    ///   title: "Basic usage",
    ///   ```
    ///   >>> #set page(width: auto)
    ///   $ atop(1, 2, delim: #("{", ")")) $
    ///
    ///   #set math.atop(delim: "[")
    ///   $ atop(3, 4) $
    ///   ```
    /// )
    ///
    /// This parameter may produce nicer delimiter spacing than wrapping in an
    /// @math.lr[`lr`] element or using @math.vec[`vec`] for two expressions:
    ///
    /// #example(
    ///   title: "Spacing compared to lr and vec",
    ///   ```
    ///   >>> #set page(width: 120pt)
    ///   #place(dy: 2.1em, line(
    ///     length: 100%,
    ///     stroke: red + .5pt,
    ///   ))
    ///   $
    ///     atop(n, k, delim: #("(", "]"))
    ///     quad lr(\( atop(n, k) \])
    ///     quad vec(n, k, delim: #("(", "]"))
    ///   $
    ///   ```
    /// )
    #[default(None)]
    pub delim: Option<DelimiterPair>,
}
