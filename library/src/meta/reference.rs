use crate::prelude::*;
use crate::text::TextNode;

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
/// Display: Reference
/// Category: meta
#[node(Show)]
pub struct RefNode {
    /// The label that should be referenced.
    #[required]
    pub target: EcoString,
}

impl Show for RefNode {
    fn show(&self, _: &mut Vt, _: &Content, _: StyleChain) -> SourceResult<Content> {
        Ok(TextNode::packed(eco_format!("@{}", self.target())))
    }
}
