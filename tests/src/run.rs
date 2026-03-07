use std::fmt::Write;
use std::path::Path;

use parking_lot::RwLock;
use typst::diag::{SourceDiagnostic, SourceResult, Warned};
use typst::foundations::{Content, Repr};
use typst::layout::PagedDocument;
use typst_html::HtmlDocument;
use typst_syntax::Spanned;

use crate::collect::{
    FileSize, Test, TestEval, TestOutput, TestOutputKind, TestStage, TestStages,
    TestTarget,
};
use crate::logger::TestResult;
use crate::notes::{Note, NoteKind, NoteStatus};
use crate::output::{
    FileOutputType, HashOutputType, HashedRef, HashedRefs, OutputType, TestDocument,
};
use crate::report::{Old, ReportFile};
use crate::world::TestWorld;
use crate::{ARGS, STORE_PATH, custom, git, output};

/// Runs a single test.
///
/// Returns whether the test passed.
pub fn run(hashes: &[RwLock<HashedRefs>], test: &mut Test) -> TestResult {
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
    test: &'a mut Test,
    world: TestWorld,
    result: TestResult,
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
    Eval { error: bool },
}

impl UnexpectedEmpty {
    fn eval(&mut self, error: bool) {
        *self = Self::Eval { error };
    }

    fn output(&mut self, output: TestOutput) {
        match self {
            Self::None => {
                *self = Self::Output(output.into());
            }
            Self::Output(stages) => {
                *stages |= output.into();
            }
            Self::Eval { .. } => (),
        }
    }
}

impl<'a> Runner<'a> {
    /// Create a new test runner.
    fn new(hashes: &'a [RwLock<HashedRefs>], test: &'a mut Test) -> Self {
        let world = TestWorld::new(test.body.source.clone());
        Self {
            hashes,
            test,
            world,
            result: TestResult::default(),
            unexpected_empty: UnexpectedEmpty::None,
            unexpected_non_empty: UnexpectedNonEmpty::None,
        }
    }

    /// Run the test.
    fn run(mut self) -> TestResult {
        if ARGS.syntax {
            log!(into: self.result.infos, "tree: {:#?}", self.test.body.source.root());
        }

        // Unconditionally eval the document to check for empty/non-empty
        // content. This result is cached, so calling compile below won't
        // duplicate any work.
        let evaluated = self.eval();
        if self.test.attrs.parsed_stages().contains(TestStages::EVAL) {
            // Enforce that `eval` tests produce empty content. Otherwise there
            // might be code inside the content that will only be executed
            // during layout/realization.
            if let Ok(content) = &evaluated.output
                && !output::is_empty_content(content)
            {
                self.unexpected_non_empty.eval(content.clone());
            }
        } else {
            // Enforce that tests which don't have the `eval` attribute produce
            // non-empty content and don't error in the `eval` stage.
            if evaluated.output.as_ref().is_ok_and(output::is_empty_content)
                || evaluated.output.is_err()
            {
                self.unexpected_empty.eval(evaluated.output.is_err());
            }
        }

        // Only compile paged document when the paged target is explicitly
        // specified or required by paged outputs.
        if self.test.should_run(TestTarget::Paged) {
            let mut doc = self.compile::<PagedDocument>(evaluated.clone());
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
            if self.test.should_run(TestOutput::Svg) {
                self.run_hash_test::<output::Svg>(doc.as_ref());
            }
            if self.test.should_run(TestOutput::Pdf) {
                let pdf = self.run_hash_test::<output::Pdf>(doc.as_ref());
                if self.test.should_run(TestOutput::Pdftags) {
                    self.run_file_test::<output::Pdftags>(pdf.as_ref());
                }
            }
        }

        // Only compile html document when the html target is specified.
        if self.test.should_run(TestTarget::Html) {
            let doc = self.compile::<HtmlDocument>(evaluated);
            self.run_file_test::<output::Html>(doc.as_ref());
        }

        self.handle_empty();
        self.handle_annotations();

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
            UnexpectedEmpty::Eval { error } => {
                if error {
                    log!(
                        self,
                        "[{}] test errored in the [eval] stage",
                        self.test.attrs.implied_stages()
                    );
                } else {
                    log!(
                        self,
                        "[{}] test produced empty content",
                        self.test.attrs.implied_stages()
                    );
                }
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

    /// Handle error/warning annotation issues.
    fn handle_annotations(&mut self) {
        let mut needs_update = false;
        let mut inconsistent_stages = false;
        let mut consistent_set = TestStages::all();

        for Note { status, seen, kind, range, message } in self.test.body.notes.iter() {
            // Set `needs_update` in one place for clarity.
            needs_update |= match &status {
                NoteStatus::Annotated { .. } => seen.is_empty(),
                NoteStatus::Updated { .. } => true,
                NoteStatus::Emitted => true,
            };

            if seen.is_empty() {
                let NoteStatus::Annotated { pos } = &status else { unreachable!() };
                if !ARGS.update {
                    log!(self, "not emitted");
                    log!(self, "  {kind}: {range} {message} ({pos})");
                }
                continue;
            }

            // Even if a diagnostic is emitted and annotated, it may only have
            // been emitted in a subset of the ran stages. This may be an error,
            // but only if that subset isn't "covered" by each ran stage.
            // For example, if we have a `paged html` test that sees an error
            // only in `eval`, this is fine because `eval` is covered by both
            // `paged` and `html`. But we do not support errors occuring in
            // only one of `paged` or `html` when both are given.
            let ran_stages = self.test.attrs.implied_stages() & ARGS.required_stages();
            // Whether the seen stages are implied by all ran stages.
            let fully_covered =
                ran_stages.iter().all(|s| s.with_required().intersects(*seen));
            if !fully_covered {
                consistent_set &= *seen;
                inconsistent_stages = true;
                let siblings = ran_stages & seen.with_siblings();
                log!(self, "only emitted in [{seen}] but expected in [{siblings}]");
            }

            if ARGS.update && fully_covered {
                continue;
            }

            // Log errors with the annotated vs. emitted diagnostics.
            match &status {
                NoteStatus::Emitted => {
                    log!(self, "not annotated");
                    // The `#` ensures the range includes line numbers.
                    log!(self, "  {kind}: {range:#} {message}");
                }
                NoteStatus::Annotated { pos } | NoteStatus::Updated { pos, .. }
                    if !fully_covered =>
                {
                    // Just print the annotation if not fully covered.
                    log!(self, "  {kind}: {range} {message} ({pos})");
                }
                NoteStatus::Annotated { .. } => {} // Annotated and emitted!
                NoteStatus::Updated { pos, annotated } => {
                    let (anot_kind, anot_range, anot_message) = &**annotated;
                    // Kind is wrong.
                    if anot_kind != kind {
                        log!(self, "mismatched error kind ({pos}):");
                        log!(self, "  annotated | `{anot_kind}`");
                        log!(self, "  emitted   | `{kind}`");
                    }
                    // Range is wrong.
                    if let Some((anot_r, emit_r)) = anot_range.diff(range) {
                        log!(self, "mismatched range ({pos}):");
                        if anot_message == message {
                            log!(self, "  message   | {anot_kind}: {anot_message}");
                        }
                        let pad = 10.max(anot_r.len()).max(emit_r.len());
                        let anot_text = anot_range.text();
                        log!(self, "  annotated | {anot_r:<pad$} | {}", anot_text);
                        log!(self, "  emitted   | {emit_r:<pad$} | {}", range.text());
                    }
                    // Message is wrong.
                    if anot_message != message {
                        log!(self, "mismatched message ({pos}):");
                        log!(self, "  annotated | {anot_message}");
                        log!(self, "  emitted   | {message}");
                    }
                }
            }
        }

        if needs_update {
            if ARGS.update && inconsistent_stages {
                // We can't update notes if they were emitted inconsistently.
                log!(self, "unable to update test annotations");
            } else if ARGS.update {
                let (new_body, note_stats) = self.test.body.write_seen_annotations();
                self.result.updated_body = Some(new_body);

                let stats = note_stats
                    .into_iter()
                    .filter(|(_, count)| *count > 0)
                    .map(|(verb, count)| format!("{verb} {count}"))
                    .collect::<Vec<String>>()
                    .join(", ");

                // We won't actually update until we've finished running every
                // test, but it's not really worth explaining.
                log!(into: self.result.infos, "updated test annotations ({stats})");
            } else {
                self.result.mismatched_output = true;
            }
        }

        if inconsistent_stages {
            log!(self, "errors/warnings were emitted differently across multiple stages");
            if consistent_set.is_empty() {
                log!(self, "consider moving to multiple tests with different stages");
            } else {
                // This will probably be Eval if anything.
                log!(self, "consider changing the test to only {consistent_set}");
            }
        }
    }

    /// Evaluate document content, this is the target agnostic part of compilation.
    fn eval(&mut self) -> Warned<SourceResult<Content>> {
        let evaluated = eval::eval(&self.world);

        let Warned { output, warnings } = &evaluated;
        for warning in warnings {
            self.check_diagnostic(NoteKind::Warning, warning, TestEval);
        }

        if let Err(errors) = output {
            for error in errors.iter() {
                self.check_diagnostic(NoteKind::Error, error, TestEval);
            }
        }

        evaluated
    }

    /// Compile a document with the specified target.
    ///
    /// Conceptually, this function takes the evaluated content as input and
    /// produces a document. In practice it also re-evaluates the sources and
    /// thus generates duplicate diagnostics for the eval stage, so we filter
    /// those out.
    fn compile<D: TestDocument>(
        &mut self,
        evaluated: Warned<SourceResult<Content>>,
    ) -> Option<D> {
        let Warned { output, warnings } = typst::compile::<D>(&self.world);

        let warnings = eval::deduplicate_with(warnings, &evaluated.warnings);
        for warning in warnings.iter() {
            self.check_diagnostic(NoteKind::Warning, warning, D::TARGET);
        }

        match output {
            Ok(output) => Some(output),
            Err(errors) => {
                let eval_errors = (evaluated.output.as_ref().err())
                    .map(|errors| errors.as_slice())
                    .unwrap_or(&[]);
                let errors = eval::deduplicate_with(errors, eval_errors);

                for error in errors.iter() {
                    self.check_diagnostic(NoteKind::Error, error, D::TARGET);
                }

                None
            }
        }
    }

    /// Run test for an output format that produces a file reference.
    fn run_file_test<T: FileOutputType>(
        &mut self,
        doc: Option<&T::Doc>,
    ) -> Option<T::Live> {
        let live = self.run_test::<T>(doc);
        {
            let output = doc.zip(live.as_ref());
            let live_data = self.save_live::<T>(output);
            if self.test.should_check(T::OUTPUT) {
                let output = output.and_then(|(doc, live)| Some((doc, live, live_data?)));
                self.check_file_ref::<T>(output)
            }
        }
        live
    }

    /// sun test for an output format that produces a hashed reference.
    fn run_hash_test<T: HashOutputType>(
        &mut self,
        doc: Option<&T::Doc>,
    ) -> Option<T::Live> {
        let live = self.run_test::<T>(doc);
        {
            let output = doc.zip(live.as_ref());
            let live_data = self.save_live::<T>(output);
            if self.test.should_check(T::OUTPUT) {
                let output = output.and_then(|(doc, live)| Some((doc, live, live_data?)));
                self.check_hash_ref::<T>(output);
            }
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

    fn save_live<'d, T: OutputType>(
        &self,
        output: Option<(&'d T::Doc, &'d T::Live)>,
    ) -> Option<impl AsRef<[u8]> + use<'d, T>> {
        let live_path = T::OUTPUT.live_path(&self.test.name);

        // Convert live output, so it can be written to disk.
        let live_data = output.map(|(doc, live)| T::save_live(doc, live));

        match (output, &live_data) {
            (Some((doc, live)), Some(live_data)) if !T::is_empty(doc, live) => {
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

        live_data
    }

    /// Check that the document output matches the existing file reference.
    /// On mismatch, (over-)write or remove the reference if the `--update` flag
    /// is provided.
    fn check_file_ref<T: FileOutputType>(
        &mut self,
        output: Option<(&T::Doc, &T::Live, impl AsRef<[u8]>)>,
    ) {
        let live_path = T::OUTPUT.live_path(&self.test.name);
        let ref_path = T::OUTPUT.file_ref_path(&self.test.name);

        let old_ref_data = read_ref_data(&ref_path);

        let live = match self.expect_output::<T>(&output) {
            Ok(non_empty) => match non_empty.and(output) {
                Some((_, live, _)) => live,
                None => return,
            },
            Err(()) => {
                self.result.mismatched_output = true;

                if ARGS.gen_report() {
                    let old = old_ref_data.map(|data| (ref_path, Old::Data(data)));
                    let new = output.map(|(_, _, data)| (live_path, data));
                    let file_report = make_report::<T>(old, new);
                    self.result.add_report(self.test.name.clone(), file_report);
                }
                return;
            }
        };

        // Happy path: output is ok and doesn't need to be updated.
        if old_ref_data.as_ref().is_some_and(|r| T::matches(r, live)) {
            return;
        }

        let new_ref_data = T::save_ref(live);
        let new_ref_data = new_ref_data.as_ref();
        if ARGS.update {
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
            let old = if let Some(old_ref_data) = old_ref_data {
                log!(self, "mismatched output");
                log!(self, "  live      | {}", live_path.display());
                log!(self, "  ref       | {}", ref_path.display());

                Some((ref_path, Old::Data(old_ref_data)))
            } else {
                log!(self, "missing reference output");
                log!(self, "  live      | {}", live_path.display());

                None
            };

            if ARGS.gen_report() {
                let file_report = make_report::<T>(old, Some((live_path, new_ref_data)));
                self.result.add_report(self.test.name.clone(), file_report);
            }
        }
    }

    /// Check that the document output matches the existing hashed reference.
    /// On mismatch, (over-)write or remove the reference if the `--update` flag
    /// is provided.
    fn check_hash_ref<T: HashOutputType>(
        &mut self,
        output: Option<(&T::Doc, &T::Live, impl AsRef<[u8]>)>,
    ) {
        let live_path = T::OUTPUT.live_path(&self.test.name);
        let old_hash = self.hashes[T::INDEX].read().get(&self.test.name);

        let (live, live_data) = match self.expect_output::<T>(&output) {
            Ok(non_empty) => match non_empty.and(output) {
                Some((_, live, data)) => (live, data),
                None => return,
            },
            Err(()) => {
                self.result.mismatched_output = true;

                if ARGS.gen_report() {
                    let old = old_hash.map(|old_hash| {
                        let old_hash_path =
                            T::OUTPUT.hash_path(old_hash, &self.test.name);
                        let old_live_data = self.read_old_live_data::<T>(old_hash);
                        (old_hash_path, old_live_data)
                    });

                    let new = output.map(|(_, live, data)| {
                        let new_hash = T::make_hash(live);
                        let new_hash_path =
                            T::OUTPUT.hash_path(new_hash, &self.test.name);
                        (new_hash_path, data)
                    });

                    let file_report = make_report::<T>(old, new);
                    self.result.add_report(self.test.name.clone(), file_report);
                }

                return;
            }
        };

        // Happy path: output is ok and doesn't need to be updated.
        let new_hash = T::make_hash(live);
        if old_hash.as_ref().is_some_and(|h| *h == new_hash) {
            return;
        }

        if ARGS.update {
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
            let old = if let Some(old_hash) = old_hash {
                log!(self, "mismatched output");
                log!(self, "  live      | {}", live_path.display());
                log!(self, "  old       | {old_hash}");
                log!(self, "  new       | {new_hash}");

                let old_hash_path = T::OUTPUT.hash_path(old_hash, &self.test.name);
                let old_live_data = self.read_old_live_data::<T>(old_hash);
                Some((old_hash_path, old_live_data))
            } else {
                log!(self, "missing reference hash");
                log!(self, "  live      | {}", live_path.display());
                log!(self, "  new       | {new_hash}");

                None
            };

            if ARGS.gen_report() {
                let new_hash_path = T::OUTPUT.hash_path(new_hash, &self.test.name);
                let file_report = make_report::<T>(old, Some((new_hash_path, live_data)));
                self.result.add_report(self.test.name.clone(), file_report);
            }
        }
    }

    /// Check if the output matches what the attributes and test annotations
    /// expect.
    /// The `Ok` case returns whether an expected output is present and
    /// should be compared to a reference.
    fn expect_output<T: OutputType>(
        &mut self,
        output: &Option<(&T::Doc, &T::Live, impl AsRef<[u8]>)>,
    ) -> Result<Option<()>, ()> {
        let Some((doc, live, _)) = output else {
            if !self.test.body.has_error() {
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

    fn read_old_live_data<T: HashOutputType>(
        &mut self,
        old_hash: HashedRef,
    ) -> Old<Vec<u8>> {
        let old_hash_path = T::OUTPUT.hash_path(old_hash, &self.test.name);

        let old_live_data = std::fs::read(&old_hash_path).inspect_err(|_| {
            log!(self, "  missing old live output {}", old_hash_path.display());
        });
        match old_live_data {
            Ok(data) => Old::Data(data),
            Err(_) => Old::Missing(old_hash),
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
        let stage = stage.into();

        let emitted_diag =
            Note::emitted(kind, stage, &diag.message, diag.span, &self.world);
        self.test.body.mark_seen_or_update(emitted_diag);

        // Check hints.
        for Spanned { v: hint, span } in &diag.hints {
            // HACK: This hint only gets emitted in debug builds, so filter it
            // out to make the test suite also pass for release builds.
            if hint == "set `RUST_BACKTRACE` to `1` or `full` to capture a backtrace" {
                continue;
            }

            let emitted_hint = Note::emitted(
                NoteKind::Hint,
                stage,
                hint,
                span.or(diag.span),
                &self.world,
            );
            self.test.body.mark_seen_or_update(emitted_hint);
        }
    }
}

// Convenience wrapper that handles both owned and borrowed data.
fn make_report<T: OutputType>(
    a: Option<(impl AsRef<Path>, Old<impl AsRef<[u8]>>)>,
    b: Option<(impl AsRef<Path>, impl AsRef<[u8]>)>,
) -> ReportFile {
    let a = (a.as_ref())
        .map(|(path, data)| (path.as_ref(), data.as_ref().map(|d| d.as_ref())));
    let b = b.as_ref().map(|(path, data)| (path.as_ref(), data.as_ref()));
    T::make_report(a, b.ok_or(()))
}

/// Read a reference file either from a specific git base revision, or from
/// the file system.
pub fn read_ref_data(ref_path: &Path) -> Option<Vec<u8>> {
    match &ARGS.base_revision {
        Some(rev) => git::read_file(rev, ref_path),
        None => std::fs::read(ref_path).ok(),
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
    pub fn deduplicate(diags: EcoVec<SourceDiagnostic>) -> EcoVec<SourceDiagnostic> {
        deduplicate_with(diags, [])
    }

    // Deduplicate diagnostics with a set of already existing ones.
    pub fn deduplicate_with<'a>(
        mut diags: EcoVec<SourceDiagnostic>,
        existing: impl IntoIterator<Item = &'a SourceDiagnostic>,
    ) -> EcoVec<SourceDiagnostic> {
        let hash =
            |diag: &SourceDiagnostic| typst_utils::hash128(&(&diag.span, &diag.message));

        let mut unique = existing.into_iter().map(hash).collect::<FxHashSet<_>>();
        diags.retain(|diag| unique.insert(hash(diag)));
        diags
    }
}
