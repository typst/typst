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

/// Counts through pages, elements, and more.
#[derive(Clone, PartialEq, Hash)]
pub struct Counter(CounterKey);

impl Counter {
    /// Create a new counter from a key.
    pub fn new(key: CounterKey) -> Self {
        Self(key)
    }

    /// The counter for the given node.
    pub fn of(id: NodeId) -> Self {
        Self::new(CounterKey::Selector(Selector::Node(id, None)))
    }

    /// Call a method on counter.
    pub fn call_method(
        self,
        vm: &mut Vm,
        method: &str,
        mut args: Args,
        span: Span,
    ) -> SourceResult<Value> {
        let pattern = |s| NumberingPattern::from_str(s).unwrap().into();
        let value = match method {
            "display" => self
                .display(
                    args.eat()?.unwrap_or_else(|| pattern("1.1")),
                    args.named("both")?.unwrap_or(false),
                )
                .into(),
            "at" => self.at(&mut vm.vt, args.expect("location")?)?.into(),
            "final" => self.final_(&mut vm.vt, args.expect("location")?)?.into(),
            "update" => self.update(args.expect("value or function")?).into(),
            "step" => self
                .update(CounterUpdate::Step(
                    args.named("level")?.unwrap_or(NonZeroUsize::ONE),
                ))
                .into(),
            _ => bail!(span, "type counter has no method `{}`", method),
        };
        args.finish()?;
        Ok(value)
    }

    /// Display the current value of the counter.
    pub fn display(self, numbering: Numbering, both: bool) -> Content {
        DisplayNode::new(self, numbering, both).pack()
    }

    /// Get the value of the state at the given location.
    pub fn at(&self, vt: &mut Vt, id: StableId) -> SourceResult<CounterState> {
        let sequence = self.sequence(vt)?;
        let offset = vt.introspector.query_before(self.selector(), id).len();
        let (mut state, page) = sequence[offset].clone();
        if self.is_page() {
            let delta = vt.introspector.page(id).get() - page.get();
            state.step(NonZeroUsize::ONE, delta);
        }
        Ok(state)
    }

    /// Get the value of the state at the final location.
    pub fn final_(&self, vt: &mut Vt, _: StableId) -> SourceResult<CounterState> {
        let sequence = self.sequence(vt)?;
        let (mut state, page) = sequence.last().unwrap().clone();
        if self.is_page() {
            let delta = vt.introspector.pages().get() - page.get();
            state.step(NonZeroUsize::ONE, delta);
        }
        Ok(state)
    }

    /// Get the current and final value of the state combined in one state.
    pub fn both(&self, vt: &mut Vt, id: StableId) -> SourceResult<CounterState> {
        let sequence = self.sequence(vt)?;
        let offset = vt.introspector.query_before(self.selector(), id).len();
        let (mut at_state, at_page) = sequence[offset].clone();
        let (mut final_state, final_page) = sequence.last().unwrap().clone();
        if self.is_page() {
            let at_delta = vt.introspector.page(id).get() - at_page.get();
            at_state.step(NonZeroUsize::ONE, at_delta);
            let final_delta = vt.introspector.pages().get() - final_page.get();
            final_state.step(NonZeroUsize::ONE, final_delta);
        }
        Ok(CounterState(smallvec![at_state.first(), final_state.first()]))
    }

    /// Produce content that performs a state update.
    pub fn update(self, update: CounterUpdate) -> Content {
        UpdateNode::new(self, update).pack()
    }

    /// Produce the whole sequence of counter states.
    ///
    /// This has to happen just once for all counters, cutting down the number
    /// of counter updates from quadratic to linear.
    fn sequence(
        &self,
        vt: &mut Vt,
    ) -> SourceResult<EcoVec<(CounterState, NonZeroUsize)>> {
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
    ) -> SourceResult<EcoVec<(CounterState, NonZeroUsize)>> {
        let mut vt = Vt { world, tracer, provider, introspector };
        let mut state = CounterState(match &self.0 {
            CounterKey::Selector(_) => smallvec![],
            _ => smallvec![NonZeroUsize::ONE],
        });
        let mut page = NonZeroUsize::ONE;
        let mut stops = eco_vec![(state.clone(), page)];

        for node in introspector.query(self.selector()) {
            if self.is_page() {
                let id = node.stable_id().unwrap();
                let prev = page;
                page = introspector.page(id);

                let delta = page.get() - prev.get();
                if delta > 0 {
                    state.step(NonZeroUsize::ONE, delta);
                }
            }

            if let Some(update) = match node.to::<UpdateNode>() {
                Some(node) => Some(node.update()),
                None => match node.with::<dyn Count>() {
                    Some(countable) => countable.update(),
                    None => Some(CounterUpdate::Step(NonZeroUsize::ONE)),
                },
            } {
                state.update(&mut vt, update)?;
            }

            stops.push((state.clone(), page));
        }

        Ok(stops)
    }

    /// The selector relevant for this counter's updates.
    fn selector(&self) -> Selector {
        let mut selector = Selector::Node(
            NodeId::of::<UpdateNode>(),
            Some(dict! { "counter" => self.clone() }),
        );

        if let CounterKey::Selector(key) = &self.0 {
            selector = Selector::Any(eco_vec![selector, key.clone()]);
        }

        selector
    }

    /// Whether this is the page counter.
    fn is_page(&self) -> bool {
        self.0 == CounterKey::Page
    }
}

impl Debug for Counter {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("counter(")?;
        self.0.fmt(f)?;
        f.write_char(')')
    }
}

cast_from_value! {
    Counter: "counter",
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

/// An update to perform on a counter.
#[derive(Clone, PartialEq, Hash)]
pub enum CounterUpdate {
    /// Set the counter to the specified state.
    Set(CounterState),
    /// Increase the number for the given level by one.
    Step(NonZeroUsize),
    /// Apply the given function to the counter's state.
    Func(Func),
}

impl Debug for CounterUpdate {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("..")
    }
}

cast_from_value! {
    CounterUpdate: "counter update",
    v: CounterState => Self::Set(v),
    v: Func => Self::Func(v),
}

/// Nodes that have special counting behaviour.
pub trait Count {
    /// Get the counter update for this node.
    fn update(&self) -> Option<CounterUpdate>;
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

    /// Get the first number of the state.
    pub fn first(&self) -> NonZeroUsize {
        self.0.first().copied().unwrap_or(NonZeroUsize::ONE)
    }

    /// Display the counter state with a numbering.
    pub fn display(&self, vt: &mut Vt, numbering: &Numbering) -> SourceResult<Content> {
        Ok(numbering.apply_vt(vt, &self.0)?.display())
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

cast_to_value! {
    v: CounterState => Value::Array(v.0.into_iter().map(Into::into).collect())
}

/// Executes a display of a state.
///
/// Display: State
/// Category: special
#[node(Locatable, Show)]
struct DisplayNode {
    /// The counter.
    #[required]
    counter: Counter,

    /// The numbering to display the counter with.
    #[required]
    numbering: Numbering,

    /// Whether to display both the current and final value.
    #[required]
    both: bool,
}

impl Show for DisplayNode {
    fn show(&self, vt: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        let id = self.0.stable_id().unwrap();
        let counter = self.counter();
        let numbering = self.numbering();
        let state = if self.both() { counter.both(vt, id) } else { counter.at(vt, id) }?;
        state.display(vt, &numbering)
    }
}

/// Executes a display of a state.
///
/// Display: State
/// Category: special
#[node(Locatable, Show)]
struct UpdateNode {
    /// The counter.
    #[required]
    counter: Counter,

    /// The update to perform on the counter.
    #[required]
    update: CounterUpdate,
}

impl Show for UpdateNode {
    fn show(&self, _: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        Ok(Content::empty())
    }
}
