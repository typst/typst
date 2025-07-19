use hayro::{FontData, FontQuery, InterpreterSettings, RenderSettings, StandardFont};
use image::imageops::FilterType;
use image::{GenericImageView, Rgba};
use std::sync::Arc;
use tiny_skia as sk;
use tiny_skia::IntSize;
use typst_library::foundations::Smart;
use typst_library::layout::Size;
use typst_library::text::{FontBook, FontStretch, FontStyle, FontVariant, FontWeight};
use typst_library::visualize::{Image, ImageKind, ImageScaling, PdfImage};

use crate::{AbsExt, State};

/// Render a raster or SVG image into the canvas.
pub fn render_image(
    canvas: &mut sk::Pixmap,
    state: State,
    image: &Image,
    size: Size,
) -> Option<()> {
    let ts = state.transform;
    let view_width = size.x.to_f32();
    let view_height = size.y.to_f32();

    // For better-looking output, resize `image` to its final size before
    // painting it to `canvas`. For the math, see:
    // https://github.com/typst/typst/issues/1404#issuecomment-1598374652
    let theta = f32::atan2(-ts.kx, ts.sx);

    // To avoid division by 0, choose the one of { sin, cos } that is
    // further from 0.
    let prefer_sin = theta.sin().abs() > std::f32::consts::FRAC_1_SQRT_2;
    let scale_x =
        f32::abs(if prefer_sin { ts.kx / theta.sin() } else { ts.sx / theta.cos() });

    let aspect = (image.width() as f32) / (image.height() as f32);
    let w = (scale_x * view_width.max(aspect * view_height)).ceil() as u32;
    let h = ((w as f32) / aspect).ceil() as u32;

    let pixmap = build_texture(image, w, h)?;
    let paint_scale_x = view_width / pixmap.width() as f32;
    let paint_scale_y = view_height / pixmap.height() as f32;

    let paint = sk::Paint {
        shader: sk::Pattern::new(
            (*pixmap).as_ref(),
            sk::SpreadMode::Pad,
            sk::FilterQuality::Nearest,
            1.0,
            sk::Transform::from_scale(paint_scale_x, paint_scale_y),
        ),
        ..Default::default()
    };

    let rect = sk::Rect::from_xywh(0.0, 0.0, view_width, view_height)?;
    canvas.fill_rect(rect, &paint, ts, state.mask);

    Some(())
}

/// Prepare a texture for an image at a scaled size.
#[comemo::memoize]
fn build_texture(image: &Image, w: u32, h: u32) -> Option<Arc<sk::Pixmap>> {
    let texture = match image.kind() {
        ImageKind::Raster(raster) => {
            let mut texture = sk::Pixmap::new(w, h)?;
            let w = texture.width();
            let h = texture.height();

            let buf;
            let dynamic = raster.dynamic();
            let resized = if (w, h) == (dynamic.width(), dynamic.height()) {
                // Small optimization to not allocate in case image is not resized.
                dynamic
            } else {
                let upscale = w > dynamic.width();
                let filter = match image.scaling() {
                    Smart::Custom(ImageScaling::Pixelated) => FilterType::Nearest,
                    _ if upscale => FilterType::CatmullRom,
                    _ => FilterType::Lanczos3, // downscale
                };
                buf = dynamic.resize_exact(w, h, filter);
                &buf
            };

            for ((_, _, src), dest) in resized.pixels().zip(texture.pixels_mut()) {
                let Rgba([r, g, b, a]) = src;
                *dest = sk::ColorU8::from_rgba(r, g, b, a).premultiply();
            }

            texture
        }
        ImageKind::Svg(svg) => {
            let mut texture = sk::Pixmap::new(w, h)?;
            let tree = svg.tree();
            let ts = tiny_skia::Transform::from_scale(
                w as f32 / tree.size().width(),
                h as f32 / tree.size().height(),
            );
            resvg::render(tree, ts, &mut texture.as_mut());

            texture
        }
        ImageKind::Pdf(pdf) => build_pdf_texture(pdf, w, h)?,
    };

    Some(Arc::new(texture))
}

// Keep this in sync with `typst-svg`!
fn build_pdf_texture(pdf: &PdfImage, w: u32, h: u32) -> Option<sk::Pixmap> {
    let sf = pdf.standard_fonts().clone();

    let select_standard_font = move |font: StandardFont| -> Option<FontData> {
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
            StandardFont::ZapfDingBats => sf.zapf_dingbats.clone(),
            StandardFont::Symbol => sf.symbol.clone(),
        };

        bytes.map(|d| {
            let font_data: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(d.clone());

            font_data
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

    sk::Pixmap::from_vec(hayro_pix.take_u8(), IntSize::from_wh(w, h)?)
}
