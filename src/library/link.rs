//! Hyperlinking.

use super::prelude::*;
use super::TextNode;
use crate::util::EcoString;

/// Link text and other elements to an URL.
pub struct LinkNode;

#[class]
impl LinkNode {
    fn construct(_: &mut EvalContext, args: &mut Args) -> TypResult<Node> {
        let url = args.expect::<EcoString>("url")?;
        let body = args.find().unwrap_or_else(|| {
            let mut text = url.as_str();
            for prefix in ["mailto:", "tel:"] {
                text = text.trim_start_matches(prefix);
            }
            let shorter = text.len() < url.len();
            Node::Text(if shorter { text.into() } else { url.clone() })
        });

        Ok(body.styled(TextNode::LINK, Some(url)))
    }
}
