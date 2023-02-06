use crate::prelude::*;
use crate::text::TextNode;

/// # Reference
/// A reference to a label.
///
/// *Note: This function is currently unimplemented.*
///
/// The reference function produces a textual reference to a label. For example,
/// a reference to a heading will yield an appropriate string such as "Section
/// 1" for a reference to the first heading's label. The references are also
/// links to the respective labels.
///
/// ## Syntax
/// This function also has dedicated syntax: A reference to a label can be
/// created by typing an `@` followed by the name of the label (e.g. `[=
/// Introduction <intro>]` can be referenced by typing `[@intro]`).
///
/// ## Parameters
/// - target: `Label` (positional, required)
///   The label that should be referenced.
///
/// ## Category
/// meta
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
