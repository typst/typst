use std::sync::Arc;

use super::Content;
use crate::diag::TypResult;
use crate::eval::{Args, Func, Value};
use crate::frame::{Element, Frame};
use crate::geom::{Point, Transform};
use crate::syntax::Spanned;
use crate::Context;

/// A node that can realize itself with its own location.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct LocateNode(Spanned<Func>);

impl LocateNode {
    /// Create a new locate node.
    pub fn new(recipe: Spanned<Func>) -> Self {
        Self(recipe)
    }

    /// Realize the node.
    pub fn realize(&self, ctx: &mut Context) -> TypResult<Content> {
        let idx = ctx.pins.cursor();
        let location = ctx.pins.next();
        let dict = dict! {
            "page" => Value::Int(location.page as i64),
            "x" => Value::Length(location.pos.x.into()),
            "y" => Value::Length(location.pos.y.into()),
        };

        let args = Args::new(self.0.span, [Value::Dict(dict)]);
        Ok(Content::Pin(idx) + self.0.v.call_detached(ctx, args)?.display())
    }
}

/// Manages ordered pins.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct PinBoard {
    /// All currently pinned locations.
    pins: Vec<Location>,
    /// The index of the next pin in order.
    cursor: usize,
}

impl PinBoard {
    /// Create an empty pin board.
    pub fn new() -> Self {
        Self { pins: vec![], cursor: 0 }
    }

    /// The number of pins on the board.
    pub fn len(&self) -> usize {
        self.pins.len()
    }

    /// How many pins are resolved in comparison to an earlier snapshot.
    pub fn resolved(&self, prev: &Self) -> usize {
        self.pins.iter().zip(&prev.pins).filter(|(a, b)| a == b).count()
    }

    /// Access the next pin location.
    pub fn next(&mut self) -> Location {
        let cursor = self.cursor;
        self.jump(self.cursor + 1);
        self.pins[cursor]
    }

    /// The current cursor.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Set the current cursor.
    pub fn jump(&mut self, cursor: usize) {
        if cursor >= self.pins.len() {
            let loc = self.pins.last().copied().unwrap_or_default();
            self.pins.resize(cursor + 1, loc);
        }
        self.cursor = cursor;
    }

    /// Reset the cursor and remove all unused pins.
    pub fn reset(&mut self) {
        self.pins.truncate(self.cursor);
        self.cursor = 0;
    }

    /// Locate all pins in the frames.
    pub fn locate(&mut self, frames: &[Arc<Frame>]) {
        for (i, frame) in frames.iter().enumerate() {
            self.locate_impl(1 + i, frame, Transform::identity());
        }
    }

    /// Locate all pins in a frame.
    fn locate_impl(&mut self, page: usize, frame: &Frame, ts: Transform) {
        for &(pos, ref element) in &frame.elements {
            match element {
                Element::Group(group) => {
                    let ts = ts
                        .pre_concat(Transform::translate(pos.x, pos.y))
                        .pre_concat(group.transform);
                    self.locate_impl(page, &group.frame, ts);
                }

                Element::Pin(idx) => {
                    let pin = &mut self.pins[*idx];
                    pin.page = page;
                    pin.pos = pos.transform(ts);
                }

                _ => {}
            }
        }
    }
}

/// A physical location in a document.
#[derive(Debug, Default, Copy, Clone, PartialEq, Hash)]
pub struct Location {
    /// The page, starting at 1.
    pub page: usize,
    /// The exact coordinates on the page (from the top left, as usual).
    pub pos: Point,
}
