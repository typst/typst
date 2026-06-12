mod pdf;

use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::hash::Hash;

use ecow::EcoVec;

use crate::Feature;
use crate::foundations::{Element, Module, NativeElement, Scope, StyleChain};

pub use self::pdf::*;

pub fn module(formats: &[Format]) -> Module {
    let mut format = Scope::deduplicating();
    format.start_category(crate::Category::Format);

    for f in formats.iter() {
        let binding = format.define(f.elem.name(), f.elem);
        if let Some(feature) = f.feature {
            binding.feature(feature);
        }
    }

    Module::new("format", format)
}

// TODO: docs
#[derive(Debug, Clone, Hash)]
pub struct Format {
    pub elem: Element,
    options: fn() -> FormatOption,
    feature: Option<Feature>,
}

impl Format {
    pub fn new<E: FormatElement>() -> Self {
        Self {
            elem: E::ELEM,
            options: || E::Options::default().into(),
            feature: None,
        }
    }

    pub fn feature(mut self, feature: Feature) -> Self {
        self.feature = Some(feature);
        self
    }

    pub fn default_options(&self) -> FormatOption {
        (self.options)()
    }
}

pub trait FormatElement: NativeElement {
    type Options: Populate + Default + Clone + 'static;
}

/// A type that can be populated from a [`StyleChain`].
#[expect(private_bounds)]
pub trait Populate: Bounds {
    /// Populate this type with details from the given styles.
    fn populate(&mut self, styles: StyleChain);

    fn dyn_clone(&self) -> Box<dyn Populate>;

    fn describe(&self) -> (&'static str, &'static str);
}

trait Bounds: Send + Sync + Any + 'static {
    fn dyn_hash(&self, state: &mut dyn std::hash::Hasher);

    fn dyn_debug(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result;
}

impl<T: Hash + Debug + Send + Sync + 'static> Bounds for T {
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

/// A list of formatoptions
#[derive(Debug, Clone, Hash)]
pub struct FormatOptions(EcoVec<FormatOption>);

impl FormatOptions {
    /// Get a concrete format option type.
    pub fn get<T: FormatElement>(&self) -> &T::Options {
        // TODO: Maybe just return default options, if the document doesn't have
        // the format registered?
        self.0
            .iter()
            .find_map(FormatOption::downcast::<T>)
            .unwrap_or_else(|| {
                let list = typst_utils::display(|f| {
                    if self.0.is_empty() {
                        f.write_str("  none")?;
                    }
                    for o in self.0.iter() {
                        let (format, options) = o.0.describe();
                        writeln!(f, "- Format `{format}` with options `{options:?}`")?;
                    }
                    Ok(())
                });
                let format = std::any::type_name::<T>();
                let options = std::any::type_name::<T::Options>();
                panic!(
                    "Format `{format}` with type `{options}` not found, \
                    available are:\n{list}\n \
                    hint: if you're a developer, you need to register `Library::formats`"
                );
            })
    }
}

impl FormatOptions {
    /// Initialize default format options from a list of formats.
    pub fn new(formats: &[Format]) -> Self {
        Self(formats.iter().map(Format::default_options).collect())
    }

    /// Populate the format options with details from the given styles.
    pub fn populate(&mut self, styles: StyleChain) {
        // TODO: More fine-grained field assignments that track spans?
        // - Possibly use a map from Elements to options?
        for o in self.0.make_mut() {
            o.populate(styles);
        }
    }
}

pub struct FormatOption(Box<dyn Populate>);

impl FormatOption {
    pub fn populate(&mut self, styles: StyleChain) {
        self.0.populate(styles);
    }

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
        Self(self.0.dyn_clone())
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
/// # Example
/// ```
/// use typst_library::format::{Fields, Complete};
/// struct Options<T: Fields = Complete> {
///     size: T::Value<u32>,
/// }
///
/// /// `Options<Complete>` will look like this:
/// struct OptionsComplete {
///     size: u32,
/// }
///
/// /// `Options<Partial>` will look like this:
/// struct OptionsPartial {
///     size: Option<u32>,
/// }
/// ```
pub trait Fields: Default {
    type Value<T: Debug + Clone + Eq + PartialEq + Hash + Default>: Debug
        + Clone
        + Default
        + Eq
        + PartialEq
        + Hash;
}

/// Marker for types with fully resolved fields.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct Complete;

impl Fields for Complete {
    type Value<T: Debug + Default + Clone + Eq + PartialEq + Hash> = T;
}

/// Marker for types with optional/partial fields.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct Partial;

impl Fields for Partial {
    type Value<T: Debug + Default + Clone + Eq + PartialEq + Hash> = Option<T>;
}
