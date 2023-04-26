mod markup_rules;

use crate::diag::SourceDiagnostic;
use crate::syntax::{Source, SyntaxNode};

use markup_rules::EmptyStrong;

pub fn lint(source: &Source) -> Vec<SourceDiagnostic> {
    let rules = rules();

    let mut warnings = vec![];
    let mut parents = vec![source.root()];
    while !parents.is_empty() {
        let node = parents.pop().unwrap();
        parents.extend(node.children().rev());

        for rule in rules.iter().filter(|&r| r.accept(node)) {
            warnings.append(&mut rule.lint(node));
        }
    }

    warnings
}

trait Rule {
    fn accept(&self, node: &SyntaxNode) -> bool;

    fn lint(&self, node: &SyntaxNode) -> Vec<SourceDiagnostic>;

    fn as_dyn(self) -> Box<dyn Rule>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}

fn rules() -> Vec<Box<dyn Rule>> {
    vec![EmptyStrong.as_dyn()]
}
