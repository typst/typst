use krilla::tagging::Tag;
use typst_library::foundations::Packed;
use typst_library::model::OutlineEntry;

use crate::tags::{GroupContents, GroupId, Groups, TagNode};

#[derive(Clone, Debug)]
pub struct OutlineCtx {
    stack: Vec<OutlineSection>,
}

impl OutlineCtx {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    pub fn insert(
        &mut self,
        groups: &mut Groups,
        group_id: GroupId,
        entry: Packed<OutlineEntry>,
        contents: GroupContents,
    ) {
        let expected_len = entry.level.get() - 1;
        if self.stack.len() < expected_len {
            self.stack.resize_with(expected_len, OutlineSection::new);
        } else {
            while self.stack.len() > expected_len {
                self.finish_section(groups, group_id);
            }
        }

        let section_entry = groups.init_tag(Tag::TOCI, contents);
        self.push(groups, group_id, section_entry);
    }

    fn finish_section(&mut self, groups: &mut Groups, group_id: GroupId) {
        let sub_section = groups.new_virtual(Tag::TOC, self.stack.pop().unwrap().entries);
        self.push(groups, group_id, sub_section);
    }

    fn push(&mut self, groups: &mut Groups, group_id: GroupId, entry: TagNode) {
        match self.stack.last_mut() {
            Some(section) => section.push(entry),
            None => groups.get_mut(group_id).nodes.push(entry),
        }
    }

    pub fn build_outline(
        mut self,
        groups: &mut Groups,
        contents: GroupContents,
    ) -> TagNode {
        while !self.stack.is_empty() {
            self.finish_section(groups, contents.id);
        }
        groups.init_tag(Tag::TOC, contents)
    }
}

#[derive(Clone, Debug)]
pub struct OutlineSection {
    entries: Vec<TagNode>,
}

impl OutlineSection {
    const fn new() -> Self {
        OutlineSection { entries: Vec::new() }
    }

    fn push(&mut self, entry: TagNode) {
        self.entries.push(entry);
    }
}
