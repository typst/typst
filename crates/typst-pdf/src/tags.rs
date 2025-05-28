use std::cell::OnceCell;

use krilla::surface::Surface;
use krilla::tagging::{ContentTag, Identifier, Node, Tag, TagGroup, TagTree};
use typst_library::foundations::{Content, StyleChain};
use typst_library::introspection::Location;
use typst_library::model::{HeadingElem, OutlineElem, OutlineEntry};

use crate::convert::GlobalContext;

pub(crate) struct Tags {
    /// The intermediary stack of nested tag groups.
    pub(crate) stack: Vec<(Location, Tag, Vec<TagNode>)>,
    pub(crate) placeholders: Vec<OnceCell<Node>>,
    pub(crate) in_artifact: bool,

    /// The output.
    pub(crate) tree: Vec<TagNode>,
}

pub(crate) enum TagNode {
    Group(Tag, Vec<TagNode>),
    Leaf(Identifier),
    /// Allows inserting a placeholder into the tag tree.
    /// Currently used for [`krilla::page::Page::add_tagged_annotation`].
    Placeholder(Placeholder),
}

#[derive(Clone, Copy)]
pub(crate) struct Placeholder(usize);

impl Tags {
    pub(crate) fn new() -> Self {
        Self {
            stack: Vec::new(),
            placeholders: Vec::new(),
            in_artifact: false,

            tree: Vec::new(),
        }
    }

    pub(crate) fn reserve_placeholder(&mut self) -> Placeholder {
        let idx = self.placeholders.len();
        self.placeholders.push(OnceCell::new());
        Placeholder(idx)
    }

    pub(crate) fn init_placeholder(&mut self, placeholder: Placeholder, node: Node) {
        self.placeholders[placeholder.0]
            .set(node)
            .map_err(|_| ())
            .expect("placeholder to be uninitialized");
    }

    pub(crate) fn take_placeholder(&mut self, placeholder: Placeholder) -> Node {
        self.placeholders[placeholder.0]
            .take()
            .expect("initialized placeholder node")
    }

    pub(crate) fn push(&mut self, node: TagNode) {
        if let Some((_, _, nodes)) = self.stack.last_mut() {
            nodes.push(node);
        } else {
            self.tree.push(node);
        }
    }

    pub(crate) fn build_tree(&mut self) -> TagTree {
        let mut tree = TagTree::new();
        let nodes = std::mem::take(&mut self.tree);
        // PERF: collect into vec and construct TagTree directly from tag nodes.
        for node in nodes.into_iter().map(|node| self.resolve_node(node)) {
            tree.push(node);
        }
        tree
    }

    /// Resolves [`Placeholder`] nodes.
    fn resolve_node(&mut self, node: TagNode) -> Node {
        match node {
            TagNode::Group(tag, nodes) => {
                let mut group = TagGroup::new(tag);
                // PERF: collect into vec and construct TagTree directly from tag nodes.
                for node in nodes.into_iter().map(|node| self.resolve_node(node)) {
                    group.push(node);
                }
                Node::Group(group)
            }
            TagNode::Leaf(identifier) => Node::Leaf(identifier),
            TagNode::Placeholder(placeholder) => self.take_placeholder(placeholder),
        }
    }

    pub(crate) fn context_supports(&self, tag: &Tag) -> bool {
        let Some((_, parent, _)) = self.stack.last() else { return true };

        use Tag::*;

        match parent {
            Part => true,
            Article => !matches!(tag, Article),
            Section => true,
            BlockQuote => todo!(),
            Caption => todo!(),
            TOC => matches!(tag, TOC | TOCI),
            // TODO: NonStruct is allowed to but (currently?) not supported by krilla
            TOCI => matches!(tag, TOC | Lbl | Reference | P),
            Index => todo!(),
            P => todo!(),
            H1(_) => todo!(),
            H2(_) => todo!(),
            H3(_) => todo!(),
            H4(_) => todo!(),
            H5(_) => todo!(),
            H6(_) => todo!(),
            L(_list_numbering) => todo!(),
            LI => todo!(),
            Lbl => todo!(),
            LBody => todo!(),
            Table => todo!(),
            TR => todo!(),
            TH(_table_header_scope) => todo!(),
            TD => todo!(),
            THead => todo!(),
            TBody => todo!(),
            TFoot => todo!(),
            InlineQuote => todo!(),
            Note => todo!(),
            Reference => todo!(),
            BibEntry => todo!(),
            Code => todo!(),
            Link => todo!(),
            Annot => todo!(),
            Figure(_) => todo!(),
            Formula(_) => todo!(),
            Datetime => todo!(),
            Terms => todo!(),
            Title => todo!(),
        }
    }
}

pub(crate) fn handle_open_tag(
    gc: &mut GlobalContext,
    surface: &mut Surface,
    elem: &Content,
) {
    if gc.tags.in_artifact {
        return;
    }

    let Some(loc) = elem.location() else { return };

    let tag = if let Some(heading) = elem.to_packed::<HeadingElem>() {
        let level = heading.resolve_level(StyleChain::default());
        let name = heading.body.plain_text().to_string();
        match level.get() {
            1 => Tag::H1(Some(name)),
            2 => Tag::H2(Some(name)),
            3 => Tag::H3(Some(name)),
            4 => Tag::H4(Some(name)),
            5 => Tag::H5(Some(name)),
            // TODO: when targeting PDF 2.0 headings `> 6` are supported
            _ => Tag::H6(Some(name)),
        }
    } else if let Some(_) = elem.to_packed::<OutlineElem>() {
        Tag::TOC
    } else if let Some(_outline_entry) = elem.to_packed::<OutlineEntry>() {
        Tag::TOCI
    } else {
        return;
    };

    if !gc.tags.context_supports(&tag) {
        // TODO: error or warning?
    }

    // close previous marked-content and open a nested tag.
    if !gc.tags.stack.is_empty() {
        surface.end_tagged();
    }
    let content_id = surface.start_tagged(krilla::tagging::ContentTag::Other);

    gc.tags.stack.push((loc, tag, vec![TagNode::Leaf(content_id)]));
}

pub(crate) fn handle_close_tag(
    gc: &mut GlobalContext,
    surface: &mut Surface,
    loc: &Location,
) {
    let Some((_, tag, nodes)) = gc.tags.stack.pop_if(|(l, ..)| l == loc) else {
        return;
    };

    surface.end_tagged();

    if let Some((_, _, parent_nodes)) = gc.tags.stack.last_mut() {
        parent_nodes.push(TagNode::Group(tag, nodes));

        // TODO: somehow avoid empty marked-content sequences
        let id = surface.start_tagged(ContentTag::Other);
        parent_nodes.push(TagNode::Leaf(id));
    } else {
        gc.tags.tree.push(TagNode::Group(tag, nodes));
    }
}
