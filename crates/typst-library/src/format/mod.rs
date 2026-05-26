use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::hash::Hash;

use ecow::EcoVec;

use crate::Feature;
use crate::foundations::{
    Element, Module, NativeElement, NativeRuleMap, Scope, StyleChain,
};

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

// TODO: docs
#[derive(Debug, Clone, Hash)]
pub struct Format {
    pub elem: Element,
    options: fn() -> FormatOption,
    feature: Option<Feature>,
    rules: Option<fn(&mut NativeRuleMap)>,
}

impl Format {
    /// Create a new format with an associated [`FormatElement`].
    pub const fn new<E: FormatElement>() -> Self {
        Self {
            elem: E::ELEM,
            options: || E::Options::default().into(),
            feature: None,
            rules: None,
        }
    }

    /// Gate the format behind a feature flag.
    pub const fn with_feature(mut self, feature: Feature) -> Self {
        self.feature = Some(feature);
        self
    }

    /// Add format specific rules that will be registered.
    pub const fn with_rules(mut self, register: fn(&mut NativeRuleMap)) -> Self {
        self.rules = Some(register);
        self
    }

    pub fn default_options(&self) -> FormatOption {
        (self.options)()
    }

    pub fn register_rules(&self, rules: &mut NativeRuleMap) {
        if let Some(register) = self.rules {
            register(rules);
        }
    }
}

pub trait FormatElement: NativeElement {
    type Options: Populate + Default + Clone + Hash;
}

/// A type that can be populated from a [`StyleChain`].
///
/// This is used inside [`FormatOption`].
pub trait Populate: Debug + Any + Send + Sync + 'static {
    /// Populate this type with details from the given local styles.
    fn populate(&mut self, styles: StyleChain);
}

trait Bounds: Populate {
    fn dyn_clone(&self) -> Box<dyn Bounds>;
    fn dyn_hash(&self, state: &mut dyn std::hash::Hasher);
}

impl<T> Bounds for T
where
    T: Populate + Clone + Hash,
{
    fn dyn_clone(&self) -> Box<dyn Bounds> {
        Box::new(self.clone())
    }

    fn dyn_hash(&self, mut state: &mut dyn std::hash::Hasher) {
        // Also hash the TypeId since values with different types but
        // equal data should be different.
        TypeId::of::<Self>().hash(&mut state);
        self.hash(&mut state);
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
                let format = std::any::type_name::<T>();
                let options = std::any::type_name::<T::Options>();
                panic!(
                    "format `{format}` with options `{options}` not found\n\
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

pub struct FormatOption(Box<dyn Bounds>);

impl FormatOption {
    pub fn populate(&mut self, styles: StyleChain) {
        self.0.populate(styles);
    }

    pub fn downcast<T: FormatElement>(&self) -> Option<&T::Options> {
        let inner: &dyn Bounds = &*self.0;
        (inner as &dyn Any).downcast_ref()
    }
}

impl<T> From<T> for FormatOption
where
    T: Populate + Clone + Hash + Debug + Send + Sync + 'static,
{
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
        self.0.fmt(f)
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
