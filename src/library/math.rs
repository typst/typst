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
    fn construct(_: &mut EvalContext, args: &mut Args) -> TypResult<Template> {
        Ok(Template::show(Self {
            formula: args.expect("formula")?,
            display: args.named("display")?.unwrap_or(false),
        }))
    }
}

impl Show for MathNode {
    fn show(&self, _: StyleChain) -> Template {
        let mut template = Template::Text(self.formula.trim().into());
        if self.display {
            template = Template::Block(template.pack());
        }
        template.monospaced()
    }
}
