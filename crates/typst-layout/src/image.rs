use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::foundations::{Packed, StyleChain};
use typst_library::introspection::Locator;
use typst_library::layout::{
    Abs, Axes, FixedAlignment, Fragment, Frame, FrameItem, Point, Region, Regions, Size,
};
use typst_library::visualize::{Curve, Image, ImageElem, ImageFit};

/// Layout the image.
#[typst_macros::time(span = elem.span())]
pub fn layout_image(
    elem: &Packed<ImageElem>,
    engine: &mut Engine,
    _: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let image = elem.decode(engine, styles)?;

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
    let fit = elem.fit.get(styles);
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
    frame.push(Point::zero(), FrameItem::Image(image, fitted, elem.span()));
    frame.resize(target, Axes::splat(FixedAlignment::Center));

    // Create a clipping group if only part of the image should be visible.
    if fit == ImageFit::Cover && !target.fits(fitted) {
        frame.clip(Curve::rect(frame.size()));
    }

    Ok(frame)
}

/// Layout the image across multiple regions, slicing at page boundaries.
#[typst_macros::time(span = elem.span())]
pub fn layout_image_breakable(
    elem: &Packed<ImageElem>,
    engine: &mut Engine,
    _: Locator,
    styles: StyleChain,
    regions: Regions,
) -> SourceResult<Fragment> {
    let image = elem.decode(engine, styles)?;

    // Determine the image's pixel aspect ratio.
    let pxw = image.width();
    let pxh = image.height();
    let px_ratio = pxw / pxh;

    // Use the base region for sizing (full page dimensions).
    let base = regions.base();
    let expand = regions.expand;

    let region_ratio = base.x / base.y;
    let wide = px_ratio > region_ratio;

    // Compute the full target size the image wants to occupy.
    let target = if expand.x && expand.y {
        base
    } else if expand.x {
        Size::new(base.x, base.x / px_ratio)
    } else if expand.y {
        Size::new(base.y * px_ratio, base.y)
    } else {
        let dpi = image.dpi().unwrap_or(Image::DEFAULT_DPI);
        let natural = Axes::new(pxw, pxh).map(|v| Abs::inches(v / dpi));
        Size::new(natural.x.min(base.x), natural.y.min(base.x / px_ratio))
    };

    // Compute the fitted image size.
    let fit = elem.fit.get(styles);
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

    // Build the full image frame.
    let mut full_frame = Frame::soft(fitted);
    full_frame.push(Point::zero(), FrameItem::Image(image, fitted, elem.span()));
    full_frame.resize(target, Axes::splat(FixedAlignment::Center));

    if fit == ImageFit::Cover && !target.fits(fitted) {
        full_frame.clip(Curve::rect(full_frame.size()));
    }

    let total_height = full_frame.height();

    // Fast path: image fits in the first region.
    if regions.size.y.fits(total_height) || !regions.may_break() {
        return Ok(Fragment::frame(full_frame));
    }

    // Slice the image across regions.
    let mut frames = vec![];
    let mut remaining = total_height;
    let mut y_offset = Abs::zero();
    let mut iter = regions;

    loop {
        let available = iter.size.y;
        let slice_height = remaining.min(available);
        let slice_size = Size::new(target.x, slice_height);

        // Create a frame showing the appropriate vertical slice.
        let mut frame = Frame::soft(full_frame.size());
        for (point, item) in full_frame.items() {
            frame.push(*point, item.clone());
        }
        frame.translate(Point::new(Abs::zero(), -y_offset));
        frame.set_size(slice_size);
        frame.clip(Curve::rect(slice_size));

        frames.push(frame);

        remaining -= slice_height;
        y_offset += slice_height;

        if remaining <= Abs::zero() || !iter.may_break() {
            break;
        }

        iter.next();
    }

    Ok(Fragment::frames(frames))
}
