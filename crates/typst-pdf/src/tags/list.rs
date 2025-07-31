use krilla::tagging::{ListNumbering, Tag, TagKind};

use crate::tags::{GroupContents, TagNode};

#[derive(Clone, Debug)]
pub struct ListCtx {
    numbering: ListNumbering,
    items: Vec<ListItem>,
}

#[derive(Clone, Debug)]
struct ListItem {
    label: TagNode,
    body: Option<TagNode>,
    sub_list: Option<TagNode>,
}

impl ListCtx {
    pub fn new(numbering: ListNumbering) -> Self {
        Self { numbering, items: Vec::new() }
    }

    pub fn push_label(&mut self, contents: GroupContents) {
        self.items.push(ListItem {
            label: TagNode::group(Tag::Lbl, contents),
            body: None,
            sub_list: None,
        });
    }

    pub fn push_body(&mut self, mut contents: GroupContents) {
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
        if let [.., TagNode::Group(group)] = contents.nodes.as_slice()
            && let TagKind::L(_) = group.tag
        {
            item.sub_list = contents.nodes.pop();
        }

        item.body = Some(TagNode::group(Tag::LBody, contents));
    }

    pub fn push_bib_entry(&mut self, contents: GroupContents) {
        let nodes = vec![TagNode::group(Tag::BibEntry, contents)];
        // Bibliography lists cannot be nested, but may be missing labels.
        let body = TagNode::virtual_group(Tag::LBody, nodes);
        if let Some(item) = self.items.last_mut().filter(|item| item.body.is_none()) {
            item.body = Some(body);
        } else {
            self.items.push(ListItem {
                label: TagNode::empty_group(Tag::Lbl),
                body: Some(body),
                sub_list: None,
            });
        }
    }

    pub fn build_list(self, mut contents: GroupContents) -> TagNode {
        for item in self.items.into_iter() {
            contents.nodes.push(TagNode::virtual_group(
                Tag::LI,
                vec![
                    item.label,
                    item.body.unwrap_or_else(|| TagNode::empty_group(Tag::LBody)),
                ],
            ));
            if let Some(sub_list) = item.sub_list {
                contents.nodes.push(sub_list);
            }
        }
        TagNode::group(Tag::L(self.numbering), contents)
    }
}
