use std::sync::Arc;

use krilla::action::RenditionAction;
use krilla::annotation::{Annotation, ScreenAnnotation};
use krilla::geom as kg;
use krilla::surface::Surface;
use typst_library::diag::SourceResult;
use typst_library::layout::{Point, Size};
use typst_library::visualize::Video;
use typst_syntax::Span;

use crate::convert::{FrameContext, GlobalContext};
use crate::image::handle_image;
use crate::util::PointExt;

pub(crate) fn handle_video(
    gc: &mut GlobalContext,
    fc: &mut FrameContext,
    video: &Video,
    size: Size,
    surface: &mut Surface,
    span: Span,
) -> SourceResult<()> {
    // 1. Render the poster image as the visual fallback.
    handle_image(gc, fc, video.poster(), size, surface, span)?;

    // 2. Compute bounding box (same logic as link.rs).
    let rect = bounding_box(fc, size);

    // 3. Build the screen annotation.
    let video_data: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(video.data().clone());

    let annotation = Annotation::new_screen(
        ScreenAnnotation {
            rect,
            action: RenditionAction {
                data: video_data,
                mime_type: video.mime_type().to_string(),
                filename: video.filename().to_string(),
            },
        },
        video.alt().map(String::from),
    );

    // 4. Collect the annotation for later addition to the page.
    fc.push_video_annotation(annotation);

    Ok(())
}

/// Compute the bounding box of the transformed rectangle for this frame.
fn bounding_box(fc: &FrameContext, size: Size) -> kg::Rect {
    let pos = Point::zero();
    let points = [
        pos + Point::with_y(size.y),
        pos + size.to_point(),
        pos + Point::with_x(size.x),
        pos,
    ];

    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for point in points {
        let p = point.transform(fc.state().transform()).to_krilla();
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }

    kg::Rect::from_ltrb(min_x, min_y, max_x, max_y).unwrap()
}
