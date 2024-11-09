use std::ffi::OsStr;
use std::ops::Neg;

use typst_library::diag::{bail, warning, At, SourceResult, StrResult};
use typst_library::engine::Engine;
use typst_library::foundations::{Packed, Scope, Smart, StyleChain};
use typst_library::introspection::Locator;
use typst_library::layout::{
    Abs, Axes, FixedAlignment, Frame, FrameItem, GroupItem, Point, Region,
    Size,
};
use typst_library::loading::Readable;
use typst_library::routines::EvalMode;
use typst_library::text::families;
use typst_library::visualize::{
    Image, ImageElem, ImageFit, ImageFormat, ImageKind, Path, RasterFormat, SvgImage,
    VectorFormat,
};
use typst_syntax::Span;

use usvg::{tiny_skia_path, Node, TextAnchor, Transform};

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
    let data = elem.data();
    let format = match elem.format(styles) {
        Smart::Custom(v) => v,
        Smart::Auto => determine_format(elem.path().as_str(), data).at(span)?,
    };

    // Warn the user if the image contains a foreign object. Not perfect
    // because the svg could also be encoded, but that's an edge case.
    if format == ImageFormat::Vector(VectorFormat::Svg) {
        let has_foreign_object =
            data.as_str().is_some_and(|s| s.contains("<foreignObject"));

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
    let image = Image::with_fonts(
        data.clone().into(),
        format,
        elem.alt(styles),
        engine.world,
        elem.eval(styles),
        &families(styles).collect::<Vec<_>>(),
    )
    .at(span)?;

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
    frame.push(Point::zero(), FrameItem::Image(image.clone(), fitted, span));
    let mut clip = false;
    if let ImageKind::Svg(svg) = image.kind() {
        /// The idea is as follows: By allowing to "evaluate" SVGs, users can insert arbitrary Typst
        /// markup into their SVG and have it interpreted as such, instead of being interpreted as
        /// normal text. This allows for example for the inclusion of math content in SVGs, or
        /// ensure that SVG text looks consistent to other text in the document.
        ///
        /// The algorithm is as follows:
        /// - Any text node will be given visibility hidden (this already happened when generating
        ///   the usvg tree).
        /// - We iterate over all text nodes, determine their position and calculate what
        ///   their position would be after evaluating all transforms.
        /// - We calculate the rotation of the text, and also resolve the `text-anchor` property
        /// - We evaluate the SVG text and then place it accordingly.
        fn eval_text_nodes(
            svg: &SvgImage,
            group: &usvg::Group,
            image_size: Size,
            engine: &mut Engine,
            span: Span,
            styles: StyleChain,
            parent_frame: &mut Frame,
        ) -> SourceResult<()> {
            for child in group.children() {
                match child {
                    Node::Group(g) => {
                        eval_text_nodes(
                            svg,
                            g,
                            image_size,
                            engine,
                            span,
                            styles,
                            parent_frame,
                        )?;
                    }
                    Node::Text(t) => {
                        for chunk in t.chunks() {
                            // Calculate the x and y coordinates of the text node in Typst coordinates,
                            // after all SVG transforms have been applied.
                            let (x, y) = {
                                let x_scale =
                                    image_size.x.to_raw() as f32 / svg.width() as f32;
                                let y_scale =
                                    image_size.y.to_raw() as f32 / svg.height() as f32;

                                let x = chunk.x().unwrap_or(0.0);
                                let y = chunk.y().unwrap_or(0.0);
                                // Determine the actual x/y coordinates of the text nodes after all
                                // SVG transforms have been applied.
                                let mut pos = tiny_skia_path::Point::from_xy(x, y);
                                t.abs_transform().map_point(&mut pos);

                                // Convert to Typst coordinates
                                (pos.x * x_scale, pos.y * y_scale)
                            };

                            let val = (engine.routines.eval_string)(
                                engine.routines,
                                engine.world,
                                chunk.text(),
                                span,
                                EvalMode::Markup,
                                Scope::new(),
                            )?
                            .display();

                            let locator = Locator::root();
                            let region = Region::new(
                                Size::new(Abs::inf(), Abs::inf()),
                                Axes::splat(false),
                            );
                            let inner_frame = crate::layout_frame(
                                engine, &val, locator, styles, region,
                            )?;

                            let anchor_shift = match chunk.anchor() {
                                TextAnchor::Start => 0.0,
                                TextAnchor::Middle => {
                                    -(inner_frame.size().x.to_raw() / 2.0) as f32
                                }
                                TextAnchor::End => -inner_frame.size().x.to_raw() as f32,
                            };

                            let baseline = inner_frame.baseline();
                            let mut text_frame = GroupItem::new(inner_frame);

                            // Calculate how much the text is rotated.
                            let rotation = -t
                                .abs_transform()
                                .kx
                                .atan2(t.abs_transform().sx)
                                .to_degrees();

                            // Put everything together!
                            let transform =
                                // Shift text to actual position.
                                Transform::from_translate(x, y)
                                    // Apply rotation to the text.
                                    .pre_concat(Transform::from_rotate(rotation))
                                    // Account for the `text-anchor` property in SVG.
                                    .pre_concat(Transform::from_translate(anchor_shift, 0.0))
                                    // Shift baseline, since Typst text is placed on the top side,
                                    // while SVG text is placed on the bottom side.
                                    .pre_concat(Transform::from_translate(0.0, baseline.neg().to_raw() as f32));

                            text_frame.transform = transform.into();
                            parent_frame
                                .push(Point::zero(), FrameItem::Group(text_frame));
                        }
                    }
                    _ => {}
                }
            }

            Ok(())
        }

        if elem.eval(styles) {
            clip = true;

            eval_text_nodes(
                svg,
                svg.tree().root(),
                fitted,
                engine,
                span,
                styles,
                &mut frame,
            )?;
        }
    }
    frame.resize(target, Axes::splat(FixedAlignment::Center));

    // Create a clipping group if only part of the image should be visible,
    // or if the SVG is evaluated, in which overlaid text might exceed the
    // bounding box.
    clip |= fit == ImageFit::Cover && !target.fits(fitted);

    if clip {
        frame.clip(Path::rect(frame.size()));
    }

    Ok(frame)
}

/// Determine the image format based on path and data.
fn determine_format(path: &str, data: &Readable) -> StrResult<ImageFormat> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_lowercase();

    Ok(match ext.as_str() {
        "png" => ImageFormat::Raster(RasterFormat::Png),
        "jpg" | "jpeg" => ImageFormat::Raster(RasterFormat::Jpg),
        "gif" => ImageFormat::Raster(RasterFormat::Gif),
        "svg" | "svgz" => ImageFormat::Vector(VectorFormat::Svg),
        _ => match &data {
            Readable::Str(_) => ImageFormat::Vector(VectorFormat::Svg),
            Readable::Bytes(bytes) => match RasterFormat::detect(bytes) {
                Some(f) => ImageFormat::Raster(f),
                None => bail!("unknown image format"),
            },
        },
    })
}
