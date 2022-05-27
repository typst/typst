use super::TextNode;
use crate::library::prelude::*;
use crate::util::EcoString;

/// Link text and other elements to an URL.
#[derive(Debug, Hash)]
pub struct LinkNode {
    /// The destination the link points to.
    pub dest: Destination,
    /// How the link is represented.
    pub body: Option<Content>,
}

#[node(showable)]
impl LinkNode {
    /// The fill color of text in the link. Just the surrounding text color
    /// if `auto`.
    pub const FILL: Smart<Paint> = Smart::Auto;
    /// Whether to underline link.
    pub const UNDERLINE: Smart<bool> = Smart::Auto;

    fn construct(_: &mut Machine, args: &mut Args) -> TypResult<Content> {
        Ok(Content::show({
            let dest = args.expect::<Destination>("destination")?;
            let body = match dest {
                Destination::Url(_) => args.eat()?,
                Destination::Internal(_, _) => Some(args.expect("body")?),
            };
            Self { dest, body }
        }))
    }
}

castable! {
    Destination,
    Expected: "string or dictionary with `page`, `x`, and `y` keys",
    Value::Str(string) => Self::Url(string),
    Value::Dict(dict) => {
        let page: i64 = dict.get(&EcoString::from_str("page"))?.clone().cast()?;
        let x: RawLength = dict.get(&EcoString::from_str("x"))?.clone().cast()?;
        let y: RawLength = dict.get(&EcoString::from_str("y"))?.clone().cast()?;
        Self::Internal(page as usize, Point::new(x.length, y.length))
    },
}

impl Show for LinkNode {
    fn unguard(&self, sel: Selector) -> ShowNode {
        Self {
            dest: self.dest.clone(),
            body: self.body.as_ref().map(|body| body.unguard(sel)),
        }
        .pack()
    }

    fn encode(&self, _: StyleChain) -> Dict {
        dict! {
            "url" => match &self.dest {
                Destination::Url(url) => Value::Str(url.clone()),
                Destination::Internal(page, point) => Value::Dict(dict!{
                    "page" => Value::Int(*page as i64),
                    "x" => Value::Length(point.x.into()),
                    "y" => Value::Length(point.y.into()),
                }),
            },
            "body" => match &self.body {
                Some(body) => Value::Content(body.clone()),
                None => Value::None,
            },
        }
    }

    fn realize(&self, _: &mut Context, _: StyleChain) -> TypResult<Content> {
        Ok(self.body.clone().unwrap_or_else(|| match &self.dest {
            Destination::Url(url) => {
                let mut text = url.as_str();
                for prefix in ["mailto:", "tel:"] {
                    text = text.trim_start_matches(prefix);
                }
                let shorter = text.len() < url.len();
                Content::Text(if shorter { text.into() } else { url.clone() })
            }
            Destination::Internal(_, _) => panic!("missing body"),
        }))
    }

    fn finalize(
        &self,
        _: &mut Context,
        styles: StyleChain,
        mut realized: Content,
    ) -> TypResult<Content> {
        let mut map = StyleMap::new();
        map.set(TextNode::LINK, Some(self.dest.clone()));

        if let Smart::Custom(fill) = styles.get(Self::FILL) {
            map.set(TextNode::FILL, fill);
        }

        if match styles.get(Self::UNDERLINE) {
            Smart::Auto => match &self.dest {
                Destination::Url(_) => true,
                Destination::Internal(_, _) => false,
            },
            Smart::Custom(underline) => underline,
        } {
            realized = realized.underlined();
        }

        Ok(realized.styled_with_map(map))
    }
}
