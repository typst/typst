//! Drawing and configuration actions composing layouts.

use std::fmt::{self, Debug, Formatter};
use serde::ser::{Serialize, Serializer, SerializeTuple};
use toddle::query::FontIndex;

use crate::size::{Size, Size2D};
use super::Layout;
use self::LayoutAction::*;


/// A layouting action, which is the basic building block layouts are composed
/// of.
#[derive(Clone, PartialEq)]
pub enum LayoutAction {
    /// Move to an absolute position.
    MoveAbsolute(Size2D),
    /// Set the font given the index from the font loader and font size.
    SetFont(FontIndex, Size),
    /// Write text at the current position.
    WriteText(String),
    /// Visualize a box for debugging purposes.
    DebugBox(Size2D),
}

impl Serialize for LayoutAction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match self {
            LayoutAction::MoveAbsolute(pos) => {
                let mut tup = serializer.serialize_tuple(2)?;
                tup.serialize_element(&0u8)?;
                tup.serialize_element(&pos)?;
                tup.end()
            }
            LayoutAction::SetFont(index, size) => {
                let mut tup = serializer.serialize_tuple(4)?;
                tup.serialize_element(&1u8)?;
                tup.serialize_element(index)?;
                tup.serialize_element(size)?;
                tup.end()
            }
            LayoutAction::WriteText(text) => {
                let mut tup = serializer.serialize_tuple(2)?;
                tup.serialize_element(&2u8)?;
                tup.serialize_element(text)?;
                tup.end()
            }
            LayoutAction::DebugBox(size) => {
                let mut tup = serializer.serialize_tuple(2)?;
                tup.serialize_element(&3u8)?;
                tup.serialize_element(&size)?;
                tup.end()
            }
        }
    }
}

impl Debug for LayoutAction {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use LayoutAction::*;
        match self {
            MoveAbsolute(s) => write!(f, "move {} {}", s.x, s.y),
            SetFont(i, s) => write!(f, "font {}_{} {}", i.id, i.variant, s),
            WriteText(s) => write!(f, "write {:?}", s),
            DebugBox(s) => write!(f, "box {} {}", s.x, s.y),
        }
    }
}

/// A sequence of layouting actions.
///
/// The sequence of actions is optimized as the actions are added. For example,
/// a font changing option will only be added if the selected font is not
/// already active. All configuration actions (like moving, setting fonts, ...)
/// are only flushed when content is written.
///
/// Furthermore, the action list can translate absolute position into a
/// coordinate system with a different origin. This is realized in the
/// `add_layout` method, which allows a layout to be added at a position,
/// effectively translating all movement actions inside the layout by the
/// position.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutActions {
    origin: Size2D,
    actions: Vec<LayoutAction>,
    active_font: (FontIndex, Size),
    next_pos: Option<Size2D>,
    next_font: Option<(FontIndex, Size)>,
}

impl LayoutActions {
    /// Create a new action list.
    pub fn new() -> LayoutActions {
        LayoutActions {
            actions: vec![],
            origin: Size2D::ZERO,
            active_font: (FontIndex::MAX, Size::ZERO),
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
    pub fn extend<I>(&mut self, actions: I) where I: IntoIterator<Item = LayoutAction> {
        for action in actions.into_iter() {
            self.add(action);
        }
    }

    /// Add a layout at a position. All move actions inside the layout are
    /// translated by the position.
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

    /// Return the list of actions as a vector.
    pub fn into_vec(self) -> Vec<LayoutAction> {
        self.actions
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
