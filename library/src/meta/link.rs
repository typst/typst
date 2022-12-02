use crate::prelude::*;
use crate::text::TextNode;

/// Link text and other elements to a destination.
#[derive(Debug, Hash)]
pub struct LinkNode {
    /// The destination the link points to.
    pub dest: Destination,
    /// How the link is represented.
    pub body: Content,
}

impl LinkNode {
    /// Create a link node from a URL with its bare text.
    pub fn from_url(url: EcoString) -> Self {
        let mut text = url.as_str();
        for prefix in ["mailto:", "tel:"] {
            text = text.trim_start_matches(prefix);
        }
        let shorter = text.len() < url.len();
        let body = TextNode::packed(if shorter { text.into() } else { url.clone() });
        Self { dest: Destination::Url(url), body }
    }
}

#[node(Show, Finalize)]
impl LinkNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let dest = args.expect::<Destination>("destination")?;
        Ok(match dest {
            Destination::Url(url) => match args.eat()? {
                Some(body) => Self { dest: Destination::Url(url), body },
                None => Self::from_url(url),
            },
            Destination::Internal(_) => Self { dest, body: args.expect("body")? },
        }
        .pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "url" => Some(match &self.dest {
                Destination::Url(url) => Value::Str(url.clone().into()),
                Destination::Internal(loc) => Value::Dict(loc.encode()),
            }),
            "body" => Some(Value::Content(self.body.clone())),
            _ => None,
        }
    }
}

impl Show for LinkNode {
    fn show(&self, _: &mut Vt, _: &Content, _: StyleChain) -> Content {
        self.body.clone()
    }
}

impl Finalize for LinkNode {
    fn finalize(&self, realized: Content) -> Content {
        realized.styled(Meta::DATA, vec![Meta::Link(self.dest.clone())])
    }
}
