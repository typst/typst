//! Mathematical formulas.

use crate::library::layout::BlockSpacing;
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
    /// The raw text's font family. Just the normal text family if `auto`.
    #[property(referenced)]
    pub const FAMILY: Smart<FontFamily> =
        Smart::Custom(FontFamily::new("Latin Modern Math"));

    /// The spacing above display math.
    #[property(resolve, shorthand(around))]
    pub const ABOVE: Option<BlockSpacing> = Some(Ratio::one().into());
    /// The spacing below display math.
    #[property(resolve, shorthand(around))]
    pub const BELOW: Option<BlockSpacing> = Some(Ratio::one().into());

    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Content> {
        Ok(Content::show(Self {
            formula: args.expect("formula")?,
            display: args.named("display")?.unwrap_or(false),
        }))
    }
}

impl Show for MathNode {
    fn unguard(&self, _: Selector) -> ShowNode {
        Self { formula: self.formula.clone(), ..*self }.pack()
    }

    fn encode(&self, _: StyleChain) -> Dict {
        dict! {
            "formula" => Value::Str(self.formula.clone()),
            "display" => Value::Bool(self.display)
        }
    }

    fn realize(&self, _: &mut Context, _: StyleChain) -> TypResult<Content> {
        let mut realized = Content::Text(self.formula.trim().into());
        if self.display {
            realized = Content::block(realized);
        }
        Ok(realized)
    }

    fn finalize(
        &self,
        _: &mut Context,
        styles: StyleChain,
        mut realized: Content,
    ) -> TypResult<Content> {
        let mut map = StyleMap::new();
        if let Smart::Custom(family) = styles.get(Self::FAMILY) {
            map.set_family(family.clone(), styles);
        }

        if self.display {
            realized = realized.spaced(styles.get(Self::ABOVE), styles.get(Self::BELOW));
        }

        Ok(realized.styled_with_map(map))
    }
}
