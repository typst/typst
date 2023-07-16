use ecow::EcoString;

use crate::{
    diag::{warning, SourceDiagnostic},
    syntax::{ast::Strong, Source, SyntaxKind, SyntaxNode},
};

/// Lints a [`Source`] and returns a list of all warnings found.
pub fn lint(source: &Source) -> Vec<SourceDiagnostic> {
    let mut warnings = Vec::new();
    let mut parents = vec![source.root()];

    while let Some(node) = parents.pop() {
        parents.extend(node.children().rev());

        warnings.extend(lint_node(node));
    }

    warnings
}

/// Lints a [`SyntaxNode`] and returns a list of all warnings found.
fn lint_node(node: &SyntaxNode) -> Vec<SourceDiagnostic> {
    let mut warnings = Vec::new();

    if let Some(diag) = empty_bold(node) {
        warnings.push(diag);
    }

    warnings
}

/// Check if a [`SyntaxNode`] is an empty `Strong` node.
fn empty_bold(node: &SyntaxNode) -> Option<SourceDiagnostic> {
    if node.kind() == SyntaxKind::Strong
        && node.cast::<Strong>().unwrap().body().exprs().count() == 0
    {
        Some(warning!(node.span(), "no text within stars").with_hint(EcoString::from(
            "using multiple consecutive stars (e.g. **) has no additional effect",
        )))
    } else {
        None
    }
}
