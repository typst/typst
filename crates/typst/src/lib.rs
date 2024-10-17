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
//!   Next, the content is [layouted] into a [document] containing one [frame]
//!   per page with items at fixed positions.
//! - **Exporting:**
//!   These frames can finally be exported into an output format (currently PDF,
//!   PNG, or SVG).
//!
//! [tokens]: syntax::SyntaxKind
//! [parsed]: syntax::parse
//! [syntax tree]: syntax::SyntaxNode
//! [AST]: syntax::ast
//! [evaluate]: eval::eval
//! [module]: foundations::Module
//! [content]: foundations::Content
//! [layouted]: crate::layout::layout_document
//! [document]: model::Document
//! [frame]: layout::Frame

#![recursion_limit = "1000"]
#![allow(clippy::comparison_chain)]
#![allow(clippy::wildcard_in_or_patterns)]
#![allow(clippy::manual_range_contains)]

extern crate self as typst;

pub mod diag;
pub mod embed;
pub mod engine;
pub mod eval;
pub mod foundations;
pub mod introspection;
pub mod layout;
pub mod loading;
pub mod math;
pub mod model;
pub mod realize;
pub mod symbols;
pub mod text;
pub mod visualize;

#[doc(inline)]
pub use typst_syntax as syntax;
#[doc(inline)]
pub use typst_utils as utils;

use std::collections::HashSet;
use std::ops::{Deref, Range};

use comemo::{Track, Tracked, Validate};
use ecow::{eco_format, eco_vec, EcoString, EcoVec};
use typst_timing::{timed, TimingScope};

use crate::diag::{
    warning, FileError, FileResult, SourceDiagnostic, SourceResult, Warned,
};
use crate::engine::{Engine, Route, Sink, Traced};
use crate::foundations::{
    Array, Bytes, Datetime, Dict, Module, Scope, StyleChain, Styles, Value,
};
use crate::introspection::Introspector;
use crate::layout::{Alignment, Dir};
use crate::model::Document;
use crate::syntax::package::PackageSpec;
use crate::syntax::{FileId, Source, Span};
use crate::text::{Font, FontBook};
use crate::utils::LazyHash;
use crate::visualize::Color;

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
    let content = crate::eval::eval(
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
        let _scope = TimingScope::new(ITER_NAMES[iter], None);

        subsink = Sink::new();

        let constraint = <Introspector as Validate>::Constraint::new();
        let mut engine = Engine {
            world,
            introspector: document.introspector.track_with(&constraint),
            traced,
            sink: subsink.track_mut(),
            route: Route::default(),
        };

        // Layout!
        document = crate::layout::layout_document(&mut engine, &content, styles)?;
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
        let hash = crate::utils::hash128(&(&diag.span, &diag.message));
        unique.insert(hash)
    });
    diags
}

/// The environment in which typesetting occurs.
///
/// All loading functions (`main`, `source`, `file`, `font`) should perform
/// internal caching so that they are relatively cheap on repeated invocations
/// with the same argument. [`Source`], [`Bytes`], and [`Font`] are
/// all reference-counted and thus cheap to clone.
///
/// The compiler doesn't do the caching itself because the world has much more
/// information on when something can change. For example, fonts typically don't
/// change and can thus even be cached across multiple compilations (for
/// long-running applications like `typst watch`). Source files on the other
/// hand can change and should thus be cleared after each compilation. Advanced
/// clients like language servers can also retain the source files and
/// [edit](Source::edit) them in-place to benefit from better incremental
/// performance.
#[comemo::track]
pub trait World: Send + Sync {
    /// The standard library.
    ///
    /// Can be created through `Library::build()`.
    fn library(&self) -> &LazyHash<Library>;

    /// Metadata about all known fonts.
    fn book(&self) -> &LazyHash<FontBook>;

    /// Get the file id of the main source file.
    fn main(&self) -> FileId;

    /// Try to access the specified source file.
    fn source(&self, id: FileId) -> FileResult<Source>;

    /// Try to access the specified file.
    fn file(&self, id: FileId) -> FileResult<Bytes>;

    /// Try to access the font with the given index in the font book.
    fn font(&self, index: usize) -> Option<Font>;

    /// Get the current date.
    ///
    /// If no offset is specified, the local date should be chosen. Otherwise,
    /// the UTC date should be chosen with the corresponding offset in hours.
    ///
    /// If this function returns `None`, Typst's `datetime` function will
    /// return an error.
    fn today(&self, offset: Option<i64>) -> Option<Datetime>;

    /// A list of all available packages and optionally descriptions for them.
    ///
    /// This function is optional to implement. It enhances the user experience
    /// by enabling autocompletion for packages. Details about packages from the
    /// `@preview` namespace are available from
    /// `https://packages.typst.org/preview/index.json`.
    fn packages(&self) -> &[(PackageSpec, Option<EcoString>)] {
        &[]
    }
}

macro_rules! delegate_for_ptr {
    ($W:ident for $ptr:ty) => {
        impl<$W: World> World for $ptr {
            fn library(&self) -> &LazyHash<Library> {
                self.deref().library()
            }

            fn book(&self) -> &LazyHash<FontBook> {
                self.deref().book()
            }

            fn main(&self) -> FileId {
                self.deref().main()
            }

            fn source(&self, id: FileId) -> FileResult<Source> {
                self.deref().source(id)
            }

            fn file(&self, id: FileId) -> FileResult<Bytes> {
                self.deref().file(id)
            }

            fn font(&self, index: usize) -> Option<Font> {
                self.deref().font(index)
            }

            fn today(&self, offset: Option<i64>) -> Option<Datetime> {
                self.deref().today(offset)
            }

            fn packages(&self) -> &[(PackageSpec, Option<EcoString>)] {
                self.deref().packages()
            }
        }
    };
}

delegate_for_ptr!(W for std::boxed::Box<W>);
delegate_for_ptr!(W for std::sync::Arc<W>);
delegate_for_ptr!(W for &W);

/// Helper methods on [`World`] implementations.
pub trait WorldExt {
    /// Get the byte range for a span.
    ///
    /// Returns `None` if the `Span` does not point into any source file.
    fn range(&self, span: Span) -> Option<Range<usize>>;
}

impl<T: World> WorldExt for T {
    fn range(&self, span: Span) -> Option<Range<usize>> {
        self.source(span.id()?).ok()?.range(span)
    }
}

/// Definition of Typst's standard library.
#[derive(Debug, Clone, Hash)]
pub struct Library {
    /// The module that contains the definitions that are available everywhere.
    pub global: Module,
    /// The module that contains the definitions available in math mode.
    pub math: Module,
    /// The default style properties (for page size, font selection, and
    /// everything else configurable via set and show rules).
    pub styles: Styles,
    /// The standard library as a value.
    /// Used to provide the `std` variable.
    pub std: Value,
}

impl Library {
    /// Create a new builder for a library.
    pub fn builder() -> LibraryBuilder {
        LibraryBuilder::default()
    }
}

impl Default for Library {
    /// Constructs the standard library with the default configuration.
    fn default() -> Self {
        Self::builder().build()
    }
}

/// Configurable builder for the standard library.
///
/// This struct is created by [`Library::builder`].
#[derive(Debug, Clone, Default)]
pub struct LibraryBuilder {
    inputs: Option<Dict>,
}

impl LibraryBuilder {
    /// Configure the inputs visible through `sys.inputs`.
    pub fn with_inputs(mut self, inputs: Dict) -> Self {
        self.inputs = Some(inputs);
        self
    }

    /// Consumes the builder and returns a `Library`.
    pub fn build(self) -> Library {
        let math = math::module();
        let inputs = self.inputs.unwrap_or_default();
        let global = global(math.clone(), inputs);
        let std = Value::Module(global.clone());
        Library { global, math, styles: Styles::new(), std }
    }
}

/// Construct the module with global definitions.
fn global(math: Module, inputs: Dict) -> Module {
    let mut global = Scope::deduplicating();
    self::foundations::define(&mut global, inputs);
    self::model::define(&mut global);
    self::text::define(&mut global);
    global.reset_category();
    global.define_module(math);
    self::layout::define(&mut global);
    self::visualize::define(&mut global);
    self::introspection::define(&mut global);
    self::loading::define(&mut global);
    self::symbols::define(&mut global);
    self::embed::define(&mut global);
    prelude(&mut global);
    Module::new("global", global)
}

/// Defines scoped values that are globally available, too.
fn prelude(global: &mut Scope) {
    global.reset_category();
    global.define("black", Color::BLACK);
    global.define("gray", Color::GRAY);
    global.define("silver", Color::SILVER);
    global.define("white", Color::WHITE);
    global.define("navy", Color::NAVY);
    global.define("blue", Color::BLUE);
    global.define("aqua", Color::AQUA);
    global.define("teal", Color::TEAL);
    global.define("eastern", Color::EASTERN);
    global.define("purple", Color::PURPLE);
    global.define("fuchsia", Color::FUCHSIA);
    global.define("maroon", Color::MAROON);
    global.define("red", Color::RED);
    global.define("orange", Color::ORANGE);
    global.define("yellow", Color::YELLOW);
    global.define("olive", Color::OLIVE);
    global.define("green", Color::GREEN);
    global.define("lime", Color::LIME);
    global.define("luma", Color::luma_data());
    global.define("oklab", Color::oklab_data());
    global.define("oklch", Color::oklch_data());
    global.define("rgb", Color::rgb_data());
    global.define("cmyk", Color::cmyk_data());
    global.define("range", Array::range_data());
    global.define("ltr", Dir::LTR);
    global.define("rtl", Dir::RTL);
    global.define("ttb", Dir::TTB);
    global.define("btt", Dir::BTT);
    global.define("start", Alignment::START);
    global.define("left", Alignment::LEFT);
    global.define("center", Alignment::CENTER);
    global.define("right", Alignment::RIGHT);
    global.define("end", Alignment::END);
    global.define("top", Alignment::TOP);
    global.define("horizon", Alignment::HORIZON);
    global.define("bottom", Alignment::BOTTOM);
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
