//! Mathematical formulas.

use crate::library::prelude::*;

/// A mathematical formula.
#[derive(Debug, Hash)]
pub struct MathNode {
    /// The formula.
    pub formula: EcoString,
    /// Whether the formula is display-level.
    pub display: bool,
}

#[node(showable)]
impl MathNode {
    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Content> {
        Ok(Content::show(Self {
            formula: args.expect("formula")?,
            display: args.named("display")?.unwrap_or(false),
        }))
    }
}

impl Show for MathNode {
    fn show(&self, ctx: &mut Context, styles: StyleChain) -> TypResult<Content> {
        Ok(styles
            .show(self, ctx, [
                Value::Str(self.formula.clone()),
                Value::Bool(self.display),
            ])?
            .unwrap_or_else(|| {
                let mut content = Content::Text(self.formula.trim().into());
                if self.display {
                    content = Content::Block(content.pack());
                }
                content.monospaced()
            }))
    }
}
