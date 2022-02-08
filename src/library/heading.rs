//! Document-structuring section headings.

use super::prelude::*;
use super::{FontFamily, TextNode};

/// A section heading.
#[derive(Debug, Hash)]
pub struct HeadingNode {
    /// The logical nesting depth of the section, starting from one. In the
    /// default style, this controls the text size of the heading.
    pub level: usize,
    /// The heading's contents.
    pub body: Template,
}

#[class]
impl HeadingNode {
    /// The heading's font family.
    pub const FAMILY: Smart<FontFamily> = Smart::Auto;
    /// The size of text in the heading. Just the surrounding text size if
    /// `auto`.
    pub const SIZE: Smart<Linear> = Smart::Auto;
    /// The fill color of text in the heading. Just the surrounding text color
    /// if `auto`.
    pub const FILL: Smart<Paint> = Smart::Auto;
    /// Whether text in the heading is strengthend.
    pub const STRONG: bool = true;
    /// Whether text in the heading is emphasized.
    pub const EMPH: bool = false;
    /// Whether the heading is underlined.
    pub const UNDERLINE: bool = false;
    /// The extra padding above the heading.
    pub const ABOVE: Length = Length::zero();
    /// The extra padding below the heading.
    pub const BELOW: Length = Length::zero();

    fn construct(_: &mut EvalContext, args: &mut Args) -> TypResult<Template> {
        Ok(Template::show(Self {
            body: args.expect("body")?,
            level: args.named("level")?.unwrap_or(1),
        }))
    }
}

impl Show for HeadingNode {
    fn show(&self, styles: StyleChain) -> Template {
        let mut map = StyleMap::new();

        let upscale = (1.6 - 0.1 * self.level as f64).max(0.75);
        map.set(
            TextNode::SIZE,
            styles.get(Self::SIZE).unwrap_or(Relative::new(upscale).into()),
        );

        if let Smart::Custom(family) = styles.get_ref(Self::FAMILY) {
            map.set(
                TextNode::FAMILY,
                std::iter::once(family)
                    .chain(styles.get_ref(TextNode::FAMILY))
                    .cloned()
                    .collect(),
            );
        }

        if let Smart::Custom(fill) = styles.get(Self::FILL) {
            map.set(TextNode::FILL, fill);
        }

        if styles.get(Self::STRONG) {
            map.set(TextNode::STRONG, true);
        }

        if styles.get(Self::EMPH) {
            map.set(TextNode::EMPH, true);
        }

        let mut body = self.body.clone();
        if styles.get(Self::UNDERLINE) {
            body = body.underlined();
        }

        let mut seq = vec![];

        let above = styles.get(Self::ABOVE);
        if !above.is_zero() {
            seq.push(Template::Vertical(above.into()));
        }

        seq.push(body);

        let below = styles.get(Self::BELOW);
        if !below.is_zero() {
            seq.push(Template::Vertical(below.into()));
        }

        Template::block(Template::sequence(seq).styled_with_map(map))
    }
}
