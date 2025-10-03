use krilla::tagging::Tag;

use crate::tags::groups::{GroupId, GroupKind, Groups};
use crate::tags::resolve::TagNode;

#[derive(Debug, Clone)]
pub struct ListCtx {
    last_item: Option<ListItem>,
}

#[derive(Debug, Clone)]
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

    fn ensure_within_item(&mut self, groups: &mut Groups, list: GroupId) -> GroupId {
        if let Some(item) = self.last_item.take() {
            item.id
        } else {
            let item = groups.push_tag(list, Tag::LI);
            groups.push_tag(item, Tag::Lbl);
            item
        }
    }

    pub fn push_body(&mut self, groups: &mut Groups, list: GroupId, body: GroupId) {
        let item = self.ensure_within_item(groups, list);
        groups.push_group(item, body);

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
        // Bibliography lists are always flat, so there is no need to check for
        // an inner list. If they do contain a list it is semantically unrelated
        // and can be left within the list body.
        let item = self.ensure_within_item(groups, list);
        let body = groups.push_tag(item, Tag::LBody);
        groups.push_group(body, bib_entry);
    }
}
