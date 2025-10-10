use std::num::NonZeroU16;

use ecow::EcoVec;
use krilla::tagging::{self as kt, Node, Tag, TagGroup, TagKind};
use krilla::tagging::{Identifier, TagTree};
use typst_library::diag::{SourceDiagnostic, SourceResult, error};
use typst_library::text::Locale;

use crate::PdfOptions;
use crate::convert::{GlobalContext, to_span};
use crate::tags::context::{Annotations, BBoxCtx, Ctx};
use crate::tags::groups::{Group, GroupId, GroupKind, TagStorage};
use crate::tags::tree::ResolvedTextAttrs;
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
    gc.tags.tree.assert_finished_traversal();

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
        errors: std::mem::take(&mut gc.tags.tree.errors),
    };

    let mut children = Vec::with_capacity(root.nodes().len());
    let mut accum = Accumulator::new(ElementKind::Grouping, &mut children);

    for child in root.nodes().iter() {
        resolve_node(&mut resolver, &mut doc_lang, &mut None, &mut accum, child);
    }

    if !resolver.errors.is_empty() {
        return Err(resolver.errors);
    }

    accum.finish();
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
    accum: &mut Accumulator,
    id: GroupId,
) {
    let group = rs.groups.get(id);

    let tag = build_group_tag(rs, group);
    let mut lang = group.kind.lang().filter(|_| tag.is_some());
    let mut bbox = rs.ctx.bbox(&group.kind).cloned();
    let mut nodes = Vec::new();
    let mut children = {
        let nesting = tag.as_ref().map(element_kind).unwrap_or(accum.nesting);
        let buf = if tag.is_some() { &mut nodes } else { &mut accum.buf };
        Accumulator::new(nesting, buf)
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
        if group.kind.is_artifact() {
            for child in group.nodes().iter() {
                resolve_artifact_node(rs, bbox, child);
            }
        } else {
            children.buf.reserve(group.nodes().len());
            for child in group.nodes().iter() {
                resolve_node(rs, lang, bbox, &mut children, child);
            }
        }
    });

    // Try to propagate the group's language to the parent tag.
    let lang = util::propagate_lang(parent_lang, lang.flatten());

    // Update the parent bbox.
    if let Some((parent, child)) = parent_bbox.as_mut().zip(bbox.as_ref()) {
        parent.expand_page(child);
    }

    // Omit the weak group if it is empty.
    if group.weak && children.num_inserted == 0 {
        return;
    }

    // If this isn't a tagged group the children we're directly inserted into
    // the parent.
    let Some(mut tag) = tag else { return };

    tag.set_lang(lang.map(|l| l.rfc_3066().to_string()));
    if let Some(bbox) = bbox {
        match &mut tag {
            TagKind::Table(tag) => tag.set_bbox(bbox.to_krilla()),
            TagKind::Figure(tag) => tag.set_bbox(bbox.to_krilla()),
            TagKind::Formula(tag) => tag.set_bbox(bbox.to_krilla()),
            _ => (),
        }
    }

    children.finish();

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

fn build_group_tag(rs: &mut Resolver, group: &Group) -> Option<TagKind> {
    let tag = match &group.kind {
        GroupKind::Root(_) => unreachable!(),
        GroupKind::Artifact(_) => return None,
        GroupKind::LogicalParent(_) => return None,
        GroupKind::LogicalChild(_, _) => return None,
        GroupKind::Outline(_, _) => Tag::TOC.into(),
        GroupKind::OutlineEntry(_, _) => Tag::TOCI.into(),
        GroupKind::Table(id, _, _) => rs.ctx.tables.get(*id).build_tag(),
        GroupKind::TableCell(_, tag, _) => rs.tags.take(*tag),
        GroupKind::Grid(_, _) => Tag::Div.into(),
        GroupKind::GridCell(_, _) => Tag::Div.into(),
        GroupKind::InternalGridCell(_) => {
            unreachable!("should be swapped out in `close_group`")
        }
        GroupKind::List(_, numbering, _) => Tag::L(*numbering).into(),
        GroupKind::ListItemLabel(_) => Tag::Lbl.into(),
        GroupKind::ListItemBody(_) => Tag::LBody.into(),
        GroupKind::TermsItemLabel(_) => Tag::Lbl.into(),
        GroupKind::TermsItemBody(_, _) => Tag::LBody.into(),
        GroupKind::BibEntry(_) => Tag::BibEntry.into(),
        GroupKind::Figure(id, _, _) => rs.ctx.figures.get(*id).build_tag()?,
        GroupKind::FigureCaption(_, _) => Tag::Caption.into(),
        GroupKind::Image(image, _, _) => {
            let alt = image.alt.opt_ref().map(Into::into);
            Tag::Figure(alt).with_placement(Some(kt::Placement::Block)).into()
        }
        GroupKind::Formula(equation, _, _) => {
            let alt = equation.alt.opt_ref().map(Into::into);
            let placement = equation.block.val().then_some(kt::Placement::Block);
            Tag::Formula(alt).with_placement(placement).into()
        }
        GroupKind::Link(_, _) => Tag::Link.into(),
        GroupKind::CodeBlock(_) => {
            Tag::Code.with_placement(Some(kt::Placement::Block)).into()
        }
        GroupKind::CodeBlockLine(_) => Tag::P.into(),
        GroupKind::Par(_) => Tag::P.into(),
        GroupKind::TextAttr(_) => return None,
        GroupKind::Transparent => return None,
        GroupKind::Standard(tag, _) => rs.tags.take(*tag),
    };

    let tag = tag.with_location(Some(group.span.into_raw()));

    if rs.flatten && !group.kind.is_link() {
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
                    "{validator} error: the first heading must be of level 1"
                ));
            } else {
                rs.errors.push(error!(
                    span,
                    "{validator} error: skipped from heading level \
                     {prev_level} to {next_level}";
                    hint: "heading levels must be consecutive"
                ));
            }
        }

        rs.last_heading_level = Some(next_level);
    }

    Some(tag)
}

struct Accumulator<'a> {
    nesting: ElementKind,
    buf: &'a mut Vec<Node>,
    num_inserted: usize,
    // Whether the last node is a `Span` used to wrap marked content sequences
    // inside a grouping element. Groupings element may not contain marked
    // content sequences directly.
    grouping_span: Option<Vec<Node>>,
}

impl std::ops::Drop for Accumulator<'_> {
    fn drop(&mut self) {
        self.push_grouping_span();
    }
}

impl<'a> Accumulator<'a> {
    fn new(nesting: ElementKind, buf: &'a mut Vec<Node>) -> Self {
        Self { nesting, buf, num_inserted: 0, grouping_span: None }
    }

    fn push_buf(&mut self, node: Node) {
        self.buf.push(node);
        self.num_inserted += 1;
    }

    fn push_grouping_span(&mut self) {
        if let Some(span_nodes) = self.grouping_span.take() {
            let tag = Tag::Span.with_placement(Some(kt::Placement::Block));
            let group = TagGroup::with_children(tag, span_nodes);
            self.push_buf(group.into());
        }
    }

    fn push(&mut self, mut node: Node) {
        if self.nesting == ElementKind::Grouping {
            match &mut node {
                Node::Group(group) => {
                    self.push_grouping_span();

                    // Ensure ILSE have block placement when inside grouping elements.
                    if element_kind(&group.tag) == ElementKind::Inline {
                        group.tag.set_placement(Some(kt::Placement::Block));
                    }

                    self.push_buf(node);
                }
                Node::Leaf(_) => {
                    let span_nodes = self.grouping_span.get_or_insert_default();
                    span_nodes.push(node);
                }
            }
        } else {
            self.push_buf(node);
        }
    }

    fn extend(&mut self, nodes: impl ExactSizeIterator<Item = Node>) {
        self.buf.reserve(nodes.len());
        for node in nodes {
            self.push(node);
        }
    }

    // Postfix drop.
    fn finish(self) {}
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
        | TagKind::Formula(_) => ElementKind::Inline,
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
    is_valid: impl Fn(&TagKind) -> bool,
) {
    let parent_span = to_span(parent.location());

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
                hint: "this is probably caused by a show rule"
            ));
        }
    }

    if contains_leaf_nodes {
        let validator = rs.options.standards.config.validator().as_str();
        let parent = tag_name(parent);
        rs.errors.push(error!(
            parent_span,
            "{validator} error: invalid {parent} structure";
            hint: "{parent} may not contain marked content directly";
            hint: "this is probably caused by a show rule"
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
        TagKind::NonStruct(_) => "non structural element (NonStruct)",
        TagKind::Datetime(_) => "date time (Span)",
        TagKind::Terms(_) => "terms (P)",
        TagKind::Title(_) => "title (Title)",
        TagKind::Strong(_) => "strong (Strong/Span)",
        TagKind::Em(_) => "emph (Em/Span)",
    }
}
