use ecow::EcoString;
use krilla::configure::Validator;
use krilla::page::Page;
use krilla::surface::Surface;
use krilla::tagging::{self as kt, Node, Placement, Tag, TagKind};
use krilla::tagging::{ArtifactType, ContentTag, SpanTag, TagTree};
use typst_library::diag::SourceResult;
use typst_library::foundations::Content;
use typst_library::introspection::Location;
use typst_library::layout::{PagedDocument, Point, Rect, Size};
use typst_library::model::{EmphElem, StrongElem};
use typst_library::text::{
    HighlightElem, Lang, OverlineElem, ScriptKind, StrikeElem, SubElem, SuperElem,
    TextItem, UnderlineElem,
};
use typst_library::visualize::{Image, Shape};

use crate::PdfOptions;
use crate::convert::{FrameContext, GlobalContext};
use crate::link::{LinkAnnotation, LinkAnnotationKind};
use crate::tags::context::{Annotations, BBoxCtx, FigureCtx, TableCtx, TagNode};
use crate::tags::text::{TextAttr, TextDecoKind};
use crate::tags::tree::{Step, Tree};
use crate::tags::util::{IdVec, PropertyOptRef, PropertyValCloned, PropertyValCopied};

pub use context::{AnnotationId, Tags};
pub use groups::{Group, GroupId, GroupKind, Groups};

mod context;
mod groups;
mod text;
mod tree;
mod util;

pub fn init(document: &PagedDocument, options: &PdfOptions) -> SourceResult<Tags> {
    let tree = if !options.disable_tags {
        tree::build(document, options)?
    } else {
        Tree::empty()
    };
    Ok(Tags::new(tree))
}

pub fn finish(tags: &mut Tags) -> (Option<Lang>, TagTree) {
    assert!(tags.tree.finished_traversal(), "tree traversal didn't complete properly");

    let group = tags.tree.groups.get(tags.tree.root());
    let GroupKind::Root(mut doc_lang) = group.kind else { unreachable!() };

    let mut children = Vec::with_capacity(group.nodes().len());

    for child in group.nodes().iter() {
        resolve_node(
            &tags.tree.groups,
            &tags.tree.tables,
            &mut tags.annotations,
            &mut doc_lang,
            &mut None,
            &mut children,
            child,
        );
    }

    (doc_lang, TagTree::from(children))
}

/// Resolves nodes into an accumulator.
fn resolve_node(
    groups: &Groups,
    tables: &IdVec<TableCtx>,
    annotations: &mut Annotations,
    parent_lang: &mut Option<Lang>,
    parent_bbox: &mut Option<BBoxCtx>,
    accum: &mut Vec<Node>,
    node: &TagNode,
) {
    match &node {
        TagNode::Group(id) => {
            resolve_group_node(
                groups,
                tables,
                annotations,
                parent_lang,
                parent_bbox,
                accum,
                *id,
            );
        }
        TagNode::Leaf(identifier) => {
            accum.push(Node::Leaf(*identifier));
        }
        TagNode::Annotation(id) => {
            accum.push(annotations.take(*id));
        }
        TagNode::Text(attrs, ids) => {
            attrs.resolve_nodes(accum, ids);
        }
    }
}

fn resolve_group_node(
    groups: &Groups,
    tables: &IdVec<TableCtx>,
    annotations: &mut Annotations,
    mut parent_lang: &mut Option<Lang>,
    mut parent_bbox: &mut Option<BBoxCtx>,
    accum: &mut Vec<Node>,
    id: GroupId,
) {
    let group = groups.get(id);

    let mut tag = build_group_tag(tables, group);
    let mut bbox = group.kind.bbox().cloned();
    let mut nodes = Vec::new();

    let group_bbox = if bbox.is_some() { &mut bbox } else { &mut parent_bbox };

    // In PDF 1.7, don't include artifacts in the tag tree. In PDF 2.0
    // this might become an `Artifact` tag.
    if group.kind.is_artifact() {
        for child in group.nodes().iter() {
            resolve_artifact_node(groups, group_bbox, child);
        }
    } else {
        nodes = Vec::with_capacity(group.nodes().len());
        let lang = tag.as_mut().map(|(_, lang)| lang).unwrap_or(&mut parent_lang);
        for child in group.nodes().iter() {
            resolve_node(
                groups,
                tables,
                annotations,
                lang,
                group_bbox,
                &mut nodes,
                child,
            );
        }

        // Insert logical children at the end of the parent.
        if let GroupKind::LogicalParent(children) = &group.kind {
            for child in children.iter() {
                resolve_group_node(
                    groups,
                    tables,
                    annotations,
                    lang,
                    group_bbox,
                    &mut nodes,
                    *child,
                );
            }
        }
    }

    // Update the parent bbox.
    if let Some((parent, child)) = parent_bbox.as_mut().zip(bbox.as_ref()) {
        parent.expand_page(child);
    }

    if let Some((mut tag, mut group_lang)) = tag {
        // Try to propagate the groups language to the parent tag.
        if let Some(lang) = group_lang
            && parent_lang.is_none_or(|l| l == lang)
        {
            *parent_lang = Some(lang);
            group_lang = None;
        }

        tag.set_location(Some(group.span.into_raw()));
        tag.set_lang(group_lang.map(|l| l.as_str().to_string()));
        if let Some(bbox) = bbox {
            match &mut tag {
                TagKind::Table(tag) => tag.set_bbox(bbox.to_krilla()),
                TagKind::Figure(tag) => tag.set_bbox(bbox.to_krilla()),
                TagKind::Formula(tag) => tag.set_bbox(bbox.to_krilla()),
                _ => (),
            }
        }

        accum.push(Node::Group(kt::TagGroup::with_children(tag, nodes)));
    } else {
        accum.extend(nodes);
    }
}

/// Currently only done to resolve bounding boxes.
fn resolve_artifact_node(
    groups: &Groups,
    mut parent_bbox: &mut Option<BBoxCtx>,
    node: &TagNode,
) {
    match &node {
        TagNode::Group(id) => {
            let group = groups.get(*id);

            let mut bbox = group.kind.bbox().cloned();
            let group_bbox = if bbox.is_some() { &mut bbox } else { &mut parent_bbox };
            for child in group.nodes().iter() {
                resolve_artifact_node(groups, group_bbox, child);
            }

            // Update the parent bbox.
            if let Some((parent, child)) = parent_bbox.as_mut().zip(bbox.as_ref()) {
                parent.expand_page(child);
            }
        }
        TagNode::Leaf(..) => (),
        TagNode::Annotation(..) => (),
        TagNode::Text(..) => (),
    }
}

fn build_group_tag(
    tables: &IdVec<TableCtx>,
    group: &Group,
) -> Option<(TagKind, Option<Lang>)> {
    Some(match &group.kind {
        GroupKind::Root(_) => unreachable!(),
        GroupKind::Artifact(_) => return None,
        GroupKind::LogicalParent(_) => return None,
        GroupKind::LogicalChild => return None,
        GroupKind::Outline(_, lang) => (Tag::TOC.into(), *lang),
        GroupKind::OutlineEntry(_, lang) => (Tag::TOCI.into(), *lang),
        GroupKind::Table(id, _, lang) => (tables.get(*id).build_tag(), *lang),
        // TODO: store tags in a separate list and take them here.
        GroupKind::TableCell(_, tag, lang) => (tag.clone(), *lang),
        GroupKind::Grid(_, lang) => (Tag::Div.into(), *lang),
        GroupKind::GridCell(_, lang) => (Tag::Div.into(), *lang),
        GroupKind::List(_, numbering, lang) => (Tag::L(*numbering).into(), *lang),
        GroupKind::ListItemLabel(lang) => (Tag::Lbl.into(), *lang),
        GroupKind::ListItemBody(lang) => (Tag::LBody.into(), *lang),
        GroupKind::BibEntry(lang) => (Tag::BibEntry.into(), *lang),
        GroupKind::Figure(ctx, lang) => {
            let tag = Tag::Figure(ctx.alt.as_ref().map(String::from)).into();
            (tag, *lang)
        }
        GroupKind::Formula(ctx, lang) => {
            let tag = Tag::Formula(ctx.alt.as_ref().map(String::from)).into();
            (tag, *lang)
        }
        GroupKind::Link(link, lang) => {
            let alt = link.alt.as_ref().map(EcoString::to_string);
            (Tag::Link.with_alt_text(alt).into(), *lang)
        }
        GroupKind::CodeBlock(lang) => {
            let tag = Tag::Code.with_placement(Some(Placement::Block)).into();
            (tag, *lang)
        }
        GroupKind::CodeBlockLine(lang) => (Tag::Span.into(), *lang),
        GroupKind::Standard(tag, lang) => (tag.clone(), *lang),
    })
}

pub fn handle_start(
    gc: &mut GlobalContext,
    surface: &mut Surface,
    elem: &Content,
) -> SourceResult<()> {
    if disabled(gc) {
        return Ok(());
    }

    progress::<tree::StartTag>(gc, surface);

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

    progress::<tree::EndTag>(gc, surface);

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

    progress::<tree::EnterLogicalChild>(gc, surface);

    let res = group_fn(gc, surface);

    progress::<tree::LeaveLogicalChild>(gc, surface);

    res
}

fn progress<S: Step>(gc: &mut GlobalContext, surface: &mut Surface) {
    tree::step::<S>(&mut gc.tags.tree, surface);
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
/// disabled by the user using the [`PdfOptions::disable_tags`] flag, or we're
/// inside a tiling.
pub fn disabled(gc: &GlobalContext) -> bool {
    gc.options.disable_tags || gc.tags.in_tiling
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

    // Marked content
    let lang = gc.tags.try_set_lang(text.lang);
    let lang = lang.as_ref().map(Lang::as_str);
    let content = ContentTag::Span(SpanTag::empty().with_lang(lang));
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
