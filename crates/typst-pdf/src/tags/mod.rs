use krilla::configure::Validator;
use krilla::page::Page;
use krilla::surface::Surface;
use krilla::tagging::{ArtifactType, ContentTag, SpanTag};
use typst_library::diag::SourceResult;
use typst_library::foundations::Content;
use typst_library::introspection::Location;
use typst_library::layout::{PagedDocument, Point, Rect, Size};
use typst_library::model::{EmphElem, StrongElem};
use typst_library::text::{
    HighlightElem, Locale, OverlineElem, ScriptKind, StrikeElem, SubElem, SuperElem,
    TextItem, UnderlineElem,
};
use typst_library::visualize::{Image, Shape};

use crate::PdfOptions;
use crate::convert::{FrameContext, GlobalContext};
use crate::link::{LinkAnnotation, LinkAnnotationKind};
use crate::tags::text::{TextAttr, TextDecoKind};
use crate::tags::tree::Tree;
use crate::tags::util::{PropertyOptRef, PropertyValCloned, PropertyValCopied};

pub use crate::tags::context::{AnnotationId, Tags};
pub use crate::tags::groups::GroupId;
pub use crate::tags::resolve::resolve;

mod context;
mod groups;
mod resolve;
mod text;
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

pub fn handle_start(
    gc: &mut GlobalContext,
    surface: &mut Surface,
    elem: &Content,
) -> SourceResult<()> {
    if disabled(gc) {
        return Ok(());
    }

    tree::step_start_tag(&mut gc.tags.tree, surface);

    if let Some(_strong) = elem.to_packed::<StrongElem>() {
        gc.tags.text_attrs.push(elem, TextAttr::Strong);
    } else if let Some(_emph) = elem.to_packed::<EmphElem>() {
        gc.tags.text_attrs.push(elem, TextAttr::Emph);
    } else if let Some(sub) = elem.to_packed::<SubElem>() {
        let baseline_shift = sub.baseline.val();
        let lineheight = sub.size.val();
        let kind = ScriptKind::Sub;
        gc.tags.text_attrs.push_script(elem, kind, baseline_shift, lineheight);
    } else if let Some(sup) = elem.to_packed::<SuperElem>() {
        let baseline_shift = sup.baseline.val();
        let lineheight = sup.size.val();
        let kind = ScriptKind::Super;
        gc.tags.text_attrs.push_script(elem, kind, baseline_shift, lineheight);
    } else if let Some(highlight) = elem.to_packed::<HighlightElem>() {
        let paint = highlight.fill.opt_ref();
        gc.tags.text_attrs.push_highlight(elem, paint);
    } else if let Some(underline) = elem.to_packed::<UnderlineElem>() {
        let kind = TextDecoKind::Underline;
        let stroke = underline.stroke.val_cloned();
        gc.tags.text_attrs.push_deco(gc.options, elem, kind, stroke)?;
    } else if let Some(overline) = elem.to_packed::<OverlineElem>() {
        let kind = TextDecoKind::Overline;
        let stroke = overline.stroke.val_cloned();
        gc.tags.text_attrs.push_deco(gc.options, elem, kind, stroke)?;
    } else if let Some(strike) = elem.to_packed::<StrikeElem>() {
        let kind = TextDecoKind::Strike;
        let stroke = strike.stroke.val_cloned();
        gc.tags.text_attrs.push_deco(gc.options, elem, kind, stroke)?;
    }

    Ok(())
}

pub fn handle_end(
    gc: &mut GlobalContext,
    surface: &mut Surface,
    loc: Location,
) -> SourceResult<()> {
    if disabled(gc) {
        return Ok(());
    }

    tree::step_end_tag(&mut gc.tags.tree, surface);

    gc.tags.text_attrs.pop(loc);

    Ok(())
}

pub fn group<T>(
    gc: &mut GlobalContext,
    surface: &mut Surface,
    parent: Option<Location>,
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
        let annotation = krilla::annotation::Annotation::new_link(
            krilla::annotation::LinkAnnotation::new_with_quad_points(
                a.quad_points,
                a.target,
            ),
            a.alt,
        )
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

    let attrs = gc.tags.text_attrs.resolve(text);

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
