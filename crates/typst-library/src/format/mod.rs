//! Specify configurable export formats.
//!
//! All export formats have to be registered in [`Library::formats`], usually
//! through [`LibraryBuilder::with_formats()`]. When using the `typst` crate's
//! `LibraryExt::default()` or `LibraryExt::builder()` methods, the standard
//! formats will automatically be registered.
//!
//! Each [`Format`] has an associated [`FormatElement`] with an [`FormatOption`]
//! type. The format element will be defined on the `std.format` binding, making
//! it available to set rules.
//! The options are stored in the compiled document's [`FormatOptions`] and can
//! be accessed using the [`Document::options()`] trait method. The concrete
//! options type can be retreived using [`FormatOptions::get()`] providing the
//! format element as a generic parameter.
//!
//! # Example
//! ```
//! use typst_library::{Library, LibraryBuilder};
//! use typst_library::diag::{SourceResult, bail};
//! use typst_library::engine::Engine;
//! use typst_library::format::{Complete, Format, FormatElement, Fields, Populate, SpannedValue};
//! use typst_library::foundations::{Args, Construct, Content, StyleChain, elem};
//! use typst_library::model::Document;
//! use typst_syntax::Spanned;
//!
//! #[elem(Construct)]
//! pub struct Epub {
//!     #[default]
//!     pub pretty: bool
//! }
//!
//! impl FormatElement for Epub {
//!     type Options = EpubFormatOptions;
//! }
//!
//! impl Construct for Epub {
//!     fn construct(_: &mut Engine, args: &mut Args) -> SourceResult<Content> {
//!         bail!(args.span, "cannot be constructed manually")
//!     }
//! }
//!
//! #[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
//! pub struct EpubFormatOptions<T: Fields = Complete> {
//!     pub pretty: T::Value<Epub, { Epub::pretty.index() }>,
//! }
//!
//! /// `EpubFormatOptions<Complete>` will look like this:
//! pub struct EpubFormatOptionsComplete {
//!     pub size: SpannedValue<Epub, 0>,
//! }
//!
//! /// `EpubFormatOptions<Partial>` will look like this:
//! pub struct EpubFormatOptionsPartial {
//!     pub size: Option<u32>,
//! }
//!
//! impl Populate for EpubFormatOptions {
//!     fn populate(&mut self, styles: Spanned<StyleChain>) {
//!         // The `SpannedValue::populate` call automatically stores the span
//!         // of the specific set rule.
//!         self.pretty.populate(styles);
//!     }
//!
//!     fn dyn_clone(&self) -> Box<dyn Populate> {
//!         Box::new(self.clone())
//!     }
//! }
//!
//! /// Add the epub format to the library.
//! fn setup_library(builder: LibraryBuilder) -> LibraryBuilder {
//!     builder.with_formats([Format::new::<Epub>()])
//! }
//!
//! /// In the export crate.
//! fn export_epub(doc: impl Document) {
//!     let options = doc.options().get::<Epub>();
//!     // Export an epub here...
//! }
//! ```
//!
//! # Internals
//! The [`FormatOptions`] as top-level set-rules have to be special-cased and
//! are handled in a similar way as the [`DocumentInfo`], through the
//! [`RealizationKind::Document`].
//!
//! [`Library::formats`]: crate::Library::formats
//! [`LibraryBuilder::with_formats()`]: crate::LibraryBuilder::with_formats()
//! [`Document::options()`]: crate::model::Document::options()
//! [`DocumentInfo`]: crate::model::DocumentInfo
//! [`RealizationKind::Document`]: crate::routines::RealizationKind::Document

use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use ecow::EcoVec;
use typst_syntax::{Span, Spanned};

use crate::Feature;
use crate::foundations::{
    Element, Field, Module, NativeElement, NativeRuleMap, Scope, SettableProperty,
    StyleChain,
};

/// The global `format` module on which all registered formats will be defined.
pub fn module(formats: &[Format]) -> Module {
    let mut format = Scope::deduplicating();
    format.start_category(crate::Category::Format);

    for f in formats {
        let binding = format.define(f.elem.name(), f.elem);
        if let Some(feature) = f.feature {
            binding.with_feature(feature);
        }
    }

    Module::new("format", format)
}

/// An export format with an associated [`FormatElement`].
///
/// See the [module level](self) docs for more information.
#[derive(Debug, Clone, Hash)]
pub struct Format {
    pub elem: Element,
    options: fn() -> FormatOption,
    feature: Option<Feature>,
    rules: Option<fn(&mut NativeRuleMap)>,
}

impl Format {
    /// Create a new format with an associated [`FormatElement`].
    pub fn new<E: FormatElement>() -> Self {
        Self {
            elem: E::ELEM,
            options: || E::Options::default().into(),
            feature: None,
            rules: None,
        }
    }

    /// Gate the format behind a feature flag.
    pub fn feature(mut self, feature: Feature) -> Self {
        self.feature = Some(feature);
        self
    }

    /// Add format specific rules that will be registered.
    pub fn rules(mut self, register: fn(&mut NativeRuleMap)) -> Self {
        self.rules = Some(register);
        self
    }

    /// Get the default format options of this format.
    pub fn default_options(&self) -> FormatOption {
        (self.options)()
    }

    /// Register the rules specific to this format.
    pub fn register_rules(&self, rules: &mut NativeRuleMap) {
        if let Some(register) = self.rules {
            register(rules);
        }
    }
}

/// An export format element with associated options that can be configured
/// using set rules.
///
/// See the [module level](self) docs for more information.
pub trait FormatElement: NativeElement {
    type Options: Populate + Default + Clone + 'static;
}

/// A type that can be populated from a [`StyleChain`].
///
/// This is used inside [`FormatOption`].
#[expect(private_bounds)]
pub trait Populate: Bounds {
    /// Populate this type with details from the given local styles.
    fn populate(&mut self, styles: Spanned<StyleChain>);
}

trait Bounds: Send + Sync + Any + 'static {
    fn dyn_clone(&self) -> Box<dyn Any>;
    fn dyn_hash(&self, state: &mut dyn std::hash::Hasher);
    fn dyn_debug(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result;
}

impl<T: Clone + Hash + Debug + Send + Sync + 'static> Bounds for T {
    fn dyn_clone(&self) -> Box<dyn Any> {
        Box::new(self.clone())
    }

    fn dyn_hash(&self, mut state: &mut dyn std::hash::Hasher) {
        // Also hash the TypeId since values with different types but
        // equal data should be different.
        TypeId::of::<Self>().hash(&mut state);
        self.hash(&mut state);
    }

    fn dyn_debug(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.fmt(f)
    }
}

/// A list of format options that have been configured by the document.
///
/// See the [module level](self) docs for more information.
#[derive(Debug, Clone, Hash)]
pub struct FormatOptions(EcoVec<FormatOption>);

impl FormatOptions {
    /// Initialize default format options from a list of formats.
    pub fn new(formats: &[Format]) -> Self {
        Self(formats.iter().map(Format::default_options).collect())
    }

    /// Get a concrete format option type.
    ///
    /// For example with `typst_pdf`, the `PdfFormatOptions` can be retrieved
    /// like this on a compiled document.
    /// ```rust,ignore
    /// let pdf_options = document.options().get::<typst_pdf::Pdf>();
    /// ```
    pub fn get<T: FormatElement>(&self) -> &T::Options {
        // TODO: Maybe just return default options, if the document doesn't have
        // the format registered?
        self.0
            .iter()
            .find_map(FormatOption::downcast::<T>)
            .unwrap_or_else(|| {
                let format = std::any::type_name::<T>();
                let options = std::any::type_name::<T::Options>();
                panic!(
                    "format `{format}` with options `{options}` not found\n\
                     hint: if you're a developer, you need to register `Library::formats`"
                );
            })
    }

    /// Populate the format options with details from the given styles.
    pub fn populate(&mut self, styles: Spanned<StyleChain>) {
        for o in self.0.make_mut() {
            o.populate(styles);
        }
    }
}

pub struct FormatOption(Box<dyn Populate>);

impl FormatOption {
    /// Populate these options from the spanned styles.
    pub fn populate(&mut self, styles: Spanned<StyleChain>) {
        self.0.populate(styles);
    }

    /// Attempt to downcast this option to a concrete format option type.
    pub fn downcast<T: FormatElement>(&self) -> Option<&T::Options> {
        let inner: &dyn Populate = &*self.0;
        (inner as &dyn Any).downcast_ref()
    }
}

impl<T: Populate> From<T> for FormatOption {
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}

impl Clone for FormatOption {
    fn clone(&self) -> Self {
        let reference: &(dyn Populate + 'static) = &*self.0;
        let cloned = self.dyn_clone();
        // SAFETY: `self.0` is required to implement `Populate`, thus the cloned
        // `Box<dyn Any>` can be transformed into a `Box<dyn Populate>` using
        // the vtable of `self.0`.
        Self(unsafe { typst_utils::fat::cast_box(reference, cloned) })
    }
}

impl Hash for FormatOption {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.dyn_hash(state);
    }
}

impl Debug for FormatOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.dyn_debug(f)
    }
}

/// A trait that allows specifying [`Complete`] and [`Partial`] types.
///
/// Types can take a generic marker and wrap their fields in the
/// [`Fields::Value`] type, which will either wrap them in an `Option` for the
/// [`Partial`] tag or required for the [`Complete`] tag.
///
/// See the [module level](self) docs for more information.
pub trait Fields: Default {
    type Value<E, const I: u8>: Debug + Default + Clone + Eq + Hash
    where
        E: SettableProperty<I>,
        E::Type: Debug + Clone + Eq + Hash;
}

/// Marker for types with fully resolved fields.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Complete;

impl Fields for Complete {
    type Value<E, const I: u8>
        = SpannedValue<E, I>
    where
        E: SettableProperty<I>,
        E::Type: Debug + Clone + Eq + Hash;
}

/// A [`Spanned`] value that is associtated with a specific [`Field`] of an
/// element. This allows reusing the [`Default`] value specified on the element,
/// by reading from an empty [`StyleChain`].
#[derive(Debug, Clone, Hash)]
pub struct SpannedValue<E, const I: u8>
where
    E: SettableProperty<I>,
{
    /// The format element's field this value should be read from.
    field: PhantomData<Field<E, I>>,
    pub v: E::Type,
    pub span: Span,
}

impl<E, const I: u8> SpannedValue<E, I>
where
    E: SettableProperty<I>,
{
    /// Create a new spanned value.
    pub fn new(v: E::Type, span: Span) -> Self {
        Self { span, v, field: PhantomData }
    }

    /// Create a new spanned value with a detached span.
    pub fn detached(v: E::Type) -> Self {
        Self::new(v, Span::detached())
    }

    /// Populate this value from the given local styles.
    /// If the field has been set, this will also update the span.
    pub fn populate(&mut self, styles: Spanned<StyleChain>) {
        if styles.v.has(Field::<E, I>::new()) {
            *self =
                SpannedValue::new(styles.v.get_cloned(Field::<E, I>::new()), styles.span);
        }
    }
}

impl<E, const I: u8> Default for SpannedValue<E, I>
where
    E: SettableProperty<I>,
    E::Type: Debug + Clone + Eq + Hash,
{
    fn default() -> Self {
        Self::new(
            StyleChain::default().get_cloned(Field::<E, I>::new()),
            Span::detached(),
        )
    }
}

impl<E, const I: u8> Copy for SpannedValue<E, I>
where
    E: SettableProperty<I>,
    E::Type: Copy,
{
}

impl<E, const I: u8> Eq for SpannedValue<E, I>
where
    E: SettableProperty<I>,
    E::Type: Eq,
{
}

impl<E, const I: u8> PartialEq for SpannedValue<E, I>
where
    E: SettableProperty<I>,
    E::Type: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.v == other.v && self.span == other.span
    }
}

impl<E, const I: u8> From<Spanned<E::Type>> for SpannedValue<E, I>
where
    E: SettableProperty<I>,
{
    fn from(value: Spanned<E::Type>) -> Self {
        Self::new(value.v, value.span)
    }
}

/// Marker for types with optional/partial fields.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Partial;

impl Fields for Partial {
    type Value<E, const I: u8>
        = Option<E::Type>
    where
        E: SettableProperty<I>,
        E::Type: Debug + Clone + Eq + Hash;
}

impl Partial {
    /// If present, returns the partial value with a detached span, otherwise
    /// the default value is returned.
    pub fn resolve<E, const I: u8>(
        partial: <Partial as Fields>::Value<E, I>,
        default: <Complete as Fields>::Value<E, I>,
    ) -> <Complete as Fields>::Value<E, I>
    where
        E: SettableProperty<I>,
        E::Type: Debug + Copy + Clone + Eq + Hash,
    {
        partial.map(SpannedValue::detached).unwrap_or(default)
    }

    /// If present, returns the partial value with a detached span, otherwise
    /// the default value is returned.
    pub fn resolve_cloned<E, const I: u8>(
        partial: &<Partial as Fields>::Value<E, I>,
        default: &<Complete as Fields>::Value<E, I>,
    ) -> <Complete as Fields>::Value<E, I>
    where
        E: SettableProperty<I>,
        E::Type: Debug + Clone + Eq + Hash,
    {
        partial
            .clone()
            .map(SpannedValue::detached)
            .unwrap_or_else(|| default.clone())
    }
}
