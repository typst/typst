use krilla::tagging::Tag;

use crate::tags::groups::{GroupId, GroupKind, Groups};
use crate::tags::resolve::TagNode;

#[derive(Clone, Debug)]
pub struct ListCtx {
    last_item: Option<ListItem>,
}

#[derive(Clone, Debug)]
struct ListItem {
    /// The id of the `LI` tag.
    id: GroupId,
}

impl ListCtx {
    pub fn new() -> Self {
        Self { last_item: None }
    }

    pub fn push_label(&mut self, groups: &mut Groups, list: GroupId, label: GroupId) {
        if let Some(item) = self.last_item.take() {
            groups.push_tag(item.id, Tag::LBody);
        }

        let parent = groups.push_tag(list, Tag::LI);
        groups.push_group(parent, label);

        self.last_item = Some(ListItem { id: parent });
    }

    pub fn push_body(&mut self, groups: &mut Groups, list: GroupId, body: GroupId) {
        let item = self.last_item.take().expect("ListItemLabel");

        groups.push_group(item.id, body);

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
        if let &[.., TagNode::Group(id)] = groups.get(body).nodes()
            && let GroupKind::List(..) = groups.get_mut(id).kind
        {
            groups.get_mut(body).pop_node();
            groups.get_mut(id).parent = list;
            groups.push_group(list, id);
        }
    }

    pub fn push_bib_entry(
        &mut self,
        groups: &mut Groups,
        list: GroupId,
        bib_entry: GroupId,
    ) {
        // Bibliography lists cannot be nested, but may be missing labels.
        let item = if let Some(item) = self.last_item.take() {
            item.id
        } else {
            let item = groups.push_tag(list, Tag::LI);
            groups.push_tag(item, Tag::Lbl);
            item
        };

        let body = groups.push_tag(item, Tag::LBody);
        groups.push_group(body, bib_entry);
    }
}
