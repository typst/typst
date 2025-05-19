use std::fmt::Write;
use std::ops::Range;
use std::path::PathBuf;

use ecow::eco_vec;
use tiny_skia as sk;
use typst::diag::{SourceDiagnostic, Warned};
use typst::html::HtmlDocument;
use typst::layout::{Abs, Frame, FrameItem, PagedDocument, Transform};
use typst::visualize::Color;
use typst::{Document, World, WorldExt};
use typst_pdf::PdfOptions;
use typst_syntax::{FileId, Lines};

use crate::collect::{Attr, FileSize, NoteKind, Test};
use crate::logger::TestResult;
use crate::world::{system_path, TestWorld};

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
                mismatched_output: false,
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
        if render {
            self.run_test::<PagedDocument>();
        }
        if html {
            self.run_test::<HtmlDocument>();
        }

        self.handle_not_emitted();
        self.handle_not_annotated();

        self.result
    }

    /// Run test specific to document format.
    fn run_test<D: OutputType>(&mut self) {
        let Warned { output, warnings } = typst::compile(&self.world);
        let (doc, errors) = match output {
            Ok(doc) => (Some(doc), eco_vec![]),
            Err(errors) => (None, errors),
        };

        if doc.is_none() && errors.is_empty() {
            log!(self, "no document, but also no errors");
        }

        D::check_custom(self, doc.as_ref());
        self.check_output(doc.as_ref());

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
            let note_range = self.format_range(note.file, &note.range);
            if first {
                log!(self, "not emitted");
                first = false;
            }
            log!(self, "  {}: {note_range} {} ({})", note.kind, note.message, note.pos,);
        }
    }

    /// Check that the document output is correct.
    fn check_output<D: OutputType>(&mut self, document: Option<&D>) {
        let live_path = D::live_path(&self.test.name);
        let ref_path = D::ref_path(&self.test.name);
        let ref_data = std::fs::read(&ref_path);

        let Some(document) = document else {
            if ref_data.is_ok() {
                log!(self, "missing document");
                log!(self, "  ref       | {}", ref_path.display());
            }
            return;
        };

        let skippable = match D::is_skippable(document) {
            Ok(skippable) => skippable,
            Err(()) => {
                log!(self, "document has zero pages");
                return;
            }
        };

        // Tests without visible output and no reference output don't need to be
        // compared.
        if skippable && ref_data.is_err() {
            std::fs::remove_file(&live_path).ok();
            return;
        }

        // Render and save live version.
        let live = document.make_live();
        document.save_live(&self.test.name, &live);

        // Compare against reference output if available.
        // Test that is ok doesn't need to be updated.
        if ref_data.as_ref().is_ok_and(|r| D::matches(&live, r)) {
            return;
        }

        if crate::ARGS.update {
            if skippable {
                std::fs::remove_file(&ref_path).unwrap();
                log!(
                    into: self.result.infos,
                    "removed reference output ({})", ref_path.display()
                );
            } else {
                let ref_data = D::make_ref(live);
                if !self.test.attrs.contains(&Attr::Large)
                    && ref_data.len() > crate::REF_LIMIT
                {
                    log!(self, "reference output would exceed maximum size");
                    log!(self, "  maximum   | {}", FileSize(crate::REF_LIMIT));
                    log!(self, "  size      | {}", FileSize(ref_data.len()));
                    log!(self, "please try to minimize the size of the test (smaller pages, less text, etc.)");
                    log!(self, "if you think the test cannot be reasonably minimized, mark it as `large`");
                    return;
                }
                std::fs::write(&ref_path, &ref_data).unwrap();
                log!(
                    into: self.result.infos,
                    "updated reference output ({}, {})",
                    ref_path.display(),
                    FileSize(ref_data.len()),
                );
            }
        } else {
            self.result.mismatched_output = true;
            if ref_data.is_ok() {
                log!(self, "mismatched output");
                log!(self, "  live      | {}", live_path.display());
                log!(self, "  ref       | {}", ref_path.display());
            } else {
                log!(self, "missing reference output");
                log!(self, "  live      | {}", live_path.display());
            }
        }
    }

    /// Compare a subset of notes with a given kind against diagnostics of
    /// that same kind.
    fn check_diagnostic(&mut self, kind: NoteKind, diag: &SourceDiagnostic) {
        // TODO: remove this once HTML export is stable
        if diag.message == "html export is under active development and incomplete" {
            return;
        }

        let message = diag.message.replace("\\", "/");
        let range = self.world.range(diag.span);
        self.validate_note(kind, diag.span.id(), range.clone(), &message);

        // Check hints.
        for hint in &diag.hints {
            self.validate_note(NoteKind::Hint, diag.span.id(), range.clone(), hint);
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
        file: Option<FileId>,
        range: Option<Range<usize>>,
        message: &str,
    ) {
        // Try to find perfect match.
        let file = file.unwrap_or(self.test.source.id());
        if let Some((i, _)) = self.test.notes.iter().enumerate().find(|&(i, note)| {
            !self.seen[i]
                && note.kind == kind
                && note.range == range
                && note.message == message
                && note.file == file
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
            let diag_range = self.format_range(file, &range);
            log!(into: self.not_annotated, "  {kind}: {diag_range} {}", message);
            return;
        };

        // Mark this annotation as visited and return it.
        self.seen[i] = true;

        // Range is wrong.
        if range != note.range {
            let note_range = self.format_range(note.file, &note.range);
            let note_text = self.text_for_range(note.file, &note.range);
            let diag_range = self.format_range(note.file, &range);
            let diag_text = self.text_for_range(note.file, &range);
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
    fn text_for_range(&self, file: FileId, range: &Option<Range<usize>>) -> String {
        let Some(range) = range else { return "No text".into() };
        if range.is_empty() {
            "(empty)".into()
        } else if file == self.test.source.id() {
            format!(
                "`{}`",
                self.test.source.text()[range.clone()]
                    .replace('\n', "\\n")
                    .replace('\r', "\\r")
            )
        } else {
            let bytes = self.world.file(file).unwrap();
            let text = String::from_utf8_lossy(&bytes);
            format!("`{}`", text[range.clone()].replace('\n', "\\n").replace('\r', "\\r"))
        }
    }

    /// Display a byte range as a line:column range.
    fn format_range(&self, file: FileId, range: &Option<Range<usize>>) -> String {
        let Some(range) = range else { return "No range".into() };

        let mut preamble = String::new();
        if file != self.test.source.id() {
            preamble = format!("\"{}\" ", system_path(file).unwrap().display());
        }

        if range.start == range.end {
            format!("{preamble}{}", self.format_pos(file, range.start))
        } else {
            format!(
                "{preamble}{}-{}",
                self.format_pos(file, range.start),
                self.format_pos(file, range.end)
            )
        }
    }

    /// Display a position as a line:column pair.
    fn format_pos(&self, file: FileId, pos: usize) -> String {
        let res = if file != self.test.source.id() {
            let bytes = self.world.file(file).unwrap();
            let lines = Lines::from_bytes(&bytes).unwrap();
            lines.byte_to_line_column(pos).map(|(line, col)| (line + 1, col + 1))
        } else {
            (self.test.source.lines())
                .byte_to_line_column(pos)
                .map(|(line, col)| (line + 1, col + 1))
        };
        let Some((line, col)) = res else {
            return "oob".into();
        };

        if line == 1 {
            format!("{col}")
        } else {
            format!("{line}:{col}")
        }
    }
}

/// An output type we can test.
trait OutputType: Document {
    /// The type that represents live output.
    type Live;

    /// The path at which the live output is stored.
    fn live_path(name: &str) -> PathBuf;

    /// The path at which the reference output is stored.
    fn ref_path(name: &str) -> PathBuf;

    /// Whether the test output is trivial and needs no reference output.
    fn is_skippable(&self) -> Result<bool, ()> {
        Ok(false)
    }

    /// Produces the live output.
    fn make_live(&self) -> Self::Live;

    /// Saves the live output.
    fn save_live(&self, name: &str, live: &Self::Live);

    /// Produces the reference output from the live output.
    fn make_ref(live: Self::Live) -> Vec<u8>;

    /// Checks whether the live and reference output match.
    fn matches(live: &Self::Live, ref_data: &[u8]) -> bool;

    /// Runs additional checks.
    #[expect(unused_variables)]
    fn check_custom(runner: &mut Runner, doc: Option<&Self>) {}
}

impl OutputType for PagedDocument {
    type Live = tiny_skia::Pixmap;

    fn live_path(name: &str) -> PathBuf {
        format!("{}/render/{}.png", crate::STORE_PATH, name).into()
    }

    fn ref_path(name: &str) -> PathBuf {
        format!("{}/{}.png", crate::REF_PATH, name).into()
    }

    fn is_skippable(&self) -> Result<bool, ()> {
        /// Whether rendering of a frame can be skipped.
        fn skippable_frame(frame: &Frame) -> bool {
            frame.items().all(|(_, item)| match item {
                FrameItem::Group(group) => skippable_frame(&group.frame),
                FrameItem::Tag(_) => true,
                _ => false,
            })
        }

        match self.pages.as_slice() {
            [] => Err(()),
            [page] => Ok(page.frame.width().approx_eq(Abs::pt(120.0))
                && page.frame.height().approx_eq(Abs::pt(20.0))
                && page.fill.is_auto()
                && skippable_frame(&page.frame)),
            _ => Ok(false),
        }
    }

    fn make_live(&self) -> Self::Live {
        render(self, 1.0)
    }

    fn save_live(&self, name: &str, live: &Self::Live) {
        // Save live version, possibly rerendering if different scale is
        // requested.
        let mut pixmap_live = live;
        let slot;
        let scale = crate::ARGS.scale;
        if scale != 1.0 {
            slot = render(self, scale);
            pixmap_live = &slot;
        }
        let data: Vec<u8> = pixmap_live.encode_png().unwrap();
        std::fs::write(Self::live_path(name), data).unwrap();

        // Write PDF if requested.
        if crate::ARGS.pdf() {
            let pdf_path = format!("{}/pdf/{}.pdf", crate::STORE_PATH, name);
            let pdf = typst_pdf::pdf(self, &PdfOptions::default()).unwrap();
            std::fs::write(pdf_path, pdf).unwrap();
        }

        // Write SVG if requested.
        if crate::ARGS.svg() {
            let svg_path = format!("{}/svg/{}.svg", crate::STORE_PATH, name);
            let svg = typst_svg::svg_merged(self, Abs::pt(5.0));
            std::fs::write(svg_path, svg).unwrap();
        }
    }

    fn make_ref(live: Self::Live) -> Vec<u8> {
        let opts = oxipng::Options::max_compression();
        let data = live.encode_png().unwrap();
        oxipng::optimize_from_memory(&data, &opts).unwrap()
    }

    fn matches(live: &Self::Live, ref_data: &[u8]) -> bool {
        let ref_pixmap = sk::Pixmap::decode_png(ref_data).unwrap();
        approx_equal(live, &ref_pixmap)
    }

    fn check_custom(runner: &mut Runner, doc: Option<&Self>) {
        let errors = crate::custom::check(runner.test, &runner.world, doc);
        if !errors.is_empty() {
            log!(runner, "custom check failed");
            for line in errors.lines() {
                log!(runner, "  {line}");
            }
        }
    }
}

impl OutputType for HtmlDocument {
    type Live = String;

    fn live_path(name: &str) -> PathBuf {
        format!("{}/html/{}.html", crate::STORE_PATH, name).into()
    }

    fn ref_path(name: &str) -> PathBuf {
        format!("{}/html/{}.html", crate::REF_PATH, name).into()
    }

    fn make_live(&self) -> Self::Live {
        // TODO: Do this earlier to be able to process export errors.
        typst_html::html(self).unwrap()
    }

    fn save_live(&self, name: &str, live: &Self::Live) {
        std::fs::write(Self::live_path(name), live).unwrap();
    }

    fn make_ref(live: Self::Live) -> Vec<u8> {
        live.into_bytes()
    }

    fn matches(live: &Self::Live, ref_data: &[u8]) -> bool {
        live.as_bytes() == ref_data
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
