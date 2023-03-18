use std::fmt::{self, Debug, Formatter, Write};
use std::str::FromStr;

use ecow::{eco_vec, EcoVec};
use smallvec::{smallvec, SmallVec};
use typst::eval::Tracer;

use super::{Numbering, NumberingPattern};
use crate::layout::PageNode;
use crate::prelude::*;

/// Count through pages, elements, and more.
///
/// Display: Counter
/// Category: meta
/// Returns: counter
#[func]
pub fn counter(
    /// The key that identifies this counter.
    key: CounterKey,
) -> Value {
    Value::dynamic(Counter::new(key))
}

/// Identifies a counter.
#[derive(Clone, PartialEq, Hash)]
pub enum CounterKey {
    /// The page counter.
    Page,
    /// Counts elements matching the given selectors. Only works for locatable
    /// elements or labels.
    Selector(Selector),
    /// Counts through manual counters with the same key.
    Str(Str),
}

cast_from_value! {
    CounterKey,
    v: Str => Self::Str(v),
    label: Label => Self::Selector(Selector::Label(label)),
    func: Func => {
        let Some(id) = func.id() else {
            return Err("this function is not selectable".into());
        };

        if id == NodeId::of::<PageNode>() {
            return Ok(Self::Page);
        }

        if !Content::new(id).can::<dyn Locatable>() {
            Err(eco_format!("cannot count through {}s", id.name))?;
        }

        Self::Selector(Selector::Node(id, None))
    }
}

impl Debug for CounterKey {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Page => f.pad("page"),
            Self::Selector(selector) => selector.fmt(f),
            Self::Str(str) => str.fmt(f),
        }
    }
}

/// Call a method on counter.
pub fn counter_method(
    counter: Counter,
    method: &str,
    mut args: Args,
    span: Span,
) -> SourceResult<Value> {
    let pattern = |s| NumberingPattern::from_str(s).unwrap().into();
    let action = match method {
        "get" => CounterAction::Get(args.eat()?.unwrap_or_else(|| pattern("1.1"))),
        "final" => CounterAction::Final(args.eat()?.unwrap_or_else(|| pattern("1.1"))),
        "both" => CounterAction::Both(args.eat()?.unwrap_or_else(|| pattern("1/1"))),
        "step" => CounterAction::Update(CounterUpdate::Step(
            args.named("level")?.unwrap_or(NonZeroUsize::ONE),
        )),
        "update" => CounterAction::Update(args.expect("value or function")?),
        _ => bail!(span, "type counter has no method `{}`", method),
    };

    args.finish()?;

    let content = CounterNode::new(counter, action).pack();
    Ok(Value::Content(content))
}

/// Executes an action on a counter.
///
/// Display: Counter
/// Category: special
#[node(Locatable, Show)]
pub struct CounterNode {
    /// The counter key.
    #[required]
    pub counter: Counter,

    /// The action.
    #[required]
    pub action: CounterAction,
}

impl Show for CounterNode {
    fn show(&self, vt: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        match self.action() {
            CounterAction::Get(numbering) => {
                self.counter().resolve(vt, self.0.stable_id(), &numbering)
            }
            CounterAction::Final(numbering) => {
                self.counter().resolve(vt, None, &numbering)
            }
            CounterAction::Both(numbering) => {
                let both = match &numbering {
                    Numbering::Pattern(pattern) => pattern.pieces() >= 2,
                    _ => false,
                };

                let counter = self.counter();
                let id = self.0.stable_id();
                if !both {
                    return counter.resolve(vt, id, &numbering);
                }

                let sequence = counter.sequence(
                    vt.world,
                    TrackedMut::reborrow_mut(&mut vt.tracer),
                    TrackedMut::reborrow_mut(&mut vt.provider),
                    vt.introspector,
                )?;

                Ok(match (sequence.single(id), sequence.single(None)) {
                    (Some(current), Some(total)) => {
                        numbering.apply_vt(vt, &[current, total])?.display()
                    }
                    _ => Content::empty(),
                })
            }
            CounterAction::Update(_) => Ok(Content::empty()),
        }
    }
}

/// The action to perform on a counter.
#[derive(Clone, PartialEq, Hash)]
pub enum CounterAction {
    /// Displays the current value.
    Get(Numbering),
    /// Displays the final value.
    Final(Numbering),
    /// If given a pattern with at least two parts, displays the current value
    /// together with the final value. Otherwise, displays just the current
    /// value.
    Both(Numbering),
    /// Updates the value, possibly based on the previous one.
    Update(CounterUpdate),
}

cast_from_value! {
    CounterAction: "counter action",
}

impl Debug for CounterAction {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Get(_) => f.pad("get(..)"),
            Self::Final(_) => f.pad("final(..)"),
            Self::Both(_) => f.pad("both(..)"),
            Self::Update(_) => f.pad("update(..)"),
        }
    }
}

/// An update to perform on a counter.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum CounterUpdate {
    /// Set the counter to the specified state.
    Set(CounterState),
    /// Increase the number for the given level by one.
    Step(NonZeroUsize),
    /// Apply the given function to the counter's state.
    Func(Func),
}

cast_from_value! {
    CounterUpdate,
    v: CounterState => Self::Set(v),
    v: Func => Self::Func(v),
}

/// Nodes that have special counting behaviour.
pub trait Count {
    /// Get the counter update for this node.
    fn update(&self) -> Option<CounterUpdate>;
}

/// Counts through pages, elements, and more.
#[derive(Clone, PartialEq, Hash)]
pub struct Counter {
    /// The key that identifies the counter.
    pub key: CounterKey,
}

impl Counter {
    /// Create a new counter from a key.
    pub fn new(key: CounterKey) -> Self {
        Self { key }
    }

    /// The counter for the given node.
    pub fn of(id: NodeId) -> Self {
        Self::new(CounterKey::Selector(Selector::Node(id, None)))
    }

    /// Display the value of the counter at the postition of the given stable
    /// id.
    pub fn resolve(
        &self,
        vt: &mut Vt,
        stop: Option<StableId>,
        numbering: &Numbering,
    ) -> SourceResult<Content> {
        if !vt.introspector.init() {
            return Ok(Content::empty());
        }

        let sequence = self.sequence(
            vt.world,
            TrackedMut::reborrow_mut(&mut vt.tracer),
            TrackedMut::reborrow_mut(&mut vt.provider),
            vt.introspector,
        )?;

        Ok(match sequence.at(stop) {
            Some(state) => numbering.apply_vt(vt, &state.0)?.display(),
            None => Content::empty(),
        })
    }

    /// Produce the whole sequence of counter states.
    ///
    /// This has to happen just once for all counters, cutting down the number
    /// of counter updates from quadratic to linear.
    #[comemo::memoize]
    fn sequence(
        &self,
        world: Tracked<dyn World>,
        tracer: TrackedMut<Tracer>,
        provider: TrackedMut<StabilityProvider>,
        introspector: Tracked<Introspector>,
    ) -> SourceResult<CounterSequence> {
        let mut vt = Vt { world, tracer, provider, introspector };
        let mut search = Selector::Node(
            NodeId::of::<CounterNode>(),
            Some(dict! { "counter" => self.clone() }),
        );

        if let CounterKey::Selector(selector) = &self.key {
            search = Selector::Any(eco_vec![search, selector.clone()]);
        }

        let mut stops = EcoVec::new();
        let mut state = CounterState(match &self.key {
            CounterKey::Selector(_) => smallvec![],
            _ => smallvec![NonZeroUsize::ONE],
        });

        let is_page = self.key == CounterKey::Page;
        let mut prev_page = NonZeroUsize::ONE;

        for node in introspector.query(search) {
            let id = node.stable_id().unwrap();
            if is_page {
                let page = introspector.page(id);
                let delta = page.get() - prev_page.get();
                if delta > 0 {
                    state.step(NonZeroUsize::ONE, delta);
                }
                prev_page = page;
            }

            if let Some(update) = match node.to::<CounterNode>() {
                Some(counter) => match counter.action() {
                    CounterAction::Update(update) => Some(update),
                    _ => None,
                },
                None => match node.with::<dyn Count>() {
                    Some(countable) => countable.update(),
                    None => Some(CounterUpdate::Step(NonZeroUsize::ONE)),
                },
            } {
                state.update(&mut vt, update)?;
            }

            stops.push((id, state.clone()));
        }

        Ok(CounterSequence { stops, is_page })
    }
}

impl Debug for Counter {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("counter(")?;
        self.key.fmt(f)?;
        f.write_char(')')
    }
}

cast_from_value! {
    Counter: "counter",
}

/// A sequence of counter values.
#[derive(Debug, Clone)]
struct CounterSequence {
    stops: EcoVec<(StableId, CounterState)>,
    is_page: bool,
}

impl CounterSequence {
    fn at(&self, stop: Option<StableId>) -> Option<CounterState> {
        let entry = match stop {
            Some(stop) => self.stops.iter().find(|&&(id, _)| id == stop),
            None => self.stops.last(),
        };

        if let Some((_, state)) = entry {
            return Some(state.clone());
        }

        if self.is_page {
            return Some(CounterState(smallvec![NonZeroUsize::ONE]));
        }

        None
    }

    fn single(&self, stop: Option<StableId>) -> Option<NonZeroUsize> {
        Some(*self.at(stop)?.0.first()?)
    }
}

/// Counts through elements with different levels.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct CounterState(pub SmallVec<[NonZeroUsize; 3]>);

impl CounterState {
    /// Advance the counter and return the numbers for the given heading.
    pub fn update(&mut self, vt: &mut Vt, update: CounterUpdate) -> SourceResult<()> {
        match update {
            CounterUpdate::Set(state) => *self = state,
            CounterUpdate::Step(level) => self.step(level, 1),
            CounterUpdate::Func(func) => {
                *self = func
                    .call_vt(vt, self.0.iter().copied().map(Into::into))?
                    .cast()
                    .at(func.span())?
            }
        }
        Ok(())
    }

    /// Advance the number of the given level by the specified amount.
    pub fn step(&mut self, level: NonZeroUsize, by: usize) {
        let level = level.get();

        if self.0.len() >= level {
            self.0[level - 1] = self.0[level - 1].saturating_add(by);
            self.0.truncate(level);
        }

        while self.0.len() < level {
            self.0.push(NonZeroUsize::ONE);
        }
    }
}

cast_from_value! {
    CounterState,
    num: NonZeroUsize => Self(smallvec![num]),
    array: Array => Self(array
        .into_iter()
        .map(Value::cast)
        .collect::<StrResult<_>>()?),
}
