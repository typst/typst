use std::fmt::Write;

use ecow::{EcoString, eco_format};
use typst::World;
use typst::foundations::Smart;
use typst::introspection::{Location, Tag};
use typst::layout::{Frame, FrameItem, PagedDocument};
use typst::model::DocumentInfo;

use crate::collect::{Attrs, Test};
use crate::world::TestWorld;

/// We don't want to panic when there is a failure.
macro_rules! test_eq {
    ($sink:expr, $lhs:expr, $rhs:expr) => {
        if $lhs != $rhs {
            writeln!(&mut $sink, "{:?} != {:?}", $lhs, $rhs).unwrap();
        }
    };
}

/// Math fonts to test when the `math-fonts` attribute is set.
const MATH_FONTS: [(&str, &str); 11] = [
    ("default", "New Computer Modern Math"),
    ("asana", "Asana Math"),
    ("concrete", "Concrete Math"),
    ("garamond", "Garamond-Math"),
    ("ibm-plex", "IBM Plex Math"),
    ("libertinus", "Libertinus Math"),
    ("noto-sans", "Noto Sans Math"),
    ("pennstander", "Pennstander Math"),
    ("stix-two", "STIX Two Math"),
    ("tex-gyre-bonum", "TeX Gyre Bonum Math"),
    ("xits", "XITS Math"),
];

/// Generates test variants based on attributes.
///
/// Returns a list of pairs `(test_name, optional_source_prepend)`.
pub fn generate_variants(
    base_name: &EcoString,
    attrs: Attrs,
) -> Vec<(EcoString, Option<String>)> {
    if attrs.math_fonts {
        MATH_FONTS
            .iter()
            .map(|(short, full)| {
                let name = eco_format!("{base_name}-{short}");
                let prepend = format!(
                    "#show math.equation: set text(font: \"{full}\", fallback: false)\n"
                );
                (name, Some(prepend))
            })
            .collect()
    } else {
        vec![(base_name.clone(), None)]
    }
}

/// Run special checks for specific tests for which it is not worth it to create
/// custom annotations.
pub fn check(test: &Test, world: &TestWorld, doc: Option<&PagedDocument>) -> String {
    let mut sink = String::new();
    match test.name.as_str() {
        "document-set-author-date" => {
            let info = info(doc);
            test_eq!(sink, info.author, ["A", "B"]);
            test_eq!(sink, info.date, Smart::Custom(world.today(None)));
        }
        "issue-4065-document-context" => {
            let info = info(doc);
            test_eq!(sink, info.title.as_deref(), Some("Top level"));
        }
        "issue-4769-document-context-conditional" => {
            let info = info(doc);
            test_eq!(sink, info.author, ["Changed"]);
            test_eq!(sink, info.title.as_deref(), Some("Alternative"));
        }
        "tags-grouping" | "tags-textual" => {
            if let Some(doc) = doc {
                if let Err(message) = check_balanced(doc) {
                    sink.push_str(message);
                }
            } else {
                sink.push_str("missing document");
            }
        }
        _ => {}
    }
    sink
}

/// Extract the document information.
fn info(doc: Option<&PagedDocument>) -> DocumentInfo {
    doc.map(|doc| doc.info.clone()).unwrap_or_default()
}

/// Naive check for whether tags are balanced in the document.
///
/// This is kept minimal for now: It does not handle groups with parents and
/// does not print useful debugging information. This is currently only run for
/// specific tests that are known not to have those. We might want to extend
/// this to the whole test suite in the future. Then we'll need to handle
/// insertions and provide a better debugging experience. However, there are
/// scenarios that are inherently (and correctly) unbalanced and we'd need some
/// way to opt out for those (via something like `large`).
fn check_balanced(doc: &PagedDocument) -> Result<(), &'static str> {
    fn visit(stack: &mut Vec<Location>, frame: &Frame) -> Result<(), &'static str> {
        for (_, item) in frame.items() {
            match item {
                FrameItem::Tag(tag) => match tag {
                    Tag::Start(..) => stack.push(tag.location()),
                    Tag::End(..) => {
                        if stack.pop() != Some(tag.location()) {
                            return Err("tags are unbalanced");
                        }
                    }
                },
                FrameItem::Group(group) => {
                    if group.parent.is_some() {
                        return Err("groups with parents are not supported");
                    }
                    visit(stack, &group.frame)?
                }
                _ => {}
            }
        }
        Ok(())
    }

    let mut stack = Vec::new();
    doc.pages.iter().try_for_each(|page| visit(&mut stack, &page.frame))
}
