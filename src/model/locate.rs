use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use super::Content;
use crate::diag::TypResult;
use crate::eval::{Args, Array, Dict, Func, Value};
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
    pub fn entry(&self, recipe: Spanned<Func>, value: Option<Value>) -> LocateNode {
        LocateNode::entry(self.clone(), recipe, value)
    }

    /// Do something with all entries of a group.
    pub fn all(&self, recipe: Spanned<Func>) -> LocateNode {
        LocateNode::all(self.clone(), recipe)
    }
}

impl Debug for Group {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "group({:?})", self.0)
    }
}

/// A node that can be realized with pinned document locations.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct LocateNode(Arc<Repr>);

impl LocateNode {
    /// Create a new locatable single node.
    pub fn single(recipe: Spanned<Func>) -> Self {
        Self(Arc::new(Repr::Single(SingleNode(recipe))))
    }

    /// Create a new locatable group entry node.
    pub fn entry(group: Group, recipe: Spanned<Func>, value: Option<Value>) -> Self {
        Self(Arc::new(Repr::Entry(EntryNode { group, recipe, value })))
    }

    /// Create a new all node with access to a group's members.
    pub fn all(group: Group, recipe: Spanned<Func>) -> Self {
        Self(Arc::new(Repr::All(AllNode { group, recipe })))
    }

    /// Realize the node.
    pub fn realize(&self, ctx: &mut Context) -> TypResult<Content> {
        match self.0.as_ref() {
            Repr::Single(single) => single.realize(ctx),
            Repr::Entry(entry) => entry.realize(ctx),
            Repr::All(all) => all.realize(ctx),
        }
    }
}

/// The different kinds of locate nodes.
#[derive(Debug, Clone, PartialEq, Hash)]
enum Repr {
    /// A single `locate(me => ...)`.
    Single(SingleNode),
    /// A locatable group entry.
    Entry(EntryNode),
    /// A recipe for all entries of a group.
    All(AllNode),
}

/// A solo locatable node.
#[derive(Debug, Clone, PartialEq, Hash)]
struct SingleNode(Spanned<Func>);

impl SingleNode {
    fn realize(&self, ctx: &mut Context) -> TypResult<Content> {
        let idx = ctx.pins.cursor();
        let pin = ctx.pins.next(None, None);
        let dict = pin.encode(None);
        let args = Args::new(self.0.span, [Value::Dict(dict)]);
        Ok(Content::Pin(idx) + self.0.v.call_detached(ctx, args)?.display())
    }
}

/// A group node which can interact with its peer's details.
#[derive(Debug, Clone, PartialEq, Hash)]
struct EntryNode {
    /// Which group the node belongs to, if any.
    group: Group,
    /// The recipe to execute.
    recipe: Spanned<Func>,
    /// An arbitrary attached value.
    value: Option<Value>,
}

impl EntryNode {
    fn realize(&self, ctx: &mut Context) -> TypResult<Content> {
        let idx = ctx.pins.cursor();
        let pin = ctx.pins.next(Some(self.group.clone()), self.value.clone());

        // Determine the index among the peers.
        let index = ctx
            .pins
            .iter()
            .filter(|other| other.is_in(&self.group) && other.loc.flow < pin.loc.flow)
            .count();

        let dict = pin.encode(Some(index));
        let mut args = Args::new(self.recipe.span, [Value::Dict(dict)]);

        // Collect all members if requested.
        if self.recipe.v.argc() == Some(2) {
            let all = ctx.pins.encode_group(&self.group);
            args.push(self.recipe.span, Value::Array(all))
        }

        Ok(Content::Pin(idx) + self.recipe.v.call_detached(ctx, args)?.display())
    }
}

/// A node with access to the group's members without being one itself.
#[derive(Debug, Clone, PartialEq, Hash)]
struct AllNode {
    /// Which group.
    group: Group,
    /// The recipe to execute.
    recipe: Spanned<Func>,
}

impl AllNode {
    fn realize(&self, ctx: &mut Context) -> TypResult<Content> {
        let all = ctx.pins.encode_group(&self.group);
        let args = Args::new(self.recipe.span, [Value::Array(all)]);
        Ok(self.recipe.v.call_detached(ctx, args)?.display())
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
    pub fn next(&mut self, group: Option<Group>, value: Option<Value>) -> Pin {
        if self.frozen > 0 {
            return Pin::default();
        }

        let cursor = self.cursor;
        self.jump(self.cursor + 1);

        let pin = &mut self.pins[cursor];
        pin.group = group;
        pin.value = value;
        pin.clone()
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

    /// Encode a group into a user-facing array.
    pub fn encode_group(&self, group: &Group) -> Array {
        let mut all: Vec<_> = self.iter().filter(|other| other.is_in(group)).collect();
        all.sort_by_key(|pin| pin.loc.flow);
        all.iter()
            .enumerate()
            .map(|(index, member)| Value::Dict(member.encode(Some(index))))
            .collect()
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

/// A document pin.
#[derive(Debug, Default, Clone, PartialEq, Hash)]
pub struct Pin {
    /// The physical location of the pin in the document.
    loc: Location,
    /// The group the pin belongs to, if any.
    group: Option<Group>,
    /// An arbitrary attached value.
    value: Option<Value>,
}

impl Pin {
    /// Whether the pin is part of the given group.
    fn is_in(&self, group: &Group) -> bool {
        self.group.as_ref() == Some(group)
    }

    /// Encode into a user-facing dictionary.
    fn encode(&self, index: Option<usize>) -> Dict {
        let mut dict = dict! {
            "page" => Value::Int(self.loc.page as i64),
            "x" => Value::Length(self.loc.pos.x.into()),
            "y" => Value::Length(self.loc.pos.y.into()),
        };

        if let Some(value) = &self.value {
            dict.insert("value".into(), value.clone());
        }

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
