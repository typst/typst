use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use super::Content;
use crate::diag::TypResult;
use crate::eval::{Args, Dict, Func, Value};
use crate::frame::{Element, Frame};
use crate::geom::{Point, Transform};
use crate::syntax::Spanned;
use crate::util::EcoString;
use crate::Context;

/// A group of locatable elements.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Group(EcoString);

impl Group {
    /// Create a group of elements that is identified by a string key.
    pub fn new(key: EcoString) -> Self {
        Self(key)
    }

    /// Add an entry to the group.
    pub fn entry(&self, recipe: Spanned<Func>) -> LocateNode {
        LocateNode { recipe, group: Some(self.clone()) }
    }
}

impl Debug for Group {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "group({:?})", self.0)
    }
}

/// A node that can realize itself with its own location.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct LocateNode {
    recipe: Spanned<Func>,
    group: Option<Group>,
}

impl LocateNode {
    /// Create a new locate node.
    pub fn new(recipe: Spanned<Func>) -> Self {
        Self { recipe, group: None }
    }

    /// Realize the node.
    pub fn realize(&self, ctx: &mut Context) -> TypResult<Content> {
        let idx = ctx.pins.cursor();
        let pin = ctx.pins.next(self.group.clone());

        // Determine the index among the peers.
        let index = self.group.as_ref().map(|_| {
            ctx.pins
                .iter()
                .filter(|other| {
                    other.group == self.group && other.loc.flow < pin.loc.flow
                })
                .count()
        });

        let dict = pin.encode(index);
        let mut args = Args::new(self.recipe.span, [Value::Dict(dict)]);

        // Collect all members if requested.
        if self.group.is_some() && self.recipe.v.argc() == Some(2) {
            let mut all: Vec<_> =
                ctx.pins.iter().filter(|other| other.group == self.group).collect();

            all.sort_by_key(|pin| pin.loc.flow);

            let array = all
                .iter()
                .enumerate()
                .map(|(index, member)| Value::Dict(member.encode(Some(index))))
                .collect();

            args.push(self.recipe.span, Value::Array(array))
        }

        Ok(Content::Pin(idx) + self.recipe.v.call_detached(ctx, args)?.display())
    }
}

/// Manages pins.
#[derive(Debug, Clone, Hash)]
pub struct PinBoard {
    /// All currently active pins.
    pins: Vec<Pin>,
    /// The index of the next pin in order.
    cursor: usize,
    /// If larger than zero, the board is frozen.
    frozen: usize,
}

/// A document pin.
#[derive(Debug, Default, Clone, PartialEq, Hash)]
pub struct Pin {
    /// The physical location of the pin in the document.
    loc: Location,
    /// The group the pin belongs to, if any.
    group: Option<Group>,
}

impl Pin {
    /// Encode into a user-facing dictionary.
    fn encode(&self, index: Option<usize>) -> Dict {
        let mut dict = dict! {
            "page" => Value::Int(self.loc.page as i64),
            "x" => Value::Length(self.loc.pos.x.into()),
            "y" => Value::Length(self.loc.pos.y.into()),
        };

        if let Some(index) = index {
            dict.insert("index".into(), Value::Int(index as i64));
        }

        dict
    }
}

/// A physical location in a document.
#[derive(Debug, Default, Copy, Clone, PartialEq, Hash)]
pub struct Location {
    /// The page, starting at 1.
    pub page: usize,
    /// The exact coordinates on the page (from the top left, as usual).
    pub pos: Point,
    /// The flow index.
    pub flow: usize,
}

impl PinBoard {
    /// Create an empty pin board.
    pub fn new() -> Self {
        Self { pins: vec![], cursor: 0, frozen: 0 }
    }

    /// The number of pins on the board.
    pub fn len(&self) -> usize {
        self.pins.len()
    }

    /// Iterate over all pins on the board.
    pub fn iter(&self) -> std::slice::Iter<Pin> {
        self.pins.iter()
    }

    /// Freeze the board to prevent modifications.
    pub fn freeze(&mut self) {
        self.frozen += 1;
    }

    /// Freeze the board to prevent modifications.
    pub fn unfreeze(&mut self) {
        self.frozen -= 1;
    }

    /// Access the next pin.
    pub fn next(&mut self, group: Option<Group>) -> Pin {
        if self.frozen > 0 {
            return Pin::default();
        }

        let cursor = self.cursor;
        self.jump(self.cursor + 1);
        self.pins[cursor].group = group;
        self.pins[cursor].clone()
    }

    /// The current cursor.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Set the current cursor.
    pub fn jump(&mut self, cursor: usize) {
        if self.frozen > 0 {
            return;
        }

        self.cursor = cursor;
        if cursor >= self.pins.len() {
            self.pins.resize(cursor, Pin::default());
        }
    }

    /// Reset the cursor and remove all unused pins.
    pub fn reset(&mut self) {
        self.pins.truncate(self.cursor);
        self.cursor = 0;
    }

    /// Locate all pins in the frames.
    pub fn locate(&mut self, frames: &[Arc<Frame>]) {
        let mut flow = 0;
        for (i, frame) in frames.iter().enumerate() {
            locate_impl(
                &mut self.pins,
                &mut flow,
                1 + i,
                frame,
                Transform::identity(),
            );
        }
    }

    /// How many pins are resolved in comparison to an earlier snapshot.
    pub fn resolved(&self, prev: &Self) -> usize {
        self.pins.iter().zip(&prev.pins).filter(|(a, b)| a == b).count()
    }
}

/// Locate all pins in a frame.
fn locate_impl(
    pins: &mut [Pin],
    flow: &mut usize,
    page: usize,
    frame: &Frame,
    ts: Transform,
) {
    for &(pos, ref element) in &frame.elements {
        match element {
            Element::Group(group) => {
                let ts = ts
                    .pre_concat(Transform::translate(pos.x, pos.y))
                    .pre_concat(group.transform);
                locate_impl(pins, flow, page, &group.frame, ts);
            }

            Element::Pin(idx) => {
                let loc = &mut pins[*idx].loc;
                loc.page = page;
                loc.pos = pos.transform(ts);
                loc.flow = *flow;
                *flow += 1;
            }

            _ => {}
        }
    }
}
