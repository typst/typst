use std::borrow::Cow;
use std::sync::Arc;

use base64::Engine;
use ecow::{eco_format, EcoString};
use hayro::{FontData, FontQuery, InterpreterSettings, RenderSettings, StandardFont};
use image::{codecs::png::PngEncoder, ImageEncoder};
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
    let (format, data): (&str, Cow<[u8]>) = match image.kind() {
        ImageKind::Raster(raster) => match raster.format() {
            RasterFormat::Exchange(format) => (
                match format {
                    ExchangeFormat::Png => "png",
                    ExchangeFormat::Jpg => "jpeg",
                    ExchangeFormat::Gif => "gif",
                    ExchangeFormat::Webp => "webp",
                },
                Cow::Borrowed(raster.data()),
            ),
            RasterFormat::Pixel(_) => ("png", {
                buf = vec![];
                let mut encoder = PngEncoder::new(&mut buf);
                if let Some(icc_profile) = raster.icc() {
                    encoder.set_icc_profile(icc_profile.to_vec()).ok();
                }
                raster.dynamic().write_with_encoder(encoder).unwrap();
                Cow::Borrowed(buf.as_slice())
            }),
        },
        ImageKind::Svg(svg) => ("svg+xml", Cow::Borrowed(svg.data())),
        ImageKind::Pdf(pdf) => {
            // To make sure the image isn't pixelated, we always scale up so the lowest
            // dimension has at least 1000 pixels. However, we only scale up as much so that the
            // largest dimension doesn't exceed 3000 pixels.
            const MIN_RES: f32 = 1000.0;
            const MAX_RES: f32 = 3000.0;

            let base_width = pdf.width();
            let w_scale = (MIN_RES / base_width).max(MAX_RES / base_width);
            let base_height = pdf.height();
            let h_scale = (MIN_RES / base_height).min(MAX_RES / base_height);

            let total_scale = w_scale.min(h_scale);

            let width = (base_width * total_scale).ceil() as u32;
            let height = (base_height * total_scale).ceil() as u32;

            ("png", Cow::Owned(pdf_to_png(pdf, width, height)))
        }
    };

    let mut url = eco_format!("data:image/{format};base64,");
    let data = base64::engine::general_purpose::STANDARD.encode(data);
    url.push_str(&data);
    url
}

// Keep this in sync with `typst-png`!
#[comemo::memoize]
fn pdf_to_png(pdf: &PdfImage, w: u32, h: u32) -> Vec<u8> {
    let sf = pdf.standard_fonts().clone();

    let select_standard_font = move |font: StandardFont| -> Option<(FontData, u32)> {
        let bytes = match font {
            StandardFont::Helvetica => sf.helvetica.normal.clone(),
            StandardFont::HelveticaBold => sf.helvetica.bold.clone(),
            StandardFont::HelveticaOblique => sf.helvetica.italic.clone(),
            StandardFont::HelveticaBoldOblique => sf.helvetica.bold_italic.clone(),
            StandardFont::Courier => sf.courier.normal.clone(),
            StandardFont::CourierBold => sf.courier.bold.clone(),
            StandardFont::CourierOblique => sf.courier.italic.clone(),
            StandardFont::CourierBoldOblique => sf.courier.bold_italic.clone(),
            StandardFont::TimesRoman => sf.times.normal.clone(),
            StandardFont::TimesBold => sf.times.bold.clone(),
            StandardFont::TimesItalic => sf.times.italic.clone(),
            StandardFont::TimesBoldItalic => sf.times.bold_italic.clone(),
            StandardFont::ZapfDingBats => sf.zapf_dingbats.clone().map(|d| (d, 0)),
            StandardFont::Symbol => sf.symbol.clone().map(|d| (d, 0)),
        };

        bytes.map(|d| {
            let font_data: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(d.0.clone());

            (font_data, d.1)
        })
    };

    let interpreter_settings = InterpreterSettings {
        font_resolver: Arc::new(move |query| match query {
            FontQuery::Standard(s) => select_standard_font(*s),
            FontQuery::Fallback(f) => select_standard_font(f.pick_standard_font()),
        }),
        warning_sink: Arc::new(|_| {}),
    };
    let page = pdf.page();

    let render_settings = RenderSettings {
        x_scale: w as f32 / pdf.width(),
        y_scale: h as f32 / pdf.height(),
        width: Some(w as u16),
        height: Some(h as u16),
    };

    let hayro_pix = hayro::render(page, &interpreter_settings, &render_settings);

    hayro_pix.take_png()
}
