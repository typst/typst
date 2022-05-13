use super::TextNode;
use crate::library::prelude::*;
use crate::util::EcoString;

/// Link text and other elements to an URL.
#[derive(Debug, Hash)]
pub struct LinkNode {
    /// The url the link points to.
    pub url: EcoString,
    /// How the link is represented.
    pub body: Option<Content>,
}

#[node(showable)]
impl LinkNode {
    /// The fill color of text in the link. Just the surrounding text color
    /// if `auto`.
    pub const FILL: Smart<Paint> = Smart::Auto;
    /// Whether to underline link.
    pub const UNDERLINE: bool = true;

    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Content> {
        Ok(Content::show(Self {
            url: args.expect::<EcoString>("url")?,
            body: args.eat()?,
        }))
    }
}

impl Show for LinkNode {
    fn unguard(&self, sel: Selector) -> ShowNode {
        Self {
            url: self.url.clone(),
            body: self.body.as_ref().map(|body| body.unguard(sel)),
        }
        .pack()
    }

    fn encode(&self, _: StyleChain) -> Dict {
        dict! {
            "url" => Value::Str(self.url.clone()),
            "body" => match &self.body {
                Some(body) => Value::Content(body.clone()),
                None => Value::None,
            },
        }
    }

    fn realize(&self, _: &mut Context, _: StyleChain) -> TypResult<Content> {
        Ok(self.body.clone().unwrap_or_else(|| {
            let url = &self.url;
            let mut text = url.as_str();
            for prefix in ["mailto:", "tel:"] {
                text = text.trim_start_matches(prefix);
            }
            let shorter = text.len() < url.len();
            Content::Text(if shorter { text.into() } else { url.clone() })
        }))
    }

    fn finalize(
        &self,
        _: &mut Context,
        styles: StyleChain,
        mut realized: Content,
    ) -> TypResult<Content> {
        let mut map = StyleMap::new();
        map.set(TextNode::LINK, Some(self.url.clone()));

        if let Smart::Custom(fill) = styles.get(Self::FILL) {
            map.set(TextNode::FILL, fill);
        }

        if styles.get(Self::UNDERLINE) {
            realized = realized.underlined();
        }

        Ok(realized.styled_with_map(map))
    }
}
