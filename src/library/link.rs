//! Hyperlinking.

use super::prelude::*;
use super::TextNode;
use crate::util::EcoString;

/// Link text and other elements to an URL.
pub struct LinkNode;

#[class]
impl LinkNode {
    fn construct(_: &mut EvalContext, args: &mut Args) -> TypResult<Node> {
        let url: String = args.expect::<EcoString>("url")?.into();
        let body = args.find().unwrap_or_else(|| {
            let mut text = url.as_str();
            for prefix in ["mailto:", "tel:"] {
                text = text.trim_start_matches(prefix);
            }
            Node::Text(text.into())
        });

        Ok(body.styled(TextNode::LINK, Some(url)))
    }
}
