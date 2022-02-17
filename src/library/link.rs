//! Hyperlinking.

use super::prelude::*;
use super::TextNode;
use crate::util::EcoString;

/// Link text and other elements to an URL.
#[derive(Debug, Hash)]
pub struct LinkNode {
    /// The url the link points to.
    pub url: EcoString,
    /// How the link is represented.
    pub body: Template,
}

#[class]
impl LinkNode {
    /// The fill color of text in the link. Just the surrounding text color
    /// if `auto`.
    pub const FILL: Smart<Paint> = Smart::Auto;
    /// Whether to underline link.
    pub const UNDERLINE: bool = true;

    fn construct(_: &mut Vm, args: &mut Args) -> TypResult<Template> {
        let url = args.expect::<EcoString>("url")?;
        let body = args.find()?.unwrap_or_else(|| {
            let mut text = url.as_str();
            for prefix in ["mailto:", "tel:"] {
                text = text.trim_start_matches(prefix);
            }
            let shorter = text.len() < url.len();
            Template::Text(if shorter { text.into() } else { url.clone() })
        });

        Ok(Template::show(Self { url, body }))
    }
}

impl Show for LinkNode {
    fn show(&self, _: &mut Vm, styles: StyleChain) -> TypResult<Template> {
        let mut map = StyleMap::new();
        map.set(TextNode::LINK, Some(self.url.clone()));

        if let Smart::Custom(fill) = styles.get(Self::FILL) {
            map.set(TextNode::FILL, fill);
        }

        let mut body = self.body.clone();
        if styles.get(Self::UNDERLINE) {
            body = body.underlined();
        }

        Ok(body.styled_with_map(map))
    }
}
