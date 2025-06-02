use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::foundations::{Content, NativeElement};
use typst_library::introspection::{ManualPageCounter, SplitLocator, Tag};
use typst_library::layout::{
    ArtifactKind, ArtifactMarker, Frame, FrameItem, Page, Point,
};

use super::LayoutedPage;

/// Piece together the inner page frame and the marginals. We can only do this
/// at the very end because inside/outside margins require knowledge of the
/// physical page number, which is unknown during parallel layout.
pub fn finalize(
    engine: &mut Engine,
    locator: &mut SplitLocator,
    counter: &mut ManualPageCounter,
    tags: &mut Vec<Tag>,
    LayoutedPage {
        inner,
        mut margin,
        binding,
        two_sided,
        header,
        footer,
        background,
        foreground,
        fill,
        numbering,
        supplement,
    }: LayoutedPage,
) -> SourceResult<Page> {
    // If two sided, left becomes inside and right becomes outside.
    // Thus, for left-bound pages, we want to swap on even pages and
    // for right-bound pages, we want to swap on odd pages.
    if two_sided && binding.swap(counter.physical()) {
        std::mem::swap(&mut margin.left, &mut margin.right);
    }

    // Create a frame for the full page.
    let mut frame = Frame::hard(inner.size() + margin.sum_by_axis());

    // Add tags.
    for tag in tags.drain(..) {
        frame.push(Point::zero(), FrameItem::Tag(tag));
    }

    // Add the "before" marginals. The order in which we push things here is
    // important as it affects the relative ordering of introspectable elements
    // and thus how counters resolve.
    if let Some(background) = background {
        let tag = ArtifactMarker::new(ArtifactKind::Page).pack();
        push_tagged(engine, locator, &mut frame, Point::zero(), background, tag);
    }
    if let Some(header) = header {
        let tag = ArtifactMarker::new(ArtifactKind::Header).pack();
        push_tagged(engine, locator, &mut frame, Point::with_x(margin.left), header, tag);
    }

    // Add the inner contents.
    frame.push_frame(Point::new(margin.left, margin.top), inner);

    // Add the "after" marginals.
    if let Some(footer) = footer {
        let y = frame.height() - footer.height();
        let tag = ArtifactMarker::new(ArtifactKind::Footer).pack();
        push_tagged(engine, locator, &mut frame, Point::new(margin.left, y), footer, tag);
    }
    if let Some(foreground) = foreground {
        frame.push_frame(Point::zero(), foreground);
    }

    // Apply counter updates from within the page to the manual page counter.
    counter.visit(engine, &frame)?;

    // Get this page's number and then bump the counter for the next page.
    let number = counter.logical();
    counter.step();

    Ok(Page { frame, fill, numbering, supplement, number })
}

fn push_tagged(
    engine: &mut Engine,
    locator: &mut SplitLocator,
    frame: &mut Frame,
    mut pos: Point,
    inner: Frame,
    mut tag: Content,
) {
    // TODO: use general PDF Tagged/Artifact element that wraps some content and
    // is also available to the user.
    let key = typst_utils::hash128(&tag);
    let loc = locator.next_location(engine.introspector, key);
    tag.set_location(loc);
    frame.push(pos, FrameItem::Tag(Tag::Start(tag)));

    let height = inner.height();
    frame.push_frame(pos, inner);

    pos.y += height;
    frame.push(pos, FrameItem::Tag(Tag::End(loc, key)));
}
