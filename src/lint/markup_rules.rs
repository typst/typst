use super::Rule;

use crate::diag::{warning, SourceDiagnostic};
use crate::syntax::ast::Strong;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone)]
pub struct EmptyStrong;
impl Rule for EmptyStrong {
    fn accept(&self, node: &SyntaxNode) -> bool {
        node.kind() == SyntaxKind::Strong
    }

    fn lint(&self, node: &SyntaxNode) -> Vec<SourceDiagnostic> {
        if node.cast::<Strong>().unwrap().body().exprs().count() == 0 {
            vec![warning!(node.span(), "empty strong")]
        } else {
            vec![]
        }
    }
}
