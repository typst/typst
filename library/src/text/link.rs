use super::TextNode;
use crate::prelude::*;

/// Link text and other elements to a destination.
#[derive(Debug, Hash)]
pub struct LinkNode {
    /// The destination the link points to.
    pub dest: Destination,
    /// How the link is represented.
    pub body: Option<Content>,
}

impl LinkNode {
    /// Create a link node from a URL with its bare text.
    pub fn from_url(url: EcoString) -> Self {
        Self { dest: Destination::Url(url), body: None }
    }
}

#[node(Show, Finalize)]
impl LinkNode {
    /// The fill color of text in the link. Just the surrounding text color
    /// if `auto`.
    pub const FILL: Smart<Paint> = Smart::Auto;
    /// Whether to underline the link.
    pub const UNDERLINE: Smart<bool> = Smart::Auto;

    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        let dest = args.expect::<Destination>("destination")?;
        let body = match dest {
            Destination::Url(_) => args.eat()?,
            Destination::Internal(_) => Some(args.expect("body")?),
        };
        Ok(Self { dest, body }.pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "url" => Some(match &self.dest {
                Destination::Url(url) => Value::Str(url.clone().into()),
                Destination::Internal(loc) => Value::Dict(loc.encode()),
            }),
            "body" => Some(match &self.body {
                Some(body) => Value::Content(body.clone()),
                None => Value::None,
            }),
            _ => None,
        }
    }
}

impl Show for LinkNode {
    fn unguard_parts(&self, sel: Selector) -> Content {
        Self {
            dest: self.dest.clone(),
            body: self.body.as_ref().map(|body| body.unguard(sel)),
        }
        .pack()
    }

    fn show(&self, _: Tracked<dyn World>, _: StyleChain) -> SourceResult<Content> {
        Ok(self.body.clone().unwrap_or_else(|| match &self.dest {
            Destination::Url(url) => {
                let mut text = url.as_str();
                for prefix in ["mailto:", "tel:"] {
                    text = text.trim_start_matches(prefix);
                }
                let shorter = text.len() < url.len();
                TextNode::packed(if shorter { text.into() } else { url.clone() })
            }
            Destination::Internal(_) => Content::empty(),
        }))
    }
}

impl Finalize for LinkNode {
    fn finalize(
        &self,
        _: Tracked<dyn World>,
        styles: StyleChain,
        mut realized: Content,
    ) -> SourceResult<Content> {
        let mut map = StyleMap::new();
        map.set(TextNode::LINK, Some(self.dest.clone()));

        if let Smart::Custom(fill) = styles.get(Self::FILL) {
            map.set(TextNode::FILL, fill);
        }

        if match styles.get(Self::UNDERLINE) {
            Smart::Auto => match &self.dest {
                Destination::Url(_) => true,
                Destination::Internal(_) => false,
            },
            Smart::Custom(underline) => underline,
        } {
            realized = realized.underlined();
        }

        Ok(realized.styled_with_map(map))
    }
}
