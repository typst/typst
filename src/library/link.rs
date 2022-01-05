//! Hyperlinking.

use super::prelude::*;
use super::{ImageNode, ShapeNode, TextNode};
use crate::util::EcoString;

/// `link`: Link text and other elements to an URL.
pub fn link(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let url: String = args.expect::<EcoString>("url")?.into();
    let body = args.find().unwrap_or_else(|| {
        let mut text = url.as_str();
        for prefix in ["mailto:", "tel:"] {
            text = text.trim_start_matches(prefix);
        }
        Node::Text(text.into())
    });

    let mut passed = StyleMap::new();
    passed.set(TextNode::LINK, Some(url.clone()));
    passed.set(ImageNode::LINK, Some(url.clone()));
    passed.set(ShapeNode::LINK, Some(url));
    Ok(Value::Node(body.styled(passed)))
}
