//! System-related things.

use crate::{
    Feature,
    foundations::{
        Args, Construct, Content, Dict, Engine, Module, Scope, SourceResult, Version,
        bail, cast, elem,
    },
};

use super::{StyleChain, Styles};

/// A module with system-related things.
pub fn module(inputs: Dict) -> Module {
    let mut scope = Scope::deduplicating();
    scope.define(
        "version",
        Version::from_iter([
            env!("CARGO_PKG_VERSION_MAJOR").parse::<u32>().unwrap(),
            env!("CARGO_PKG_VERSION_MINOR").parse::<u32>().unwrap(),
            env!("CARGO_PKG_VERSION_PATCH").parse::<u32>().unwrap(),
        ]),
    );
    scope.define("inputs", inputs);
    scope.define_elem::<DefaultsElem>();
    Module::new("sys", scope)
}

#[derive(Debug, Default)]
pub struct Defaults {
    pub format: Format,
    pub features: Vec<Feature>,
}

#[elem(Construct)]
pub struct DefaultsElem {
    #[ghost]
    pub format: Format,
    #[ghost]
    pub features: Vec<Feature>,
}

impl Construct for DefaultsElem {
    fn construct(_: &mut Engine, args: &mut Args) -> SourceResult<Content> {
        bail!(args.span, "can only be used in set rules")
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    #[default]
    Pdf,
    Png,
    Svg,
    Html,
}

cast! {
    Format,
    self => match self {
        Self::Pdf => "pdf",
        Self::Png => "png",
        Self::Svg => "svg",
        Self::Html => "html"
    }.into_value(),
    "pdf" => Self::Pdf,
    "png" => Self::Png,
    "svg" => Self::Svg,
    "html" => Self::Html,
}

cast! {
    Feature,
    self => match self {
        Self::Html => "html",
        Self::A11yExtras => "a11y-extras",
    }.into_value(),
    "html" => Self::Html,
    "a11y-extras" => Self::A11yExtras,
}

impl Defaults {
    pub fn populate(&mut self, styles: &Styles) {
        let chain = StyleChain::new(styles);
        if styles.has(DefaultsElem::format) {
            self.format = chain.get(DefaultsElem::format);
        }
        if styles.has(DefaultsElem::features) {
            self.features = chain.get_cloned(DefaultsElem::features);
        }
    }
}
