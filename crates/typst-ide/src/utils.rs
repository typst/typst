use std::fmt::Write;
use std::ops::ControlFlow;

use comemo::Track;
use ecow::{EcoString, eco_format};
use typst::engine::{Engine, Route, Sink, Traced};
use typst::foundations::{Scope, Value};
use typst::introspection::Introspector;
use typst::syntax::{LinkedNode, SyntaxKind};
use typst::text::{FontInfo, FontStyle};
use typst::utils::Protected;

use crate::IdeWorld;

/// Create a temporary engine and run a task on it.
pub fn with_engine<F, T>(world: &dyn IdeWorld, f: F) -> T
where
    F: FnOnce(&mut Engine) -> T,
{
    let introspector = Introspector::default();
    let traced = Traced::default();
    let mut sink = Sink::new();
    let mut engine = Engine {
        routines: &typst::ROUTINES,
        world: world.upcast().track(),
        introspector: Protected::new(introspector.track()),
        traced: traced.track(),
        sink: sink.track_mut(),
        route: Route::default(),
    };

    f(&mut engine)
}

/// Create a short description of a font family.
pub fn summarize_font_family(mut variants: Vec<&FontInfo>) -> EcoString {
    variants.sort_by_key(|info| info.variant);

    let mut has_italic = false;
    let mut min_weight = u16::MAX;
    let mut max_weight = 0;
    for info in &variants {
        let weight = info.variant.weight.to_number();
        has_italic |= info.variant.style == FontStyle::Italic;
        min_weight = min_weight.min(weight);
        max_weight = min_weight.max(weight);
    }

    let count = variants.len();
    let mut detail = eco_format!("{count} variant{}.", if count == 1 { "" } else { "s" });

    if min_weight == max_weight {
        write!(detail, " Weight {min_weight}.").unwrap();
    } else {
        write!(detail, " Weights {min_weight}–{max_weight}.").unwrap();
    }

    if has_italic {
        detail.push_str(" Has italics.");
    }

    detail
}

/// The global definitions at the given node.
pub fn globals<'a>(world: &'a dyn IdeWorld, leaf: &LinkedNode) -> &'a Scope {
    let in_math = matches!(
        leaf.parent_kind(),
        Some(SyntaxKind::Equation)
            | Some(SyntaxKind::Math)
            | Some(SyntaxKind::MathFrac)
            | Some(SyntaxKind::MathAttach)
    ) && leaf
        .prev_leaf()
        .is_none_or(|prev| !matches!(prev.kind(), SyntaxKind::Hash));

    let library = world.library();
    if in_math { library.math.scope() } else { library.global.scope() }
}

/// Checks whether the given value or any of its constituent parts satisfy the
/// predicate.
pub fn check_value_recursively(
    value: &Value,
    predicate: impl Fn(&Value) -> bool,
) -> bool {
    let mut searcher = Searcher { steps: 0, predicate, max_steps: 1000 };
    match searcher.find(value) {
        ControlFlow::Break(matching) => matching,
        ControlFlow::Continue(_) => false,
    }
}

/// Recursively searches for a value that passes the filter, but without
/// exceeding a maximum number of search steps.
struct Searcher<F> {
    max_steps: usize,
    steps: usize,
    predicate: F,
}

impl<F> Searcher<F>
where
    F: Fn(&Value) -> bool,
{
    fn find(&mut self, value: &Value) -> ControlFlow<bool> {
        if (self.predicate)(value) {
            return ControlFlow::Break(true);
        }

        if self.steps > self.max_steps {
            return ControlFlow::Break(false);
        }

        self.steps += 1;

        match value {
            Value::Dict(dict) => {
                self.find_iter(dict.iter().map(|(_, v)| v))?;
            }
            Value::Content(content) => {
                self.find_iter(content.fields().iter().map(|(_, v)| v))?;
            }
            Value::Module(module) => {
                self.find_iter(module.scope().iter().map(|(_, b)| b.read()))?;
            }
            _ => {}
        }

        ControlFlow::Continue(())
    }

    fn find_iter<'a>(
        &mut self,
        iter: impl Iterator<Item = &'a Value>,
    ) -> ControlFlow<bool> {
        for item in iter {
            self.find(item)?;
        }
        ControlFlow::Continue(())
    }
}
