use std::cell::Cell;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use super::Content;
use crate::diag::TypResult;
use crate::eval::{Args, Array, Dict, Func, Value};
use crate::frame::{Element, Frame, Location};
use crate::geom::{Point, Transform};
use crate::memo::Track;
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

    /// Create a new node with access to a group's members.
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

/// An ungrouped locatable node.
#[derive(Debug, Clone, PartialEq, Hash)]
struct SingleNode(Spanned<Func>);

impl SingleNode {
    fn realize(&self, ctx: &mut Context) -> TypResult<Content> {
        let idx = ctx.pins.cursor;
        let pin = ctx.pins.get_or_create(None, None);
        let dict = pin.encode(None);
        let args = Args::new(self.0.span, [Value::Dict(dict)]);
        Ok(Content::Pin(idx) + self.0.v.call_detached(ctx, args)?.display())
    }
}

/// A locatable grouped node which can interact with its peers' details.
#[derive(Debug, Clone, PartialEq, Hash)]
struct EntryNode {
    /// Which group the node belongs to.
    group: Group,
    /// The recipe to execute.
    recipe: Spanned<Func>,
    /// An arbitrary attached value.
    value: Option<Value>,
}

impl EntryNode {
    fn realize(&self, ctx: &mut Context) -> TypResult<Content> {
        let idx = ctx.pins.cursor;
        let pin = ctx.pins.get_or_create(Some(self.group.clone()), self.value.clone());

        // Determine the index among the peers.
        let index = ctx
            .pins
            .iter()
            .enumerate()
            .filter(|&(k, other)| {
                other.is_in(&self.group)
                    && if k < idx {
                        other.flow <= pin.flow
                    } else {
                        other.flow < pin.flow
                    }
            })
            .count();

        // Prepare first argument.
        let dict = pin.encode(Some(index));
        let mut args = Args::new(self.recipe.span, [Value::Dict(dict)]);

        // Collect all group members if second argument is requested.
        if self.recipe.v.argc() == Some(2) {
            let all = ctx.pins.encode_group(&self.group);
            args.push(self.recipe.span, Value::Array(all))
        }

        Ok(Content::Pin(idx) + self.recipe.v.call_detached(ctx, args)?.display())
    }
}

/// A node with access to a group's members.
#[derive(Debug, Clone, PartialEq, Hash)]
struct AllNode {
    /// Which group the node has access to.
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

/// Manages document pins.
#[derive(Debug, Clone)]
pub struct PinBoard {
    /// All currently active pins.
    list: Vec<Pin>,
    /// The index of the next pin, in order.
    cursor: usize,
    /// If larger than zero, the board is frozen and the cursor will not be
    /// advanced. This is used to disable pinning during measure-only layouting.
    frozen: usize,
    /// Whether the board was accessed.
    pub(super) dirty: Cell<bool>,
}

impl PinBoard {
    /// Create an empty pin board.
    pub fn new() -> Self {
        Self {
            list: vec![],
            cursor: 0,
            frozen: 0,
            dirty: Cell::new(false),
        }
    }
}

/// Internal methods for implementation of locatable nodes.
impl PinBoard {
    /// Access or create the next pin.
    fn get_or_create(&mut self, group: Option<Group>, value: Option<Value>) -> Pin {
        self.dirty.set(true);
        if self.frozen() {
            return Pin::default();
        }

        let cursor = self.cursor;
        self.cursor += 1;
        if self.cursor >= self.list.len() {
            self.list.resize(self.cursor, Pin::default());
        }

        let pin = &mut self.list[cursor];
        pin.group = group;
        pin.value = value;
        pin.clone()
    }

    /// Encode a group into a user-facing array.
    fn encode_group(&self, group: &Group) -> Array {
        self.dirty.set(true);
        let mut all: Vec<_> = self.iter().filter(|pin| pin.is_in(group)).collect();
        all.sort_by_key(|pin| pin.flow);
        all.iter()
            .enumerate()
            .map(|(index, member)| Value::Dict(member.encode(Some(index))))
            .collect()
    }

    /// Iterate over all pins on the board.
    fn iter(&self) -> std::slice::Iter<Pin> {
        self.dirty.set(true);
        self.list.iter()
    }
}

/// Caching related methods.
impl PinBoard {
    /// The current cursor.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// All pins from `prev` to the current cursor.
    pub fn from(&self, prev: usize) -> Vec<Pin> {
        self.list[prev .. self.cursor].to_vec()
    }

    /// Add the given pins at the given location and set the cursor behind them.
    pub fn replay(&mut self, at: usize, pins: Vec<Pin>) {
        if !self.frozen() {
            self.cursor = at + pins.len();
            let end = self.cursor.min(self.list.len());
            self.list.splice(at .. end, pins);
        }
    }
}

/// Control methods that are called during layout.
impl PinBoard {
    /// Freeze the board to prevent modifications.
    pub fn freeze(&mut self) {
        self.frozen += 1;
    }

    /// Freeze the board to prevent modifications.
    pub fn unfreeze(&mut self) {
        self.frozen -= 1;
    }

    /// Whether the board is currently frozen.
    pub fn frozen(&self) -> bool {
        self.frozen > 0
    }
}

/// Methods that are called in between layout passes.
impl PinBoard {
    /// Reset the cursor and remove all unused pins.
    pub fn reset(&mut self) {
        self.list.truncate(self.cursor);
        self.cursor = 0;
        self.dirty.set(false);
    }

    /// Locate all pins in the frames.
    pub fn locate(&mut self, frames: &[Arc<Frame>]) {
        let mut flow = 0;
        for (i, frame) in frames.iter().enumerate() {
            locate_in_frame(
                &mut self.list,
                &mut flow,
                1 + i,
                frame,
                Transform::identity(),
            );
        }
    }

    /// How many pins are unresolved in comparison to an earlier snapshot.
    pub fn unresolved(&self, prev: &Self) -> usize {
        self.list.len() - self.list.iter().zip(&prev.list).filter(|(a, b)| a == b).count()
    }
}

/// Locate all pins in a frame.
fn locate_in_frame(
    pins: &mut [Pin],
    flow: &mut usize,
    page: usize,
    frame: &Frame,
    ts: Transform,
) {
    for &(pos, ref element) in frame.elements() {
        match element {
            Element::Group(group) => {
                let ts = ts
                    .pre_concat(Transform::translate(pos.x, pos.y))
                    .pre_concat(group.transform);
                locate_in_frame(pins, flow, page, &group.frame, ts);
            }

            Element::Pin(idx) => {
                let pin = &mut pins[*idx];
                pin.loc.page = page;
                pin.loc.pos = pos.transform(ts);
                pin.flow = *flow;
                *flow += 1;
            }

            _ => {}
        }
    }
}

impl Hash for PinBoard {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.list.hash(state);
        self.cursor.hash(state);
        self.frozen.hash(state);
    }
}

/// Describes pin usage.
#[derive(Debug, Copy, Clone)]
pub struct PinConstraint(pub Option<u64>);

impl Track for PinBoard {
    type Constraint = PinConstraint;

    fn key<H: Hasher>(&self, _: &mut H) {}

    fn matches(&self, constraint: &Self::Constraint) -> bool {
        match constraint.0 {
            Some(hash) => fxhash::hash64(self) == hash,
            None => true,
        }
    }
}

/// A document pin.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Pin {
    /// The physical location of the pin in the document.
    loc: Location,
    /// The flow index.
    flow: usize,
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
        let mut dict = self.loc.encode();

        if let Some(value) = &self.value {
            dict.insert("value".into(), value.clone());
        }

        if let Some(index) = index {
            dict.insert("index".into(), Value::Int(index as i64));
        }

        dict
    }
}

impl Default for Pin {
    fn default() -> Self {
        Self {
            loc: Location { page: 0, pos: Point::zero() },
            flow: 0,
            group: None,
            value: None,
        }
    }
}
