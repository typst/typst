use krilla::configure::Validator;
use krilla::geom as kg;
use krilla::page::Page;
use krilla::surface::Surface;
use krilla::tagging::{ArtifactType, ContentTag, SpanTag};
use typst_library::diag::{SourceResult, StrResult, bail};
use typst_library::layout::{FrameParent, PagedDocument, Point, Rect, Size};
use typst_library::text::{Locale, TextItem};
use typst_library::visualize::{Image, ImageKind, Shape};
use typst_syntax::Span;

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

pub fn init(document: &PagedDocument, options: &PdfOptions) -> SourceResult<Tags> {
    let tree = if options.tagged {
        tree::build(document, options)?
    } else {
        Tree::empty(document, options)
    };
    Ok(Tags::new(tree))
}

pub fn handle_start(gc: &mut GlobalContext, surface: &mut Surface) {
    if disabled(gc) {
        return;
    }

    tree::step_start_tag(&mut gc.tags.tree, surface);
}

pub fn handle_end(gc: &mut GlobalContext, surface: &mut Surface) {
    if disabled(gc) {
        return;
    }

    tree::step_end_tag(&mut gc.tags.tree, surface);
}

pub fn group<T>(
    gc: &mut GlobalContext,
    surface: &mut Surface,
    parent: Option<FrameParent>,
    group_fn: impl FnOnce(&mut GlobalContext, &mut Surface) -> T,
) -> T {
    if disabled(gc) || parent.is_none() {
        return group_fn(gc, surface);
    }

    tree::enter_logical_child(&mut gc.tags.tree, surface);

    let res = group_fn(gc, surface);

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
    f: impl FnOnce(&mut GlobalContext, &mut Surface) -> T,
) -> T {
    if disabled(gc) {
        return f(gc, surface);
    }

    let prev = gc.tags.in_tiling;
    gc.tags.in_tiling = true;
    let mark_artifact = gc.tags.tree.parent_artifact().is_none();
    if mark_artifact {
        surface.start_tagged(ContentTag::Artifact(ArtifactType::Other));
    }

    let res = f(gc, surface);

    if mark_artifact {
        surface.end_tagged();
    }
    gc.tags.in_tiling = prev;

    res
}

/// Whether tag generation is currently disabled. Either because it has been
/// disabled by the user using the [`PdfOptions::tagged`] flag, or we're inside
/// a tiling.
pub fn disabled(gc: &GlobalContext) -> bool {
    !gc.options.tagged || gc.tags.in_tiling
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
) -> SourceResult<TagHandle<'a, 'b>> {
    if disabled(gc) {
        return Ok(TagHandle { surface, started: false });
    }

    update_bbox(gc, fc, || text.bbox());

    if gc.tags.tree.parent_artifact().is_some() {
        return Ok(TagHandle { surface, started: false });
    } else if !text.selectable {
        let span = text.glyphs.first().map(|g| g.span.0).unwrap_or_else(Span::detached);
        bail!(span, "unselectable text must be wrapped in `pdf.artifact`");
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

    Ok(TagHandle { surface, started: true })
}

pub fn image<'a, 'b>(
    gc: &mut GlobalContext,
    fc: &FrameContext,
    surface: &'b mut Surface<'a>,
    image: &Image,
    size: Size,
) -> StrResult<TagHandle<'a, 'b>> {
    if disabled(gc) {
        return Ok(TagHandle { surface, started: false });
    }

    update_bbox(gc, fc, || Rect::from_pos_size(Point::zero(), size));

    if gc.tags.tree.parent_artifact().is_some() {
        return Ok(TagHandle { surface, started: false });
    } else if let ImageKind::Svg(svg) = image.kind()
        && !svg.is_selectable()
    {
        bail!(
            "SVG images embedded with unselectable text must be wrapped in `pdf.artifact`"
        );
    }

    let content = ContentTag::Span(SpanTag::empty().with_alt_text(image.alt()));
    let id = surface.start_tagged(content);
    gc.tags.push_leaf(id);

    Ok(TagHandle { surface, started: true })
}

pub fn shape<'a, 'b>(
    gc: &mut GlobalContext,
    fc: &FrameContext,
    surface: &'b mut Surface<'a>,
    shape: &Shape,
) -> TagHandle<'a, 'b> {
    if disabled(gc) {
        return TagHandle { surface, started: false };
    }

    update_bbox(gc, fc, || shape.geometry.bbox());

    if gc.tags.tree.parent_artifact().is_some() {
        return TagHandle { surface, started: false };
    }

    surface.start_tagged(ContentTag::Artifact(ArtifactType::Other));

    TagHandle { surface, started: true }
}

fn update_bbox(
    gc: &mut GlobalContext,
    fc: &FrameContext,
    compute_bbox: impl FnOnce() -> Rect,
) {
    if let Some(bbox) = gc.tags.tree.parent_bbox()
        && gc.options.standards.config.validator() == Validator::UA1
    {
        bbox.expand_frame(fc, compute_bbox);
    }
}
