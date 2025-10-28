use krilla::tagging::TagKind;
use krilla::tagging::{self as kt, Tag};
use smallvec::SmallVec;
use typst_library::foundations::Packed;
use typst_library::model::FigureElem;

use crate::tags::context::FigureId;
use crate::tags::groups::{Group, GroupId, GroupKind, Groups};
use crate::tags::resolve::TagNode;
use crate::tags::tree::Tree;
use crate::tags::util::PropertyOptRef;

#[derive(Debug, Clone, PartialEq)]
pub struct FigureCtx {
    pub group_id: GroupId,
    pub elem: Packed<FigureElem>,
    pub captions: SmallVec<[GroupId; 1]>,
    pub tag_kind: FigureTagKind,
}

/// Which tag should be used to represent this figure in the PDF tag tree.
///
/// There is a fundamental mismatch between Typst figures and PDF figures.
/// In Typst a figure is used to group some illustrative content, optionally give
/// it a caption, and often reference it by labelling it.
/// In PDF a figure is more comparable to an image, and in PDF/UA it *must* have
/// an alternative description. This alternative description *must* describe the
/// entire enclosed content and any AT will not attempt to interpret any content
/// within that tag. This makes the `Figure` tag completely unsuited for figures
/// that contain tables or other structured data.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub enum FigureTagKind {
    /// Use a `Figure` tag.
    Figure,
    /// Use a `Div` tag.
    #[default]
    Div,
    /// Don't use any tag.
    None,
}

impl FigureCtx {
    pub fn new(group_id: GroupId, elem: Packed<FigureElem>) -> Self {
        Self {
            group_id,
            elem,
            captions: SmallVec::new(),
            tag_kind: FigureTagKind::default(),
        }
    }

    pub fn build_tag(&self) -> Option<TagKind> {
        let alt = self.elem.alt.opt_ref().map(Into::into);
        Some(match self.tag_kind {
            FigureTagKind::Figure => {
                Tag::Figure(alt).with_placement(Some(kt::Placement::Block)).into()
            }
            FigureTagKind::Div => Tag::Div
                .with_alt_text(alt)
                .with_placement(Some(kt::Placement::Block))
                .into(),
            FigureTagKind::None => return None,
        })
    }

    /// Generate an enclosing `Div` tag if there is a caption.
    pub fn build_wrapper_tag(&self) -> Option<TagKind> {
        (!self.captions.is_empty()).then_some(Tag::Div.into())
    }
}

pub fn build_figure(tree: &mut Tree, figure_id: FigureId) {
    let figure_ctx = tree.ctx.figures.get_mut(figure_id);
    let group = tree.groups.get(figure_ctx.group_id);
    let wrapper = group.parent;

    if figure_ctx.elem.alt.opt_ref().is_some() {
        // If a figure has an alternative description, always use the
        // figure tag.
        figure_ctx.tag_kind = FigureTagKind::Figure;
    } else if let Some(child) = single_semantic_child(&tree.groups, group) {
        if let GroupKind::Table(..) = &tree.groups.get(child).kind {
            // Move the caption inside the table.
            let table = child;
            let captions = std::mem::take(&mut figure_ctx.captions);
            for caption in captions.iter() {
                tree.groups.get_mut(*caption).parent = table;
            }
            tree.groups.prepend_groups(table, &captions);
        }

        // Omit an additional wrapping tag.
        figure_ctx.tag_kind = FigureTagKind::None;
    } else if !group.nodes().iter().any(|n| matches!(n, TagNode::Group(_))) {
        // The figure contains only marked content.
        figure_ctx.tag_kind = FigureTagKind::Figure;
    }

    // Insert the captions inside the figure.
    // an enclosing element.
    if !figure_ctx.captions.is_empty() {
        for &caption in figure_ctx.captions.iter() {
            tree.groups.get_mut(caption).parent = wrapper;
        }
        tree.groups.prepend_groups(wrapper, &figure_ctx.captions);
    }
}

fn single_semantic_child<'a>(
    groups: &'a Groups,
    mut group: &'a Group,
) -> Option<GroupId> {
    while let [TagNode::Group(id)] = group.nodes() {
        group = groups.get(*id);
        if group.kind.is_semantic() {
            return Some(*id);
        }
    }
    None
}
