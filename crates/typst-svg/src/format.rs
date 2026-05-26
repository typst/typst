use typst_library::diag::{SourceResult, bail};
use typst_library::engine::Engine;
use typst_library::format::{Complete, Fields, Format, FormatElement, Partial, Populate};
use typst_library::foundations::{Args, Construct, Content, StyleChain};
use typst_macros::elem;

pub fn format() -> Format {
    Format::new::<Svg>()
}

/// The SVG format.
#[elem(Construct)]
pub struct Svg {
    /// Wether to format the SVG in a human readable way.
    #[default(false)]
    pub pretty: bool,
}

impl FormatElement for Svg {
    type Options = SvgFormatOptions;
}

impl Construct for Svg {
    fn construct(_: &mut Engine, args: &mut Args) -> SourceResult<Content> {
        bail!(args.span, "cannot be constructed manually")
    }
}

/// Document settings for PNG  export.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct SvgFormatOptions<F: Fields = Complete> {
    pub pretty: F::Value<bool>,
}

impl SvgFormatOptions {
    /// Retrieve SVG format options from the style chain.
    pub fn get_in(styles: StyleChain) -> Self {
        Self { pretty: styles.get(Svg::pretty) }
    }
}

impl SvgFormatOptions<Partial> {
    /// Resolves the [`Partial`] options to [`Complete`] ones, given defaults.
    pub fn resolve(&self, default: &SvgFormatOptions) -> SvgFormatOptions {
        SvgFormatOptions { pretty: self.pretty.unwrap_or(default.pretty) }
    }
}

impl Populate for SvgFormatOptions {
    fn populate(&mut self, styles: StyleChain) {
        *self = Self::get_in(styles);
    }

    fn dyn_clone(&self) -> Box<dyn Populate> {
        Box::new(self.clone())
    }

    fn describe(&self) -> (&'static str, &'static str) {
        (std::any::type_name::<Svg>(), std::any::type_name::<SvgFormatOptions>())
    }
}
