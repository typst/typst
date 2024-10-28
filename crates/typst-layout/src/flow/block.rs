use once_cell::unsync::Lazy;
use smallvec::SmallVec;
use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::foundations::{Packed, Resolve, StyleChain};
use typst_library::introspection::Locator;
use typst_library::layout::{
    Abs, Axes, BlockBody, BlockElem, Fragment, Frame, FrameKind, Region, Regions, Rel,
    Sides, Size, Sizing,
};
use typst_library::visualize::Stroke;
use typst_utils::Numeric;

use crate::shapes::{clip_rect, fill_and_stroke};

/// Lay this out as an unbreakable block.
#[typst_macros::time(name = "block", span = elem.span())]
pub fn layout_single_block(
    elem: &Packed<BlockElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    // Fetch sizing properties.
    let width = elem.width(styles);
    let height = elem.height(styles);
    let inset = elem.inset(styles).unwrap_or_default();

    // Build the pod regions.
    let pod = unbreakable_pod(&width.into(), &height, &inset, styles, region.size);

    // Layout the body.
    let body = elem.body(styles);
    let mut frame = match body {
        // If we have no body, just create one frame. Its size will be
        // adjusted below.
        None => Frame::hard(Size::zero()),

        // If we have content as our body, just layout it.
        Some(BlockBody::Content(body)) => {
            crate::layout_frame(engine, body, locator.relayout(), styles, pod)?
        }

        // If we have a child that wants to layout with just access to the
        // base region, give it that.
        Some(BlockBody::SingleLayouter(callback)) => {
            callback.call(engine, locator, styles, pod)?
        }

        // If we have a child that wants to layout with full region access,
        // we layout it.
        Some(BlockBody::MultiLayouter(callback)) => {
            let expand = (pod.expand | region.expand) & pod.size.map(Abs::is_finite);
            let pod = Region { expand, ..pod };
            callback.call(engine, locator, styles, pod.into())?.into_frame()
        }
    };

    // Explicit blocks are boundaries for gradient relativeness.
    if matches!(body, None | Some(BlockBody::Content(_))) {
        frame.set_kind(FrameKind::Hard);
    }

    // Enforce a correct frame size on the expanded axes. Do this before
    // applying the inset, since the pod shrunk.
    frame.set_size(pod.expand.select(pod.size, frame.size()));

    // Apply the inset.
    if !inset.is_zero() {
        crate::pad::grow(&mut frame, &inset);
    }

    // Prepare fill and stroke.
    let fill = elem.fill(styles);
    let stroke = elem
        .stroke(styles)
        .unwrap_or_default()
        .map(|s| s.map(Stroke::unwrap_or_default));

    // Only fetch these if necessary (for clipping or filling/stroking).
    let outset = Lazy::new(|| elem.outset(styles).unwrap_or_default());
    let radius = Lazy::new(|| elem.radius(styles).unwrap_or_default());

    // Clip the contents, if requested.
    if elem.clip(styles) {
        let size = frame.size() + outset.relative_to(frame.size()).sum_by_axis();
        frame.clip(clip_rect(size, &radius, &stroke));
    }

    // Add fill and/or stroke.
    if fill.is_some() || stroke.iter().any(Option::is_some) {
        fill_and_stroke(&mut frame, fill, &stroke, &outset, &radius, elem.span());
    }

    // Assign label to each frame in the fragment.
    if let Some(label) = elem.label() {
        frame.label(label);
    }

    Ok(frame)
}

/// Lay this out as a breakable block.
#[typst_macros::time(name = "block", span = elem.span())]
pub fn layout_multi_block(
    elem: &Packed<BlockElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    regions: Regions,
) -> SourceResult<Fragment> {
    // Fetch sizing properties.
    let width = elem.width(styles);
    let height = elem.height(styles);
    let inset = elem.inset(styles).unwrap_or_default();

    // Allocate a small vector for backlogs.
    let mut buf = SmallVec::<[Abs; 2]>::new();

    // Build the pod regions.
    let pod = breakable_pod(&width.into(), &height, &inset, styles, regions, &mut buf);

    // Layout the body.
    let body = elem.body(styles);
    let mut fragment = match body {
        // If we have no body, just create one frame plus one per backlog
        // region. We create them zero-sized; if necessary, their size will
        // be adjusted below.
        None => {
            let mut frames = vec![];
            frames.push(Frame::hard(Size::zero()));
            if pod.expand.y {
                let mut iter = pod;
                while !iter.backlog.is_empty() {
                    frames.push(Frame::hard(Size::zero()));
                    iter.next();
                }
            }
            Fragment::frames(frames)
        }

        // If we have content as our body, just layout it.
        Some(BlockBody::Content(body)) => {
            let mut fragment =
                crate::layout_fragment(engine, body, locator.relayout(), styles, pod)?;

            // If the body is automatically sized and produced more than one
            // fragment, ensure that the width was consistent across all
            // regions. If it wasn't, we need to relayout with expansion.
            if !pod.expand.x
                && fragment
                    .as_slice()
                    .windows(2)
                    .any(|w| !w[0].width().approx_eq(w[1].width()))
            {
                let max_width =
                    fragment.iter().map(|frame| frame.width()).max().unwrap_or_default();
                let pod = Regions {
                    size: Size::new(max_width, pod.size.y),
                    expand: Axes::new(true, pod.expand.y),
                    ..pod
                };
                fragment = crate::layout_fragment(engine, body, locator, styles, pod)?;
            }

            fragment
        }

        // If we have a child that wants to layout with just access to the
        // base region, give it that.
        Some(BlockBody::SingleLayouter(callback)) => {
            let pod = Region::new(pod.base(), pod.expand);
            callback.call(engine, locator, styles, pod).map(Fragment::frame)?
        }

        // If we have a child that wants to layout with full region access,
        // we layout it.
        //
        // For auto-sized multi-layouters, we propagate the outer expansion
        // so that they can decide for themselves. We also ensure again to
        // only expand if the size is finite.
        Some(BlockBody::MultiLayouter(callback)) => {
            let expand = (pod.expand | regions.expand) & pod.size.map(Abs::is_finite);
            let pod = Regions { expand, ..pod };
            callback.call(engine, locator, styles, pod)?
        }
    };

    // Prepare fill and stroke.
    let fill = elem.fill(styles);
    let stroke = elem
        .stroke(styles)
        .unwrap_or_default()
        .map(|s| s.map(Stroke::unwrap_or_default));

    // Only fetch these if necessary (for clipping or filling/stroking).
    let outset = Lazy::new(|| elem.outset(styles).unwrap_or_default());
    let radius = Lazy::new(|| elem.radius(styles).unwrap_or_default());

    // Fetch/compute these outside of the loop.
    let clip = elem.clip(styles);
    let has_fill_or_stroke = fill.is_some() || stroke.iter().any(Option::is_some);
    let has_inset = !inset.is_zero();
    let is_explicit = matches!(body, None | Some(BlockBody::Content(_)));

    // Skip filling/stroking the first frame if it is empty and a non-empty
    // one follows.
    let mut skip_first = false;
    if let [first, rest @ ..] = fragment.as_slice() {
        skip_first = has_fill_or_stroke
            && first.is_empty()
            && rest.iter().any(|frame| !frame.is_empty());
    }

    // Post-process to apply insets, clipping, fills, and strokes.
    for (i, (frame, region)) in fragment.iter_mut().zip(pod.iter()).enumerate() {
        // Explicit blocks are boundaries for gradient relativeness.
        if is_explicit {
            frame.set_kind(FrameKind::Hard);
        }

        // Enforce a correct frame size on the expanded axes. Do this before
        // applying the inset, since the pod shrunk.
        frame.set_size(pod.expand.select(region, frame.size()));

        // Apply the inset.
        if has_inset {
            crate::pad::grow(frame, &inset);
        }

        // Clip the contents, if requested.
        if clip {
            let size = frame.size() + outset.relative_to(frame.size()).sum_by_axis();
            frame.clip(clip_rect(size, &radius, &stroke));
        }

        // Add fill and/or stroke.
        if has_fill_or_stroke && (i > 0 || !skip_first) {
            fill_and_stroke(frame, fill.clone(), &stroke, &outset, &radius, elem.span());
        }
    }

    // Assign label to each frame in the fragment.
    if let Some(label) = elem.label() {
        for frame in fragment.iter_mut() {
            frame.label(label);
        }
    }

    Ok(fragment)
}

/// Builds the pod region for an unbreakable sized container.
pub(crate) fn unbreakable_pod(
    width: &Sizing,
    height: &Sizing,
    inset: &Sides<Rel<Abs>>,
    styles: StyleChain,
    base: Size,
) -> Region {
    // Resolve the size.
    let mut size = Size::new(
        match width {
            // - For auto, the whole region is available.
            // - Fr is handled outside and already factored into the `region`,
            //   so we can treat it equivalently to 100%.
            Sizing::Auto | Sizing::Fr(_) => base.x,
            // Resolve the relative sizing.
            Sizing::Rel(rel) => rel.resolve(styles).relative_to(base.x),
        },
        match height {
            Sizing::Auto | Sizing::Fr(_) => base.y,
            Sizing::Rel(rel) => rel.resolve(styles).relative_to(base.y),
        },
    );

    // Take the inset, if any, into account.
    if !inset.is_zero() {
        size = crate::pad::shrink(size, inset);
    }

    // If the child is manually, the size is forced and we should enable
    // expansion.
    let expand = Axes::new(
        *width != Sizing::Auto && size.x.is_finite(),
        *height != Sizing::Auto && size.y.is_finite(),
    );

    Region::new(size, expand)
}

/// Builds the pod regions for a breakable sized container.
fn breakable_pod<'a>(
    width: &Sizing,
    height: &Sizing,
    inset: &Sides<Rel<Abs>>,
    styles: StyleChain,
    regions: Regions,
    buf: &'a mut SmallVec<[Abs; 2]>,
) -> Regions<'a> {
    let base = regions.base();

    // The vertical region sizes we're about to build.
    let first;
    let full;
    let backlog: &mut [Abs];
    let last;

    // If the block has a fixed height, things are very different, so we
    // handle that case completely separately.
    match height {
        Sizing::Auto | Sizing::Fr(_) => {
            // If the block is automatically sized, we can just inherit the
            // regions.
            first = regions.size.y;
            full = regions.full;
            buf.extend_from_slice(regions.backlog);
            backlog = buf;
            last = regions.last;
        }

        Sizing::Rel(rel) => {
            // Resolve the sizing to a concrete size.
            let resolved = rel.resolve(styles).relative_to(base.y);

            // Since we're manually sized, the resolved size is the base height.
            full = resolved;

            // Distribute the fixed height across a start region and a backlog.
            (first, backlog) = distribute(resolved, regions, buf);

            // If the height is manually sized, we don't want a final repeatable
            // region.
            last = None;
        }
    };

    // Resolve the horizontal sizing to a concrete width and combine
    // `width` and `first` into `size`.
    let mut size = Size::new(
        match width {
            Sizing::Auto | Sizing::Fr(_) => regions.size.x,
            Sizing::Rel(rel) => rel.resolve(styles).relative_to(base.x),
        },
        first,
    );

    // Take the inset, if any, into account, applying it to the
    // individual region components.
    let (mut full, mut last) = (full, last);
    if !inset.is_zero() {
        crate::pad::shrink_multiple(&mut size, &mut full, backlog, &mut last, inset);
    }

    // If the child is manually, the size is forced and we should enable
    // expansion.
    let expand = Axes::new(
        *width != Sizing::Auto && size.x.is_finite(),
        *height != Sizing::Auto && size.y.is_finite(),
    );

    Regions { size, full, backlog, last, expand }
}

/// Distribute a fixed height spread over existing regions into a new first
/// height and a new backlog.
fn distribute<'a>(
    height: Abs,
    mut regions: Regions,
    buf: &'a mut SmallVec<[Abs; 2]>,
) -> (Abs, &'a mut [Abs]) {
    // Build new region heights from old regions.
    let mut remaining = height;
    loop {
        let limited = regions.size.y.clamp(Abs::zero(), remaining);
        buf.push(limited);
        remaining -= limited;
        if remaining.approx_empty()
            || !regions.may_break()
            || (!regions.may_progress() && limited.approx_empty())
        {
            break;
        }
        regions.next();
    }

    // If there is still something remaining, apply it to the
    // last region (it will overflow, but there's nothing else
    // we can do).
    if !remaining.approx_empty() {
        if let Some(last) = buf.last_mut() {
            *last += remaining;
        }
    }

    // Distribute the heights to the first region and the
    // backlog. There is no last region, since the height is
    // fixed.
    (buf[0], &mut buf[1..])
}
