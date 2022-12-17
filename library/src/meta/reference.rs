use crate::prelude::*;
use crate::text::TextNode;

/// A reference to a label.
///
/// # Parameters
/// - target: Label (positional, required)
///   The label that should be referenced.
///
/// # Tags
/// - meta
#[func]
#[capable(Show)]
#[derive(Debug, Hash)]
pub struct RefNode(pub EcoString);

#[node]
impl RefNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("target")?).pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "target" => Some(Value::Str(self.0.clone().into())),
            _ => None,
        }
    }
}

impl Show for RefNode {
    fn show(&self, _: &mut Vt, _: &Content, _: StyleChain) -> SourceResult<Content> {
        Ok(TextNode::packed(format_eco!("@{}", self.0)))
    }
}
