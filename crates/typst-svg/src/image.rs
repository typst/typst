use std::sync::Arc;

use base64::Engine;
use ecow::{EcoString, eco_format};
use hayro::{FontData, FontQuery, InterpreterSettings, RenderSettings, StandardFont};
use image::{ImageEncoder, codecs::png::PngEncoder};
use typst_library::foundations::Smart;
use typst_library::layout::{Abs, Axes};
use typst_library::visualize::{
    ExchangeFormat, Image, ImageKind, ImageScaling, PdfImage, RasterFormat,
};

use crate::SVGRenderer;

impl SVGRenderer<'_> {
    /// Render an image element.
    pub(super) fn render_image(&mut self, image: &Image, size: &Axes<Abs>) {
        let url = convert_image_to_base64_url(image);
        self.xml.start_element("image");
        self.xml.write_attribute("xlink:href", &url);
        self.xml.write_attribute("width", &size.x.to_pt());
        self.xml.write_attribute("height", &size.y.to_pt());
        self.xml.write_attribute("preserveAspectRatio", "none");
        if let Some(value) = convert_image_scaling(image.scaling()) {
            self.xml
                .write_attribute("style", &format_args!("image-rendering: {value}"))
        }
        self.xml.end_element();
    }
}

/// Converts an image scaling to a CSS `image-rendering` propery value.
pub fn convert_image_scaling(scaling: Smart<ImageScaling>) -> Option<&'static str> {
    match scaling {
        Smart::Auto => None,
        Smart::Custom(ImageScaling::Smooth) => {
            // This is still experimental and not implemented in all major browsers.
            // https://developer.mozilla.org/en-US/docs/Web/CSS/image-rendering#browser_compatibility
            Some("smooth")
        }
        Smart::Custom(ImageScaling::Pixelated) => Some("pixelated"),
    }
}

/// Encode an image into a data URL. The format of the URL is
/// `data:image/{format};base64,`.
#[comemo::memoize]
pub fn convert_image_to_base64_url(image: &Image) -> EcoString {
    let mut buf;
    let (format, data): (&str, &[u8]) = match image.kind() {
        ImageKind::Raster(raster) => match raster.format() {
            RasterFormat::Exchange(format) => (
                match format {
                    ExchangeFormat::Png => "png",
                    ExchangeFormat::Jpg => "jpeg",
                    ExchangeFormat::Gif => "gif",
                    ExchangeFormat::Webp => "webp",
                },
                raster.data(),
            ),
            RasterFormat::Pixel(_) => ("png", {
                buf = vec![];
                let mut encoder = PngEncoder::new(&mut buf);
                if let Some(icc_profile) = raster.icc() {
                    encoder.set_icc_profile(icc_profile.to_vec()).ok();
                }
                raster.dynamic().write_with_encoder(encoder).unwrap();
                buf.as_slice()
            }),
        },
        ImageKind::Svg(svg) => ("svg+xml", svg.data()),
        ImageKind::Pdf(pdf) => {
            // To make sure the image isn't pixelated, we always scale up so the
            // lowest dimension has at least 1000 pixels. However, we only scale
            // up as much so that the largest dimension doesn't exceed 3000
            // pixels.
            const MIN_RES: f32 = 1000.0;
            const MAX_RES: f32 = 3000.0;

            let base_width = pdf.width();
            let w_scale = (MIN_RES / base_width).max(MAX_RES / base_width);
            let base_height = pdf.height();
            let h_scale = (MIN_RES / base_height).min(MAX_RES / base_height);
            let total_scale = w_scale.min(h_scale);
            let width = (base_width * total_scale).ceil() as u32;
            let height = (base_height * total_scale).ceil() as u32;

            buf = pdf_to_png(pdf, width, height);
            ("png", buf.as_slice())
        }
    };

    let mut url = eco_format!("data:image/{format};base64,");
    let data = base64::engine::general_purpose::STANDARD.encode(data);
    url.push_str(&data);
    url
}

// Keep this in sync with `typst-png`!
fn pdf_to_png(pdf: &PdfImage, w: u32, h: u32) -> Vec<u8> {
    let select_standard_font = move |font: StandardFont| -> Option<(FontData, u32)> {
        let bytes = match font {
            StandardFont::Helvetica => typst_assets::pdf::SANS,
            StandardFont::HelveticaBold => typst_assets::pdf::SANS_BOLD,
            StandardFont::HelveticaOblique => typst_assets::pdf::SANS_ITALIC,
            StandardFont::HelveticaBoldOblique => typst_assets::pdf::SANS_BOLD_ITALIC,
            StandardFont::Courier => typst_assets::pdf::FIXED,
            StandardFont::CourierBold => typst_assets::pdf::FIXED_BOLD,
            StandardFont::CourierOblique => typst_assets::pdf::FIXED_ITALIC,
            StandardFont::CourierBoldOblique => typst_assets::pdf::FIXED_BOLD_ITALIC,
            StandardFont::TimesRoman => typst_assets::pdf::SERIF,
            StandardFont::TimesBold => typst_assets::pdf::SERIF_BOLD,
            StandardFont::TimesItalic => typst_assets::pdf::SERIF_ITALIC,
            StandardFont::TimesBoldItalic => typst_assets::pdf::SERIF_BOLD_ITALIC,
            StandardFont::ZapfDingBats => typst_assets::pdf::DING_BATS,
            StandardFont::Symbol => typst_assets::pdf::SYMBOL,
        };
        Some((Arc::new(bytes), 0))
    };

    let interpreter_settings = InterpreterSettings {
        font_resolver: Arc::new(move |query| match query {
            FontQuery::Standard(s) => select_standard_font(*s),
            FontQuery::Fallback(f) => select_standard_font(f.pick_standard_font()),
        }),
        warning_sink: Arc::new(|_| {}),
    };

    let render_settings = RenderSettings {
        x_scale: w as f32 / pdf.width(),
        y_scale: h as f32 / pdf.height(),
        width: Some(w as u16),
        height: Some(h as u16),
    };

    let hayro_pix = hayro::render(pdf.page(), &interpreter_settings, &render_settings);

    hayro_pix.take_png()
}
