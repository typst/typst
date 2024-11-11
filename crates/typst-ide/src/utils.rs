use std::fmt::Write;

use comemo::Track;
use ecow::{eco_format, EcoString};
use typst::engine::{Engine, Route, Sink, Traced};
use typst::foundations::Scope;
use typst::introspection::Introspector;
use typst::syntax::{LinkedNode, SyntaxKind};
use typst::text::{FontInfo, FontStyle};

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
        introspector: introspector.track(),
        traced: traced.track(),
        sink: sink.track_mut(),
        route: Route::default(),
    };

    f(&mut engine)
}

/// Extract the first sentence of plain text of a piece of documentation.
///
/// Removes Markdown formatting.
pub fn plain_docs_sentence(docs: &str) -> EcoString {
    let mut s = unscanny::Scanner::new(docs);
    let mut output = EcoString::new();
    let mut link = false;
    while let Some(c) = s.eat() {
        match c {
            '`' => {
                let mut raw = s.eat_until('`');
                if (raw.starts_with('{') && raw.ends_with('}'))
                    || (raw.starts_with('[') && raw.ends_with(']'))
                {
                    raw = &raw[1..raw.len() - 1];
                }

                s.eat();
                output.push('`');
                output.push_str(raw);
                output.push('`');
            }
            '[' => link = true,
            ']' if link => {
                if s.eat_if('(') {
                    s.eat_until(')');
                    s.eat();
                } else if s.eat_if('[') {
                    s.eat_until(']');
                    s.eat();
                }
                link = false
            }
            '*' | '_' => {}
            '.' => {
                output.push('.');
                break;
            }
            _ => output.push(c),
        }
    }

    output
}

/// Create a short description of a font family.
pub fn summarize_font_family<'a>(
    variants: impl Iterator<Item = &'a FontInfo>,
) -> EcoString {
    let mut infos: Vec<_> = variants.collect();
    infos.sort_by_key(|info| info.variant);

    let mut has_italic = false;
    let mut min_weight = u16::MAX;
    let mut max_weight = 0;
    for info in &infos {
        let weight = info.variant.weight.to_number();
        has_italic |= info.variant.style == FontStyle::Italic;
        min_weight = min_weight.min(weight);
        max_weight = min_weight.max(weight);
    }

    let count = infos.len();
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
    );

    let library = world.library();
    if in_math {
        library.math.scope()
    } else {
        library.global.scope()
    }
}
