use std::fmt::Write;
use std::ops::Range;
use std::path::Path;

use ecow::eco_vec;
use tiny_skia as sk;
use typst::diag::{SourceDiagnostic, Warned};
use typst::html::HtmlDocument;
use typst::layout::{Abs, Frame, FrameItem, Page, PagedDocument, Transform};
use typst::visualize::Color;
use typst::WorldExt;
use typst_pdf::PdfOptions;

use crate::collect::{Attr, FileSize, NoteKind, Test};
use crate::logger::TestResult;
use crate::world::TestWorld;

/// Runs a single test.
///
/// Returns whether the test passed.
pub fn run(test: &Test) -> TestResult {
    Runner::new(test).run()
}

/// Write a line to a log sink, defaulting to the test's error log.
macro_rules! log {
    (into: $sink:expr, $($tts:tt)*) => {
        writeln!($sink, $($tts)*).unwrap();
    };
    ($runner:expr, $($tts:tt)*) => {
        writeln!(&mut $runner.result.errors, $($tts)*).unwrap();
    };
}

/// Runs a single test.
pub struct Runner<'a> {
    test: &'a Test,
    world: TestWorld,
    seen: Vec<bool>,
    result: TestResult,
    not_annotated: String,
}

impl<'a> Runner<'a> {
    /// Create a new test runner.
    fn new(test: &'a Test) -> Self {
        Self {
            test,
            world: TestWorld::new(test.source.clone()),
            seen: vec![false; test.notes.len()],
            result: TestResult {
                errors: String::new(),
                infos: String::new(),
                mismatched_image: false,
            },
            not_annotated: String::new(),
        }
    }

    /// Run the test.
    fn run(mut self) -> TestResult {
        if crate::ARGS.syntax {
            log!(into: self.result.infos, "tree: {:#?}", self.test.source.root());
        }

        let html = self.test.attrs.contains(&Attr::Html);
        let render = !html || self.test.attrs.contains(&Attr::Render);
        if html {
            self.run_html();
        }
        if render {
            self.run_render();
        }

        self.handle_not_emitted();
        self.handle_not_annotated();

        self.result
    }

    fn run_render(&mut self) {
        let Warned { output, warnings } = typst::compile(&self.world);
        let (doc, errors) = match output {
            Ok(doc) => (Some(doc), eco_vec![]),
            Err(errors) => (None, errors),
        };

        if doc.is_none() && errors.is_empty() {
            log!(self, "no document, but also no errors");
        }

        self.check_custom(doc.as_ref());
        self.check_document(doc.as_ref());

        for error in &errors {
            self.check_diagnostic(NoteKind::Error, error);
        }

        for warning in &warnings {
            self.check_diagnostic(NoteKind::Warning, warning);
        }
    }

    fn run_html(&mut self) {
        let Warned { output, warnings } = typst::compile(&self.world);
        let (doc, errors) = match output {
            Ok(doc) => (Some(doc), eco_vec![]),
            Err(errors) => (None, errors),
        };

        if doc.is_none() && errors.is_empty() {
            log!(self, "no document, but also no errors");
        }

        self.check_html(doc.as_ref());

        for error in &errors {
            self.check_diagnostic(NoteKind::Error, error);
        }

        for warning in &warnings {
            self.check_diagnostic(NoteKind::Warning, warning);
        }
    }

    /// Handle errors that weren't annotated.
    fn handle_not_annotated(&mut self) {
        if !self.not_annotated.is_empty() {
            log!(self, "not annotated");
            self.result.errors.push_str(&self.not_annotated);
        }
    }

    /// Handle notes that weren't handled before.
    fn handle_not_emitted(&mut self) {
        let mut first = true;
        for (note, &seen) in self.test.notes.iter().zip(&self.seen) {
            if seen {
                continue;
            }
            let note_range = self.format_range(&note.range);
            if first {
                log!(self, "not emitted");
                first = false;
            }
            log!(self, "  {}: {note_range} {} ({})", note.kind, note.message, note.pos,);
        }
    }

    /// Run custom checks for which it is not worth to create special
    /// annotations.
    fn check_custom(&mut self, doc: Option<&PagedDocument>) {
        let errors = crate::custom::check(self.test, &self.world, doc);
        if !errors.is_empty() {
            log!(self, "custom check failed");
            for line in errors.lines() {
                log!(self, "  {line}");
            }
        }
    }

    /// Check that the document output is correct.
    fn check_html(&mut self, document: Option<&HtmlDocument>) {
        let live_path = format!("{}/html/{}.html", crate::STORE_PATH, self.test.name);
        let ref_path = format!("{}/html/{}.html", crate::REF_PATH, self.test.name);
        let has_ref = Path::new(&ref_path).exists();

        let Some(document) = document else {
            if has_ref {
                log!(self, "missing document");
                log!(self, "  ref       | {ref_path}");
            }
            return;
        };

        let data = typst_html::html(document).unwrap();
        dbg!(&live_path);
        std::fs::write(&live_path, &data).unwrap();

        // Compare against reference if available.
        let equal = has_ref && {
            let ref_data = std::fs::read_to_string(&ref_path).unwrap();
            data == ref_data
        };

        // Test that is ok doesn't need to be updated.
        if equal {
            return;
        }

        if crate::ARGS.update {
            let ref_data = data;
            if !self.test.attrs.contains(&Attr::Large)
                && ref_data.len() > crate::REF_LIMIT
            {
                log!(self, "reference HTML would exceed maximum size");
                log!(self, "  maximum   | {}", FileSize(crate::REF_LIMIT));
                log!(self, "  size      | {}", FileSize(ref_data.len()));
                log!(
                    self,
                    "please try to minimize the size of the test (less text, etc.)"
                );
                log!(self, "if you think the test cannot be reasonably minimized, mark it as `large`");
                return;
            }
            std::fs::write(&ref_path, &ref_data).unwrap();
            log!(
                into: self.result.infos,
                "updated reference HTML ({ref_path}, {})",
                FileSize(ref_data.len()),
            );
        } else {
            // TODO: rename to `mismatched`
            self.result.mismatched_image = true;
            if has_ref {
                log!(self, "mismatched rendering");
                log!(self, "  live      | {live_path}");
                log!(self, "  ref       | {ref_path}");
            } else {
                log!(self, "missing reference image");
                log!(self, "  live      | {live_path}");
            }
        }
    }

    /// Check that the document output is correct.
    fn check_document(&mut self, document: Option<&PagedDocument>) {
        let live_path = format!("{}/render/{}.png", crate::STORE_PATH, self.test.name);
        let ref_path = format!("{}/{}.png", crate::REF_PATH, self.test.name);
        let has_ref = Path::new(&ref_path).exists();

        let Some(document) = document else {
            if has_ref {
                log!(self, "missing document");
                log!(self, "  ref       | {ref_path}");
            }
            return;
        };

        let skippable = match document.pages.as_slice() {
            [] => {
                log!(self, "document has zero pages");
                return;
            }
            [page] => skippable(page),
            _ => false,
        };

        // Tests without visible output and no reference image don't need to be
        // compared.
        if skippable && !has_ref {
            std::fs::remove_file(&live_path).ok();
            return;
        }

        // Render the live version.
        let pixmap = render(document, 1.0);

        // Save live version, possibly rerendering if different scale is
        // requested.
        let mut pixmap_live = &pixmap;
        let slot;
        let scale = crate::ARGS.scale;
        if scale != 1.0 {
            slot = render(document, scale);
            pixmap_live = &slot;
        }
        let data = pixmap_live.encode_png().unwrap();
        std::fs::write(&live_path, data).unwrap();

        // Write PDF if requested.
        if crate::ARGS.pdf() {
            let pdf_path = format!("{}/pdf/{}.pdf", crate::STORE_PATH, self.test.name);
            let pdf = typst_pdf::pdf(document, &PdfOptions::default()).unwrap();
            std::fs::write(pdf_path, pdf).unwrap();
        }

        // Write SVG if requested.
        if crate::ARGS.svg() {
            let svg_path = format!("{}/svg/{}.svg", crate::STORE_PATH, self.test.name);
            let svg = typst_svg::svg_merged(document, Abs::pt(5.0));
            std::fs::write(svg_path, svg).unwrap();
        }

        // Compare against reference image if available.
        let equal = has_ref && {
            let ref_data = std::fs::read(&ref_path).unwrap();
            let ref_pixmap = sk::Pixmap::decode_png(&ref_data).unwrap();
            approx_equal(&pixmap, &ref_pixmap)
        };

        // Test that is ok doesn't need to be updated.
        if equal {
            return;
        }

        if crate::ARGS.update {
            if skippable {
                std::fs::remove_file(&ref_path).unwrap();
                log!(
                    into: self.result.infos,
                    "removed reference image ({ref_path})"
                );
            } else {
                let opts = oxipng::Options::max_compression();
                let data = pixmap.encode_png().unwrap();
                let ref_data = oxipng::optimize_from_memory(&data, &opts).unwrap();
                if !self.test.attrs.contains(&Attr::Large)
                    && ref_data.len() > crate::REF_LIMIT
                {
                    log!(self, "reference image would exceed maximum size");
                    log!(self, "  maximum   | {}", FileSize(crate::REF_LIMIT));
                    log!(self, "  size      | {}", FileSize(ref_data.len()));
                    log!(self, "please try to minimize the size of the test (smaller pages, less text, etc.)");
                    log!(self, "if you think the test cannot be reasonably minimized, mark it as `large`");
                    return;
                }
                std::fs::write(&ref_path, &ref_data).unwrap();
                log!(
                    into: self.result.infos,
                    "updated reference image ({ref_path}, {})",
                    FileSize(ref_data.len()),
                );
            }
        } else {
            self.result.mismatched_image = true;
            if has_ref {
                log!(self, "mismatched rendering");
                log!(self, "  live      | {live_path}");
                log!(self, "  ref       | {ref_path}");
            } else {
                log!(self, "missing reference image");
                log!(self, "  live      | {live_path}");
            }
        }
    }

    /// Compare a subset of notes with a given kind against diagnostics of
    /// that same kind.
    fn check_diagnostic(&mut self, kind: NoteKind, diag: &SourceDiagnostic) {
        // Ignore diagnostics from other sources than the test file itself.
        if diag.span.id().is_some_and(|id| id != self.test.source.id()) {
            return;
        }

        let message = diag.message.replace("\\", "/");
        let range = self.world.range(diag.span);
        self.validate_note(kind, range.clone(), &message);

        // Check hints.
        for hint in &diag.hints {
            self.validate_note(NoteKind::Hint, range.clone(), hint);
        }
    }

    /// Try to find a matching note for the given `kind`, `range`, and
    /// `message`.
    ///
    /// - If found, marks it as seen and returns it.
    /// - If none was found, emits a "Not annotated" error and returns nothing.
    fn validate_note(
        &mut self,
        kind: NoteKind,
        range: Option<Range<usize>>,
        message: &str,
    ) {
        // Try to find perfect match.
        if let Some((i, _)) = self.test.notes.iter().enumerate().find(|&(i, note)| {
            !self.seen[i]
                && note.kind == kind
                && note.range == range
                && note.message == message
        }) {
            self.seen[i] = true;
            return;
        }

        // Try to find closely matching annotation. If the note has the same
        // range or message, it's most likely the one we're interested in.
        let Some((i, note)) = self.test.notes.iter().enumerate().find(|&(i, note)| {
            !self.seen[i]
                && note.kind == kind
                && (note.range == range || note.message == message)
        }) else {
            // Not even a close match, diagnostic is not annotated.
            let diag_range = self.format_range(&range);
            log!(into: self.not_annotated, "  {kind}: {diag_range} {}", message);
            return;
        };

        // Mark this annotation as visited and return it.
        self.seen[i] = true;

        // Range is wrong.
        if range != note.range {
            let note_range = self.format_range(&note.range);
            let note_text = self.text_for_range(&note.range);
            let diag_range = self.format_range(&range);
            let diag_text = self.text_for_range(&range);
            log!(self, "mismatched range ({}):", note.pos);
            log!(self, "  message   | {}", note.message);
            log!(self, "  annotated | {note_range:<9} | {note_text}");
            log!(self, "  emitted   | {diag_range:<9} | {diag_text}");
        }

        // Message is wrong.
        if message != note.message {
            log!(self, "mismatched message ({}):", note.pos);
            log!(self, "  annotated | {}", note.message);
            log!(self, "  emitted   | {message}");
        }
    }

    /// Display the text for a range.
    fn text_for_range(&self, range: &Option<Range<usize>>) -> String {
        let Some(range) = range else { return "No text".into() };
        if range.is_empty() {
            "(empty)".into()
        } else {
            format!("`{}`", self.test.source.text()[range.clone()].replace('\n', "\\n"))
        }
    }

    /// Display a byte range as a line:column range.
    fn format_range(&self, range: &Option<Range<usize>>) -> String {
        let Some(range) = range else { return "No range".into() };
        if range.start == range.end {
            self.format_pos(range.start)
        } else {
            format!("{}-{}", self.format_pos(range.start,), self.format_pos(range.end,))
        }
    }

    /// Display a position as a line:column pair.
    fn format_pos(&self, pos: usize) -> String {
        if let (Some(line_idx), Some(column_idx)) =
            (self.test.source.byte_to_line(pos), self.test.source.byte_to_column(pos))
        {
            let line = self.test.pos.line + line_idx;
            let column = column_idx + 1;
            if line == 1 {
                format!("{column}")
            } else {
                format!("{line}:{column}")
            }
        } else {
            "oob".into()
        }
    }
}

/// Draw all frames into one image with padding in between.
fn render(document: &PagedDocument, pixel_per_pt: f32) -> sk::Pixmap {
    for page in &document.pages {
        let limit = Abs::cm(100.0);
        if page.frame.width() > limit || page.frame.height() > limit {
            panic!("overlarge frame: {:?}", page.frame.size());
        }
    }

    let gap = Abs::pt(1.0);
    let mut pixmap =
        typst_render::render_merged(document, pixel_per_pt, gap, Some(Color::BLACK));

    let gap = (pixel_per_pt * gap.to_pt() as f32).round();

    let mut y = 0.0;
    for page in &document.pages {
        let ts =
            sk::Transform::from_scale(pixel_per_pt, pixel_per_pt).post_translate(0.0, y);
        render_links(&mut pixmap, ts, &page.frame);
        y += (pixel_per_pt * page.frame.height().to_pt() as f32).round().max(1.0) + gap;
    }

    pixmap
}

/// Draw extra boxes for links so we can see whether they are there.
fn render_links(canvas: &mut sk::Pixmap, ts: sk::Transform, frame: &Frame) {
    for (pos, item) in frame.items() {
        let ts = ts.pre_translate(pos.x.to_pt() as f32, pos.y.to_pt() as f32);
        match *item {
            FrameItem::Group(ref group) => {
                let ts = ts.pre_concat(to_sk_transform(&group.transform));
                render_links(canvas, ts, &group.frame);
            }
            FrameItem::Link(_, size) => {
                let w = size.x.to_pt() as f32;
                let h = size.y.to_pt() as f32;
                let rect = sk::Rect::from_xywh(0.0, 0.0, w, h).unwrap();
                let mut paint = sk::Paint::default();
                paint.set_color_rgba8(40, 54, 99, 40);
                canvas.fill_rect(rect, &paint, ts, None);
            }
            _ => {}
        }
    }
}

/// Whether rendering of a frame can be skipped.
fn skippable(page: &Page) -> bool {
    page.frame.width().approx_eq(Abs::pt(120.0))
        && page.frame.height().approx_eq(Abs::pt(20.0))
        && page.fill.is_auto()
        && skippable_frame(&page.frame)
}

/// Whether rendering of a frame can be skipped.
fn skippable_frame(frame: &Frame) -> bool {
    frame.items().all(|(_, item)| match item {
        FrameItem::Group(group) => skippable_frame(&group.frame),
        FrameItem::Tag(_) => true,
        _ => false,
    })
}

/// Whether two pixel images are approximately equal.
fn approx_equal(a: &sk::Pixmap, b: &sk::Pixmap) -> bool {
    a.width() == b.width()
        && a.height() == b.height()
        && a.data().iter().zip(b.data()).all(|(&a, &b)| a.abs_diff(b) <= 1)
}

/// Convert a Typst transform to a tiny-skia transform.
fn to_sk_transform(transform: &Transform) -> sk::Transform {
    let Transform { sx, ky, kx, sy, tx, ty } = *transform;
    sk::Transform::from_row(
        sx.get() as _,
        ky.get() as _,
        kx.get() as _,
        sy.get() as _,
        tx.to_pt() as f32,
        ty.to_pt() as f32,
    )
}
