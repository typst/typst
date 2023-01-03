use super::*;

/// # Fraction
/// A mathematical fraction.
///
/// ## Syntax
/// This function also has dedicated syntax: Use a slash to turn neighbouring
/// expressions into a fraction. Multiple atoms can be grouped into a single
/// expression using round grouping parenthesis. Such parentheses are removed
/// from the output, but you can nest multiple to force them.
///
/// ## Example
/// ```
/// $ 1/2 < (x+1)/2 $
/// $ ((x+1)) / 2 = frac(a, b) $
/// ```
///
/// ## Parameters
/// - num: Content (positional, required)
///   The fraction's numerator.
///
/// - denom: Content (positional, required)
///   The fraction's denominator.
///
/// ## Category
/// math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct FracNode {
    /// The numerator.
    pub num: Content,
    /// The denominator.
    pub denom: Content,
}

#[node]
impl FracNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let num = args.expect("numerator")?;
        let denom = args.expect("denominator")?;
        Ok(Self { num, denom }.pack())
    }
}

impl Texify for FracNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\frac{");
        self.num.texify_unparen(t)?;
        t.push_str("}{");
        self.denom.texify_unparen(t)?;
        t.push_str("}");
        Ok(())
    }
}

/// # Binomial
/// A binomial expression.
///
/// ## Example
/// ```
/// $ binom(n, k) $
/// ```
///
/// ## Parameters
/// - upper: Content (positional, required)
///   The binomial's upper index.
///
/// - lower: Content (positional, required)
///   The binomial's lower index.
///
/// ## Category
/// math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct BinomNode {
    /// The upper index.
    pub upper: Content,
    /// The lower index.
    pub lower: Content,
}

#[node]
impl BinomNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let upper = args.expect("upper index")?;
        let lower = args.expect("lower index")?;
        Ok(Self { upper, lower }.pack())
    }
}

impl Texify for BinomNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\binom{");
        self.upper.texify(t)?;
        t.push_str("}{");
        self.lower.texify(t)?;
        t.push_str("}");
        Ok(())
    }
}
