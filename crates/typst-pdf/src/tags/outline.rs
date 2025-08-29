use krilla::tagging::Tag;
use typst_library::foundations::Packed;
use typst_library::model::OutlineEntry;

use crate::tags::{GroupContents, TagNode};

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
        outline_nodes: &mut Vec<TagNode>,
        entry: Packed<OutlineEntry>,
        contents: GroupContents,
    ) {
        let expected_len = entry.level.get() - 1;
        if self.stack.len() < expected_len {
            self.stack.resize_with(expected_len, OutlineSection::new);
        } else {
            while self.stack.len() > expected_len {
                self.finish_section(outline_nodes);
            }
        }

        let section_entry = TagNode::group(Tag::TOCI, contents);
        self.push(outline_nodes, section_entry);
    }

    fn finish_section(&mut self, outline_nodes: &mut Vec<TagNode>) {
        let sub_section = self.stack.pop().unwrap().into_tag();
        self.push(outline_nodes, sub_section);
    }

    fn push(&mut self, outline_nodes: &mut Vec<TagNode>, entry: TagNode) {
        match self.stack.last_mut() {
            Some(section) => section.push(entry),
            None => outline_nodes.push(entry),
        }
    }

    pub fn build_outline(mut self, mut contents: GroupContents) -> TagNode {
        while !self.stack.is_empty() {
            self.finish_section(&mut contents.nodes);
        }
        TagNode::group(Tag::TOC, contents)
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

    fn into_tag(self) -> TagNode {
        TagNode::virtual_group(Tag::TOC, self.entries)
    }
}
