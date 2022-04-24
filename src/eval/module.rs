use super::Scope;
use crate::model::Content;
use crate::source::{SourceId, SourceStore};

/// An evaluated module, ready for importing or layouting.
#[derive(Debug, Clone)]
pub struct Module {
    /// The top-level definitions that were bound in this module.
    pub scope: Scope,
    /// The module's layoutable contents.
    pub content: Content,
    /// The source file revisions this module depends on.
    pub deps: Vec<(SourceId, usize)>,
}

impl Module {
    /// Whether the module is still valid for the given sources.
    pub fn valid(&self, sources: &SourceStore) -> bool {
        self.deps.iter().all(|&(id, rev)| rev == sources.get(id).rev())
    }
}
