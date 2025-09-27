use std::num::NonZeroU16;

use krilla::tagging::{self as kt, Node, Tag, TagKind};
use krilla::tagging::{Identifier, TagTree};
use typst_library::diag::{SourceResult, bail};
use typst_library::text::Locale;

use crate::PdfOptions;
use crate::convert::{GlobalContext, to_span};
use crate::tags::context::{Annotations, BBoxCtx, Ctx};
use crate::tags::groups::{Group, GroupId, GroupKind, TagStorage};
use crate::tags::text::ResolvedTextAttrs;
use crate::tags::util::{self, IdVec, PropertyOptRef, PropertyValCopied};
use crate::tags::{AnnotationId, disabled};

#[derive(Debug, Clone, PartialEq)]
pub enum TagNode {
    Group(GroupId),
    Leaf(Identifier),
    /// Allows inserting a annotation into the tag tree.
    /// Currently used for [`krilla::page::Page::add_tagged_annotation`].
    Annotation(AnnotationId),
    /// If the attributes are non-empty this will resolve to a [`Tag::Span`],
    /// otherwise the items are inserted directly.
    Text(ResolvedTextAttrs, Vec<Identifier>),
}

struct Resolver<'a> {
    options: &'a PdfOptions<'a>,
    ctx: &'a Ctx,
    groups: &'a IdVec<Group>,
    tags: &'a mut TagStorage,
    annotations: &'a mut Annotations,
    last_heading_level: Option<NonZeroU16>,
    flatten: bool,
}

impl<'a> Resolver<'a> {
    fn with_flatten<T>(&mut self, flatten: bool, f: impl FnOnce(&mut Self) -> T) -> T {
        let prev = self.flatten;
        self.flatten |= flatten;
        let res = f(self);
        self.flatten = prev;
        res
    }
}

pub fn resolve(gc: &mut GlobalContext) -> SourceResult<(Option<Locale>, TagTree)> {
    assert!(gc.tags.tree.finished_traversal(), "tree traversal didn't complete properly");

    let root = gc.tags.tree.groups.list.get(GroupId::ROOT);
    let GroupKind::Root(mut doc_lang) = root.kind else { unreachable!() };

    if disabled(gc) {
        return Ok((doc_lang, TagTree::new()));
    }

    let mut resolver = Resolver {
        options: gc.options,
        ctx: &gc.tags.tree.ctx,
        groups: &gc.tags.tree.groups.list,
        tags: &mut gc.tags.tree.groups.tags,
        annotations: &mut gc.tags.annotations,
        last_heading_level: None,
        flatten: false,
    };

    let mut children = Vec::with_capacity(root.nodes().len());

    for child in root.nodes().iter() {
        resolve_node(&mut resolver, &mut doc_lang, &mut None, &mut children, child)?;
    }

    Ok((doc_lang, TagTree::from(children)))
}

/// Resolves nodes into an accumulator.
fn resolve_node(
    rs: &mut Resolver,
    parent_lang: &mut Option<Locale>,
    parent_bbox: &mut Option<BBoxCtx>,
    accum: &mut Vec<Node>,
    node: &TagNode,
) -> SourceResult<()> {
    match &node {
        TagNode::Group(id) => {
            resolve_group_node(rs, parent_lang, parent_bbox, accum, *id)?;
        }
        TagNode::Leaf(identifier) => {
            accum.push(Node::Leaf(*identifier));
        }
        TagNode::Annotation(id) => {
            accum.push(rs.annotations.take(*id));
        }
        TagNode::Text(attrs, ids) => {
            attrs.resolve_nodes(accum, ids);
        }
    }
    Ok(())
}

fn resolve_group_node(
    rs: &mut Resolver,
    parent_lang: &mut Option<Locale>,
    mut parent_bbox: &mut Option<BBoxCtx>,
    accum: &mut Vec<Node>,
    id: GroupId,
) -> SourceResult<()> {
    let group = rs.groups.get(id);

    let tag = build_group_tag(rs, group)?;
    let mut lang = group.kind.lang().filter(|_| tag.is_some());
    let mut bbox = rs.ctx.bbox(&group.kind).cloned();
    let mut nodes = Vec::new();

    // If a tag has an alternative description specified, flatten the children
    // tags, only retaining link tags, because they are required. The inner tags
    // won't be ingested by AT anyway, but would still have to comply with all
    // rules, which can be annoying.
    let flatten = tag.as_ref().is_some_and(|t| t.alt_text().is_some());
    rs.with_flatten(flatten, |rs| -> SourceResult<()> {
        let lang = lang.as_mut().unwrap_or(parent_lang);
        let bbox = if bbox.is_some() { &mut bbox } else { &mut parent_bbox };

        // In PDF 1.7, don't include artifacts in the tag tree. In PDF 2.0
        // this might become an `Artifact` tag.
        if group.kind.is_artifact() {
            for child in group.nodes().iter() {
                resolve_artifact_node(rs, bbox, child);
            }
        } else {
            nodes = Vec::with_capacity(group.nodes().len());
            for child in group.nodes().iter() {
                resolve_node(rs, lang, bbox, &mut nodes, child)?;
            }
        }
        Ok(())
    })?;

    // Try to propagate the group's language to the parent tag.
    let lang = util::propagate_lang(parent_lang, lang.flatten());

    // Update the parent bbox.
    if let Some((parent, child)) = parent_bbox.as_mut().zip(bbox.as_ref()) {
        parent.expand_page(child);
    }

    // If this isn't a tagged group, forward the children to the parent.
    let Some(mut tag) = tag else {
        accum.extend(nodes);
        return Ok(());
    };

    tag.set_lang(lang.map(|l| l.rfc_3066().to_string()));
    if let Some(bbox) = bbox {
        match &mut tag {
            TagKind::Table(tag) => tag.set_bbox(bbox.to_krilla()),
            TagKind::Figure(tag) => tag.set_bbox(bbox.to_krilla()),
            TagKind::Formula(tag) => tag.set_bbox(bbox.to_krilla()),
            _ => (),
        }
    }

    accum.push(Node::Group(kt::TagGroup::with_children(tag, nodes)));

    Ok(())
}

/// Currently only done to resolve bounding boxes.
fn resolve_artifact_node(
    rs: &mut Resolver,
    mut parent_bbox: &mut Option<BBoxCtx>,
    node: &TagNode,
) {
    match &node {
        TagNode::Group(id) => {
            let group = rs.groups.get(*id);
            let mut bbox = rs.ctx.bbox(&group.kind).cloned();

            {
                let bbox = if bbox.is_some() { &mut bbox } else { &mut parent_bbox };
                for child in group.nodes().iter() {
                    resolve_artifact_node(rs, bbox, child);
                }
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

fn build_group_tag(rs: &mut Resolver, group: &Group) -> SourceResult<Option<TagKind>> {
    let tag = match &group.kind {
        GroupKind::Root(_) => unreachable!(),
        GroupKind::Artifact(_) => return Ok(None),
        GroupKind::LogicalParent(_) => return Ok(None),
        GroupKind::LogicalChild => return Ok(None),
        GroupKind::Outline(_, _) => Tag::TOC.into(),
        GroupKind::OutlineEntry(_, _) => Tag::TOCI.into(),
        GroupKind::Table(id, _, _) => rs.ctx.tables.get(*id).build_tag(),
        GroupKind::TableCell(_, tag, _) => rs.tags.take(*tag),
        GroupKind::Grid(_, _) => Tag::Div.into(),
        GroupKind::GridCell(_, _) => Tag::Div.into(),
        GroupKind::List(_, numbering, _) => Tag::L(*numbering).into(),
        GroupKind::ListItemLabel(_) => Tag::Lbl.into(),
        GroupKind::ListItemBody(_) => Tag::LBody.into(),
        GroupKind::TermsItemLabel(_) => Tag::Lbl.into(),
        GroupKind::TermsItemBody(_, _) => Tag::LBody.into(),
        GroupKind::BibEntry(_) => Tag::BibEntry.into(),
        GroupKind::Figure(id, _, _) => {
            let Some(tag) = rs.ctx.figures.get(*id).build_tag() else { return Ok(None) };
            tag
        }
        GroupKind::FigureCaption(_, _) => Tag::Caption.into(),
        GroupKind::Image(image, _, _) => {
            let alt = image.alt.opt_ref().map(String::from);
            Tag::Figure(alt).with_placement(Some(kt::Placement::Block)).into()
        }
        GroupKind::Formula(equation, _, _) => {
            let alt = equation.alt.opt_ref().map(String::from);
            let placement = equation.block.val().then_some(kt::Placement::Block);
            Tag::Formula(alt).with_placement(placement).into()
        }
        GroupKind::Link(_, _) => Tag::Link.into(),
        GroupKind::CodeBlock(_) => {
            Tag::Code.with_placement(Some(kt::Placement::Block)).into()
        }
        GroupKind::CodeBlockLine(_) => Tag::P.into(),
        GroupKind::Standard(tag, _) => rs.tags.take(*tag),
    };

    let tag = tag.with_location(Some(group.span.into_raw()));

    if rs.flatten && !group.kind.is_link() {
        return Ok(None);
    }

    // Check that no heading levels were skipped.
    if let TagKind::Hn(tag) = &tag {
        let prev_level = rs.last_heading_level.map_or(0, |l| l.get());
        let next_level = tag.level();
        if rs.options.is_pdf_ua() && next_level.get().saturating_sub(prev_level) > 1 {
            let span = to_span(tag.as_any().location);
            let validator = rs.options.standards.config.validator().as_str();
            if rs.last_heading_level.is_none() {
                bail!(span, "{validator} error: the first heading must be of level 1");
            } else {
                bail!(
                    span,
                    "{validator} error: skipped from heading level \
                        {prev_level} to {next_level}";
                    hint: "heading levels must be consecutive"
                );
            }
        }

        rs.last_heading_level = Some(next_level);
    }

    Ok(Some(tag))
}
