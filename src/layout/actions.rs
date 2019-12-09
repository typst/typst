//! Drawing and cofiguration actions composing layouts.

use std::fmt::{self, Display, Formatter};

use super::*;
use LayoutAction::*;

/// A layouting action.
#[derive(Clone)]
pub enum LayoutAction {
    /// Move to an absolute position.
    MoveAbsolute(Size2D),
    /// Set the font by index and font size.
    SetFont(usize, Size),
    /// Write text starting at the current position.
    WriteText(String),
    /// Visualize a box for debugging purposes.
    DebugBox(Size2D),
}

impl Serialize for LayoutAction {
    fn serialize<W: Write>(&self, f: &mut W) -> io::Result<()> {
        match self {
            MoveAbsolute(s) => write!(f, "m {:.4} {:.4}", s.x.to_pt(), s.y.to_pt()),
            SetFont(i, s) => write!(f, "f {} {}", i, s.to_pt()),
            WriteText(s) => write!(f, "w {}", s),
            DebugBox(s) => write!(f, "b {} {}", s.x.to_pt(), s.y.to_pt()),
        }
    }
}

impl Display for LayoutAction {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use LayoutAction::*;
        match self {
            MoveAbsolute(s) => write!(f, "move {} {}", s.x, s.y),
            SetFont(i, s) => write!(f, "font {} {}", i, s),
            WriteText(s) => write!(f, "write \"{}\"", s),
            DebugBox(s) => write!(f, "box {}", s),
        }
    }
}

debug_display!(LayoutAction);

/// A sequence of layouting actions.
///
/// The sequence of actions is optimized as the actions are added. For example,
/// a font changing option will only be added if the selected font is not already active.
/// All configuration actions (like moving, setting fonts, ...) are only flushed when
/// content is written.
///
/// Furthermore, the action list can translate absolute position into a coordinate system
/// with a different origin. This is realized in the `add_box` method, which allows a layout to
/// be added at a position, effectively translating all movement actions inside the layout
/// by the position.
#[derive(Debug, Clone)]
pub struct LayoutActions {
    pub origin: Size2D,
    actions: Vec<LayoutAction>,
    active_font: (usize, Size),
    next_pos: Option<Size2D>,
    next_font: Option<(usize, Size)>,
}

impl LayoutActions {
    /// Create a new action list.
    pub fn new() -> LayoutActions {
        LayoutActions {
            actions: vec![],
            origin: Size2D::zero(),
            active_font: (std::usize::MAX, Size::zero()),
            next_pos: None,
            next_font: None,
        }
    }

    /// Add an action to the list.
    pub fn add(&mut self, action: LayoutAction) {
        match action {
            MoveAbsolute(pos) => self.next_pos = Some(self.origin + pos),
            SetFont(index, size) => {
                self.next_font = Some((index, size));
            }

            _ => {
                self.flush_position();
                self.flush_font();

                self.actions.push(action);
            }
        }
    }

    /// Add a series of actions.
    pub fn extend<I>(&mut self, actions: I)
    where I: IntoIterator<Item = LayoutAction> {
        for action in actions.into_iter() {
            self.add(action);
        }
    }

    /// Add a layout at a position. All move actions inside the layout are translated
    /// by the position.
    pub fn add_layout(&mut self, position: Size2D, layout: Layout) {
        self.flush_position();

        self.origin = position;
        self.next_pos = Some(position);

        self.extend(layout.actions);
    }

    /// Whether there are any actions in this list.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Return the list of actions as a vector, leaving an empty
    /// vector in its position.
    pub fn to_vec(&mut self) -> Vec<LayoutAction> {
        std::mem::replace(&mut self.actions, vec![])
    }

    /// Append a cached move action if one is cached.
    fn flush_position(&mut self) {
        if let Some(target) = self.next_pos.take() {
            self.actions.push(MoveAbsolute(target));
        }
    }

    /// Append a cached font-setting action if one is cached.
    fn flush_font(&mut self) {
        if let Some((index, size)) = self.next_font.take() {
            if (index, size) != self.active_font {
                self.actions.push(SetFont(index, size));
                self.active_font = (index, size);
            }
        }
    }
}
