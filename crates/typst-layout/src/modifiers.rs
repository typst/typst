use std::sync::Arc;

use typst_library::foundations::StyleChain;
use typst_library::layout::{Abs, Fragment, Frame, FrameItem, HideElem, Point, Sides};
use typst_library::model::{LinkElem, Linkage};

/// Frame-level modifications resulting from styles that do not impose any
/// layout structure.
///
/// These are always applied at the highest level of style uniformity.
/// Consequently, they must be applied by all layouters that manually manage
/// styles of their children (because they can produce children with varying
/// styles). This currently includes flow, inline, and math layout.
///
/// Other layouters don't manually need to handle it because their parents that
/// result from realization will take care of it and the styles can only apply
/// to them as a whole, not part of it (since they don't manage styles).
///
/// Currently existing frame modifiers are:
/// - `HideElem::hidden`
/// - `LinkElem::dests`
#[derive(Debug, Clone)]
pub struct FrameModifiers {
    /// A destination to link to, and the location of its link marker.
    link: Option<Arc<Linkage>>,
    /// Whether the contents of the frame should be hidden.
    pub(crate) hidden: bool,
}

impl FrameModifiers {
    pub(super) fn with_link(link: Linkage) -> Self {
        Self { link: Some(Arc::new(link)), hidden: false }
    }

    /// Retrieve all modifications that should be applied per-frame.
    pub fn get_in(styles: StyleChain) -> Self {
        Self {
            link: styles.get_cloned(LinkElem::current),
            hidden: styles.get(HideElem::hidden),
        }
    }
}

/// Applies [`FrameModifiers`].
pub trait FrameModify {
    /// Apply the modifiers in-place.
    fn modify(&mut self, modifiers: &FrameModifiers);

    /// Apply the modifiers, and return the modified result.
    fn modified(mut self, modifiers: &FrameModifiers) -> Self
    where
        Self: Sized,
    {
        self.modify(modifiers);
        self
    }
}

impl FrameModify for Frame {
    fn modify(&mut self, modifiers: &FrameModifiers) {
        modify_frame(self, modifiers, None);
    }
}

impl FrameModify for Fragment {
    fn modify(&mut self, modifiers: &FrameModifiers) {
        for frame in self.iter_mut() {
            frame.modify(modifiers);
        }
    }
}

impl<T, E> FrameModify for Result<T, E>
where
    T: FrameModify,
{
    fn modify(&mut self, props: &FrameModifiers) {
        if let Ok(inner) = self {
            inner.modify(props);
        }
    }
}

pub trait FrameModifyText {
    /// Resolve and apply [`FrameModifiers`] for frames that include text. Among
    /// other effects, this applies padding around link bounding boxes.
    fn modify_text(&mut self, modifiers: &FrameModifiers, leading: Abs);

    /// Resolve and apply [`FrameModifiers`] for this shaped text frame,
    /// ignoring modifiers that are applied by the paragraph as a whole, such as
    /// links.
    fn modify_shaped_text(&mut self, styles: StyleChain);
}

impl FrameModifyText for Frame {
    fn modify_text(&mut self, modifiers: &FrameModifiers, leading: Abs) {
        let expand_y = 0.5 * leading;
        let outset = Sides::new(Abs::zero(), expand_y, Abs::zero(), expand_y);
        modify_frame(self, modifiers, Some(outset));
    }

    fn modify_shaped_text(&mut self, styles: StyleChain) {
        apply_hide(self, &FrameModifiers::get_in(styles));
    }
}

pub(super) fn modify_frame(
    frame: &mut Frame,
    modifiers: &FrameModifiers,
    link_box_outset: Option<Sides<Abs>>,
) {
    if let Some(link) = &modifiers.link {
        let mut pos = Point::zero();
        let mut size = frame.size();
        if let Some(outset) = link_box_outset {
            pos.y -= outset.top;
            pos.x -= outset.left;
            size += outset.sum_by_axis();
        }
        frame.push(pos, FrameItem::Link(link.clone(), size));
    }

    apply_hide(frame, modifiers);
}

pub(super) fn apply_hide(frame: &mut Frame, modifiers: &FrameModifiers) {
    if modifiers.hidden {
        frame.hide();
    }
}

/// Performs layout and modification in one step.
///
/// This just runs `layout(styles).modified(&FrameModifiers::get_in(styles))`,
/// but with the additional step that redundant modifiers (which are already
/// applied here) are removed from the `styles` passed to `layout`. This is used
/// for the layout of containers like `block`.
pub fn layout_and_modify<F, R>(styles: StyleChain, layout: F) -> R
where
    F: FnOnce(StyleChain) -> R,
    R: FrameModify,
{
    layout_and_modify_internal(styles, layout, true)
}

/// Performs layout and modification in one step, except for links, which are
/// handled by other parts of layout.
pub fn layout_and_modify_without_links<F, R>(styles: StyleChain<'_>, layout: F) -> R
where
    F: FnOnce(StyleChain) -> R,
    R: FrameModify,
{
    // While we could set 'modifiers.dest = None' directly here, a boolean
    // argument is used so the function remembers to reset the link in the
    // style chain before removing it from modifiers.
    layout_and_modify_internal(styles, layout, false)
}

fn layout_and_modify_internal<F, R>(styles: StyleChain, layout: F, allow_links: bool) -> R
where
    F: FnOnce(StyleChain) -> R,
    R: FrameModify,
{
    let mut modifiers = FrameModifiers::get_in(styles);

    // Disable the current link internally since it's already applied at this
    // level of layout. This means we don't generate redundant nested links,
    // which may bloat the output considerably.
    let reset;
    let outer = styles;
    let mut styles = styles;
    if modifiers.link.is_some() {
        reset = LinkElem::current.set(None).wrap();
        styles = outer.chain(&reset);
    }

    if !allow_links {
        modifiers.link = None;
    }

    layout(styles).modified(&modifiers)
}
