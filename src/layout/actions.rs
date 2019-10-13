//! Drawing and cofiguration actions composing layouts.

use std::fmt::{self, Display, Formatter};
use std::io::{self, Write};

use super::Layout;
use crate::size::Size2D;
use LayoutAction::*;

/// A layouting action.
#[derive(Clone)]
pub enum LayoutAction {
    /// Move to an absolute position.
    MoveAbsolute(Size2D),
    /// Set the font by index and font size.
    SetFont(usize, f32),
    /// Write text starting at the current position.
    WriteText(String),
    /// Visualize a box for debugging purposes.
    /// Arguments are position and size.
    DebugBox(Size2D, Size2D),
}

impl LayoutAction {
    /// Serialize this layout action into a string representation.
    pub fn serialize<W: Write>(&self, f: &mut W) -> io::Result<()> {
        use LayoutAction::*;
        match self {
            MoveAbsolute(s) => write!(f, "m {:.4} {:.4}", s.x.to_pt(), s.y.to_pt()),
            SetFont(i, s) => write!(f, "f {} {}", i, s),
            WriteText(s) => write!(f, "w {}", s),
            DebugBox(p, s) => write!(
                f,
                "b {} {} {} {}",
                p.x.to_pt(),
                p.y.to_pt(),
                s.x.to_pt(),
                s.y.to_pt()
            ),
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
            DebugBox(p, s) => write!(f, "box {} {}", p, s),
        }
    }
}

debug_display!(LayoutAction);

/// Unifies and otimizes lists of actions.
#[derive(Debug, Clone)]
pub struct LayoutActionList {
    pub origin: Size2D,
    actions: Vec<LayoutAction>,
    active_font: (usize, f32),
    next_pos: Option<Size2D>,
    next_font: Option<(usize, f32)>,
}

impl LayoutActionList {
    /// Create a new action list.
    pub fn new() -> LayoutActionList {
        LayoutActionList {
            actions: vec![],
            origin: Size2D::zero(),
            active_font: (std::usize::MAX, 0.0),
            next_pos: None,
            next_font: None,
        }
    }

    /// Add an action to the list if it is not useless
    /// (like changing to a font that is already active).
    pub fn add(&mut self, action: LayoutAction) {
        match action {
            MoveAbsolute(pos) => self.next_pos = Some(self.origin + pos),
            DebugBox(pos, size) => self.actions.push(DebugBox(self.origin + pos, size)),

            SetFont(index, size) if (index, size) != self.active_font => {
                self.next_font = Some((index, size));
            }

            _ => {
                if let Some(target) = self.next_pos.take() {
                    self.actions.push(MoveAbsolute(target));
                }
                if let Some((index, size)) = self.next_font.take() {
                    self.actions.push(SetFont(index, size));
                }

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

    /// Add all actions from a box layout at a position. A move to the position
    /// is generated and all moves inside the box layout are translated as
    /// necessary.
    pub fn add_box(&mut self, position: Size2D, layout: Layout) {
        if let Some(target) = self.next_pos.take() {
            self.actions.push(MoveAbsolute(target));
        }

        self.next_pos = Some(position);
        self.origin = position;

        if layout.debug_render {
            self.actions
                .push(LayoutAction::DebugBox(position, layout.dimensions));
        }

        self.extend(layout.actions);
    }

    /// Whether there are any actions in this list.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Return the list of actions as a vector.
    pub fn into_vec(self) -> Vec<LayoutAction> {
        self.actions
    }
}
