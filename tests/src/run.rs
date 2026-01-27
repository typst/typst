use std::fmt::Write;
use std::ops::Range;
use std::path::Path;
use std::sync::LazyLock;

use parking_lot::RwLock;
use regex::{Captures, Regex};
use typst::WorldExt;
use typst::diag::{SourceDiagnostic, Warned};
use typst::foundations::{Content, Repr};
use typst::layout::PagedDocument;
use typst_html::HtmlDocument;
use typst_syntax::FileId;

use crate::collect::{
    FileSize, NoteKind, Test, TestEval, TestOutput, TestOutputKind, TestStage,
    TestStages, TestTarget,
};
use crate::logger::TestResult;
use crate::output::{
    FileOutputType, HashOutputType, HashedRefs, OutputType, TestDocument,
};
use crate::world::{TestFiles, TestWorld};
use crate::{ARGS, STORE_PATH, custom, output};

/// Runs a single test.
///
/// Returns whether the test passed.
pub fn run(hashes: &[RwLock<HashedRefs>], test: &Test) -> TestResult {
    Runner::new(hashes, test).run()
}

/// Write all hashed references that have been updated
pub fn update_hash_refs<T: HashOutputType>(hashes: &[RwLock<HashedRefs>]) {
    let mut hashed_refs = hashes[T::INDEX].write();
    hashed_refs.sort();

    let ref_path = T::OUTPUT.hash_refs_path();
    if hashed_refs.is_empty() {
        std::fs::remove_file(ref_path).ok();
    } else {
        std::fs::write(ref_path, hashed_refs.to_string()).unwrap();
    }
}

/// Write a line to a log sink, defaulting to the test's error log.
macro_rules! log {
    (into: $sink:expr, $($tts:tt)*) => {{
        writeln!($sink, $($tts)*).unwrap();
    }};
    ($runner:expr, $($tts:tt)*) => {{
        writeln!(&mut $runner.result.errors, $($tts)*).unwrap();
    }};
}

/// Runs a single test.
pub struct Runner<'a> {
    hashes: &'a [RwLock<HashedRefs>],
    test: &'a Test,
    world: TestWorld,
    /// In which targets the note has been seen.
    seen: Vec<TestStages>,
    result: TestResult,
    not_annotated: String,
    unexpected_empty: UnexpectedEmpty,
    unexpected_non_empty: UnexpectedNonEmpty,
}

/// The test unexpectedly produced non-empty content or output.
///
/// The variants are ordered by relevance from low to high.
enum UnexpectedNonEmpty {
    None,
    Eval(Content),
    Output(TestStages),
}

impl UnexpectedNonEmpty {
    fn eval(&mut self, content: Content) {
        match self {
            Self::None | Self::Eval(_) => {
                *self = Self::Eval(content);
            }
            Self::Output(_) => (),
        }
    }

    fn output(&mut self, output: TestOutput) {
        match self {
            Self::None | Self::Eval(_) => {
                *self = Self::Output(output.into());
            }
            Self::Output(stages) => {
                *stages |= output.into();
            }
        }
    }
}

/// The test unexpectedly produced empty content or output.
///
/// The variants are ordered by relevance from low to high.
enum UnexpectedEmpty {
    None,
    Output(TestStages),
    Eval,
}

impl UnexpectedEmpty {
    fn eval(&mut self) {
        *self = Self::Eval;
    }

    fn output(&mut self, output: TestOutput) {
        match self {
            Self::None => {
                *self = Self::Output(output.into());
            }
            Self::Output(stages) => {
                *stages |= output.into();
            }
            Self::Eval => (),
        }
    }
}

impl<'a> Runner<'a> {
    /// Create a new test runner.
    fn new(hashes: &'a [RwLock<HashedRefs>], test: &'a Test) -> Self {
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
            unexpected_empty: UnexpectedEmpty::None,
            unexpected_non_empty: UnexpectedNonEmpty::None,
        }
    }

    /// Run the test.
    fn run(mut self) -> TestResult {
        if crate::ARGS.syntax {
            log!(into: self.result.infos, "tree: {:#?}", self.test.source.root());
        }

        if let Some(content) = self.eval() {
            if self.test.attrs.parsed_stages().contains(TestStages::EVAL) {
                if !output::is_empty_content(&content) {
                    self.unexpected_non_empty.eval(content.clone());
                }
            } else if output::is_empty_content(&content) {
                self.unexpected_empty.eval();
            }
        }

        // Only compile paged document when the paged target is explicitly
        // specified or required by paged outputs.
        if self.test.should_run(TestTarget::Paged) {
            let mut doc = self.compile::<PagedDocument>();
            let errors = custom::check(self.test, &self.world, doc.as_ref());
            if !errors.is_empty() {
                log!(self, "custom check failed");
                for line in errors.lines() {
                    log!(self, "  {line}");
                }
            }

            if let Some(doc) = &mut doc
                && doc.info.title.is_none()
            {
                doc.info.title = Some(self.test.name.clone());
            }

            if self.test.should_run(TestOutput::Render) {
                self.run_file_test::<output::Render>(doc.as_ref());
            }
            if self.test.should_run(TestOutput::Pdf) {
                let pdf = self.run_hash_test::<output::Pdf>(doc.as_ref());
                if self.test.should_run(TestOutput::Pdftags) {
                    self.run_file_test::<output::Pdftags>(pdf.as_ref());
                }
            }
            if self.test.should_run(TestOutput::Svg) {
                self.run_hash_test::<output::Svg>(doc.as_ref());
            }
        }

        // Only compile html document when the html target is specified.
        if self.test.should_run(TestTarget::Html) {
            let doc = self.compile::<HtmlDocument>();
            self.run_file_test::<output::Html>(doc.as_ref());
        }

        self.handle_empty();

        self.handle_not_emitted();
        self.handle_not_annotated();

        self.result
    }

    fn handle_empty(&mut self) {
        match &self.unexpected_non_empty {
            UnexpectedNonEmpty::None => (),
            UnexpectedNonEmpty::Eval(content) => {
                log!(self, "[eval] test produced non-empty content: {}", content.repr());
                log!(
                    self,
                    "  hint: consider making this a `paged empty` or `html empty` test"
                );
            }
            UnexpectedNonEmpty::Output(stages) => {
                log!(
                    self,
                    "[{}] test produced non-empty output for [{stages}]",
                    self.test.attrs.implied_stages()
                );
                log!(self, "  hint: consider removing the `empty` attribute");
            }
        }

        match self.unexpected_empty {
            UnexpectedEmpty::None => (),
            UnexpectedEmpty::Eval => {
                log!(
                    self,
                    "[{}] test produced empty content",
                    self.test.attrs.implied_stages()
                );
                log!(self, "  hint: consider making this an `eval` test");
            }
            UnexpectedEmpty::Output(stages) => {
                log!(
                    self,
                    "[{}] test produced empty output for [{stages}]",
                    self.test.attrs.implied_stages()
                );
                log!(self, "  hint: consider adding the `empty` attribute");
            }
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
        for (note, &seen) in self.test.notes.iter().zip(&self.seen) {
            let possible = self.test.attrs.implied_stages() & ARGS.required_stages();
            if seen.is_empty() && !possible.is_empty() {
                log!(self, "not emitted");
                let note_range = self.format_range(note.file, &note.range);
                log!(
                    self,
                    "  {}: {note_range} {} ({})",
                    note.kind,
                    note.message,
                    note.pos
                );
                continue;
            }

            // Figure out if the diagnostic is only emitted in a specific in a
            // specific output/target that isn't hit by all possible branches of
            // the stage tree.
            // See the doc comment on `TestStages` for an overview.
            let attr_stages = self.test.attrs.implied_stages() & ARGS.required_stages();
            let full_branch_coverage =
                attr_stages.iter().all(|s| s.with_required().intersects(seen));
            if full_branch_coverage {
                continue;
            }

            let siblings = seen.with_siblings()
                & self.test.attrs.implied_stages()
                & ARGS.required_stages();
            log!(
                self,
                "only emitted in [{seen}] but expected in [{siblings}], \
                 consider narrowing the test attributes",
            );

            let note_range = self.format_range(note.file, &note.range);
            log!(self, "  {}: {note_range} {} ({})", note.kind, note.message, note.pos);
        }
    }

    /// Evaluate document content, this is the target agnostic part of compilation.
    fn eval(&mut self) -> Option<Content> {
        let Warned { output, warnings } = eval::eval(&self.world);
        for warning in &warnings {
            self.check_diagnostic(NoteKind::Warning, warning, TestEval);
        }

        if let Err(errors) = &output {
            for error in errors.iter() {
                self.check_diagnostic(NoteKind::Error, error, TestEval);
            }
        }

        output.ok()
    }

    /// Compile a document with the specified target.
    fn compile<D: TestDocument>(&mut self) -> Option<D> {
        let Warned { output, warnings } = typst::compile::<D>(&self.world);
        for warning in &warnings {
            self.check_diagnostic(NoteKind::Warning, warning, D::TARGET);
        }

        if let Err(errors) = &output {
            for error in errors.iter() {
                self.check_diagnostic(NoteKind::Error, error, D::TARGET);
            }
        }

        output.ok()
    }

    /// Run test for an output format that produces a file reference.
    fn run_file_test<T: FileOutputType>(
        &mut self,
        doc: Option<&T::Doc>,
    ) -> Option<T::Live> {
        let live = self.run_test::<T>(doc);
        let output = doc.zip(live.as_ref());
        self.save_live::<T>(output);
        if self.test.should_check(T::OUTPUT) {
            self.check_file_ref::<T>(output)
        }
        live
    }

    /// sun test for an output format that produces a hashed reference.
    fn run_hash_test<T: HashOutputType>(
        &mut self,
        doc: Option<&T::Doc>,
    ) -> Option<T::Live> {
        let live = self.run_test::<T>(doc);
        let output = doc.zip(live.as_ref());
        self.save_live::<T>(output);
        if self.test.should_check(T::OUTPUT) {
            self.check_hash_ref::<T>(output)
        }
        live
    }

    /// Run test for a specific output format, and save the live output to disk.
    fn run_test<T: OutputType>(&mut self, doc: Option<&T::Doc>) -> Option<T::Live> {
        let doc = doc?;
        let live = T::make_live(self.test, doc);

        if let Err(errors) = &live {
            if errors.is_empty() {
                log!(self, "no document, but also no errors");
            }

            for error in errors.iter() {
                self.check_diagnostic(NoteKind::Error, error, T::OUTPUT);
            }
        }

        live.ok()
    }

    fn save_live<T: OutputType>(&self, output: Option<(&T::Doc, &T::Live)>) {
        let live_path = T::OUTPUT.live_path(&self.test.name);
        match output {
            Some((doc, live)) if !T::is_empty(doc, live) => {
                // Convert and save live version.
                let live_data = T::save_live(doc, live);

                match T::OUTPUT.kind() {
                    TestOutputKind::File => {
                        std::fs::write(&live_path, live_data).unwrap();
                    }
                    TestOutputKind::Hash(_) => {
                        // Write the file to a path of its hash.
                        let hash = T::make_hash(live);
                        let hash_path = T::OUTPUT.hash_path(hash, &self.test.name);
                        std::fs::create_dir_all(hash_path.parent().unwrap()).unwrap();
                        std::fs::write(&hash_path, live_data).unwrap();

                        // Create a link in the store directory.
                        std::fs::remove_file(&live_path).ok();

                        let relative_path = hash_path.strip_prefix(STORE_PATH).unwrap();
                        let link_path = Path::new("..").join(relative_path);

                        #[cfg(target_family = "unix")]
                        std::os::unix::fs::symlink(&link_path, &live_path).unwrap();
                        #[cfg(target_family = "windows")]
                        std::os::windows::fs::symlink_file(&link_path, &live_path)
                            .unwrap();
                    }
                }
            }
            _ => {
                // Clean live output.
                std::fs::remove_file(&live_path).ok();
            }
        }
    }

    /// Check that the document output matches the existing file reference.
    /// On mismatch, (over-)write or remove the reference if the `--update` flag
    /// is provided.
    fn check_file_ref<T: FileOutputType>(&mut self, output: Option<(&T::Doc, &T::Live)>) {
        let live_path = T::OUTPUT.live_path(&self.test.name);
        let ref_path = T::OUTPUT.file_ref_path(&self.test.name);

        let old_ref_data = std::fs::read(&ref_path).ok();

        let live = match self.expect_output::<T>(output) {
            Ok(non_empty) => match non_empty.and(output) {
                Some((_, live)) => live,
                None => return,
            },
            Err(()) => {
                self.result.mismatched_output = true;
                return;
            }
        };

        // Happy path: output is ok and doesn't need to be updated.
        if old_ref_data.as_ref().is_some_and(|r| T::matches(r, live)) {
            return;
        }

        let new_ref_data = T::save_ref(live);
        let new_ref_data = new_ref_data.as_ref();
        if crate::ARGS.update {
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
        } else {
            self.result.mismatched_output = true;
            if old_ref_data.is_some() {
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
    fn check_hash_ref<T: HashOutputType>(&mut self, output: Option<(&T::Doc, &T::Live)>) {
        let live_path = T::OUTPUT.live_path(&self.test.name);
        let old_hash = self.hashes[T::INDEX].read().get(&self.test.name);

        let live = match self.expect_output::<T>(output) {
            Ok(non_empty) => match non_empty.and(output) {
                Some((_, live)) => live,
                None => return,
            },
            Err(()) => {
                self.result.mismatched_output = true;
                return;
            }
        };

        // Happy path: output is ok and doesn't need to be updated.
        let new_hash = T::make_hash(live);
        if old_hash.as_ref().is_some_and(|h| *h == new_hash) {
            return;
        }

        if crate::ARGS.update {
            let mut hashed_refs = self.hashes[T::INDEX].write();
            let ref_path = T::OUTPUT.hash_refs_path();
            hashed_refs.update(self.test.name.clone(), new_hash);
            log!(
                into: self.result.infos,
                "updated reference hash ({}, {new_hash})",
                ref_path.display(),
            );
        } else {
            self.result.mismatched_output = true;
            if let Some(old_hash) = old_hash {
                log!(self, "mismatched output");
                log!(self, "  live      | {}", live_path.display());
                log!(self, "  old       | {old_hash}");
                log!(self, "  new       | {new_hash}");
            } else {
                log!(self, "missing reference hash");
                log!(self, "  live      | {}", live_path.display());
                log!(self, "  new       | {new_hash}");
            }
        }
    }

    /// Check if the output matches what the attributes and test annotations
    /// expect.
    /// The `Ok` case returns whether an expected output is present and
    /// should be compared to a reference.
    fn expect_output<T: OutputType>(
        &mut self,
        output: Option<(&T::Doc, &T::Live)>,
    ) -> Result<Option<()>, ()> {
        let Some((doc, live)) = output else {
            if !self.test.should_error() {
                log!(self, "missing output [{}]", T::OUTPUT);
                return Err(());
            }
            return Ok(None);
        };

        if self.test.attrs.empty {
            if !T::is_empty(doc, live) {
                self.unexpected_non_empty.output(T::OUTPUT);
                return Err(());
            }
            return Ok(None);
        } else if T::is_empty(doc, live) {
            self.unexpected_empty.output(T::OUTPUT);
            return Err(());
        }

        Ok(Some(()))
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

        let lines = self.world.lines(file).unwrap();
        lines.text()[range.clone()].replace('\n', "\\n").replace('\r', "\\r")
    }

    /// Display a byte range as a line:column range.
    fn format_range(&self, file: FileId, range: &Option<Range<usize>>) -> String {
        let Some(range) = range else { return "No range".into() };

        let mut preamble = String::new();
        if file != self.test.source.id() {
            preamble = format!("\"{}\" ", TestFiles.resolve(file).display());
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
        let lines = self.world.lines(file).unwrap();

        let res = lines.byte_to_line_column(pos).map(|(line, col)| (line + 1, col + 1));
        let Some((line, col)) = res else {
            return "oob".into();
        };

        if line == 1 { format!("{col}") } else { format!("{line}:{col}") }
    }
}

/// A bunch of copy pasted code from the `typst` crate, so we don't have to
/// change the public API.
mod eval {
    use comemo::{Track, Tracked};
    use ecow::EcoVec;
    use rustc_hash::FxHashSet;
    use typst::diag::{SourceDiagnostic, SourceResult, Warned};
    use typst::engine::{Route, Sink, Traced};
    use typst::foundations::Content;
    use typst::{ROUTINES, World};

    pub fn eval(world: &dyn World) -> Warned<SourceResult<Content>> {
        let mut sink = Sink::new();
        let output = eval_impl(world.track(), Traced::default().track(), &mut sink)
            .map_err(deduplicate);

        Warned { output, warnings: sink.warnings() }
    }

    fn eval_impl(
        world: Tracked<dyn World + '_>,
        traced: Tracked<Traced>,
        sink: &mut Sink,
    ) -> SourceResult<Content> {
        // Fetch the main source file once.
        let main = world.main();
        let main = world.source(main).expect("valid main file");

        // First evaluate the main source file into a module.
        let content = typst_eval::eval(
            &ROUTINES,
            world,
            traced,
            sink.track_mut(),
            Route::default().track(),
            &main,
        )?
        .content();

        Ok(content)
    }

    /// Deduplicate diagnostics.
    fn deduplicate(mut diags: EcoVec<SourceDiagnostic>) -> EcoVec<SourceDiagnostic> {
        let mut unique = FxHashSet::default();
        diags.retain(|diag| {
            let hash = typst_utils::hash128(&(&diag.span, &diag.message));
            unique.insert(hash)
        });
        diags
    }
}
