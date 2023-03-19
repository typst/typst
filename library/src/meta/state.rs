use std::fmt::{self, Debug, Formatter, Write};

use ecow::{eco_vec, EcoVec};
use typst::eval::Tracer;

use crate::prelude::*;

/// Handle stateful tasks.
///
/// Display: State
/// Category: meta
/// Returns: state
#[func]
pub fn state(
    /// The key that identifies this state.
    key: Str,
    /// The initial value of the state.
    #[default]
    init: Value,
) -> Value {
    Value::dynamic(State { key, init })
}

/// A state.
#[derive(Clone, PartialEq, Hash)]
pub struct State {
    /// The key that identifies the state.
    key: Str,
    /// The initial value of the state.
    init: Value,
}

impl State {
    /// Call a method on a state.
    pub fn call_method(
        self,
        vm: &mut Vm,
        method: &str,
        mut args: Args,
        span: Span,
    ) -> SourceResult<Value> {
        let value = match method {
            "display" => self.display(args.eat()?).into(),
            "at" => self.at(&mut vm.vt, args.expect("location")?)?,
            "final" => self.final_(&mut vm.vt, args.expect("location")?)?,
            "update" => self.update(args.expect("value or function")?).into(),
            _ => bail!(span, "type state has no method `{}`", method),
        };
        args.finish()?;
        Ok(value)
    }

    /// Display the current value of the state.
    pub fn display(self, func: Option<Func>) -> Content {
        DisplayNode::new(self, func).pack()
    }

    /// Get the value of the state at the given location.
    pub fn at(self, vt: &mut Vt, id: StableId) -> SourceResult<Value> {
        let sequence = self.sequence(vt)?;
        let offset = vt.introspector.query_before(self.selector(), id).len();
        Ok(sequence[offset].clone())
    }

    /// Get the value of the state at the final location.
    pub fn final_(self, vt: &mut Vt, _: StableId) -> SourceResult<Value> {
        let sequence = self.sequence(vt)?;
        Ok(sequence.last().unwrap().clone())
    }

    /// Produce content that performs a state update.
    pub fn update(self, update: StateUpdate) -> Content {
        UpdateNode::new(self, update).pack()
    }

    /// Produce the whole sequence of states.
    ///
    /// This has to happen just once for all states, cutting down the number
    /// of state updates from quadratic to linear.
    fn sequence(&self, vt: &mut Vt) -> SourceResult<EcoVec<Value>> {
        self.sequence_impl(
            vt.world,
            TrackedMut::reborrow_mut(&mut vt.tracer),
            TrackedMut::reborrow_mut(&mut vt.provider),
            vt.introspector,
        )
    }

    /// Memoized implementation of `sequence`.
    #[comemo::memoize]
    fn sequence_impl(
        &self,
        world: Tracked<dyn World>,
        tracer: TrackedMut<Tracer>,
        provider: TrackedMut<StabilityProvider>,
        introspector: Tracked<Introspector>,
    ) -> SourceResult<EcoVec<Value>> {
        let mut vt = Vt { world, tracer, provider, introspector };
        let mut state = self.init.clone();
        let mut stops = eco_vec![state.clone()];

        for node in introspector.query(self.selector()) {
            let node = node.to::<UpdateNode>().unwrap();
            match node.update() {
                StateUpdate::Set(value) => state = value,
                StateUpdate::Func(func) => state = func.call_vt(&mut vt, [state])?,
            }
            stops.push(state.clone());
        }

        Ok(stops)
    }

    /// The selector for this state's updates.
    fn selector(&self) -> Selector {
        Selector::Node(
            NodeId::of::<UpdateNode>(),
            Some(dict! { "state" => self.clone() }),
        )
    }
}

impl Debug for State {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("state(")?;
        self.key.fmt(f)?;
        f.write_str(", ")?;
        self.init.fmt(f)?;
        f.write_char(')')
    }
}

cast_from_value! {
    State: "state",
}

/// An update to perform on a state.
#[derive(Clone, PartialEq, Hash)]
pub enum StateUpdate {
    /// Set the state to the specified value.
    Set(Value),
    /// Apply the given function to the state.
    Func(Func),
}

impl Debug for StateUpdate {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("..")
    }
}

cast_from_value! {
    StateUpdate: "state update",
    v: Func => Self::Func(v),
    v: Value => Self::Set(v),
}

/// Executes a display of a state.
///
/// Display: State
/// Category: special
#[node(Locatable, Show)]
struct DisplayNode {
    /// The state.
    #[required]
    state: State,

    /// The function to display the state with.
    #[required]
    func: Option<Func>,
}

impl Show for DisplayNode {
    fn show(&self, vt: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        let id = self.0.stable_id().unwrap();
        let value = self.state().at(vt, id)?;
        Ok(match self.func() {
            Some(func) => func.call_vt(vt, [value])?.display(),
            None => value.display(),
        })
    }
}

/// Executes a display of a state.
///
/// Display: State
/// Category: special
#[node(Locatable, Show)]
struct UpdateNode {
    /// The state.
    #[required]
    state: State,

    /// The update to perform on the state.
    #[required]
    update: StateUpdate,
}

impl Show for UpdateNode {
    fn show(&self, _: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        Ok(Content::empty())
    }
}
