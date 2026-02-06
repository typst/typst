use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::foundations::{Packed, StyleChain};
use typst_library::introspection::Locator;
use typst_library::layout::{
    Abs, Axes, FixedAlignment, Frame, FrameItem, Point, Region, Size,
};
use typst_library::visualize::{Curve, Image, ImageFit, VideoElem};

/// Layout the video (using the poster image for sizing).
#[typst_macros::time(span = elem.span())]
pub fn layout_video(
    elem: &Packed<VideoElem>,
    engine: &mut Engine,
    _: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let video = elem.decode(engine, styles)?;
    let poster = video.poster();

    // Use poster dimensions for aspect ratio (same logic as layout_image).
    let pxw = poster.width();
    let pxh = poster.height();
    let px_ratio = pxw / pxh;

    let region_ratio = region.size.x / region.size.y;
    let wide = px_ratio > region_ratio;

    let target = if region.expand.x && region.expand.y {
        region.size
    } else if region.expand.x {
        Size::new(region.size.x, region.size.y.min(region.size.x / px_ratio))
    } else if region.expand.y {
        Size::new(region.size.x.min(region.size.y * px_ratio), region.size.y)
    } else {
        let dpi = poster.dpi().unwrap_or(Image::DEFAULT_DPI);
        let natural = Axes::new(pxw, pxh).map(|v| Abs::inches(v / dpi));
        Size::new(
            natural.x.min(region.size.x).min(region.size.y * px_ratio),
            natural.y.min(region.size.y).min(region.size.x / px_ratio),
        )
    };

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

    let mut frame = Frame::soft(fitted);
    frame.push(Point::zero(), FrameItem::Video(video, fitted, elem.span()));
    frame.resize(target, Axes::splat(FixedAlignment::Center));

    if fit == ImageFit::Cover && !target.fits(fitted) {
        frame.clip(Curve::rect(frame.size()));
    }

    Ok(frame)
}
