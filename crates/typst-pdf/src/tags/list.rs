use krilla::tagging::{ListNumbering, Tag, TagKind};

use crate::tags::{GroupContents, Groups, TagNode};

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

    pub fn push_label(&mut self, groups: &mut Groups, contents: GroupContents) {
        let label = groups.init_tag(Tag::Lbl, contents);
        self.items.push(ListItem { label, body: None, sub_list: None });
    }

    pub fn push_body(&mut self, groups: &mut Groups, contents: GroupContents) {
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
        if let [.., TagNode::Group(id)] = groups.get(contents.id).nodes.as_slice()
            && let Some(TagKind::L(_)) = groups.get(*id).state.tag()
        {
            item.sub_list = groups.get_mut(contents.id).nodes.pop();
        }

        item.body = Some(groups.init_tag(Tag::LBody, contents));
    }

    pub fn push_bib_entry(&mut self, groups: &mut Groups, contents: GroupContents) {
        let nodes = vec![groups.init_tag(Tag::BibEntry, contents)];
        // Bibliography lists cannot be nested, but may be missing labels.
        let body = groups.new_virtual(Tag::LBody, nodes);
        if let Some(item) = self.items.last_mut().filter(|item| item.body.is_none()) {
            item.body = Some(body);
        } else {
            self.items.push(ListItem {
                label: groups.new_empty(Tag::Lbl),
                body: Some(body),
                sub_list: None,
            });
        }
    }

    pub fn build_list(self, groups: &mut Groups, contents: GroupContents) -> TagNode {
        for item in self.items.into_iter() {
            let nodes = vec![
                item.label,
                item.body.unwrap_or_else(|| groups.new_empty(Tag::LBody)),
            ];
            let node = groups.new_virtual(Tag::LI, nodes);
            groups.get_mut(contents.id).nodes.push(node);
            if let Some(sub_list) = item.sub_list {
                groups.get_mut(contents.id).nodes.push(sub_list);
            }
        }
        groups.init_tag(Tag::L(self.numbering), contents)
    }
}
