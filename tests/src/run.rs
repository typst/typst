use std::fmt::Write;
use std::ops::Range;
use std::str::FromStr;
use std::sync::LazyLock;

use parking_lot::RwLock;
use regex::{Captures, Regex};
use rustc_hash::FxHashMap;
use typst::diag::{SourceDiagnostic, Warned};
use typst::layout::PagedDocument;
use typst::{Document, WorldExt};
use typst_html::HtmlDocument;
use typst_syntax::{FileId, Lines, VirtualPath};

use crate::collect::{FileSize, NoteKind, Test, TestStage, TestStages, TestTarget};
use crate::logger::TestResult;
use crate::output::{FileOutputType, HashOutputType, HashedRefs, OutputType};
use crate::world::{TestWorld, system_path};
use crate::{ARGS, custom, output};

type OutputHashes = FxHashMap<&'static VirtualPath, HashedRefs>;

/// Runs a single test.
///
/// Returns whether the test passed.
pub fn run(hashes: &[RwLock<OutputHashes>], test: &Test) -> TestResult {
    Runner::new(hashes, test).run()
}

/// Write all hashed references that have been updated
pub fn update_hash_refs<T: HashOutputType>(hashes: &[RwLock<OutputHashes>]) {
    #[allow(clippy::iter_over_hash_type)]
    for (source_path, hashed_refs) in hashes[T::INDEX].write().iter_mut() {
        hashed_refs.sort();
        if !hashed_refs.changed {
            continue;
        }

        let ref_path = T::OUTPUT.hashed_ref_path(source_path.as_rootless_path());
        if hashed_refs.is_empty() {
            std::fs::remove_file(ref_path).ok();
        } else {
            std::fs::write(ref_path, hashed_refs.to_string()).unwrap();
        }
    }
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
    hashes: &'a [RwLock<OutputHashes>],
    test: &'a Test,
    world: TestWorld,
    /// In which targets the note has been seen.
    seen: Vec<TestStages>,
    result: TestResult,
    not_annotated: String,
}

impl<'a> Runner<'a> {
    /// Create a new test runner.
    fn new(hashes: &'a [RwLock<OutputHashes>], test: &'a Test) -> Self {
        Self {
            hashes,
            test,
            world: TestWorld::new(test.source.clone()),
            seen: vec![TestStages::empty(); test.notes.len()],
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

        // Only compile paged document when the paged target is specified or
        // implied. If so, run tests for all paged outputs unless excluded by
        // the `--stages` flag. The test attribute still control which
        // reference outputs are saved and compared though.
        if ARGS.should_run(self.test.attrs.stages & TestStages::PAGED_STAGES) {
            let mut doc = self.compile::<PagedDocument>(TestTarget::Paged);

            if let Some(doc) = &mut doc
                && doc.info.title.is_none()
            {
                doc.info.title = Some(self.test.name.clone());
            }

            let errors = custom::check(self.test, &self.world, doc.as_ref());
            if !errors.is_empty() {
                log!(self, "custom check failed");
                for line in errors.lines() {
                    log!(self, "  {line}");
                }
            }

            if ARGS.should_run(self.test.attrs.stages & TestStages::RENDER) {
                self.run_file_test::<output::Render>(doc.as_ref());
            }
            if ARGS.should_run(self.test.attrs.stages & TestStages::PDF_STAGES) {
                let pdf = self.run_hash_test::<output::Pdf>(doc.as_ref());
                if ARGS.should_run(TestStages::PDFTAGS) {
                    self.run_file_test::<output::Pdftags>(pdf.as_ref());
                }
            }
            if ARGS.should_run(self.test.attrs.stages & TestStages::SVG) {
                self.run_hash_test::<output::Svg>(doc.as_ref());
            }
        }

        // Only compile html document when the html target is specified.
        if ARGS.should_run(self.test.attrs.stages & TestStages::HTML) {
            let doc = self.compile::<HtmlDocument>(TestTarget::Html);
            self.run_file_test::<output::Html>(doc.as_ref());
        }

        self.handle_not_emitted();
        self.handle_not_annotated();

        self.result
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
            let mut missing_stages = TestStages::empty();
            let required_stages = ARGS.stages() & self.test.attrs.stages;

            // If a test stage requiring the paged target has been specified,
            // require the annotated diagnostics to be present in any paged
            // stage. For example if `pdftags` is specified and an error is
            // emitted in the pdf stage, that is ok.
            if !(required_stages & TestStages::PAGED_STAGES).is_empty()
                && (seen & TestStages::PAGED_STAGES).is_empty()
            {
                missing_stages |= TestStages::PAGED;
            }
            if !(required_stages & TestStages::HTML).is_empty()
                && (seen & TestStages::HTML).is_empty()
            {
                missing_stages |= TestStages::HTML;
            }

            if missing_stages.is_empty() {
                continue;
            }
            let note_range = self.format_range(note.file, &note.range);
            if first {
                log!(self, "not emitted [{missing_stages}]");
                first = false;
            }
            log!(self, "  {}: {note_range} {} ({})", note.kind, note.message, note.pos,);
        }
    }

    /// Compile a document with the specified target.
    fn compile<D: Document>(&mut self, target: TestTarget) -> Option<D> {
        let Warned { output, warnings } = typst::compile::<D>(&self.world);
        for warning in &warnings {
            self.check_diagnostic(NoteKind::Warning, warning, target);
        }

        if let Err(errors) = &output {
            for error in errors.iter() {
                self.check_diagnostic(NoteKind::Error, error, target);
            }
        }

        output.ok()
    }

    /// Run test for an output format that produces a file reference.
    fn run_file_test<T: FileOutputType>(
        &mut self,
        doc: Option<&T::Doc>,
    ) -> Option<T::Live> {
        let output = self.run_test::<T>(doc);
        if self.test.attrs.should_check_ref(T::OUTPUT) {
            self.check_file_ref::<T>(&output)
        }
        output.map(|(_, live)| live)
    }

    /// Run test for an output format that produces a hashed reference.
    fn run_hash_test<T: HashOutputType>(
        &mut self,
        doc: Option<&T::Doc>,
    ) -> Option<T::Live> {
        let output = self.run_test::<T>(doc);
        if self.test.attrs.should_check_ref(T::OUTPUT) {
            self.check_hash_ref::<T>(&output)
        }
        output.map(|(_, live)| live)
    }

    /// Run test for a specific output format, and save the live output to disk.
    fn run_test<'d, T: OutputType>(
        &mut self,
        doc: Option<&'d T::Doc>,
    ) -> Option<(&'d T::Doc, T::Live)> {
        let live_path = T::OUTPUT.live_path(&self.test.name);

        let output = doc.and_then(|doc| match T::make_live(self.test, doc) {
            Ok(live) => Some((doc, live)),
            Err(errors) => {
                if errors.is_empty() {
                    log!(self, "no document, but also no errors");
                }

                for error in errors.iter() {
                    self.check_diagnostic(NoteKind::Error, error, T::OUTPUT);
                }
                None
            }
        });

        let skippable = match &output {
            Some((doc, live)) => T::is_skippable(doc, live).unwrap_or(true),
            None => false,
        };

        match &output {
            Some((doc, live)) if !skippable => {
                // Convert and save live version.
                let live_data = T::save_live(doc, live);
                std::fs::write(&live_path, live_data).unwrap();
            }
            _ => {
                // Clean live output.
                std::fs::remove_file(&live_path).ok();
            }
        }

        output
    }

    /// Check that the document output matches the existing file reference.
    /// On mismatch, (over-)write or remove the reference if the `--update` flag
    /// is provided.
    fn check_file_ref<T: FileOutputType>(&mut self, output: &Option<(&T::Doc, T::Live)>) {
        let live_path = T::OUTPUT.live_path(&self.test.name);
        let ref_path = T::OUTPUT.file_ref_path(&self.test.name);

        let old_ref_data = std::fs::read(&ref_path);
        let Some((doc, live)) = output else {
            if old_ref_data.is_ok() {
                log!(self, "missing document");
                log!(self, "  ref       | {}", ref_path.display());
            }
            return;
        };

        let skippable = match T::is_skippable(doc, live) {
            Ok(skippable) => skippable,
            Err(()) => {
                log!(self, "document has zero pages");
                return;
            }
        };

        // Tests without visible output and no reference output don't need to be
        // compared.
        if skippable && old_ref_data.is_err() {
            return;
        }

        // Compare against reference output if available.
        // Test that is ok doesn't need to be updated.
        if old_ref_data.as_ref().is_ok_and(|r| T::matches(r, live)) {
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
                let new_ref_data = T::make_ref(live);
                let new_ref_data = new_ref_data.as_ref();
                if !self.test.attrs.large && new_ref_data.len() > crate::REF_LIMIT {
                    log!(self, "reference output would exceed maximum size");
                    log!(self, "  maximum   | {}", FileSize(crate::REF_LIMIT));
                    log!(self, "  size      | {}", FileSize(new_ref_data.len()));
                    log!(
                        self,
                        "please try to minimize the size of the test (smaller pages, less text, etc.)"
                    );
                    log!(
                        self,
                        "if you think the test cannot be reasonably minimized, mark it as `large`"
                    );
                    return;
                }
                std::fs::write(&ref_path, new_ref_data).unwrap();
                log!(
                    into: self.result.infos,
                    "updated reference output ({}, {})",
                    ref_path.display(),
                    FileSize(new_ref_data.len()),
                );
            }
        } else {
            self.result.mismatched_output = true;
            if old_ref_data.is_ok() {
                log!(self, "mismatched output");
                log!(self, "  live      | {}", live_path.display());
                log!(self, "  ref       | {}", ref_path.display());
            } else {
                log!(self, "missing reference output");
                log!(self, "  live      | {}", live_path.display());
            }
        }
    }

    /// Check that the document output matches the existing hashed reference.
    /// On mismatch, (over-)write or remove the reference if the `--update` flag
    /// is provided.
    fn check_hash_ref<T: HashOutputType>(&mut self, output: &Option<(&T::Doc, T::Live)>) {
        let live_path = T::OUTPUT.live_path(&self.test.name);

        let source_path = self.test.source.id().vpath();
        let old_ref_hash =
            if let Some(hashed_refs) = self.hashes[T::INDEX].read().get(source_path) {
                hashed_refs.get(&self.test.name)
            } else {
                let ref_path = T::OUTPUT.hashed_ref_path(source_path.as_rootless_path());
                let string = std::fs::read_to_string(&ref_path).unwrap_or_default();
                let hashed_refs = HashedRefs::from_str(&string)
                    .inspect_err(|e| {
                        log!(self, "error parsing hashed refs: {e}");
                    })
                    .unwrap_or_default();

                let mut hashes = self.hashes[T::INDEX].write();
                let entry = hashes.entry(source_path).insert_entry(hashed_refs);
                entry.get().get(&self.test.name)
            };

        let Some((doc, live)) = output else {
            if old_ref_hash.is_some() {
                log!(self, "missing document");
                log!(self, "  ref       | {}", self.test.name);
            }
            return;
        };

        let skippable = match T::is_skippable(doc, live) {
            Ok(skippable) => skippable,
            Err(()) => {
                log!(self, "document has zero pages");
                return;
            }
        };

        // Tests without visible output and no reference output don't need to be
        // compared.
        if skippable && old_ref_hash.is_none() {
            return;
        }

        // Compare against reference output if available.
        // Test that is ok doesn't need to be updated.
        let new_ref_hash = T::make_hash(live);
        if old_ref_hash.as_ref().is_some_and(|h| *h == new_ref_hash) {
            return;
        }

        if crate::ARGS.update {
            let mut hashes = self.hashes[T::INDEX].write();
            let hashed_refs = hashes.get_mut(source_path).unwrap();
            let ref_path = T::OUTPUT.hashed_ref_path(source_path.as_rootless_path());
            if skippable {
                hashed_refs.remove(&self.test.name);
                log!(
                    into: self.result.infos,
                    "removed reference hash ({})", ref_path.display()
                );
            } else {
                hashed_refs.update(self.test.name.clone(), new_ref_hash);
                log!(
                    into: self.result.infos,
                    "updated reference hash ({}, {new_ref_hash})",
                    ref_path.display(),
                );
            }
        } else {
            self.result.mismatched_output = true;
            if let Some(old_ref_hash) = old_ref_hash {
                log!(self, "mismatched output");
                log!(self, "  live      | {}", live_path.display());
                log!(self, "  old       | {old_ref_hash}");
                log!(self, "  new       | {new_ref_hash}");
            } else {
                log!(self, "missing reference output");
                log!(self, "  live      | {}", live_path.display());
            }
        }
    }

    /// Compare a subset of notes with a given kind against diagnostics of
    /// that same kind.
    fn check_diagnostic(
        &mut self,
        kind: NoteKind,
        diag: &SourceDiagnostic,
        stage: impl TestStage,
    ) {
        // TODO: remove this once HTML export is stable
        if diag.message == "html export is under active development and incomplete" {
            return;
        }

        let range = self.world.range(diag.span);
        self.validate_note(kind, diag.span.id(), range, &diag.message, stage);

        // Check hints.
        for hint in &diag.hints {
            // HACK: This hint only gets emitted in debug builds, so filter it
            // out to make the test suite also pass for release builds.
            if hint.v == "set `RUST_BACKTRACE` to `1` or `full` to capture a backtrace" {
                continue;
            }

            let span = hint.span.or(diag.span);
            let range = self.world.range(span);
            self.validate_note(NoteKind::Hint, span.id(), range, &hint.v, stage);
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
        stage: impl TestStage,
    ) {
        // HACK: Replace backslashes path sepators with slashes for cross
        // platform reproducible error messages.
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new("\\((.*) (at|in) (.+)\\)").unwrap());
        let message = RE.replace(message, |caps: &Captures| {
            let path = caps[3].replace('\\', "/");
            format!("({} {} {})", &caps[1], &caps[2], path)
        });

        // Try to find perfect match.
        let file = file.unwrap_or(self.test.source.id());
        if let Some((i, _)) = self.test.notes.iter().enumerate().find(|&(i, note)| {
            !self.seen[i].contains(stage.into())
                && note.kind == kind
                && note.range == range
                && note.message == message
                && note.file == file
        }) {
            self.seen[i] |= stage.into();
            return;
        }

        // Try to find closely matching annotation. If the note has the same
        // range or message, it's most likely the one we're interested in.
        let Some((i, note)) = self.test.notes.iter().enumerate().find(|&(i, note)| {
            !self.seen[i].contains(stage.into())
                && note.kind == kind
                && (note.range == range || note.message == message)
        }) else {
            // Not even a close match, diagnostic is not annotated.
            let diag_range = self.format_range(file, &range);
            log!(into: self.not_annotated, "  {kind} [{stage}]: {diag_range} {}", message);
            return;
        };

        // Mark this annotation as visited and return it.
        self.seen[i] |= stage.into();

        // Range is wrong.
        if range != note.range {
            let note_range = self.format_range(note.file, &note.range);
            let note_text = self.text_for_range(note.file, &note.range);
            let diag_range = self.format_range(file, &range);
            let diag_text = self.text_for_range(file, &range);
            log!(self, "mismatched range [{stage}] ({}):", note.pos);
            log!(self, "  message   | {}", note.message);
            log!(self, "  annotated | {note_range:<9} | {note_text}");
            log!(self, "  emitted   | {diag_range:<9} | {diag_text}");
        }

        // Message is wrong.
        if message != note.message {
            log!(self, "mismatched message [{stage}] ({}):", note.pos);
            log!(self, "  annotated | {}", note.message);
            log!(self, "  emitted   | {message}");
        }
    }

    /// Display the text for a range.
    fn text_for_range(&self, file: FileId, range: &Option<Range<usize>>) -> String {
        let Some(range) = range else { return "No text".into() };
        if range.is_empty() {
            return "(empty)".into();
        }

        let lines = self.lookup(file);
        lines.text()[range.clone()].replace('\n', "\\n").replace('\r', "\\r")
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
        let lines = self.lookup(file);

        let res = lines.byte_to_line_column(pos).map(|(line, col)| (line + 1, col + 1));
        let Some((line, col)) = res else {
            return "oob".into();
        };

        if line == 1 { format!("{col}") } else { format!("{line}:{col}") }
    }

    #[track_caller]
    fn lookup(&self, file: FileId) -> Lines<String> {
        if self.test.source.id() == file {
            self.test.source.lines().clone()
        } else {
            self.world.lookup(file)
        }
    }
}
