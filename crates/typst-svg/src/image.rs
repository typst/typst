use base64::Engine;
use ecow::{eco_format, EcoString};
use image::{codecs::png::PngEncoder, ImageEncoder};
use typst_library::foundations::Smart;
use typst_library::layout::{Abs, Axes};
use typst_library::visualize::{
    ExchangeFormat, Image, ImageFormat, ImageKind, ImageScaling, RasterFormat,
    VectorFormat,
};

use crate::SVGRenderer;

impl SVGRenderer {
    /// Render an image element.
    pub(super) fn render_image(&mut self, image: &Image, size: &Axes<Abs>) {
        let url = convert_image_to_base64_url(image);
        self.xml.start_element("image");
        self.xml.write_attribute("xlink:href", &url);
        self.xml.write_attribute("width", &size.x.to_pt());
        self.xml.write_attribute("height", &size.y.to_pt());
        self.xml.write_attribute("preserveAspectRatio", "none");
        match image.scaling() {
            Smart::Auto => {}
            Smart::Custom(ImageScaling::Smooth) => {
                // This is still experimental and not implemented in all major browsers.
                // https://developer.mozilla.org/en-US/docs/Web/CSS/image-rendering#browser_compatibility
                self.xml.write_attribute("style", "image-rendering: smooth")
            }
            Smart::Custom(ImageScaling::Pixelated) => {
                self.xml.write_attribute("style", "image-rendering: pixelated")
            }
        }
        self.xml.end_element();
    }
}

/// Encode an image into a data URL. The format of the URL is
/// `data:image/{format};base64,`.
#[comemo::memoize]
pub fn convert_image_to_base64_url(image: &Image) -> EcoString {
    let format = match image.format() {
        ImageFormat::Raster(RasterFormat::Exchange(f)) => match f {
            ExchangeFormat::Png => "png",
            ExchangeFormat::Jpg => "jpeg",
            ExchangeFormat::Gif => "gif",
        },
        ImageFormat::Raster(RasterFormat::Pixel(_)) => "png",
        ImageFormat::Vector(f) => match f {
            VectorFormat::Svg => "svg+xml",
        },
    };

    let mut buf;
    let data = match image.kind() {
        ImageKind::Raster(raster) => match raster.format() {
            RasterFormat::Exchange(_) => raster.data(),
            RasterFormat::Pixel(_) => {
                buf = vec![];
                let mut encoder = PngEncoder::new(&mut buf);
                if let Some(icc_profile) = raster.icc() {
                    encoder.set_icc_profile(icc_profile.to_vec()).ok();
                }
                raster.dynamic().write_with_encoder(encoder).unwrap();
                buf.as_slice()
            }
        },
        ImageKind::Svg(svg) => svg.data(),
    };

    let mut url = eco_format!("data:image/{format};base64,");
    let data = base64::engine::general_purpose::STANDARD.encode(data);
    url.push_str(&data);
    url
}
