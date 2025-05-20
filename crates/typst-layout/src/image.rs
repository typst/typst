use std::ffi::OsStr;

use typst_library::diag::{warning, At, LoadedAt, SourceResult, StrResult};
use typst_library::engine::Engine;
use typst_library::foundations::{Bytes, Derived, Packed, Smart, StyleChain};
use typst_library::introspection::Locator;
use typst_library::layout::{
    Abs, Axes, FixedAlignment, Frame, FrameItem, Point, Region, Size,
};
use typst_library::loading::DataSource;
use typst_library::text::families;
use typst_library::visualize::{
    Curve, ExchangeFormat, Image, ImageElem, ImageFit, ImageFormat, ImageKind,
    RasterImage, SvgImage, VectorFormat,
};

/// Layout the image.
#[typst_macros::time(span = elem.span())]
pub fn layout_image(
    elem: &Packed<ImageElem>,
    engine: &mut Engine,
    _: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let span = elem.span();

    // Take the format that was explicitly defined, or parse the extension,
    // or try to detect the format.
    let Derived { source, derived: data } = &elem.source;
    let format = match elem.format(styles) {
        Smart::Custom(v) => v,
        Smart::Auto => determine_format(source, &data.bytes).at(span)?,
    };

    // Warn the user if the image contains a foreign object. Not perfect
    // because the svg could also be encoded, but that's an edge case.
    if format == ImageFormat::Vector(VectorFormat::Svg) {
        let has_foreign_object =
            memchr::memmem::find(&data.bytes, b"<foreignObject").is_some();

        if has_foreign_object {
            engine.sink.warn(warning!(
                span,
                "image contains foreign object";
                hint: "SVG images with foreign objects might render incorrectly in typst";
                hint: "see https://github.com/typst/typst/issues/1421 for more information"
            ));
        }
    }

    // Construct the image itself.
    let kind = match format {
        ImageFormat::Raster(format) => ImageKind::Raster(
            RasterImage::new(
                data.bytes.clone(),
                format,
                elem.icc(styles).as_ref().map(|icc| icc.derived.clone()),
            )
            .at(span)?,
        ),
        ImageFormat::Vector(VectorFormat::Svg) => ImageKind::Svg(
            SvgImage::with_fonts(
                data.bytes.clone(),
                engine.world,
                &families(styles).map(|f| f.as_str()).collect::<Vec<_>>(),
            )
            .in_text(&data)?,
        ),
    };

    let image = Image::new(kind, elem.alt(styles), elem.scaling(styles));

    // Determine the image's pixel aspect ratio.
    let pxw = image.width();
    let pxh = image.height();
    let px_ratio = pxw / pxh;

    // Determine the region's aspect ratio.
    let region_ratio = region.size.x / region.size.y;

    // Find out whether the image is wider or taller than the region.
    let wide = px_ratio > region_ratio;

    // The space into which the image will be placed according to its fit.
    let target = if region.expand.x && region.expand.y {
        // If both width and height are forced, take them.
        region.size
    } else if region.expand.x {
        // If just width is forced, take it.
        Size::new(region.size.x, region.size.y.min(region.size.x / px_ratio))
    } else if region.expand.y {
        // If just height is forced, take it.
        Size::new(region.size.x.min(region.size.y * px_ratio), region.size.y)
    } else {
        // If neither is forced, take the natural image size at the image's
        // DPI bounded by the available space.
        //
        // Division by DPI is fine since it's guaranteed to be positive.
        let dpi = image.dpi().unwrap_or(Image::DEFAULT_DPI);
        let natural = Axes::new(pxw, pxh).map(|v| Abs::inches(v / dpi));
        Size::new(
            natural.x.min(region.size.x).min(region.size.y * px_ratio),
            natural.y.min(region.size.y).min(region.size.x / px_ratio),
        )
    };

    // Compute the actual size of the fitted image.
    let fit = elem.fit(styles);
    let fitted = match fit {
        ImageFit::Cover | ImageFit::Contain => {
            if wide == (fit == ImageFit::Contain) {
                Size::new(target.x, target.x / px_ratio)
            } else {
                Size::new(target.y * px_ratio, target.y)
            }
        }
        ImageFit::Stretch => target,
    };

    // First, place the image in a frame of exactly its size and then resize
    // the frame to the target size, center aligning the image in the
    // process.
    let mut frame = Frame::soft(fitted);
    frame.push(Point::zero(), FrameItem::Image(image, fitted, span));
    frame.resize(target, Axes::splat(FixedAlignment::Center));

    // Create a clipping group if only part of the image should be visible.
    if fit == ImageFit::Cover && !target.fits(fitted) {
        frame.clip(Curve::rect(frame.size()));
    }

    Ok(frame)
}

/// Try to determine the image format based on the data.
fn determine_format(source: &DataSource, data: &Bytes) -> StrResult<ImageFormat> {
    if let DataSource::Path(path) = source {
        let ext = std::path::Path::new(path.as_str())
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or_default()
            .to_lowercase();

        match ext.as_str() {
            "png" => return Ok(ExchangeFormat::Png.into()),
            "jpg" | "jpeg" => return Ok(ExchangeFormat::Jpg.into()),
            "gif" => return Ok(ExchangeFormat::Gif.into()),
            "svg" | "svgz" => return Ok(VectorFormat::Svg.into()),
            "webp" => return Ok(ExchangeFormat::Webp.into()),
            _ => {}
        }
    }

    Ok(ImageFormat::detect(data).ok_or("unknown image format")?)
}
