use std::fmt::{self, Debug, Formatter, Write};

use ecow::EcoVec;

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

/// Call a method on a state.
pub fn state_method(
    state: State,
    method: &str,
    mut args: Args,
    span: Span,
) -> SourceResult<Value> {
    let action = match method {
        "get" => StateAction::Get(args.eat()?),
        "final" => StateAction::Final(args.eat()?),
        "update" => StateAction::Update(args.expect("value or function")?),
        _ => bail!(span, "type state has no method `{}`", method),
    };

    args.finish()?;

    let content = StateNode::new(state, action).pack();
    Ok(Value::Content(content))
}

/// Executes an action on a state.
///
/// Display: State
/// Category: special
#[node(Locatable, Show)]
pub struct StateNode {
    /// The state.
    #[required]
    pub state: State,

    /// The action.
    #[required]
    pub action: StateAction,
}

impl Show for StateNode {
    fn show(&self, vt: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        match self.action() {
            StateAction::Get(func) => self.state().resolve(vt, self.0.stable_id(), func),
            StateAction::Final(func) => self.state().resolve(vt, None, func),
            StateAction::Update(_) => Ok(Content::empty()),
        }
    }
}

/// The action to perform on the state.
#[derive(Clone, PartialEq, Hash)]
pub enum StateAction {
    /// Displays the current state.
    Get(Option<Func>),
    /// Displays the final state.
    Final(Option<Func>),
    /// Updates the state, possibly based on the previous one.
    Update(StateUpdate),
}

cast_from_value! {
    StateAction: "state action",
}

impl Debug for StateAction {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Get(_) => f.pad("get(..)"),
            Self::Final(_) => f.pad("final(..)"),
            Self::Update(_) => f.pad("update(..)"),
        }
    }
}

/// An update to perform on a state.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum StateUpdate {
    /// Set the state to the specified value.
    Set(Value),
    /// Apply the given function to the state.
    Func(Func),
}

cast_from_value! {
    StateUpdate,
    v: Func => Self::Func(v),
    v: Value => Self::Set(v),
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
    /// Display the state at the postition of the given stable id.
    fn resolve(
        &self,
        vt: &Vt,
        stop: Option<StableId>,
        func: Option<Func>,
    ) -> SourceResult<Content> {
        if !vt.introspector.init() {
            return Ok(Content::empty());
        }

        let sequence = self.sequence(vt.world, vt.introspector)?;
        Ok(match sequence.at(stop) {
            Some(value) => {
                if let Some(func) = func {
                    let args = Args::new(func.span(), [value]);
                    func.call_detached(vt.world, args)?.display()
                } else {
                    value.display()
                }
            }
            None => Content::empty(),
        })
    }

    /// Produce the whole sequence of states.
    ///
    /// This has to happen just once for all states, cutting down the number
    /// of state updates from quadratic to linear.
    #[comemo::memoize]
    fn sequence(
        &self,
        world: Tracked<dyn World>,
        introspector: Tracked<Introspector>,
    ) -> SourceResult<StateSequence> {
        let search = Selector::Node(
            NodeId::of::<StateNode>(),
            Some(dict! { "state" => self.clone() }),
        );

        let mut stops = EcoVec::new();
        let mut state = self.init.clone();

        for node in introspector.query(search) {
            let id = node.stable_id().unwrap();
            let node = node.to::<StateNode>().unwrap();

            if let StateAction::Update(update) = node.action() {
                match update {
                    StateUpdate::Set(value) => state = value,
                    StateUpdate::Func(func) => {
                        let args = Args::new(func.span(), [state]);
                        state = func.call_detached(world, args)?;
                    }
                }
            }

            stops.push((id, state.clone()));
        }

        Ok(StateSequence(stops))
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

/// A sequence of state values.
#[derive(Debug, Clone)]
struct StateSequence(EcoVec<(StableId, Value)>);

impl StateSequence {
    fn at(&self, stop: Option<StableId>) -> Option<Value> {
        let entry = match stop {
            Some(stop) => self.0.iter().find(|&&(id, _)| id == stop),
            None => self.0.last(),
        };

        entry.map(|(_, value)| value.clone())
    }
}
