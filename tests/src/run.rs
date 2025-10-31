use std::fmt::Write;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::LazyLock;

use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use typst::diag::{SourceDiagnostic, Warned};
use typst::layout::PagedDocument;
use typst::{Document, WorldExt};
use typst_html::HtmlDocument;
use typst_syntax::{FileId, Lines, VirtualPath};

use crate::collect::{
    FileSize, NoteKind, Test, TestOutput, TestStage, TestStages, TestTarget,
};
use crate::logger::TestResult;
use crate::output::{FileOutputType, HashOutputType, HashedRefs, OutputType};
use crate::world::{TestWorld, system_path};
use crate::{ARGS, REF_PATH, STORE_PATH, SUITE_PATH, custom, output};

type OutputHashes = FxHashMap<&'static VirtualPath, HashedRefs>;

static HASHES: LazyLock<[RwLock<OutputHashes>; 2]> =
    LazyLock::new(|| std::array::from_fn(|_| RwLock::new(FxHashMap::default())));

/// Runs a single test.
///
/// Returns whether the test passed.
pub fn run(test: &Test) -> TestResult {
    Runner::new(test).run()
}

/// Write all hashed references that have been updated
pub fn update_hash_refs<T: HashOutputType>() {
    #[allow(clippy::iter_over_hash_type)]
    for (source_path, hashed_refs) in HASHES[T::INDEX].write().iter_mut() {
        hashed_refs.sort();
        if !hashed_refs.changed {
            continue;
        }

        let ref_path = hashed_ref_path(T::OUTPUT, source_path.as_rootless_path());
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
    test: &'a Test,
    world: TestWorld,
    /// In which targets the note has been seen.
    seen: Vec<TestStages>,
    result: TestResult,
    not_annotated: String,
}

impl<'a> Runner<'a> {
    /// Create a new test runner.
    fn new(test: &'a Test) -> Self {
        Self {
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

        if self.test.attrs.stages.has_paged_target() {
            let doc = self.compile::<PagedDocument>(TestTarget::Paged);

            let errors = custom::check(self.test, &self.world, &doc);
            if !errors.is_empty() {
                log!(self, "custom check failed");
                for line in errors.lines() {
                    log!(self, "  {line}");
                }
            }

            self.run_file_test::<output::Render>(&doc, ARGS.render());
            self.run_hash_test::<output::Pdf>(&doc, ARGS.pdf());
            self.run_file_test::<output::Pdftags>(&doc, false);
            self.run_hash_test::<output::Svg>(&doc, ARGS.svg());
        }
        if self.test.attrs.stages.has_html_target() {
            let doc = self.compile::<HtmlDocument>(TestTarget::Html);
            self.run_file_test::<output::Html>(&doc, false);
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
            if !seen.is_empty() {
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

    fn live_path<T: OutputType>(&self) -> PathBuf {
        let dir = T::OUTPUT.sub_dir();
        let name = &self.test.name;
        let ext = T::OUTPUT.extension();
        PathBuf::from(format!("{STORE_PATH}/{dir}/{name}.{ext}"))
    }

    fn file_ref_path<T: OutputType>(&self) -> PathBuf {
        let dir = T::OUTPUT.sub_dir();
        let name = &self.test.name;
        let ext = T::OUTPUT.extension();
        PathBuf::from(format!("{REF_PATH}/{dir}/{name}.{ext}"))
    }

    fn run_file_test<T: FileOutputType>(
        &mut self,
        doc: &Option<T::Doc>,
        save_live: bool,
    ) {
        if !(save_live || self.test.attrs.stages.contains(T::OUTPUT.into())) {
            return;
        }

        let output = self.run_test::<T>(doc);
        if self.test.attrs.save_ref(T::OUTPUT) {
            self.check_file_ref::<T>(output)
        }
    }

    fn run_hash_test<T: HashOutputType>(
        &mut self,
        doc: &Option<T::Doc>,
        save_live: bool,
    ) {
        if !(save_live || self.test.attrs.stages.contains(T::OUTPUT.into())) {
            return;
        }

        let output = self.run_test::<T>(doc);
        if self.test.attrs.save_ref(T::OUTPUT) {
            self.check_hash_ref::<T>(output)
        }
    }

    /// Run test specific to document format.
    fn run_test<T: OutputType>(
        &mut self,
        doc: &Option<T::Doc>,
    ) -> Option<(T::Doc, T::Live)> {
        let live_path = self.live_path::<T>();

        let output =
            doc.clone()
                .and_then(|mut doc| match T::make_live(self.test, &mut doc) {
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

        match &output {
            Some((doc, live)) => {
                // Convert and save live version.
                let live_data = T::save_live(doc, live);
                std::fs::write(&live_path, live_data).unwrap();
            }
            None => {
                // Clean live output.
                std::fs::remove_file(&live_path).ok();
            }
        }

        output
    }

    /// Check that the document output is correct.
    fn check_file_ref<T: FileOutputType>(&mut self, output: Option<(T::Doc, T::Live)>) {
        let live_path = self.live_path::<T>();
        let ref_path = self.file_ref_path::<T>();

        let old_ref_data = std::fs::read(&ref_path);
        let Some((doc, live)) = output else {
            if old_ref_data.is_ok() {
                log!(self, "missing document");
                log!(self, "  ref       | {}", ref_path.display());
            }
            return;
        };

        let skippable = match T::is_skippable(&doc, &live) {
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
        if old_ref_data.as_ref().is_ok_and(|r| T::matches(r, &live)) {
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
                let new_ref_data = T::make_ref(&live);
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

    /// Check that the document output is correct.
    fn check_hash_ref<T: HashOutputType>(&mut self, output: Option<(T::Doc, T::Live)>) {
        let live_path = self.live_path::<T>();

        let source_path = self.test.source.id().vpath();
        let old_ref_hash =
            if let Some(hashed_refs) = HASHES[T::INDEX].read().get(source_path) {
                hashed_refs.get(&self.test.name)
            } else {
                let ref_path = hashed_ref_path(T::OUTPUT, source_path.as_rootless_path());
                let string = std::fs::read_to_string(&ref_path).unwrap_or_default();
                let hashed_refs = HashedRefs::from_str(&string)
                    .inspect_err(|e| {
                        log!(self, "error parsing hashed refs: {e}");
                    })
                    .unwrap_or_default();

                let mut hashes = HASHES[T::INDEX].write();
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

        let skippable = match T::is_skippable(&doc, &live) {
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
        let new_ref_hash = T::make_hash(&live);
        if old_ref_hash.as_ref().is_some_and(|h| *h == new_ref_hash) {
            return;
        }

        if crate::ARGS.update {
            let mut hashes = HASHES[T::INDEX].write();
            let hashed_refs = hashes.get_mut(source_path).unwrap();
            let ref_path = hashed_ref_path(T::OUTPUT, source_path.as_rootless_path());
            if skippable {
                hashed_refs.remove(&self.test.name);
                log!(
                    into: self.result.infos,
                    "removed reference hash ({})", ref_path.display()
                );
            } else {
                eprintln!("update");
                hashed_refs.update(self.test.name.clone(), new_ref_hash);
                eprintln!("1");
                log!(
                    into: self.result.infos,
                    "updated reference hash ({}, {new_ref_hash})",
                    ref_path.display(),
                );
            }
            eprintln!("updated");
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

        let message = if diag.message.contains("\\u{") {
            &diag.message
        } else {
            &diag.message.replace("\\", "/")
        };
        let range = self.world.range(diag.span);
        self.validate_note(kind, diag.span.id(), range.clone(), message, stage);

        // Check hints.
        for hint in &diag.hints {
            self.validate_note(
                NoteKind::Hint,
                diag.span.id(),
                range.clone(),
                hint,
                stage,
            );
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
            log!(into: self.not_annotated, "  ({stage}) {kind}: {diag_range} {}", message);
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

pub fn hashed_ref_path(output: TestOutput, source_path: &Path) -> PathBuf {
    let sub_dir = output.sub_dir();
    let sub_path = source_path.strip_prefix(SUITE_PATH).unwrap();
    let trimmed_path = sub_path.to_str().unwrap().strip_suffix(".typ");
    let file_name = trimmed_path.unwrap().replace("/", "-");
    PathBuf::from(format!("{REF_PATH}/{sub_dir}/{file_name}.txt"))
}
