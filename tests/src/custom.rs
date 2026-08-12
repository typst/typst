use std::fmt::Write;
use std::io::Cursor;

use typst::World;
use typst::foundations::Smart;
use typst::introspection::{Location, Tag};
use typst::layout::{Frame, FrameItem};
use typst::model::{Document, DocumentInfo};
use typst_layout::PagedDocument;
use typst_render::RenderOptions;

use crate::collect::Test;
use crate::world::TestWorld;

/// We don't want to panic when there is a failure.
macro_rules! test_eq {
    ($sink:expr, $lhs:expr, $rhs:expr) => {
        if $lhs != $rhs {
            writeln!(&mut $sink, "{:?} != {:?}", $lhs, $rhs).unwrap();
        }
    };
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
        "document-set-full-metadata" => {
            if let Some(doc) = doc {
                if let Err(message) = check_png_metadata(doc) {
                    sink.push_str(&message);
                }
            } else {
                sink.push_str("missing document");
            }
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
    doc.map(|doc| doc.info().clone()).unwrap_or_default()
}

/// Check that PNG export embeds the expected iTXt metadata chunks.
fn check_png_metadata(doc: &PagedDocument) -> Result<(), String> {
    let page = doc.pages().first().ok_or("missing page")?;
    let pixmap = typst_render::render(page, &RenderOptions::default());
    let png = typst_render::encode_png(&pixmap, doc.info())
        .map_err(|err| format!("failed to encode PNG: {err}"))?;

    let decoder = png::Decoder::new(Cursor::new(png.as_slice()));
    let reader = decoder
        .read_info()
        .map_err(|err| format!("failed to read PNG: {err}"))?;
    let info = reader.info();

    let texts: Vec<_> = info
        .utf8_text
        .iter()
        .map(|chunk| {
            let text = chunk.get_text().unwrap_or_default();
            (chunk.keyword.as_str(), text, chunk.language_tag.as_str())
        })
        .collect();

    let expect = |keyword: &str, value: &str, lang: &str| {
        texts
            .iter()
            .any(|(k, t, l)| *k == keyword && t == value && *l == lang)
    };

    let mut errors = String::new();
    if !expect("Title", "Hope", "en") {
        writeln!(errors, "missing Title=Hope (lang=en)").ok();
    }
    if !expect("Author", "Alice", "en") {
        writeln!(errors, "missing Author=Alice (lang=en)").ok();
    }
    if !expect("Author", "Bob", "en") {
        writeln!(errors, "missing Author=Bob (lang=en)").ok();
    }
    if !expect("Description", "Here lies my hopes and dreams.", "en") {
        writeln!(errors, "missing Description (lang=en)").ok();
    }
    if !expect("Creation Time", "2002-08-11", "en") {
        writeln!(errors, "missing Creation Time=2002-08-11 (lang=en)").ok();
    }
    if !expect("Software", "Typst", "en") {
        writeln!(errors, "missing Software=Typst (lang=en)").ok();
    }
    if texts.iter().any(|(k, _, _)| *k == "Comment") {
        writeln!(errors, "unexpected Comment keyword (keywords are omitted)").ok();
    }

    // Ensure we actually got iTXt (utf8_text), not only latin1 tEXt.
    if info.utf8_text.is_empty() {
        writeln!(errors, "expected iTXt chunks in utf8_text").ok();
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
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
                    visit(stack, &group.frame)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    let mut stack = Vec::new();
    doc.pages().iter().try_for_each(|page| visit(&mut stack, &page.frame))
}
