//! Mathematical formulas.

use super::prelude::*;

/// A mathematical formula.
#[derive(Debug, Hash)]
pub struct MathNode {
    /// The formula.
    pub formula: EcoString,
    /// Whether the formula is display-level.
    pub display: bool,
}

#[class]
impl MathNode {
    fn construct(_: &mut Vm, args: &mut Args) -> TypResult<Template> {
        Ok(Template::show(Self {
            formula: args.expect("formula")?,
            display: args.named("display")?.unwrap_or(false),
        }))
    }
}

impl Show for MathNode {
    fn show(&self, _: &mut Vm, _: StyleChain) -> TypResult<Template> {
        let mut template = Template::Text(self.formula.trim().into());
        if self.display {
            template = Template::Block(template.pack());
        }
        Ok(template.monospaced())
    }
}
