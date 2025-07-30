use typst_syntax::Spanned;

use crate::diag::bail;
use crate::foundations::{Cast, Content, Value, elem};
use crate::math::Mathy;

/// Fraction style
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum FracStyle {
    Vertical,
    Diagonal,
    Horizontal,
}

/// A mathematical fraction.
///
/// # Example
/// ```example
/// $ 1/2 < (x+1)/2 $
/// $ ((x+1)) / 2 = frac(a, b) $
/// ```
///
/// # Syntax
/// This function also has dedicated syntax: Use a slash to turn neighbouring
/// expressions into a fraction. Multiple atoms can be grouped into a single
/// expression using round grouping parentheses. Such parentheses are removed
/// from the output, but you can nest multiple to force them.
#[elem(title = "Fraction", Mathy)]
pub struct FracElem {
    /// The fraction's numerator.
    #[required]
    pub num: Content,

    /// The fraction's denominator.
    #[required]
    pub denom: Content,

    /// The style of the fraction.
    /// - `"vertical"`: stacked numerator and denominator with a bar.
    /// - `"diagonal"`: numerator and denominator separated by a slash.
    /// - `"horizontal"`: numerator and denominator placed inline and
    ///   parentheses are not absorbed
    ///
    /// The default style is "vertical"
    pub frac_style: Option<FracStyle>,

    /// Whether the numerator was originally surrounded by parentheses
    /// that were stripped by the parser.
    #[internal]
    #[synthesized]
    pub num_deparenthesized: bool,

    /// Whether the denominator was originally surrounded by parentheses
    /// that were stripped by the parser.
    #[internal]
    #[synthesized]
    pub denom_deparenthesized: bool,
}

/// A binomial expression.
///
/// # Example
/// ```example
/// $ binom(n, k) $
/// $ binom(n, k_1, k_2, k_3, ..., k_m) $
/// ```
#[elem(title = "Binomial", Mathy)]
pub struct BinomElem {
    /// The binomial's upper index.
    #[required]
    pub upper: Content,

    /// The binomial's lower index.
    #[required]
    #[variadic]
    #[parse(
        let values = args.all::<Spanned<Value>>()?;
        if values.is_empty() {
            // Prevents one element binomials
            bail!(args.span, "missing argument: lower");
        }
        values.into_iter().map(|spanned| spanned.v.display()).collect()
    )]
    pub lower: Vec<Content>,
}
