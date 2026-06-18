use typst_library::diag::{SourceResult, bail};
use typst_library::engine::Engine;
use typst_library::format::{Complete, Fields, Format, FormatElement, Partial, Populate};
use typst_library::foundations::{Args, Construct, Content, StyleChain};
use typst_macros::elem;
use typst_syntax::Spanned;
use typst_utils::Scalar;

pub fn format() -> Format {
    Format::new::<Png>()
}

/// The PNG format.
#[elem(Construct)]
pub struct Png {
    /// The PPI (pixels per inch) to use for PNG export.
    #[default(Scalar::new(144.0))]
    pub ppi: Scalar,
}

impl FormatElement for Png {
    type Options = PngFormatOptions;
}

impl Construct for Png {
    fn construct(_: &mut Engine, args: &mut Args) -> SourceResult<Content> {
        bail!(args.span, "cannot be constructed manually")
    }
}

/// Document settings for PNG  export.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct PngFormatOptions<F: Fields = Complete> {
    /// The number of pixels per point to render at when exporting a PNG.
    pub pixel_per_pt: F::Value<Png, { Png::ppi.index() }>,
}

impl Populate for PngFormatOptions {
    fn populate(&mut self, styles: Spanned<StyleChain>) {
        // VOLATILE: This must be updated when adding more fields.
        self.pixel_per_pt.populate(styles);
    }

    fn dyn_clone(&self) -> Box<dyn Populate> {
        Box::new(self.clone())
    }

    fn describe(&self) -> (&'static str, &'static str) {
        (std::any::type_name::<Png>(), std::any::type_name::<PngFormatOptions>())
    }
}

impl PngFormatOptions<Partial> {
    /// Resolves the [`Partial`] options to [`Complete`] ones, given defaults.
    pub fn resolve(&self, default: &PngFormatOptions) -> PngFormatOptions {
        PngFormatOptions {
            pixel_per_pt: Partial::resolve(self.pixel_per_pt, default.pixel_per_pt),
        }
    }
}
