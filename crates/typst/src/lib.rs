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
//!   Next, the content is [laid out] into a [document] containing one [frame]
//!   per page with items at fixed positions.
//! - **Exporting:**
//!   These frames can finally be exported into an output format (currently PDF,
//!   PNG, or SVG).
//!
//! [tokens]: typst_syntax::SyntaxKind
//! [parsed]: typst_syntax::parse
//! [syntax tree]: typst_syntax::SyntaxNode
//! [AST]: typst_syntax::ast
//! [evaluate]: typst_eval::eval
//! [module]: crate::foundations::Module
//! [content]: crate::foundations::Content
//! [laid out]: typst_layout::layout_document
//! [document]: crate::model::Document
//! [frame]: crate::layout::Frame

pub extern crate comemo;
pub extern crate ecow;

pub use typst_library::*;
#[doc(inline)]
pub use typst_syntax as syntax;
#[doc(inline)]
pub use typst_utils as utils;

use std::collections::HashSet;

use comemo::{Track, Tracked, Validate};
use ecow::{eco_format, eco_vec, EcoString, EcoVec};
use typst_library::diag::{warning, FileError, SourceDiagnostic, SourceResult, Warned};
use typst_library::engine::{Engine, Route, Sink, Traced};
use typst_library::foundations::{StyleChain, Styles, Value};
use typst_library::introspection::Introspector;
use typst_library::model::Document;
use typst_library::routines::Routines;
use typst_syntax::{FileId, Span};
use typst_timing::{timed, TimingScope};

/// Compile sources into a fully layouted document.
///
/// - Returns `Ok(document)` if there were no fatal errors.
/// - Returns `Err(errors)` if there were fatal errors.
#[typst_macros::time]
pub fn compile(world: &dyn World) -> Warned<SourceResult<Document>> {
    let mut sink = Sink::new();
    let output = compile_impl(world.track(), Traced::default().track(), &mut sink)
        .map_err(deduplicate);
    Warned { output, warnings: sink.warnings() }
}

/// Compiles sources and returns all values and styles observed at the given
/// `span` during compilation.
#[typst_macros::time]
pub fn trace(world: &dyn World, span: Span) -> EcoVec<(Value, Option<Styles>)> {
    let mut sink = Sink::new();
    let traced = Traced::new(span);
    compile_impl(world.track(), traced.track(), &mut sink).ok();
    sink.values()
}

/// The internal implementation of `compile` with a bit lower-level interface
/// that is also used by `trace`.
fn compile_impl(
    world: Tracked<dyn World + '_>,
    traced: Tracked<Traced>,
    sink: &mut Sink,
) -> SourceResult<Document> {
    let library = world.library();
    let styles = StyleChain::new(&library.styles);

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
    let mut document = Document::default();

    // Relayout until all introspections stabilize.
    // If that doesn't happen within five attempts, we give up.
    loop {
        // The name of the iterations for timing scopes.
        const ITER_NAMES: &[&str] =
            &["layout (1)", "layout (2)", "layout (3)", "layout (4)", "layout (5)"];
        let _scope = TimingScope::new(ITER_NAMES[iter]);

        subsink = Sink::new();

        let constraint = <Introspector as Validate>::Constraint::new();
        let mut engine = Engine {
            world,
            introspector: document.introspector.track_with(&constraint),
            traced,
            sink: subsink.track_mut(),
            route: Route::default(),
            routines: &ROUTINES,
        };

        // Layout!
        document = (engine.routines.layout_document)(&mut engine, &content, styles)?;
        iter += 1;

        if timed!("check stabilized", document.introspector.validate(&constraint)) {
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
    let mut unique = HashSet::new();
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

/// Defines implementation of various Typst compiler routines as a table of
/// function pointers.
///
/// This is essentially dynamic linking and done to allow for crate splitting.
pub static ROUTINES: Routines = Routines {
    eval_string: typst_eval::eval_string,
    eval_closure: typst_eval::eval_closure,
    realize: typst_realize::realize,
    layout_document: typst_layout::layout_document,
    layout_fragment: typst_layout::layout_fragment,
    layout_frame: typst_layout::layout_frame,
    layout_inline: typst_layout::layout_inline,
    layout_box: typst_layout::layout_box,
    layout_list: typst_layout::layout_list,
    layout_enum: typst_layout::layout_enum,
    layout_grid: typst_layout::layout_grid,
    layout_table: typst_layout::layout_table,
    layout_stack: typst_layout::layout_stack,
    layout_columns: typst_layout::layout_columns,
    layout_move: typst_layout::layout_move,
    layout_rotate: typst_layout::layout_rotate,
    layout_scale: typst_layout::layout_scale,
    layout_skew: typst_layout::layout_skew,
    layout_repeat: typst_layout::layout_repeat,
    layout_pad: typst_layout::layout_pad,
    layout_line: typst_layout::layout_line,
    layout_path: typst_layout::layout_path,
    layout_polygon: typst_layout::layout_polygon,
    layout_rect: typst_layout::layout_rect,
    layout_square: typst_layout::layout_square,
    layout_ellipse: typst_layout::layout_ellipse,
    layout_circle: typst_layout::layout_circle,
    layout_image: typst_layout::layout_image,
    layout_equation_block: typst_layout::layout_equation_block,
    layout_equation_inline: typst_layout::layout_equation_inline,
};
