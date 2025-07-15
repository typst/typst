use krilla::tagging::{ListNumbering, Tag, TagKind};

use crate::tags::TagNode;

#[derive(Debug)]
pub struct ListCtx {
    numbering: ListNumbering,
    items: Vec<ListItem>,
}

#[derive(Debug)]
struct ListItem {
    label: Vec<TagNode>,
    body: Option<Vec<TagNode>>,
    sub_list: Option<TagNode>,
}

impl ListCtx {
    pub fn new(numbering: ListNumbering) -> Self {
        Self { numbering, items: Vec::new() }
    }

    pub fn push_label(&mut self, nodes: Vec<TagNode>) {
        self.items.push(ListItem { label: nodes, body: None, sub_list: None });
    }

    pub fn push_body(&mut self, mut nodes: Vec<TagNode>) {
        let item = self.items.last_mut().expect("ListItemLabel");

        // Nested lists are expected to have the following structure:
        //
        // Typst code
        // ```
        // - a
        // - b
        //     - c
        //     - d
        // - e
        // ```
        //
        // Structure tree
        // ```
        // <L>
        //     <LI>
        //         <Lbl> `-`
        //         <LBody> `a`
        //     <LI>
        //         <Lbl> `-`
        //         <LBody> `b`
        //     <L>
        //         <LI>
        //             <Lbl> `-`
        //             <LBody> `c`
        //         <LI>
        //             <Lbl> `-`
        //             <LBody> `d`
        //     <LI>
        //         <Lbl> `-`
        //         <LBody> `d`
        // ```
        //
        // So move the nested list out of the list item.
        if let [.., TagNode::Group(TagKind::L(_), _)] = nodes.as_slice() {
            item.sub_list = nodes.pop();
        }

        item.body = Some(nodes);
    }

    pub fn push_bib_entry(&mut self, nodes: Vec<TagNode>) {
        let nodes = vec![TagNode::group(Tag::BibEntry, nodes)];
        // Bibliography lists cannot be nested, but may be missing labels.
        if let Some(item) = self.items.last_mut().filter(|item| item.body.is_none()) {
            item.body = Some(nodes);
        } else {
            self.items.push(ListItem {
                label: Vec::new(),
                body: Some(nodes),
                sub_list: None,
            });
        }
    }

    pub fn build_list(self, mut nodes: Vec<TagNode>) -> TagNode {
        for item in self.items.into_iter() {
            nodes.push(TagNode::group(
                Tag::LI,
                vec![
                    TagNode::group(Tag::Lbl, item.label),
                    TagNode::group(Tag::LBody, item.body.unwrap_or_default()),
                ],
            ));
            if let Some(sub_list) = item.sub_list {
                nodes.push(sub_list);
            }
        }
        TagNode::group(Tag::L(self.numbering), nodes)
    }
}
