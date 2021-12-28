//! Hyperlinking.

use super::prelude::*;
use crate::util::EcoString;

/// `link`: Link text or other elements.
pub fn link(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let url: String = args.expect::<EcoString>("url")?.into();
    let body = args.find().unwrap_or_else(|| {
        let mut text = url.as_str();
        for prefix in ["mailto:", "tel:"] {
            text = text.trim_start_matches(prefix);
        }
        Node::Text(text.into())
    });

    Ok(Value::Node(
        body.styled(Styles::one(LinkNode::URL, Some(url))),
    ))
}

/// Host for link styles.
#[derive(Debug, Hash)]
pub struct LinkNode;

#[properties]
impl LinkNode {
    /// An URL to link to.
    pub const URL: Option<String> = None;
}
