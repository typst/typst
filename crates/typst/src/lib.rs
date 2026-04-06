//! The compiler for the _Typst_ markup language.
//!
//! # Steps
//! - **Parsing:**
//!   The compiler first transforms a plain string into an iterator of [tokens].
//!   This token stream is [parsed] into a [syntax tree]. The tree itself is
//!   untyped, but the [AST] module provides a typed layer over it.
//! - **Evaluation:**
//!   The next step is to [evaluate] the markup. This produces a [module],
//!   consisting of a scope of values that were exported by the code and
//!   [content], a hierarchical, styled representation of what was written in
//!   the source file. The elements of the content tree are well structured and
//!   order-independent and thus much better suited for further processing than
//!   the raw markup.
//! - **Layouting:**
//!   Next, the content is [laid out] into a
//!   [`PagedDocument`](typst_layout::PagedDocument) containing one [frame] per
//!   page with items at fixed positions.
//! - **Exporting:**
//!   These frames can finally be exported into an output format (currently PDF,
//!   PNG, SVG, and HTML).
//!
//! [tokens]: typst_syntax::SyntaxKind
//! [parsed]: typst_syntax::parse
//! [syntax tree]: typst_syntax::SyntaxNode
//! [AST]: typst_syntax::ast
//! [evaluate]: typst_eval::eval
//! [module]: crate::foundations::Module
//! [content]: crate::foundations::Content
//! [laid out]: typst_layout::layout_document
//! [frame]: crate::layout::Frame

pub extern crate comemo;
pub extern crate ecow;

pub use typst_library::*;
#[doc(inline)]
pub use typst_syntax as syntax;
#[doc(inline)]
pub use typst_utils as utils;

use std::sync::LazyLock;

use arrayvec::ArrayVec;
use comemo::{Track, Tracked};
use ecow::{EcoString, EcoVec, eco_format, eco_vec};
use rustc_hash::FxHashSet;
use typst_library::diag::{
    FileError, SourceDiagnostic, SourceResult, Warned, bail, warning,
};
use typst_library::engine::{Engine, Route, Sink, Traced};
use typst_library::foundations::{
    NativeRuleMap, Output, StyleChain, Styles, Target, TargetElem, Value,
};
use typst_library::introspection::{
    EmptyIntrospector, ITER_NAMES, Introspector, MAX_ITERS,
};
use typst_library::routines::Routines;
use typst_syntax::{FileId, Span};
use typst_timing::{TimingScope, timed};
use typst_utils::Protected;

/// Compiles sources into an output.
///
/// Supported outputs are
/// - the `PagedDocument` (defined in `typst_layout`)
/// - the `HtmlDocument` (defined in `typst_html`)
///
/// Returns the compilation output alongside warnings, if any. The contained
/// result is
/// - `Ok(output)` if there were no fatal errors.
/// - `Err(errors)` if there were fatal errors.
#[typst_macros::time]
pub fn compile<T>(world: &dyn World) -> Warned<SourceResult<T>>
where
    T: Output,
{
    let mut sink = Sink::new();
    let output = compile_impl::<T>(world.track(), Traced::default().track(), &mut sink)
        .map_err(deduplicate);
    Warned { output, warnings: sink.warnings() }
}

/// Compiles sources and returns all values and styles observed at the given
/// `span` during compilation.
#[typst_macros::time]
pub fn trace<T>(world: &dyn World, span: Span) -> EcoVec<(Value, Option<Styles>)>
where
    T: Output,
{
    let mut sink = Sink::new();
    let traced = Traced::new(span);
    compile_impl::<T>(world.track(), traced.track(), &mut sink).ok();
    sink.values()
}

/// The internal implementation of `compile` with a bit lower-level interface
/// that is also used by `trace`.
fn compile_impl<T: Output>(
    world: Tracked<dyn World + '_>,
    traced: Tracked<Traced>,
    sink: &mut Sink,
) -> SourceResult<T> {
    match T::target() {
        Target::Paged => {}
        Target::Html => warn_or_error_for_html(world, sink)?,
        Target::Bundle => warn_or_error_for_bundle(world, sink)?,
    }

    let library = world.library();
    let base = StyleChain::new(&library.styles);
    let target = TargetElem::target.set(T::target()).wrap();
    let styles = base.chain(&target);
    let empty_introspector = EmptyIntrospector;

    // Fetch the main source file once.
    let main = world.main();
    let main = world
        .source(main)
        .map_err(|err| hint_invalid_main_file(world, err, main))?;

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

    let mut history: ArrayVec<T, { MAX_ITERS - 1 }> = ArrayVec::new();
    let mut document: T;

    // Relayout until all introspections stabilize.
    // If that doesn't happen within five attempts, we give up.
    loop {
        let _scope = TimingScope::new(ITER_NAMES[history.len()]);

        // Always enable layout-time eviction for memory savings.
        typst_library::engine_flags::enable_layout_eviction();
        let introspector = history
            .last()
            .map(|doc| doc.introspector())
            .unwrap_or(&empty_introspector);
        let constraint = comemo::Constraint::new();

        let mut subsink = Sink::new();
        let mut engine = Engine {
            world,
            introspector: Protected::new(introspector.track_with(&constraint)),
            traced,
            sink: subsink.track_mut(),
            route: Route::default(),
            routines: &ROUTINES,
        };

        document = T::create(&mut engine, &content, styles)?;

        if timed!("check stabilized", constraint.validate(document.introspector())) {
            sink.extend_from_sink(subsink);

            // Phase 2: Streaming re-layout for large paged documents.
            // Now that introspection has converged, re-layout with memoization
            // disabled and pages spilled to disk. This prevents holding the
            // full document in memory. Only activates for documents exceeding
            // the streaming threshold (>100 pages).
            if document.should_stream() {
                // Drop page frames but keep the introspector alive.
                // drop_pages() ensures introspector is built first.
                document.drop_pages();
                // Free all Phase 1 comemo caches — streaming bypasses them.
                comemo::evict(0);

                typst_library::engine_flags::enable_streaming_mode();

                let streaming_result = (|| -> SourceResult<T> {
                    let constraint2 = comemo::Constraint::new();
                    let mut subsink2 = Sink::new();
                    let mut engine2 = Engine {
                        world,
                        introspector: Protected::new(
                            document.introspector().track_with(&constraint2),
                        ),
                        traced,
                        sink: subsink2.track_mut(),
                        route: Route::default(),
                        routines: &ROUTINES,
                    };

                    let result = T::create(&mut engine2, &content, styles);
                    sink.extend_from_sink(subsink2);
                    result
                })();

                typst_library::engine_flags::disable_streaming_mode();

                // Replace the converged document with the streaming one.
                // The old document (with dropped pages) is dropped here.
                document = streaming_result?;
            }

            break;
        }

        if history.is_full() {
            let mut introspectors =
                [&empty_introspector as &dyn Introspector; MAX_ITERS + 1];
            for i in 1..MAX_ITERS {
                introspectors[i] = history[i - 1].introspector();
            }
            introspectors[MAX_ITERS] = document.introspector();

            let warnings = typst_library::introspection::analyze(
                world,
                &ROUTINES,
                introspectors,
                subsink.introspections(),
            );

            sink.extend_from_sink(subsink);
            for warning in warnings {
                sink.warn(warning);
            }
            break;
        }

        // Drop page frames from the document before storing in history.
        // The convergence loop only needs the introspector from previous
        // iterations, never the page data. This prevents holding multiple
        // complete copies of large documents (e.g., 300K pages) in memory.
        document.drop_pages();

        // Evict stale memoization cache entries to free memory from the
        // previous iteration's cached layout results. Use max_age 2 to
        // keep entries that might be reused in the next iteration while
        // freeing older ones.
        comemo::evict(2);

        history.push(document);
    }

    // Promote delayed errors.
    let delayed = sink.delayed();
    if !delayed.is_empty() {
        return Err(delayed);
    }

    Ok(document)
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

/// Adds useful hints when the main source file couldn't be read
/// and returns the final diagnostic.
fn hint_invalid_main_file(
    world: Tracked<dyn World + '_>,
    file_error: FileError,
    input: FileId,
) -> EcoVec<SourceDiagnostic> {
    let is_utf8_error = matches!(file_error, FileError::InvalidUtf8);
    let mut diagnostic =
        SourceDiagnostic::error(Span::detached(), EcoString::from(file_error));

    // Attempt to provide helpful hints for UTF-8 errors. Perhaps the user
    // mistyped the filename. For example, they could have written "file.pdf"
    // instead of "file.typ".
    if is_utf8_error {
        match input.vpath().extension() {
            // No hints if the file is already a .typ file. The file is indeed
            // just invalid.
            Some("typ") => return eco_vec![diagnostic],

            Some(ext) => {
                diagnostic.hint(eco_format!(
                    "a file with the `.{ext}` extension is not usually a Typst file",
                ));
            }

            None => {
                diagnostic
                    .hint("a file without an extension is not usually a Typst file");
            }
        };

        if world.source(input.map(|p| p.with_extension("typ")).intern()).is_ok() {
            diagnostic.hint("check if you meant to use the `.typ` extension instead");
        }
    }

    eco_vec![diagnostic]
}

/// HTML export will warn or error depending on whether the feature flag is enabled.
fn warn_or_error_for_html(
    world: Tracked<dyn World + '_>,
    sink: &mut Sink,
) -> SourceResult<()> {
    const ISSUE: &str = "https://github.com/typst/typst/issues/5512";
    if world.library().features.is_enabled(Feature::Html) {
        sink.warn(warning!(
            Span::detached(),
            "html export is under active development and incomplete";
            hint: "its behaviour may change at any time";
            hint: "do not rely on this feature for production use cases";
            hint: "see {ISSUE} for more information";
        ));
    } else {
        bail!(
            Span::detached(),
            "html export is only available when `--features html` is passed";
            hint: "html export is under active development and incomplete";
            hint: "see {ISSUE} for more information";
        );
    }
    Ok(())
}

/// Bundle export will warn or error depending on whether the feature flag is
/// enabled.
fn warn_or_error_for_bundle(
    world: Tracked<dyn World + '_>,
    sink: &mut Sink,
) -> SourceResult<()> {
    if world.library().features.is_enabled(Feature::Bundle) {
        sink.warn(warning!(
            Span::detached(),
            "bundle export is experimental";
            hint: "its behaviour may change at any time";
            hint: "do not rely on this feature for production use cases";
        ));
    } else {
        bail!(
            Span::detached(),
            "bundle export is only available when `--features bundle` is passed";
            hint: "bundle export is experimental";
        );
    }
    Ok(())
}

/// Provides ways to construct a [`Library`].
pub trait LibraryExt {
    /// Creates the default library.
    fn default() -> Library;

    /// Creates a builder for configuring a library.
    fn builder() -> LibraryBuilder;
}

impl LibraryExt for Library {
    fn default() -> Library {
        Self::builder().build()
    }

    fn builder() -> LibraryBuilder {
        LibraryBuilder::from_routines(&ROUTINES)
    }
}

/// Defines implementation of various Typst compiler routines as a table of
/// function pointers.
///
/// This is essentially dynamic linking and done to allow for crate splitting.
pub static ROUTINES: LazyLock<Routines> = LazyLock::new(|| Routines {
    rules: {
        let mut rules = NativeRuleMap::new();
        typst_layout::register(&mut rules);
        typst_html::register(&mut rules);
        rules
    },
    eval_string: typst_eval::eval_string,
    eval_closure: typst_eval::eval_closure,
    realize: typst_realize::realize,
    layout_frame: typst_layout::layout_frame,
    html_module: typst_html::module,
    html_span_filled: typst_html::html_span_filled,
});
