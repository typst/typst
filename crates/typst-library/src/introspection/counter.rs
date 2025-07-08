use std::num::NonZeroUsize;
use std::str::FromStr;

use comemo::{Track, Tracked, TrackedMut};
use ecow::{eco_format, eco_vec, EcoString, EcoVec};
use smallvec::{smallvec, SmallVec};
use typst_syntax::Span;
use typst_utils::NonZeroExt;

use crate::diag::{bail, At, HintedStrResult, SourceResult};
use crate::engine::{Engine, Route, Sink, Traced};
use crate::foundations::{
    cast, elem, func, scope, select_where, ty, Args, Array, Construct, Content, Context,
    Element, Func, IntoValue, Label, LocatableSelector, NativeElement, Packed, Repr,
    Selector, Show, Smart, Str, StyleChain, Value,
};
use crate::introspection::{Introspector, Locatable, Location, Tag};
use crate::layout::{Frame, FrameItem, PageElem};
use crate::math::EquationElem;
use crate::model::{FigureElem, FootnoteElem, HeadingElem, Numbering, NumberingPattern};
use crate::routines::Routines;
use crate::World;

/// Counts through pages, elements, and more.
///
/// With the counter function, you can access and modify counters for pages,
/// headings, figures, and more. Moreover, you can define custom counters for
/// other things you want to count.
///
/// Since counters change throughout the course of the document, their current
/// value is _contextual._ It is recommended to read the chapter on [context]
/// before continuing here.
///
/// # Accessing a counter { #accessing }
/// To access the raw value of a counter, we can use the [`get`]($counter.get)
/// function. This function returns an [array]: Counters can have multiple
/// levels (in the case of headings for sections, subsections, and so on), and
/// each item in the array corresponds to one level.
///
/// ```example
/// #set heading(numbering: "1.")
///
/// = Introduction
/// Raw value of heading counter is
/// #context counter(heading).get()
/// ```
///
/// # Displaying a counter { #displaying }
/// Often, we want to display the value of a counter in a more human-readable
/// way. To do that, we can call the [`display`]($counter.display) function on
/// the counter. This function retrieves the current counter value and formats
/// it either with a provided or with an automatically inferred [numbering].
///
/// ```example
/// #set heading(numbering: "1.")
///
/// = Introduction
/// Some text here.
///
/// = Background
/// The current value is: #context {
///   counter(heading).display()
/// }
///
/// Or in roman numerals: #context {
///   counter(heading).display("I")
/// }
/// ```
///
/// # Modifying a counter { #modifying }
/// To modify a counter, you can use the `step` and `update` methods:
///
/// - The `step` method increases the value of the counter by one. Because
///   counters can have multiple levels , it optionally takes a `level`
///   argument. If given, the counter steps at the given depth.
///
/// - The `update` method allows you to arbitrarily modify the counter. In its
///   basic form, you give it an integer (or an array for multiple levels). For
///   more flexibility, you can instead also give it a function that receives
///   the current value and returns a new value.
///
/// The heading counter is stepped before the heading is displayed, so
/// `Analysis` gets the number seven even though the counter is at six after the
/// second update.
///
/// ```example
/// #set heading(numbering: "1.")
///
/// = Introduction
/// #counter(heading).step()
///
/// = Background
/// #counter(heading).update(3)
/// #counter(heading).update(n => n * 2)
///
/// = Analysis
/// Let's skip 7.1.
/// #counter(heading).step(level: 2)
///
/// == Analysis
/// Still at #context {
///   counter(heading).display()
/// }
/// ```
///
/// # Page counter
/// The page counter is special. It is automatically stepped at each pagebreak.
/// But like other counters, you can also step it manually. For example, you
/// could have Roman page numbers for your preface, then switch to Arabic page
/// numbers for your main content and reset the page counter to one.
///
/// ```example
/// >>> #set page(
/// >>>   height: 100pt,
/// >>>   margin: (bottom: 24pt, rest: 16pt),
/// >>> )
/// #set page(numbering: "(i)")
///
/// = Preface
/// The preface is numbered with
/// roman numerals.
///
/// #set page(numbering: "1 / 1")
/// #counter(page).update(1)
///
/// = Main text
/// Here, the counter is reset to one.
/// We also display both the current
/// page and total number of pages in
/// Arabic numbers.
/// ```
///
/// # Custom counters
/// To define your own counter, call the `counter` function with a string as a
/// key. This key identifies the counter globally.
///
/// ```example
/// #let mine = counter("mycounter")
/// #context mine.display() \
/// #mine.step()
/// #context mine.display() \
/// #mine.update(c => c * 3)
/// #context mine.display()
/// ```
///
/// # How to step
/// When you define and use a custom counter, in general, you should first step
/// the counter and then display it. This way, the stepping behaviour of a
/// counter can depend on the element it is stepped for. If you were writing a
/// counter for, let's say, theorems, your theorem's definition would thus first
/// include the counter step and only then display the counter and the theorem's
/// contents.
///
/// ```example
/// #let c = counter("theorem")
/// #let theorem(it) = block[
///   #c.step()
///   *Theorem #context c.display():*
///   #it
/// ]
///
/// #theorem[$1 = 1$]
/// #theorem[$2 < 3$]
/// ```
///
/// The rationale behind this is best explained on the example of the heading
/// counter: An update to the heading counter depends on the heading's level. By
/// stepping directly before the heading, we can correctly step from `1` to
/// `1.1` when encountering a level 2 heading. If we were to step after the
/// heading, we wouldn't know what to step to.
///
/// Because counters should always be stepped before the elements they count,
/// they always start at zero. This way, they are at one for the first display
/// (which happens after the first step).
///
/// # Time travel
/// Counters can travel through time! You can find out the final value of the
/// counter before it is reached and even determine what the value was at any
/// particular location in the document.
///
/// ```example
/// #let mine = counter("mycounter")
///
/// = Values
/// #context [
///   Value here: #mine.get() \
///   At intro: #mine.at(<intro>) \
///   Final value: #mine.final()
/// ]
///
/// #mine.update(n => n + 3)
///
/// = Introduction <intro>
/// #lorem(10)
///
/// #mine.step()
/// #mine.step()
/// ```
///
/// # Other kinds of state { #other-state }
/// The `counter` type is closely related to [state] type. Read its
/// documentation for more details on state management in Typst and why it
/// doesn't just use normal variables for counters.
#[ty(scope)]
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Counter(CounterKey);

impl Counter {
    /// Create a new counter identified by a key.
    pub fn new(key: CounterKey) -> Counter {
        Self(key)
    }

    /// The counter for the given element.
    pub fn of(func: Element) -> Self {
        Self::new(CounterKey::Selector(Selector::Elem(func, None)))
    }

    /// Gets the current and final value of the state combined in one state.
    pub fn both(
        &self,
        engine: &mut Engine,
        location: Location,
    ) -> SourceResult<CounterState> {
        let sequence = self.sequence(engine)?;
        let offset = engine.introspector.query_count_before(&self.selector(), location);
        let (mut at_state, at_page) = sequence[offset].clone();
        let (mut final_state, final_page) = sequence.last().unwrap().clone();
        if self.is_page() {
            let at_delta =
                engine.introspector.page(location).get().saturating_sub(at_page.get());
            at_state.step(NonZeroUsize::ONE, at_delta as u64);
            let final_delta =
                engine.introspector.pages().get().saturating_sub(final_page.get());
            final_state.step(NonZeroUsize::ONE, final_delta as u64);
        }
        Ok(CounterState(smallvec![at_state.first(), final_state.first()]))
    }

    /// Gets the value of the counter at the given location. Always returns an
    /// array of integers, even if the counter has just one number.
    pub fn at_loc(
        &self,
        engine: &mut Engine,
        location: Location,
    ) -> SourceResult<CounterState> {
        let sequence = self.sequence(engine)?;
        let offset = engine.introspector.query_count_before(&self.selector(), location);
        let (mut state, page) = sequence[offset].clone();
        if self.is_page() {
            let delta =
                engine.introspector.page(location).get().saturating_sub(page.get());
            state.step(NonZeroUsize::ONE, delta as u64);
        }
        Ok(state)
    }

    /// Displays the value of the counter at the given location.
    pub fn display_at_loc(
        &self,
        engine: &mut Engine,
        loc: Location,
        styles: StyleChain,
        numbering: &Numbering,
    ) -> SourceResult<Content> {
        let context = Context::new(Some(loc), Some(styles));
        Ok(self
            .at_loc(engine, loc)?
            .display(engine, context.track(), numbering)?
            .display())
    }

    /// Produce the whole sequence of counter states.
    ///
    /// This has to happen just once for all counters, cutting down the number
    /// of counter updates from quadratic to linear.
    fn sequence(
        &self,
        engine: &mut Engine,
    ) -> SourceResult<EcoVec<(CounterState, NonZeroUsize)>> {
        self.sequence_impl(
            engine.routines,
            engine.world,
            engine.introspector,
            engine.traced,
            TrackedMut::reborrow_mut(&mut engine.sink),
            engine.route.track(),
        )
    }

    /// Memoized implementation of `sequence`.
    #[comemo::memoize]
    fn sequence_impl(
        &self,
        routines: &Routines,
        world: Tracked<dyn World + '_>,
        introspector: Tracked<Introspector>,
        traced: Tracked<Traced>,
        sink: TrackedMut<Sink>,
        route: Tracked<Route>,
    ) -> SourceResult<EcoVec<(CounterState, NonZeroUsize)>> {
        let mut engine = Engine {
            routines,
            world,
            introspector,
            traced,
            sink,
            route: Route::extend(route).unnested(),
        };

        let mut state = CounterState::init(matches!(self.0, CounterKey::Page));
        let mut page = NonZeroUsize::ONE;
        let mut stops = eco_vec![(state.clone(), page)];

        for elem in introspector.query(&self.selector()) {
            if self.is_page() {
                let prev = page;
                page = introspector.page(elem.location().unwrap());

                let delta = page.get() - prev.get();
                if delta > 0 {
                    state.step(NonZeroUsize::ONE, delta as u64);
                }
            }

            if let Some(update) = match elem.with::<dyn Count>() {
                Some(countable) => countable.update(),
                None => Some(CounterUpdate::Step(NonZeroUsize::ONE)),
            } {
                state.update(&mut engine, update)?;
            }

            stops.push((state.clone(), page));
        }

        Ok(stops)
    }

    /// The selector relevant for this counter's updates.
    fn selector(&self) -> Selector {
        let mut selector = select_where!(CounterUpdateElem, key => self.0.clone());

        if let CounterKey::Selector(key) = &self.0 {
            selector = Selector::Or(eco_vec![selector, key.clone()]);
        }

        selector
    }

    /// Whether this is the page counter.
    fn is_page(&self) -> bool {
        self.0 == CounterKey::Page
    }

    /// Shared implementation of displaying between `counter.display` and
    /// `CounterDisplayElem`.
    fn display_impl(
        &self,
        engine: &mut Engine,
        location: Location,
        numbering: Smart<Numbering>,
        both: bool,
        styles: Option<StyleChain>,
    ) -> SourceResult<Value> {
        let numbering = numbering
            .custom()
            .or_else(|| {
                let styles = styles?;
                match self.0 {
                    CounterKey::Page => styles.get_cloned(PageElem::numbering),
                    CounterKey::Selector(Selector::Elem(func, _)) => {
                        if func == HeadingElem::ELEM {
                            styles.get_cloned(HeadingElem::numbering)
                        } else if func == FigureElem::ELEM {
                            styles.get_cloned(FigureElem::numbering)
                        } else if func == EquationElem::ELEM {
                            styles.get_cloned(EquationElem::numbering)
                        } else if func == FootnoteElem::ELEM {
                            Some(styles.get_cloned(FootnoteElem::numbering))
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            })
            .unwrap_or_else(|| NumberingPattern::from_str("1.1").unwrap().into());

        let state = if both {
            self.both(engine, location)?
        } else {
            self.at_loc(engine, location)?
        };

        let context = Context::new(Some(location), styles);
        state.display(engine, context.track(), &numbering)
    }

    /// Selects all state updates.
    pub fn select_any() -> Selector {
        CounterUpdateElem::ELEM.select()
    }
}

#[scope]
impl Counter {
    /// Create a new counter identified by a key.
    #[func(constructor)]
    pub fn construct(
        /// The key that identifies this counter.
        ///
        /// - If it is a string, creates a custom counter that is only affected
        ///   by manual updates,
        /// - If it is the [`page`] function, counts through pages,
        /// - If it is a [selector], counts through elements that matches with the
        ///   selector. For example,
        ///   - provide an element function: counts elements of that type,
        ///   - provide a [`{<label>}`]($label): counts elements with that label.
        key: CounterKey,
    ) -> Counter {
        Self::new(key)
    }

    /// Retrieves the value of the counter at the current location. Always
    /// returns an array of integers, even if the counter has just one number.
    ///
    /// This is equivalent to `{counter.at(here())}`.
    #[func(contextual)]
    pub fn get(
        &self,
        engine: &mut Engine,
        context: Tracked<Context>,
        span: Span,
    ) -> SourceResult<CounterState> {
        let loc = context.location().at(span)?;
        self.at_loc(engine, loc)
    }

    /// Displays the current value of the counter with a numbering and returns
    /// the formatted output.
    #[func(contextual)]
    pub fn display(
        self,
        engine: &mut Engine,
        context: Tracked<Context>,
        span: Span,
        /// A [numbering pattern or a function]($numbering), which specifies how
        /// to display the counter. If given a function, that function receives
        /// each number of the counter as a separate argument. If the amount of
        /// numbers varies, e.g. for the heading argument, you can use an
        /// [argument sink]($arguments).
        ///
        /// If this is omitted or set to `{auto}`, displays the counter with the
        /// numbering style for the counted element or with the pattern
        /// `{"1.1"}` if no such style exists.
        #[default]
        numbering: Smart<Numbering>,
        /// If enabled, displays the current and final top-level count together.
        /// Both can be styled through a single numbering pattern. This is used
        /// by the page numbering property to display the current and total
        /// number of pages when a pattern like `{"1 / 1"}` is given.
        #[named]
        #[default(false)]
        both: bool,
    ) -> SourceResult<Value> {
        let loc = context.location().at(span)?;
        self.display_impl(engine, loc, numbering, both, context.styles().ok())
    }

    /// Retrieves the value of the counter at the given location. Always returns
    /// an array of integers, even if the counter has just one number.
    ///
    /// The `selector` must match exactly one element in the document. The most
    /// useful kinds of selectors for this are [labels]($label) and
    /// [locations]($location).
    #[func(contextual)]
    pub fn at(
        &self,
        engine: &mut Engine,
        context: Tracked<Context>,
        span: Span,
        /// The place at which the counter's value should be retrieved.
        selector: LocatableSelector,
    ) -> SourceResult<CounterState> {
        let loc = selector.resolve_unique(engine.introspector, context).at(span)?;
        self.at_loc(engine, loc)
    }

    /// Retrieves the value of the counter at the end of the document. Always
    /// returns an array of integers, even if the counter has just one number.
    #[func(contextual)]
    pub fn final_(
        &self,
        engine: &mut Engine,
        context: Tracked<Context>,
        span: Span,
    ) -> SourceResult<CounterState> {
        context.introspect().at(span)?;
        let sequence = self.sequence(engine)?;
        let (mut state, page) = sequence.last().unwrap().clone();
        if self.is_page() {
            let delta = engine.introspector.pages().get().saturating_sub(page.get());
            state.step(NonZeroUsize::ONE, delta as u64);
        }
        Ok(state)
    }

    /// Increases the value of the counter by one.
    ///
    /// The update will be in effect at the position where the returned content
    /// is inserted into the document. If you don't put the output into the
    /// document, nothing happens! This would be the case, for example, if you
    /// write `{let _ = counter(page).step()}`. Counter updates are always
    /// applied in layout order and in that case, Typst wouldn't know when to
    /// step the counter.
    #[func]
    pub fn step(
        self,
        span: Span,
        /// The depth at which to step the counter. Defaults to `{1}`.
        #[named]
        #[default(NonZeroUsize::ONE)]
        level: NonZeroUsize,
    ) -> Content {
        self.update(span, CounterUpdate::Step(level))
    }

    /// Updates the value of the counter.
    ///
    /// Just like with `step`, the update only occurs if you put the resulting
    /// content into the document.
    #[func]
    pub fn update(
        self,
        span: Span,
        /// If given an integer or array of integers, sets the counter to that
        /// value. If given a function, that function receives the previous
        /// counter value (with each number as a separate argument) and has to
        /// return the new value (integer or array).
        update: CounterUpdate,
    ) -> Content {
        CounterUpdateElem::new(self.0, update).pack().spanned(span)
    }
}

impl Repr for Counter {
    fn repr(&self) -> EcoString {
        eco_format!("counter({})", self.0.repr())
    }
}

/// Identifies a counter.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum CounterKey {
    /// The page counter.
    Page,
    /// Counts elements matching the given selectors. Only works for
    /// [locatable]($location/#locatable)
    /// elements or labels.
    Selector(Selector),
    /// Counts through manual counters with the same key.
    Str(Str),
}

cast! {
    CounterKey,
    self => match self {
        Self::Page => PageElem::ELEM.into_value(),
        Self::Selector(v) => v.into_value(),
        Self::Str(v) => v.into_value(),
    },
    v: Str => Self::Str(v),
    v: Label => Self::Selector(Selector::Label(v)),
    v: Element => {
        if v == PageElem::ELEM {
            Self::Page
        } else {
            Self::Selector(LocatableSelector::from_value(v.into_value())?.0)
        }
    },
    v: LocatableSelector => Self::Selector(v.0),
}

impl Repr for CounterKey {
    fn repr(&self) -> EcoString {
        match self {
            Self::Page => "page".into(),
            Self::Selector(selector) => selector.repr(),
            Self::Str(str) => str.repr(),
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

cast! {
    CounterUpdate,
    v: CounterState => Self::Set(v),
    v: Func => Self::Func(v),
}

/// Elements that have special counting behaviour.
pub trait Count {
    /// Get the counter update for this element.
    fn update(&self) -> Option<CounterUpdate>;
}

/// Counts through elements with different levels.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct CounterState(pub SmallVec<[u64; 3]>);

impl CounterState {
    /// Get the initial counter state for the key.
    pub fn init(page: bool) -> Self {
        // Special case, because pages always start at one.
        Self(smallvec![u64::from(page)])
    }

    /// Advance the counter and return the numbers for the given heading.
    pub fn update(
        &mut self,
        engine: &mut Engine,
        update: CounterUpdate,
    ) -> SourceResult<()> {
        match update {
            CounterUpdate::Set(state) => *self = state,
            CounterUpdate::Step(level) => self.step(level, 1),
            CounterUpdate::Func(func) => {
                *self = func
                    .call(engine, Context::none().track(), self.0.iter().copied())?
                    .cast()
                    .at(func.span())?
            }
        }
        Ok(())
    }

    /// Advance the number of the given level by the specified amount.
    pub fn step(&mut self, level: NonZeroUsize, by: u64) {
        let level = level.get();

        while self.0.len() < level {
            self.0.push(0);
        }

        self.0[level - 1] = self.0[level - 1].saturating_add(by);
        self.0.truncate(level);
    }

    /// Get the first number of the state.
    pub fn first(&self) -> u64 {
        self.0.first().copied().unwrap_or(1)
    }

    /// Display the counter state with a numbering.
    pub fn display(
        &self,
        engine: &mut Engine,
        context: Tracked<Context>,
        numbering: &Numbering,
    ) -> SourceResult<Value> {
        numbering.apply(engine, context, &self.0)
    }
}

cast! {
    CounterState,
    self => Value::Array(self.0.into_iter().map(IntoValue::into_value).collect()),
    num: u64 => Self(smallvec![num]),
    array: Array => Self(array
        .into_iter()
        .map(Value::cast)
        .collect::<HintedStrResult<_>>()?),
}

/// Executes an update of a counter.
#[elem(Construct, Locatable, Show, Count)]
struct CounterUpdateElem {
    /// The key that identifies the counter.
    #[required]
    key: CounterKey,

    /// The update to perform on the counter.
    #[required]
    #[internal]
    update: CounterUpdate,
}

impl Construct for CounterUpdateElem {
    fn construct(_: &mut Engine, args: &mut Args) -> SourceResult<Content> {
        bail!(args.span, "cannot be constructed manually");
    }
}

impl Show for Packed<CounterUpdateElem> {
    fn show(&self, _: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(Content::empty())
    }
}

impl Count for Packed<CounterUpdateElem> {
    fn update(&self) -> Option<CounterUpdate> {
        Some(self.update.clone())
    }
}

/// Executes a display of a counter.
#[elem(Construct, Locatable, Show)]
pub struct CounterDisplayElem {
    /// The counter.
    #[required]
    #[internal]
    counter: Counter,

    /// The numbering to display the counter with.
    #[required]
    #[internal]
    numbering: Smart<Numbering>,

    /// Whether to display both the current and final value.
    #[required]
    #[internal]
    both: bool,
}

impl Construct for CounterDisplayElem {
    fn construct(_: &mut Engine, args: &mut Args) -> SourceResult<Content> {
        bail!(args.span, "cannot be constructed manually");
    }
}

impl Show for Packed<CounterDisplayElem> {
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        Ok(self
            .counter
            .display_impl(
                engine,
                self.location().unwrap(),
                self.numbering.clone(),
                self.both,
                Some(styles),
            )?
            .display())
    }
}

/// An specialized handler of the page counter that tracks both the physical
/// and the logical page counter.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ManualPageCounter {
    physical: NonZeroUsize,
    logical: u64,
}

impl ManualPageCounter {
    /// Create a new fast page counter, starting at 1.
    pub fn new() -> Self {
        Self { physical: NonZeroUsize::ONE, logical: 1 }
    }

    /// Get the current physical page counter state.
    pub fn physical(&self) -> NonZeroUsize {
        self.physical
    }

    /// Get the current logical page counter state.
    pub fn logical(&self) -> u64 {
        self.logical
    }

    /// Advance past a page.
    pub fn visit(&mut self, engine: &mut Engine, page: &Frame) -> SourceResult<()> {
        for (_, item) in page.items() {
            match item {
                FrameItem::Group(group) => self.visit(engine, &group.frame)?,
                FrameItem::Tag(Tag::Start(elem)) => {
                    let Some(elem) = elem.to_packed::<CounterUpdateElem>() else {
                        continue;
                    };
                    if elem.key == CounterKey::Page {
                        let mut state = CounterState(smallvec![self.logical]);
                        state.update(engine, elem.update.clone())?;
                        self.logical = state.first();
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Step past a page _boundary._
    pub fn step(&mut self) {
        self.physical = self.physical.saturating_add(1);
        self.logical += 1;
    }
}

impl Default for ManualPageCounter {
    fn default() -> Self {
        Self::new()
    }
}
