use krilla::tagging::TagKind;
use typst_library::foundations::Packed;
use typst_library::model::OutlineEntry;

use crate::tags::TagNode;

#[derive(Debug)]
pub(crate) struct OutlineCtx {
    stack: Vec<OutlineSection>,
}

impl OutlineCtx {
    pub(crate) fn new() -> Self {
        Self { stack: Vec::new() }
    }

    pub(crate) fn insert(
        &mut self,
        outline_nodes: &mut Vec<TagNode>,
        entry: Packed<OutlineEntry>,
        nodes: Vec<TagNode>,
    ) {
        let expected_len = entry.level.get() - 1;
        if self.stack.len() < expected_len {
            self.stack.resize_with(expected_len, OutlineSection::new);
        } else {
            while self.stack.len() > expected_len {
                self.finish_section(outline_nodes);
            }
        }

        let section_entry = TagNode::Group(TagKind::TOCI.into(), nodes);
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

    pub(crate) fn build_outline(mut self, mut outline_nodes: Vec<TagNode>) -> TagNode {
        while !self.stack.is_empty() {
            self.finish_section(&mut outline_nodes);
        }
        TagNode::Group(TagKind::TOC.into(), outline_nodes)
    }
}

#[derive(Debug)]
pub(crate) struct OutlineSection {
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
        TagNode::Group(TagKind::TOC.into(), self.entries)
    }
}
