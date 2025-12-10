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
//!   Next, the content is [laid out] into a [`PagedDocument`] containing one
//!   [frame] per page with items at fixed positions.
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

use comemo::{Track, Tracked};
use ecow::{EcoString, EcoVec, eco_format, eco_vec};
use rustc_hash::FxHashSet;
use typst_html::HtmlDocument;
use typst_library::diag::{
    FileError, SourceDiagnostic, SourceResult, Warned, bail, warning,
};
use typst_library::engine::{Engine, Route, Sink, Traced};
use typst_library::foundations::{NativeRuleMap, StyleChain, Styles, Value};
use typst_library::introspection::Introspector;
use typst_library::layout::PagedDocument;
use typst_library::routines::Routines;
use typst_syntax::{FileId, Span};
use typst_timing::{TimingScope, timed};

use crate::foundations::{Target, TargetElem};
use crate::model::DocumentInfo;

/// Compile sources into a fully layouted document.
///
/// - Returns `Ok(document)` if there were no fatal errors.
/// - Returns `Err(errors)` if there were fatal errors.
#[typst_macros::time]
pub fn compile<D>(world: &dyn World) -> Warned<SourceResult<D>>
where
    D: Document,
{
    let mut sink = Sink::new();
    let output = compile_impl::<D>(world.track(), Traced::default().track(), &mut sink)
        .map_err(deduplicate);
    Warned { output, warnings: sink.warnings() }
}

/// Compiles sources and returns all values and styles observed at the given
/// `span` during compilation.
#[typst_macros::time]
pub fn trace<D>(world: &dyn World, span: Span) -> EcoVec<(Value, Option<Styles>)>
where
    D: Document,
{
    let mut sink = Sink::new();
    let traced = Traced::new(span);
    compile_impl::<D>(world.track(), traced.track(), &mut sink).ok();
    sink.values()
}

/// The internal implementation of `compile` with a bit lower-level interface
/// that is also used by `trace`.
fn compile_impl<D: Document>(
    world: Tracked<dyn World + '_>,
    traced: Tracked<Traced>,
    sink: &mut Sink,
) -> SourceResult<D> {
    if D::target() == Target::Html {
        warn_or_error_for_html(world, sink)?;
    }

    let library = world.library();
    let base = StyleChain::new(&library.styles);
    let target = TargetElem::target.set(D::target()).wrap();
    let styles = base.chain(&target);
    let empty_introspector = Introspector::default();

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

    let mut iter = 0;
    let mut subsink;
    let mut introspector = &empty_introspector;
    let mut document: D;

    // Relayout until all introspections stabilize.
    // If that doesn't happen within five attempts, we give up.
    loop {
        // The name of the iterations for timing scopes.
        const ITER_NAMES: &[&str] =
            &["layout (1)", "layout (2)", "layout (3)", "layout (4)", "layout (5)"];
        let _scope = TimingScope::new(ITER_NAMES[iter]);

        subsink = Sink::new();

        let constraint = comemo::Constraint::new();
        let mut engine = Engine {
            world,
            introspector: introspector.track_with(&constraint),
            traced,
            sink: subsink.track_mut(),
            route: Route::default(),
            routines: &ROUTINES,
        };

        // Layout!
        document = D::create(&mut engine, &content, styles)?;
        introspector = document.introspector();
        iter += 1;

        if timed!("check stabilized", constraint.validate(introspector)) {
            break;
        }

        if iter >= 5 {
            subsink.warn(warning!(
                Span::detached(), "layout did not converge within 5 attempts";
                hint: "check if any states or queries are updating themselves"
            ));
            break;
        }
    }

    sink.extend_from_sink(subsink);

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
        let path = input.vpath();
        let extension = path.as_rootless_path().extension();
        if extension.is_some_and(|extension| extension == "typ") {
            // No hints if the file is already a .typ file.
            // The file is indeed just invalid.
            return eco_vec![diagnostic];
        }

        match extension {
            Some(extension) => {
                diagnostic.hint(eco_format!(
                    "a file with the `.{}` extension is not usually a Typst file",
                    extension.to_string_lossy()
                ));
            }

            None => {
                diagnostic
                    .hint("a file without an extension is not usually a Typst file");
            }
        };

        if world.source(input.with_extension("typ")).is_ok() {
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
            hint: "see {ISSUE} for more information"
        ));
    } else {
        bail!(
            Span::detached(),
            "html export is only available when `--features html` is passed";
            hint: "html export is under active development and incomplete";
            hint: "see {ISSUE} for more information"
        );
    }
    Ok(())
}

/// A document is what results from compilation.
pub trait Document: sealed::Sealed {
    /// Get the document's metadata.
    fn info(&self) -> &DocumentInfo;

    /// Get the document's introspector.
    fn introspector(&self) -> &Introspector;
}

impl Document for PagedDocument {
    fn info(&self) -> &DocumentInfo {
        &self.info
    }

    fn introspector(&self) -> &Introspector {
        &self.introspector
    }
}

impl Document for HtmlDocument {
    fn info(&self) -> &DocumentInfo {
        &self.info
    }

    fn introspector(&self) -> &Introspector {
        &self.introspector
    }
}

/// A trait for accepting an arbitrary kind of document as input.
///
/// Can be used to accept a reference to
/// - any kind of sized type that implements [`Document`], or
/// - the trait object [`&dyn Document`].
///
/// Should be used as `impl AsDocument` rather than `&impl AsDocument`.
///
/// # Why is this needed?
/// Unfortunately, `&impl Document` can't be turned into `&dyn Document` in a
/// generic function. Directly accepting `&dyn Document` is of course also
/// possible, but is less convenient, especially in cases where the document is
/// optional.
///
/// See also
/// <https://users.rust-lang.org/t/converting-from-generic-unsized-parameter-to-trait-object/72376>
pub trait AsDocument {
    /// Turns the reference into the trait object.
    fn as_document(&self) -> &dyn Document;
}

impl AsDocument for &dyn Document {
    fn as_document(&self) -> &dyn Document {
        *self
    }
}

impl<D: Document> AsDocument for &D {
    fn as_document(&self) -> &dyn Document {
        *self
    }
}

mod sealed {
    use typst_library::foundations::{Content, Target};

    use super::*;

    pub trait Sealed {
        fn target() -> Target
        where
            Self: Sized;

        fn create(
            engine: &mut Engine,
            content: &Content,
            styles: StyleChain,
        ) -> SourceResult<Self>
        where
            Self: Sized;
    }

    impl Sealed for PagedDocument {
        fn target() -> Target {
            Target::Paged
        }

        fn create(
            engine: &mut Engine,
            content: &Content,
            styles: StyleChain,
        ) -> SourceResult<Self> {
            typst_layout::layout_document(engine, content, styles)
        }
    }

    impl Sealed for HtmlDocument {
        fn target() -> Target {
            Target::Html
        }

        fn create(
            engine: &mut Engine,
            content: &Content,
            styles: StyleChain,
        ) -> SourceResult<Self> {
            typst_html::html_document(engine, content, styles)
        }
    }
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
