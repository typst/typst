use super::*;
use crate::library::prelude::*;

/// A fraction in a mathematical formula.
#[derive(Debug, Hash)]
pub struct FracNode {
    /// The numerator.
    pub num: MathNode,
    /// The denominator.
    pub denom: MathNode,
}

impl Texify for FracNode {
    fn texify(&self) -> EcoString {
        format_eco!("\\frac{{{}}}{{{}}}", self.num.texify(), self.denom.texify())
    }
}
