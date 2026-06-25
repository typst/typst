use typst_library::diag::{SourceResult, bail};
use typst_library::engine::Engine;
use typst_library::format::{Complete, Fields, Format, FormatElement, Partial, Populate};
use typst_library::foundations::{Args, Construct, Content, StyleChain};
use typst_macros::elem;
use typst_syntax::Spanned;

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
    pub pretty: F::Value<Svg, { Svg::pretty.index() }>,
}

impl SvgFormatOptions<Partial> {
    /// Resolves the [`Partial`] options to [`Complete`] ones, given defaults.
    pub fn resolve(&self, default: &SvgFormatOptions) -> SvgFormatOptions {
        SvgFormatOptions {
            pretty: Partial::resolve(self.pretty, default.pretty),
        }
    }
}

impl Populate for SvgFormatOptions {
    fn populate(&mut self, styles: Spanned<StyleChain>) {
        // VOLATILE: This must be updated when adding more fields.
        self.pretty.populate(styles);
    }
}
