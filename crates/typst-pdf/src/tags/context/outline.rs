use krilla::tagging as kt;
use typst_library::foundations::Packed;
use typst_library::model::OutlineEntry;

use crate::tags::GroupId;
use crate::tags::groups::Groups;

#[derive(Clone, Debug)]
pub struct OutlineCtx {
    /// The stack of nested `TOC` entries.
    stack: Vec<GroupId>,
}

impl OutlineCtx {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    pub fn insert(
        &mut self,
        groups: &mut Groups,
        outline_id: GroupId,
        entry: Packed<OutlineEntry>,
        entry_id: GroupId,
    ) {
        let expected_len = entry.level.get() - 1;
        let mut parent = self.stack.last().copied().unwrap_or(outline_id);
        self.stack.resize_with(expected_len, || {
            parent = groups.push_tag(parent, kt::Tag::TOC);
            parent
        });

        let parent = self.stack.last().copied().unwrap_or(outline_id);
        groups.push_group(parent, entry_id);
    }
}
