use std::fmt::{self, Debug, Formatter, Write};
use std::str::FromStr;

use ecow::{eco_vec, EcoVec};
use smallvec::{smallvec, SmallVec};
use typst::eval::Dynamic;

use super::{Numbering, NumberingPattern};
use crate::layout::PageNode;
use crate::prelude::*;

/// Count through pages, elements, and more.
///
/// Display: Counter
/// Category: meta
/// Returns: content
#[func]
pub fn counter(key: Counter) -> Value {
    Value::dynamic(key)
}

/// Call a method on counter.
pub fn counter_method(
    dynamic: &Dynamic,
    method: &str,
    mut args: Args,
    span: Span,
) -> SourceResult<Value> {
    let counter = dynamic.downcast::<Counter>().unwrap();
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

    let content = CounterNode::new(counter.clone(), action).pack();
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
    pub key: Counter,

    /// The action.
    #[required]
    pub action: CounterAction,
}

impl Show for CounterNode {
    fn show(&self, vt: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        match self.action() {
            CounterAction::Get(numbering) => {
                self.key().resolve(vt, self.0.stable_id(), &numbering)
            }
            CounterAction::Final(numbering) => self.key().resolve(vt, None, &numbering),
            CounterAction::Both(numbering) => {
                let both = match &numbering {
                    Numbering::Pattern(pattern) => pattern.pieces() >= 2,
                    _ => false,
                };

                let key = self.key();
                let id = self.0.stable_id();
                if !both {
                    return key.resolve(vt, id, &numbering);
                }

                let sequence = key.sequence(vt.world, vt.introspector)?;
                let numbers = [sequence.single(id), sequence.single(None)];
                Ok(numbering.apply(vt.world, &numbers)?.display())
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
        f.pad("..")
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
pub enum Counter {
    /// The page counter.
    Page,
    /// Counts elements matching the given selectors. Only works for locatable
    /// elements or labels.
    Selector(Selector),
    /// Counts through manual counters with the same key.
    Str(Str),
}

impl Counter {
    /// Display the value of the counter at the postition of the given stable
    /// id.
    pub fn resolve(
        &self,
        vt: &Vt,
        stop: Option<StableId>,
        numbering: &Numbering,
    ) -> SourceResult<Content> {
        let sequence = self.sequence(vt.world, vt.introspector)?;
        let numbers = sequence.at(stop).0;
        Ok(numbering.apply(vt.world, &numbers)?.display())
    }

    /// Produce the whole sequence of counter states.
    ///
    /// This has to happen just once for all counters, cutting down the number
    /// of counter updates from quadratic to linear.
    #[comemo::memoize]
    fn sequence(
        &self,
        world: Tracked<dyn World>,
        introspector: Tracked<Introspector>,
    ) -> SourceResult<CounterSequence> {
        let mut search = Selector::Node(
            NodeId::of::<CounterNode>(),
            Some(dict! { "key" => self.clone() }),
        );

        if let Counter::Selector(selector) = self {
            search = Selector::Any(eco_vec![search, selector.clone()]);
        }

        let mut state = CounterState::new();
        let mut stops = EcoVec::new();

        let mut prev_page = NonZeroUsize::ONE;
        let is_page = *self == Self::Page;
        if is_page {
            state.0.push(prev_page);
        }

        for node in introspector.query(search) {
            let id = node.stable_id().unwrap();
            if is_page {
                let page = introspector.page(id);
                let delta = page.get() - prev_page.get();
                if let Some(delta) = NonZeroUsize::new(delta) {
                    state.step(delta);
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
                state.update(world, update)?;
            }

            stops.push((id, state.clone()));
        }

        Ok(CounterSequence { stops, is_page })
    }
}

cast_from_value! {
    Counter: "counter",
    v: Str => Self::Str(v),
    v: Selector => {
        match v {
            Selector::Node(id, _) => {
                if id == NodeId::of::<PageNode>() {
                    return Ok(Self::Page);
                }

                if !Content::new_of(id).can::<dyn Locatable>() {
                    Err(eco_format!("cannot count through {}s", id.name))?;
                }
            }
            Selector::Label(_) => {}
            Selector::Regex(_) => Err("cannot count through text")?,
            Selector::Any(_) => {}
        }
        Self::Selector(v)
    }
}

impl Debug for Counter {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("counter(")?;
        match self {
            Self::Page => f.pad("page")?,
            Self::Selector(selector) => selector.fmt(f)?,
            Self::Str(str) => str.fmt(f)?,
        }
        f.write_char(')')
    }
}

/// A sequence of counter values.
#[derive(Debug, Clone)]
struct CounterSequence {
    stops: EcoVec<(StableId, CounterState)>,
    is_page: bool,
}

impl CounterSequence {
    fn at(&self, stop: Option<StableId>) -> CounterState {
        let entry = match stop {
            Some(stop) => self.stops.iter().find(|&&(id, _)| id == stop),
            None => self.stops.last(),
        };

        if let Some((_, state)) = entry {
            return state.clone();
        }

        if self.is_page {
            return CounterState(smallvec![NonZeroUsize::ONE]);
        }

        CounterState::default()
    }

    fn single(&self, stop: Option<StableId>) -> NonZeroUsize {
        self.at(stop).0.first().copied().unwrap_or(NonZeroUsize::ONE)
    }
}

/// Counts through elements with different levels.
#[derive(Debug, Default, Clone, PartialEq, Hash)]
pub struct CounterState(pub SmallVec<[NonZeroUsize; 3]>);

impl CounterState {
    /// Create a new levelled counter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Advance the counter and return the numbers for the given heading.
    pub fn update(
        &mut self,
        world: Tracked<dyn World>,
        update: CounterUpdate,
    ) -> SourceResult<()> {
        match update {
            CounterUpdate::Set(state) => *self = state,
            CounterUpdate::Step(level) => self.step(level),
            CounterUpdate::Func(func) => {
                let args = Args::new(func.span(), self.0.iter().copied().map(Into::into));
                *self = func.call_detached(world, args)?.cast().at(func.span())?
            }
        }
        Ok(())
    }

    /// Advance the top level number by the specified amount.
    pub fn step(&mut self, level: NonZeroUsize) {
        let level = level.get();

        if self.0.len() >= level {
            self.0[level - 1] = self.0[level - 1].saturating_add(1);
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
