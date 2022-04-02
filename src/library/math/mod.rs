//! Mathematical formulas.

use crate::library::prelude::*;
use crate::library::text::FontFamily;

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
    /// The raw text's font family. Just the normal text family if `none`.
    pub const FAMILY: Smart<FontFamily> =
        Smart::Custom(FontFamily::new("Latin Modern Math"));

    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Content> {
        Ok(Content::show(Self {
            formula: args.expect("formula")?,
            display: args.named("display")?.unwrap_or(false),
        }))
    }
}

impl Show for MathNode {
    fn show(&self, ctx: &mut Context, styles: StyleChain) -> TypResult<Content> {
        let mut content = styles
            .show(self, ctx, [
                Value::Str(self.formula.clone()),
                Value::Bool(self.display),
            ])?
            .unwrap_or_else(|| Content::Text(self.formula.trim().into()));

        let mut map = StyleMap::new();
        if let Smart::Custom(family) = styles.get_cloned(Self::FAMILY) {
            map.set_family(family, styles);
        }

        content = content.styled_with_map(map);

        if self.display {
            content = Content::Block(content.pack());
        }

        Ok(content)
    }
}
