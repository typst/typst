use std::io::Cursor;

use base64::Engine;
use ecow::{eco_format, EcoString};
use image::error::UnsupportedError;
use image::{codecs::png::PngEncoder, ImageEncoder};
use typst_library::layout::{Abs, Axes};
use typst_library::visualize::{
    Image, ImageFormat, ImageKind, ImageScaling, RasterFormat, VectorFormat,
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
            ImageScaling::Auto => {}
            ImageScaling::Smooth => {
                // This is still experimental and not implemented in all major browsers[^1].
                // [^1]: https://developer.mozilla.org/en-US/docs/Web/CSS/image-rendering#browser_compatibility
                self.xml.write_attribute("style", "image-rendering: smooth")
            }
            ImageScaling::Pixelated => {
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
        ImageFormat::Raster(f) => match f {
            RasterFormat::Png => "png",
            RasterFormat::Jpg => "jpeg",
            RasterFormat::Gif => "gif",
        },
        ImageFormat::Vector(f) => match f {
            VectorFormat::Svg => "svg+xml",
        },
        ImageFormat::Pixmap(_) => "png",
    };
    let data_owned;
    let data = match image.kind() {
        ImageKind::Raster(raster) => raster.data(),
        ImageKind::Svg(svg) => svg.data(),
        ImageKind::Pixmap(pixmap) => {
            let mut data = Cursor::new(vec![]);
            let mut encoder = PngEncoder::new(&mut data);
            if let Some(icc_profile) = pixmap.icc_profile() {
                let _: Result<(), UnsupportedError> =
                    encoder.set_icc_profile(icc_profile.to_vec());
            }
            pixmap.to_image().write_with_encoder(encoder).unwrap();
            data_owned = data.into_inner();
            &*data_owned
        }
    };

    let mut url = eco_format!("data:image/{format};base64,");
    let data = base64::engine::general_purpose::STANDARD.encode(data);
    url.push_str(&data);
    url
}
