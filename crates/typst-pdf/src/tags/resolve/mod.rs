use std::num::NonZeroU16;

use ecow::EcoVec;
use krilla::tagging::{self as kt, Node, Tag, TagKind};
use krilla::tagging::{Identifier, TagTree};
use smallvec::SmallVec;
use typst_library::diag::{At, SourceDiagnostic, SourceResult, error};
use typst_library::text::Locale;
use typst_syntax::Span;

use crate::PdfOptions;
use crate::convert::{GlobalContext, to_span};
use crate::tags::context::{self, Annotations, BBoxCtx, Ctx};
use crate::tags::flat::{FlatTagData, ResolvedGroupKind};
use crate::tags::groups::{GroupId, GroupKind, TagStorage};
use crate::tags::resolve::accumulator::Accumulator;
use crate::tags::tree::ResolvedTextAttrs;
use crate::tags::util;
use crate::tags::{AnnotationId, disabled};

mod accumulator;

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
    flat: &'a FlatTagData,
    tags: &'a mut TagStorage,
    annotations: &'a mut Annotations,
    last_heading_level: Option<NonZeroU16>,
    flatten: bool,
    errors: EcoVec<SourceDiagnostic>,
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
    gc.tags.tree.assert_finished_traversal().at(Span::detached())?;

    if !disabled(gc) {
        context::finish(&mut gc.tags.tree);
    }

    // Extract doc_lang from root BEFORE flattening (flatten drains the list).
    let root = gc.tags.tree.groups.list.get(GroupId::ROOT);
    let GroupKind::Root(mut doc_lang) = root.kind else { unreachable!() };

    if disabled(gc) {
        return Ok((doc_lang, TagTree::new()));
    }

    // Flatten the Groups tree into a compact FlatTagTree representation.
    // This drains all Group data into parallel arrays, drops the HashMap,
    // and moves TagStorage out. The original Groups struct is now empty.
    let mut flat = gc.tags.tree.groups.flatten();

    let root_children = flat.data.children(GroupId::ROOT.idx());

    let mut resolver = Resolver {
        options: gc.options,
        ctx: &gc.tags.tree.ctx,
        flat: &flat.data,
        tags: &mut flat.tag_storage,
        annotations: &mut gc.tags.annotations,
        last_heading_level: None,
        flatten: false,
        errors: std::mem::take(&mut gc.tags.tree.errors),
    };

    let mut accum = Accumulator::root();
    accum.reserve(root_children.len());

    for child in root_children.iter() {
        resolve_node(&mut resolver, &mut doc_lang, &mut None, &mut accum, child);
    }

    if !resolver.errors.is_empty() {
        return Err(resolver.errors);
    }

    let children = accum.finish();

    // Drop the flat tree to free remaining memory.
    drop(flat);

    Ok((doc_lang, TagTree::from(children)))
}

/// Resolves nodes into an accumulator.
fn resolve_node(
    rs: &mut Resolver,
    parent_lang: &mut Option<Locale>,
    parent_bbox: &mut Option<BBoxCtx>,
    accum: &mut Accumulator,
    node: &TagNode,
) {
    match &node {
        TagNode::Group(id) => {
            resolve_group_node(rs, parent_lang, parent_bbox, accum, *id);
        }
        TagNode::Leaf(identifier) => {
            accum.push(Node::Leaf(*identifier));
        }
        TagNode::Annotation(id) => {
            accum.push(rs.annotations.take(*id));
        }
        TagNode::Text(attrs, ids) => {
            resolve_text(accum, attrs, ids);
        }
    }
}

fn resolve_group_node(
    rs: &mut Resolver,
    parent_lang: &mut Option<Locale>,
    mut parent_bbox: &mut Option<BBoxCtx>,
    mut accum: &mut Accumulator,
    id: GroupId,
) {
    let idx = id.idx();

    let tag = build_group_tag(rs, idx);
    let kind = rs.flat.kind(idx);
    let mut lang = rs.flat.lang(idx).filter(|_| tag.is_some());
    let mut bbox = rs.flat.bbox(idx).and_then(|id| rs.ctx.bbox_by_id(id)).cloned();
    let is_artifact = kind.is_artifact();
    let is_weak = rs.flat.is_weak(idx);
    let group_children = rs.flat.children(idx);

    // If this group doesn't produce a tag, don't create a nested accumulator
    // and push the children directly into the parent.
    let mut nested_children = None;
    let children = if let Some(tag) = &tag {
        let nesting = element_kind(tag);
        nested_children.insert(accum.nest(nesting))
    } else {
        &mut accum
    };

    // If a tag has an alternative description specified, flatten the children
    // tags, only retaining link tags, because they are required. The inner tags
    // won't be ingested by AT anyway, but would still have to comply with all
    // rules, which can be annoying.
    let flatten = tag.as_ref().is_some_and(|t| t.alt_text().is_some());

    rs.with_flatten(flatten, |rs| {
        let lang = lang.as_mut().unwrap_or(parent_lang);
        let bbox = if bbox.is_some() { &mut bbox } else { &mut parent_bbox };

        // In PDF 1.7, don't include artifacts in the tag tree. In PDF 2.0
        // this might become an `Artifact` tag.
        if is_artifact {
            for child in group_children.iter() {
                resolve_artifact_node(rs, bbox, child);
            }
        } else {
            children.reserve(group_children.len());
            for child in group_children.iter() {
                resolve_node(rs, lang, bbox, children, child);
            }
        }
    });

    // Try to propagate the group's language to the parent tag.
    let lang = util::propagate_lang(parent_lang, lang.flatten());

    // Update the parent bbox.
    if let Some((parent, child)) = parent_bbox.as_mut().zip(bbox.as_ref()) {
        parent.expand_page(child);
    }

    // If this isn't a tagged group the children have already been inserted
    // directly into the parent
    let Some((mut tag, nested_children)) = tag.zip(nested_children) else { return };

    // Omit the weak group if it is empty.
    let nodes = nested_children.finish();
    if is_weak && nodes.is_empty() {
        return;
    }

    tag.set_lang(lang.map(|l| l.rfc_3066().to_string()));
    if let Some(bbox) = bbox {
        match &mut tag {
            TagKind::Table(tag) => tag.set_bbox(bbox.to_krilla()),
            TagKind::Figure(tag) => tag.set_bbox(bbox.to_krilla()),
            TagKind::Formula(tag) => tag.set_bbox(bbox.to_krilla()),
            _ => (),
        }
    }

    if rs.options.is_pdf_ua() {
        validate_children(rs, &tag, &nodes);
    }

    accum.push(Node::Group(kt::TagGroup::with_children(tag, nodes)));
}

fn resolve_text(
    accum: &mut Accumulator,
    attrs: &ResolvedTextAttrs,
    children: &[kt::Identifier],
) {
    enum Prev<'a> {
        Children(&'a [kt::Identifier]),
        Group(kt::TagGroup),
    }

    impl Prev<'_> {
        fn into_nodes(self) -> Vec<Node> {
            match self {
                Prev::Children(ids) => ids.iter().map(|id| Node::Leaf(*id)).collect(),
                Prev::Group(group) => vec![Node::Group(group)],
            }
        }
    }

    let mut prev = Prev::Children(children);
    if attrs.script.is_some() || attrs.background.is_some() || attrs.deco.is_some() {
        let tag = Tag::Span
            .with_line_height(attrs.script.map(|s| s.lineheight))
            .with_baseline_shift(attrs.script.map(|s| s.baseline_shift))
            .with_background_color(attrs.background.flatten())
            .with_text_decoration_type(attrs.deco.map(|d| d.kind.to_krilla()))
            .with_text_decoration_color(attrs.deco.and_then(|d| d.color))
            .with_text_decoration_thickness(attrs.deco.and_then(|d| d.thickness));

        let group = kt::TagGroup::with_children(tag, prev.into_nodes());
        prev = Prev::Group(group);
    }
    if attrs.strong == Some(true) {
        let group = kt::TagGroup::with_children(Tag::Strong, prev.into_nodes());
        prev = Prev::Group(group);
    }
    if attrs.emph == Some(true) {
        let group = kt::TagGroup::with_children(Tag::Em, prev.into_nodes());
        prev = Prev::Group(group);
    }

    match prev {
        Prev::Group(group) => accum.push(Node::Group(group)),
        Prev::Children(ids) => accum.extend(ids.iter().map(|id| Node::Leaf(*id))),
    }
}

/// Currently only done to resolve bounding boxes.
fn resolve_artifact_node(
    rs: &mut Resolver,
    mut parent_bbox: &mut Option<BBoxCtx>,
    node: &TagNode,
) {
    match &node {
        TagNode::Group(id) => {
            let idx = id.idx();
            let mut bbox = rs.flat.bbox(idx).and_then(|id| rs.ctx.bbox_by_id(id)).cloned();
            let group_children = rs.flat.children(idx);

            {
                let bbox = if bbox.is_some() { &mut bbox } else { &mut parent_bbox };
                for child in group_children.iter() {
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

fn build_group_tag(rs: &mut Resolver, idx: usize) -> Option<TagKind> {
    // First pass: extract what we need from the kind without holding a
    // long-lived immutable borrow on rs.flat, so we can call
    // tag_storage.take() (mutable) when needed.
    enum TagSource {
        /// Tag was built directly from kind data.
        Direct(TagKind),
        /// Need to take from tag_storage by TagId.
        TakeFromStorage(crate::tags::context::TagId),
        /// No tag for this group kind.
        None,
        /// Unreachable (Root).
        Unreachable,
    }

    let kind = rs.flat.kind(idx);
    let is_link = kind.is_link();
    let source = match kind {
        ResolvedGroupKind::Root => TagSource::Unreachable,
        ResolvedGroupKind::Artifact(_) => TagSource::None,
        ResolvedGroupKind::LogicalParent => TagSource::None,
        ResolvedGroupKind::LogicalChild => TagSource::None,
        ResolvedGroupKind::Outline => TagSource::Direct(Tag::TOC.into()),
        ResolvedGroupKind::OutlineEntry => TagSource::Direct(Tag::TOCI.into()),
        ResolvedGroupKind::Table(id) => TagSource::Direct(rs.ctx.tables.get(*id).build_tag()),
        ResolvedGroupKind::TableCell(tag_id) => TagSource::TakeFromStorage(*tag_id),
        ResolvedGroupKind::Grid => TagSource::Direct(Tag::Div.into()),
        ResolvedGroupKind::GridCell => TagSource::Direct(Tag::Div.into()),
        ResolvedGroupKind::List(numbering) => TagSource::Direct(Tag::L(*numbering).into()),
        ResolvedGroupKind::ListItemLabel => TagSource::Direct(Tag::Lbl.into()),
        ResolvedGroupKind::ListItemBody => TagSource::Direct(Tag::LBody.into()),
        ResolvedGroupKind::TermsItemLabel => TagSource::Direct(Tag::Lbl.into()),
        ResolvedGroupKind::TermsItemBody => TagSource::Direct(Tag::LBody.into()),
        ResolvedGroupKind::BibEntry => TagSource::Direct(Tag::BibEntry.into()),
        ResolvedGroupKind::FigureWrapper(id) => {
            match rs.ctx.figures.get(*id).build_wrapper_tag() {
                Some(tag) => TagSource::Direct(tag),
                None => return None,
            }
        }
        ResolvedGroupKind::Figure(id) => {
            match rs.ctx.figures.get(*id).build_tag() {
                Some(tag) => TagSource::Direct(tag),
                None => return None,
            }
        }
        ResolvedGroupKind::FigureCaption => TagSource::Direct(Tag::Caption.into()),
        ResolvedGroupKind::Image { alt } => {
            let alt = alt.as_ref().map(Into::into);
            TagSource::Direct(Tag::Figure(alt).with_placement(Some(kt::Placement::Block)).into())
        }
        ResolvedGroupKind::Formula { alt, block } => {
            let alt = alt.as_ref().map(Into::into);
            let placement = block.then_some(kt::Placement::Block);
            TagSource::Direct(Tag::Formula(alt).with_placement(placement).into())
        }
        ResolvedGroupKind::Link => TagSource::Direct(Tag::Link.into()),
        ResolvedGroupKind::CodeBlock => {
            TagSource::Direct(Tag::Code.with_placement(Some(kt::Placement::Block)).into())
        }
        ResolvedGroupKind::CodeBlockLine => TagSource::Direct(Tag::P.into()),
        ResolvedGroupKind::Par => TagSource::Direct(Tag::P.into()),
        ResolvedGroupKind::TextAttr => TagSource::None,
        ResolvedGroupKind::Transparent => TagSource::None,
        ResolvedGroupKind::Standard(tag_id) => TagSource::TakeFromStorage(*tag_id),
    };
    // Now the immutable borrow of `kind` is dropped.

    let tag = match source {
        TagSource::Direct(tag) => tag,
        TagSource::TakeFromStorage(tag_id) => rs.tags.take(tag_id),
        TagSource::None => return None,
        TagSource::Unreachable => unreachable!(),
    };

    let span = rs.flat.span(idx);
    let tag = tag.with_location(Some(span.into_raw()));

    if rs.flatten && !is_link {
        return None;
    }

    // Check that no heading levels were skipped.
    if let TagKind::Hn(tag) = &tag {
        let prev_level = rs.last_heading_level.map_or(0, |l| l.get());
        let next_level = tag.level();
        if rs.options.is_pdf_ua() && next_level.get().saturating_sub(prev_level) > 1 {
            let span = to_span(tag.as_any().location);
            let validator = rs.options.standards.config.validator().as_str();
            if rs.last_heading_level.is_none() {
                rs.errors.push(error!(
                    span,
                    "{validator} error: the first heading must be of level 1",
                ));
            } else {
                rs.errors.push(error!(
                    span,
                    "{validator} error: skipped from heading level \
                     {prev_level} to {next_level}";
                    hint: "heading levels must be consecutive";
                ));
            }
        }

        rs.last_heading_level = Some(next_level);
    }

    Some(tag)
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum ElementKind {
    Grouping,
    Block,
    Table,
    Inline,
}

fn element_kind(tag: &TagKind) -> ElementKind {
    match tag {
        TagKind::Part(_)
        | TagKind::Article(_)
        | TagKind::Section(_)
        | TagKind::Div(_)
        | TagKind::BlockQuote(_)
        | TagKind::Caption(_)
        | TagKind::TOC(_)
        | TagKind::TOCI(_)
        | TagKind::Index(_)
        | TagKind::NonStruct(_) => ElementKind::Grouping,
        TagKind::P(_)
        | TagKind::Hn(_)
        | TagKind::L(_)
        | TagKind::LI(_)
        | TagKind::Lbl(_)
        | TagKind::LBody(_)
        | TagKind::Table(_) => ElementKind::Block,
        TagKind::THead(_)
        | TagKind::TBody(_)
        | TagKind::TFoot(_)
        | TagKind::TR(_)
        | TagKind::TH(_)
        | TagKind::TD(_) => ElementKind::Table,
        TagKind::Span(_)
        | TagKind::InlineQuote(_)
        | TagKind::Note(_)
        | TagKind::Reference(_)
        | TagKind::BibEntry(_)
        | TagKind::Code(_)
        | TagKind::Link(_)
        | TagKind::Annot(_)
        | TagKind::Figure(_)
        | TagKind::Formula(_)
        | TagKind::Form(_) => ElementKind::Inline,
        // Mapped to `Span`.
        TagKind::Datetime(_) => ElementKind::Inline,
        // Mapped to `Part`.
        TagKind::Terms(_) => ElementKind::Grouping,
        // Mapped to `P`.
        TagKind::Title(_) => ElementKind::Block,
        // Mapped to `Span`.
        TagKind::Strong(_) | TagKind::Em(_) => ElementKind::Inline,
    }
}

fn validate_children(rs: &mut Resolver, tag: &TagKind, children: &[Node]) {
    match tag {
        TagKind::TOC(_) => validate_children_groups(rs, tag, children, |child| {
            matches!(child, TagKind::TOC(_) | TagKind::TOCI(_))
        }),
        TagKind::TOCI(_) => validate_children_groups(rs, tag, children, |child| {
            matches!(
                child,
                TagKind::TOC(_)
                    | TagKind::Reference(_)
                    | TagKind::NonStruct(_)
                    | TagKind::P(_)
                    | TagKind::Lbl(_)
            )
        }),
        TagKind::L(_) => validate_children_groups(rs, tag, children, |child| {
            matches!(child, TagKind::Caption(_) | TagKind::L(_) | TagKind::LI(_))
        }),
        TagKind::LI(_) => validate_children_groups(rs, tag, children, |child| {
            matches!(child, TagKind::Lbl(_) | TagKind::LBody(_))
        }),
        TagKind::Table(_) => validate_children_groups(rs, tag, children, |child| {
            matches!(
                child,
                TagKind::Caption(_)
                    | TagKind::THead(_)
                    | TagKind::TBody(_)
                    | TagKind::TFoot(_)
                    | TagKind::TR(_)
            )
        }),
        TagKind::THead(_) | TagKind::TBody(_) | TagKind::TFoot(_) => {
            validate_children_groups(rs, tag, children, |child| {
                matches!(child, TagKind::TR(_))
            })
        }
        TagKind::TR(_) => validate_children_groups(rs, tag, children, |child| {
            matches!(child, TagKind::TD(_) | TagKind::TH(_))
        }),
        _ => (),
    }
}

fn validate_children_groups(
    rs: &mut Resolver,
    parent: &TagKind,
    children: &[Node],
    mut is_valid: impl FnMut(&TagKind) -> bool,
) {
    let parent_span = to_span(parent.location());

    let mut caption_spans = SmallVec::<[_; 3]>::new();
    let mut contains_leaf_nodes = false;
    for node in children {
        let Node::Group(child) = node else {
            contains_leaf_nodes = true;
            continue;
        };

        if !is_valid(&child.tag) {
            let validator = rs.options.standards.config.validator().as_str();
            let span = to_span(child.tag.location()).or(parent_span);
            let parent = tag_name(parent);
            let child = tag_name(&child.tag);
            rs.errors.push(error!(
                span,
                "{validator} error: invalid {parent} structure";
                hint: "{parent} may not contain {child}";
                hint: "this is probably caused by a show rule";
            ));
        } else if matches!(&child.tag, TagKind::Caption(_)) {
            caption_spans.push(to_span(child.tag.location()));
        }
    }

    if caption_spans.len() > 1 {
        let validator = rs.options.standards.config.validator().as_str();
        let parent = tag_name(parent);
        let child = tag_name(&Tag::Caption.into());

        let caption_error = |span| {
            error!(
                span,
                "{validator} error: invalid {parent} structure";
                hint: "{parent} may not contain multiple {child} tags";
                hint: "avoid manually calling `figure.caption`";
            )
        };
        if caption_spans.iter().all(|s| !s.is_detached()) {
            rs.errors.extend(caption_spans.into_iter().map(caption_error));
        } else {
            rs.errors.push(caption_error(parent_span));
        }
    }

    if contains_leaf_nodes {
        let validator = rs.options.standards.config.validator().as_str();
        let parent = tag_name(parent);
        rs.errors.push(error!(
            parent_span,
            "{validator} error: invalid {parent} structure";
            hint: "{parent} may not contain marked content directly";
            hint: "this is probably caused by a show rule";
        ));
    }
}

fn tag_name(tag: &TagKind) -> &'static str {
    match tag {
        TagKind::Part(_) => "part (Part)",
        TagKind::Article(_) => "article (Art)",
        TagKind::Section(_) => "section (Section)",
        TagKind::Div(_) => "division (Div)",
        TagKind::BlockQuote(_) => "block quote (BlockQuote)",
        TagKind::Caption(_) => "caption (Caption)",
        TagKind::TOC(_) => "outline (TOC)",
        TagKind::TOCI(_) => "outline entry (TOCI)",
        TagKind::Index(_) => "index (Index)",
        TagKind::P(_) => "paragraph (P)",
        TagKind::Hn(_) => "heading (Hn)",
        TagKind::L(_) => "list (L)",
        TagKind::LI(_) => "list item (LI)",
        TagKind::Lbl(_) => "label (Lbl)",
        TagKind::LBody(_) => "list body (LBody)",
        TagKind::Table(_) => "table (Table)",
        TagKind::TR(_) => "table row (TR)",
        TagKind::TH(_) => "table header cell (TH)",
        TagKind::TD(_) => "table data cell (TD)",
        TagKind::THead(_) => "table header (THead)",
        TagKind::TBody(_) => "table body (TBody)",
        TagKind::TFoot(_) => "table footer (TFoot)",
        TagKind::Span(_) => "span (Span)",
        TagKind::InlineQuote(_) => "inline quote (Quote)",
        TagKind::Note(_) => "note (Note)",
        TagKind::Reference(_) => "reference (Reference)",
        TagKind::BibEntry(_) => "bibliography entry (BibEntry)",
        TagKind::Code(_) => "raw text (Code)",
        TagKind::Link(_) => "link (Link)",
        TagKind::Annot(_) => "annotation (Annot)",
        TagKind::Figure(_) => "figure (Figure)",
        TagKind::Formula(_) => "equation (Formula)",
        TagKind::Form(_) => "form field (Form)",
        TagKind::NonStruct(_) => "non structural element (NonStruct)",
        TagKind::Datetime(_) => "date time (Span)",
        TagKind::Terms(_) => "terms (P)",
        TagKind::Title(_) => "title (Title)",
        TagKind::Strong(_) => "strong (Strong/Span)",
        TagKind::Em(_) => "emph (Em/Span)",
    }
}
