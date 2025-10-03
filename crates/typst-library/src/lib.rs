//! Typst's standard library.
//!
//! This crate also contains all of the compiler's central type definitions as
//! these are interwoven with the standard library types.
//!
//! In contrast to the _types,_ most of the compilation _behaviour_ is split out
//! into separate crates (`typst-eval`, `typst-realize`, `typst-layout`, etc.)
//!
//! Note that, unless you are working on the compiler itself, you will rarely
//! need to interact with this crate, as it is fully reexported by the `typst`
//! crate.

extern crate self as typst_library;

pub mod diag;
pub mod engine;
pub mod foundations;
pub mod introspection;
pub mod layout;
pub mod loading;
pub mod math;
pub mod model;
pub mod pdf;
pub mod routines;
pub mod symbols;
pub mod text;
pub mod visualize;

use std::ops::{Deref, Range};

use serde::{Deserialize, Serialize};
use typst_syntax::{FileId, Source, Span};
use typst_utils::{LazyHash, SmallBitSet};

use crate::diag::FileResult;
use crate::foundations::{Array, Binding, Bytes, Datetime, Dict, Module, Scope, Styles};
use crate::layout::{Alignment, Dir};
use crate::routines::Routines;
use crate::text::{Font, FontBook};
use crate::visualize::Color;

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
}

macro_rules! world_impl {
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
        }
    };
}

world_impl!(W for std::boxed::Box<W>);
world_impl!(W for std::sync::Arc<W>);
world_impl!(W for &W);

/// Helper methods on [`World`] implementations.
pub trait WorldExt {
    /// Get the byte range for a span.
    ///
    /// Returns `None` if the `Span` does not point into any file.
    fn range(&self, span: Span) -> Option<Range<usize>>;
}

impl<T: World + ?Sized> WorldExt for T {
    fn range(&self, span: Span) -> Option<Range<usize>> {
        span.range().or_else(|| self.source(span.id()?).ok()?.range(span))
    }
}

/// Definition of Typst's standard library.
///
/// To create and configure the standard library, use the `LibraryExt` trait
/// and call
/// - `Library::default()` for a standard configuration
/// - `Library::builder().build()` if you want to customize the library
#[derive(Debug, Clone, Hash)]
pub struct Library {
    /// The module that contains the definitions that are available everywhere.
    pub global: Module,
    /// The module that contains the definitions available in math mode.
    pub math: Module,
    /// The default style properties (for page size, font selection, and
    /// everything else configurable via set and show rules).
    pub styles: Styles,
    /// The standard library as a value. Used to provide the `std` module.
    pub std: Binding,
    /// In-development features that were enabled.
    pub features: Features,
}

/// Configurable builder for the standard library.
///
/// Constructed via the `LibraryExt` trait.
#[derive(Debug, Clone)]
pub struct LibraryBuilder {
    routines: &'static Routines,
    inputs: Option<Dict>,
    features: Features,
}

impl LibraryBuilder {
    /// Creates a new builder.
    #[doc(hidden)]
    pub fn from_routines(routines: &'static Routines) -> Self {
        Self {
            routines,
            inputs: None,
            features: Features::default(),
        }
    }

    /// Configure the inputs visible through `sys.inputs`.
    pub fn with_inputs(mut self, inputs: Dict) -> Self {
        self.inputs = Some(inputs);
        self
    }

    /// Configure in-development features that should be enabled.
    ///
    /// No guarantees whatsover!
    pub fn with_features(mut self, features: Features) -> Self {
        self.features = features;
        self
    }

    /// Consumes the builder and returns a `Library`.
    pub fn build(self) -> Library {
        let math = math::module();
        let inputs = self.inputs.unwrap_or_default();
        let global = global(self.routines, math.clone(), inputs, &self.features);
        Library {
            global: global.clone(),
            math,
            styles: Styles::new(),
            std: Binding::detached(global),
            features: self.features,
        }
    }
}

/// A selection of in-development features that should be enabled.
///
/// Can be collected from an iterator of [`Feature`]s.
#[derive(Debug, Default, Clone, Hash)]
pub struct Features(SmallBitSet);

impl Features {
    /// Check whether the given feature is enabled.
    pub fn is_enabled(&self, feature: Feature) -> bool {
        self.0.contains(feature as usize)
    }
}

impl FromIterator<Feature> for Features {
    fn from_iter<T: IntoIterator<Item = Feature>>(iter: T) -> Self {
        let mut set = SmallBitSet::default();
        for feature in iter {
            set.insert(feature as usize);
        }
        Self(set)
    }
}

/// An in-development feature that should be enabled.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum Feature {
    Html,
    A11yExtras,
}

/// A group of related standard library definitions.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    Foundations,
    Introspection,
    Layout,
    DataLoading,
    Math,
    Model,
    Symbols,
    Text,
    Visualize,
    Pdf,
    Html,
    Svg,
    Png,
}

impl Category {
    /// The kebab-case name of the category.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Foundations => "foundations",
            Self::Introspection => "introspection",
            Self::Layout => "layout",
            Self::DataLoading => "data-loading",
            Self::Math => "math",
            Self::Model => "model",
            Self::Symbols => "symbols",
            Self::Text => "text",
            Self::Visualize => "visualize",
            Self::Pdf => "pdf",
            Self::Html => "html",
            Self::Svg => "svg",
            Self::Png => "png",
        }
    }
}

/// Construct the module with global definitions.
fn global(
    routines: &Routines,
    math: Module,
    inputs: Dict,
    features: &Features,
) -> Module {
    let mut global = Scope::deduplicating();

    self::foundations::define(&mut global, inputs, features);
    self::model::define(&mut global);
    self::text::define(&mut global);
    self::layout::define(&mut global);
    self::visualize::define(&mut global);
    self::introspection::define(&mut global);
    self::loading::define(&mut global);
    self::symbols::define(&mut global);

    global.define("math", math);
    global.define("pdf", self::pdf::module(features));
    if features.is_enabled(Feature::Html) {
        global.define("html", (routines.html_module)());
    }

    prelude(&mut global);

    Module::new("global", global)
}

/// Defines scoped values that are globally available, too.
fn prelude(global: &mut Scope) {
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
