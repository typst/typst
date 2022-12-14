use crate::model::Value;
use crate::syntax::{LinkedNode, Source, SyntaxKind};
use crate::World;

/// Produce a tooltip which can be shown when a cursor position is hovered.
pub fn tooltip(world: &dyn World, source: &Source, cursor: usize) -> Option<String> {
    let leaf = LinkedNode::new(source.root()).leaf_at(cursor)?;

    // If a known identifier is under the cursor, provide its documentation.
    if let SyntaxKind::Ident(ident) = leaf.kind() {
        if let Some(value) = world.library().scope.get(ident) {
            if let Value::Func(func) = value {
                return func.doc().map(Into::into);
            }
        }
    }

    None
}
