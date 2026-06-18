use krilla::configure::PdfVersion;
use krilla::geom as kg;
use krilla::page::Page;
use krilla::surface::Surface;
use krilla::tagging::{Artifact, ArtifactType, ContentTag, SpanTag};
use typst_layout::PagedDocument;
use typst_library::diag::SourceResult;
use typst_library::format::Complete;
use typst_library::layout::{FrameParent, Point, Rect, Size};
use typst_library::text::{Locale, TextItem};
use typst_library::visualize::{Image, Shape};

use crate::PdfOptions;
use crate::convert::{FrameContext, GlobalContext};
use crate::link::{LinkAnnotation, LinkAnnotationKind};
use crate::tags::tree::Tree;

pub use crate::tags::context::{AnnotationId, Tags};
pub use crate::tags::groups::GroupId;
pub use crate::tags::resolve::resolve;

mod context;
mod groups;
mod resolve;
mod tree;
mod util;

pub fn init(
    document: &PagedDocument,
    options: &PdfOptions<Complete>,
) -> SourceResult<Tags> {
    let tree = if options.tagged() {
        tree::build(document, options)?
    } else {
        Tree::empty(document, options)
    };
    Ok(Tags::new(tree))
}

pub fn handle_start(gc: &mut GlobalContext, fc: &FrameContext, surface: &mut Surface) {
    if disabled(gc) {
        return;
    }

    tree::step_start_tag(gc, fc, surface);
}

pub fn handle_end(gc: &mut GlobalContext, fc: &FrameContext, surface: &mut Surface) {
    if disabled(gc) {
        return;
    }

    tree::step_end_tag(gc, fc, surface);
}

pub fn group<T>(
    gc: &mut GlobalContext,
    fc: &mut FrameContext,
    surface: &mut Surface,
    parent: Option<FrameParent>,
    group_fn: impl FnOnce(&mut GlobalContext, &mut FrameContext, &mut Surface) -> T,
) -> T {
    if disabled(gc) || parent.is_none() {
        return group_fn(gc, fc, surface);
    }

    tree::enter_logical_child(gc, fc, surface);

    let res = group_fn(gc, fc, surface);

    tree::leave_logical_child(&mut gc.tags.tree, surface);

    res
}

pub fn page<T>(
    gc: &mut GlobalContext,
    surface: &mut Surface,
    page_fn: impl FnOnce(&mut GlobalContext, &mut Surface) -> T,
) -> T {
    if disabled(gc) {
        return page_fn(gc, surface);
    }

    if let Some(ty) = gc.tags.tree.parent_artifact() {
        surface.start_tagged(ContentTag::Artifact(ty));
    }

    let res = page_fn(gc, surface);

    if gc.tags.tree.parent_artifact().is_some() {
        surface.end_tagged();
    }

    res
}

/// Tags are completely disabled within tags.
pub fn tiling<T>(
    gc: &mut GlobalContext,
    surface: &mut Surface,
    tiling_size: Size,
    f: impl FnOnce(&mut GlobalContext, &mut Surface) -> T,
) -> T {
    if disabled(gc) {
        return f(gc, surface);
    }

    let prev = gc.tags.in_tiling;
    gc.tags.in_tiling = true;
    let mark_artifact = gc.tags.tree.parent_artifact().is_none();
    if mark_artifact {
        let bbox = kg::Rect::from_ltrb(
            0.0,
            0.0,
            tiling_size.x.to_pt() as f32,
            tiling_size.y.to_pt() as f32,
        );
        surface.start_tagged(ContentTag::Artifact(Artifact::new(
            if gc.options.version() == PdfVersion::Pdf17 && bbox.is_none() {
                // PDF 1.7 cannot tolerate empty bounding boxes for background
                // artifacts.
                ArtifactType::Other
            } else {
                ArtifactType::Background
            },
            bbox,
        )));
    }

    let res = f(gc, surface);

    if mark_artifact {
        surface.end_tagged();
    }
    gc.tags.in_tiling = prev;

    res
}

/// Whether tag generation is currently disabled. Either because it has been
/// disabled by the user using the [`crate::PdfFormatOptions::tagged`] flag, or
/// we're inside a tiling.
pub fn disabled(gc: &GlobalContext) -> bool {
    !gc.options.tagged() || gc.tags.in_tiling
}

/// Add all annotations that were found in the page frame.
pub fn add_link_annotations(
    gc: &mut GlobalContext,
    page: &mut Page,
    annotations: impl IntoIterator<Item = LinkAnnotation>,
) {
    for a in annotations.into_iter() {
        let link_annotation = if let [rect] = a.rects.as_slice() {
            krilla::annotation::LinkAnnotation::new(*rect, a.target)
        } else {
            let quads = a.rects.iter().map(|r| kg::Quadrilateral::from(*r)).collect();
            krilla::annotation::LinkAnnotation::new_with_quad_points(quads, a.target)
        };

        let annotation = krilla::annotation::Annotation::new_link(link_annotation, a.alt)
            .with_location(Some(a.span.into_raw()));

        if let LinkAnnotationKind::Tagged(annot_id) = a.kind {
            let identifier = page.add_tagged_annotation(annotation);
            gc.tags.annotations.init(annot_id, identifier);
        } else {
            page.add_annotation(annotation);
        }
    }
}

/// Automatically calls [`Surface::end_tagged`] when dropped.
pub struct TagHandle<'a, 'b> {
    surface: &'b mut Surface<'a>,
    /// Whether this tag handle started the marked content sequence, and should
    /// thus end it when it is dropped.
    started: bool,
}

impl Drop for TagHandle<'_, '_> {
    fn drop(&mut self) {
        if self.started {
            self.surface.end_tagged();
        }
    }
}

impl<'a> TagHandle<'a, '_> {
    pub fn surface<'c>(&'c mut self) -> &'c mut Surface<'a> {
        self.surface
    }
}

pub fn text<'a, 'b>(
    gc: &mut GlobalContext,
    fc: &FrameContext,
    surface: &'b mut Surface<'a>,
    text: &TextItem,
) -> TagHandle<'a, 'b> {
    if disabled(gc) {
        return TagHandle { surface, started: false };
    }

    update_bbox(gc, fc, || text.bbox());

    if gc.tags.tree.parent_artifact().is_some() {
        return TagHandle { surface, started: false };
    }

    let attrs = tree::resolve_text_attrs(&mut gc.tags.tree, gc.options, text);

    let lang = {
        let locale = Locale::new(text.lang, text.region);
        gc.tags.tree.groups.propagate_lang(gc.tags.tree.current(), locale)
    };
    let lang_str = lang.map(Locale::rfc_3066);
    let content = ContentTag::Span(SpanTag::empty().with_lang(lang_str.as_deref()));
    let id = surface.start_tagged(content);

    gc.tags.push_text(attrs, id);

    TagHandle { surface, started: true }
}

pub fn image<'a, 'b>(
    gc: &mut GlobalContext,
    fc: &FrameContext,
    surface: &'b mut Surface<'a>,
    image: &Image,
    size: Size,
) -> TagHandle<'a, 'b> {
    if disabled(gc) {
        return TagHandle { surface, started: false };
    }

    update_bbox(gc, fc, || Rect::from_pos_size(Point::zero(), size));

    if gc.tags.tree.parent_artifact().is_some() {
        return TagHandle { surface, started: false };
    }

    let content = ContentTag::Span(SpanTag::empty().with_alt_text(image.alt()));
    let id = surface.start_tagged(content);
    gc.tags.push_leaf(id);

    TagHandle { surface, started: true }
}

pub fn shape<'a, 'b>(
    gc: &mut GlobalContext,
    fc: &FrameContext,
    surface: &'b mut Surface<'a>,
    shape: &Shape,
    artifact_type: ArtifactType,
) -> TagHandle<'a, 'b> {
    if disabled(gc) {
        return TagHandle { surface, started: false };
    }

    update_bbox(gc, fc, || shape.bbox(true));

    if gc.tags.tree.parent_artifact().is_some() {
        return TagHandle { surface, started: false };
    }

    surface.start_tagged(ContentTag::Artifact(Artifact::with_kind(
        if gc.options.version() == PdfVersion::Pdf17
            && artifact_type == ArtifactType::Background
        {
            ArtifactType::Other
        } else {
            artifact_type
        },
    )));

    TagHandle { surface, started: true }
}

fn update_bbox(
    gc: &mut GlobalContext,
    fc: &FrameContext,
    compute_bbox: impl FnOnce() -> Rect,
) {
    if let Some(bbox) = gc.tags.tree.parent_bbox()
        && gc.options.validators().accessibility().is_some()
    {
        bbox.expand_frame(fc, compute_bbox);
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use ecow::eco_vec;
    use typst_library::diag::error;
    use typst_library::format::Complete;
    use typst_library::foundations::Smart;
    use typst_library::layout::PageRanges;
    use typst_syntax::Span;
    use typst_utils::NonZeroExt;

    use crate::PdfOptions;
    use crate::format::PdfFormatOptions;

    #[test]
    fn tagged_and_page_range() {
        let mut doc_options = PdfFormatOptions::<Complete>::default();
        doc_options.tagged.v = Smart::Custom(true);
        doc_options.pages.v =
            Some(PageRanges::new(eco_vec![Some(NonZeroUsize::ONE)..=None]));
        let options = PdfOptions::default();
        let res = options.resolve(&doc_options);
        assert_eq!(
            res,
            Err(eco_vec![error!(
                Span::detached(),
                "cannot enable tagged PDF and export a page range"
            )])
        );
    }
}
