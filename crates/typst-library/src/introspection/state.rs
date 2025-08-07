use comemo::{Track, Tracked, TrackedMut};
use ecow::{EcoString, EcoVec, eco_format, eco_vec};
use typst_syntax::{Span, Spanned};

use crate::World;
use crate::diag::{At, SourceResult, bail};
use crate::engine::{Engine, Route, Sink, Traced};
use crate::foundations::{
    Args, Construct, Content, Context, Func, LocatableSelector, NativeElement, Repr,
    Selector, Str, Value, cast, elem, func, scope, select_where, ty,
};
use crate::introspection::{Introspector, Locatable, Location};
use crate::routines::Routines;

/// Manages stateful parts of your document.
///
/// Let's say you have some computations in your document and want to remember
/// the result of your last computation to use it in the next one. You might try
/// something similar to the code below and expect it to output 10, 13, 26, and
/// 21. However this **does not work** in Typst. If you test this code, you will
/// see that Typst complains with the following error message: _Variables from
/// outside the function are read-only and cannot be modified._
///
/// ```typ
/// // This doesn't work!
/// #let x = 0
/// #let compute(expr) = {
///   x = eval(
///     expr.replace("x", str(x))
///   )
///   [New value is #x. ]
/// }
///
/// #compute("10") \
/// #compute("x + 3") \
/// #compute("x * 2") \
/// #compute("x - 5")
/// ```
///
/// # State and document markup { #state-and-markup }
/// Why does it do that? Because, in general, this kind of computation with side
/// effects is problematic in document markup and Typst is upfront about that.
/// For the results to make sense, the computation must proceed in the same
/// order in which the results will be laid out in the document. In our simple
/// example, that's the case, but in general it might not be.
///
/// Let's look at a slightly different, but similar kind of state: The heading
/// numbering. We want to increase the heading counter at each heading. Easy
/// enough, right? Just add one. Well, it's not that simple. Consider the
/// following example:
///
/// ```example
/// #set heading(numbering: "1.")
/// #let template(body) = [
///   = Outline
///   ...
///   #body
/// ]
///
/// #show: template
///
/// = Introduction
/// ...
/// ```
///
/// Here, Typst first processes the body of the document after the show rule,
/// sees the `Introduction` heading, then passes the resulting content to the
/// `template` function and only then sees the `Outline`. Just counting up would
/// number the `Introduction` with `1` and the `Outline` with `2`.
///
/// # Managing state in Typst { #state-in-typst }
/// So what do we do instead? We use Typst's state management system. Calling
/// the `state` function with an identifying string key and an optional initial
/// value gives you a state value which exposes a few functions. The two most
/// important ones are `get` and `update`:
///
/// - The [`get`]($state.get) function retrieves the current value of the state.
///   Because the value can vary over the course of the document, it is a
///   _contextual_ function that can only be used when [context]($context) is
///   available.
///
/// - The [`update`]($state.update) function modifies the state. You can give it
///   any value. If given a non-function value, it sets the state to that value.
///   If given a function, that function receives the previous state and has to
///   return the new state.
///
/// Our initial example would now look like this:
///
/// ```example
/// #let s = state(0)
/// #let compute(expr) = [
///   #s.update(x =>
///     eval(expr.replace("x", str(x)))
///   )
///   New value is #context s.get().
/// ]
///
/// #compute("10") \
/// #compute("x + 3") \
/// #compute("x * 2") \
/// #compute("x - 5")
/// ```
///
/// State managed by Typst is always updated in layout order, not in evaluation
/// order. The `update` method returns content and its effect occurs at the
/// position where the returned content is inserted into the document.
///
/// As a result, we can now also store some of the computations in variables,
/// but they still show the correct results:
///
/// ```example
/// >>> #let s = state(0)
/// >>> #let compute(expr) = [
/// >>>   #s.update(x =>
/// >>>     eval(expr.replace("x", str(x)))
/// >>>   )
/// >>>   New value is #context s.get().
/// >>> ]
/// <<< ...
///
/// #let more = [
///   #compute("x * 2") \
///   #compute("x - 5")
/// ]
///
/// #compute("10") \
/// #compute("x + 3") \
/// #more
/// ```
///
/// This example is of course a bit silly, but in practice this is often exactly
/// what you want! A good example are heading counters, which is why Typst's
/// [counting system]($counter) is very similar to its state system.
///
/// # Time Travel
/// By using Typst's state management system you also get time travel
/// capabilities! We can find out what the value of the state will be at any
/// position in the document from anywhere else. In particular, the `at` method
/// gives us the value of the state at any particular location and the `final`
/// methods gives us the value of the state at the end of the document.
///
/// ```example
/// >>> #let s = state(0)
/// >>> #let compute(expr) = [
/// >>>   #s.update(x => {
/// >>>     eval(expr.replace("x", str(x)))
/// >>>   })
/// >>>   New value is #context s.get().
/// >>> ]
/// <<< ...
///
/// Value at `<here>` is
/// #context s.at(<here>)
///
/// #compute("10") \
/// #compute("x + 3") \
/// *Here.* <here> \
/// #compute("x * 2") \
/// #compute("x - 5")
/// ```
///
/// # State Keys { #keys }
/// States are primarily identified by the location of their `state(...)` call.
/// This location is _not_ determined when the call happens, like you might expect,
/// but all the way at the start of compilation after typst has read your file.
///
/// This means that two different calls can result in an identical state,
/// as long as they go through the same call expression in the file.
/// ```example
/// // All calls to `same-state` go through the same `state(...)` call here.
/// #let same-state() = state(0)
///
/// #let s1 = same-state()
/// #let s2 = same-state()
/// #s1.update(1)
/// #context s2.get()
/// ```
///
/// To remedy this, you can specify a key; States with different keys will be
/// different even when they go through the same `state(...)` call.
/// ```example
/// #let same-state(key) = state(key, 0)
///
/// #let s1 = same-state("a")
/// #let s2 = same-state("b")
/// #s1.update(1)
/// #context s2.get()
/// ```
///
/// On the other hand, states from different locations are still different even
/// when they have the same key.
/// ```example
/// #let s1 = state("key", 0)
/// #let s2 = state("key", 0)
/// #s1.update(1)
/// #context s2.get()
/// ```
///
/// If you construct two states with identical locations and keys but different
/// initial values, then they will each use their own initial value but share
/// updates. Specifically, the value of a state at some location in the document
/// will be computed from that state's initial value and all preceding updates
/// for the state's location-key combination.
/// ```example
/// #let same-state(init) = state(init)
///
/// #let banana = same-state("🍌")
/// #let broccoli = same-state("🥦")
///
/// #banana.update(it => it + "😋")
///
/// #context [
///   - #same-state("🍎").get()
///   - #banana.get()
///   - #broccoli.get()
/// ]
/// ```
///
/// # A word of caution { #caution }
/// To resolve the values of all states, Typst evaluates parts of your code
/// multiple times. However, there is no guarantee that your state manipulation
/// can actually be completely resolved.
///
/// For instance, if you generate state updates depending on the final value of
/// a state, the results might never converge. The example below illustrates
/// this. We initialize our state with `1` and then update it to its own final
/// value plus 1. So it should be `2`, but then its final value is `2`, so it
/// should be `3`, and so on. This example displays a finite value because Typst
/// simply gives up after a few attempts.
///
/// ```example
/// // This is bad!
/// #let s = state(1)
/// #context s.update(s.final() + 1)
/// #context s.get()
/// ```
///
/// In general, you should try not to generate state updates from within context
/// expressions. If possible, try to express your updates as non-contextual
/// values or functions that compute the new value from the previous value.
/// Sometimes, it cannot be helped, but in those cases it is up to you to ensure
/// that the result converges.
#[ty(scope)]
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct State {
    /// The key that identifies the state.
    key: Spanned<Str>,
    /// The initial value of the state.
    init: Value,
}

impl State {
    /// Create a new state identified by a key.
    pub fn new(key: Spanned<Str>, init: Value) -> State {
        Self { key, init }
    }

    /// Get the value of the state at the given location.
    pub fn at_loc(&self, engine: &mut Engine, loc: Location) -> SourceResult<Value> {
        let sequence = self.sequence(engine)?;
        let offset = engine.introspector.query_count_before(&self.selector(), loc);
        Ok(sequence[offset].clone())
    }

    /// Produce the whole sequence of states.
    ///
    /// This has to happen just once for all states, cutting down the number
    /// of state updates from quadratic to linear.
    fn sequence(&self, engine: &mut Engine) -> SourceResult<EcoVec<Value>> {
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
    ) -> SourceResult<EcoVec<Value>> {
        let mut engine = Engine {
            routines,
            world,
            introspector,
            traced,
            sink,
            route: Route::extend(route).unnested(),
        };
        let mut state = self.init.clone();
        let mut stops = eco_vec![state.clone()];

        for elem in introspector.query(&self.selector()) {
            let elem = elem.to_packed::<StateUpdateElem>().unwrap();
            match &elem.update {
                StateUpdate::Set(value) => state = value.clone(),
                StateUpdate::Func(func) => {
                    state = func.call(&mut engine, Context::none().track(), [state])?
                }
            }
            stops.push(state.clone());
        }

        Ok(stops)
    }

    /// The selector for this state's updates.
    fn selector(&self) -> Selector {
        select_where!(StateUpdateElem, key => self.key.clone())
    }

    /// Selects all state updates.
    pub fn select_any() -> Selector {
        StateUpdateElem::ELEM.select()
    }
}

#[scope]
impl State {
    /// Create a new state.
    #[func(constructor)]
    pub fn construct(
        engine: &mut Engine,
        args: &mut Args,
        /// The key that identifies this state.
        ///
        /// See {#keys} for details.
        #[external]
        #[default("".into())]
        key: Str,
        /// The initial value of the state.
        ///
        /// See {#keys} for details.
        #[external]
        init: Value,
    ) -> SourceResult<State> {
        let key: Str;
        let init: Value;
        if args.remaining() <= 1 {
            let arg: Spanned<Value> = args.expect("key or init")?;
            key = "".into();
            init = arg.v;
            if let Value::Str(s) = &init {
                engine.sink.warn(crate::diag::warning!(arg.span,
                    "state behavior changed from the previous version";
                    hint: "this now constructs an anonymous state with initial value `{}`", s.repr();
                    hint: "use an explicit key to silence this warning: `state(\"\", {})`", s.repr()
                ))
            }
        } else {
            key = args.eat()?.unwrap();
            init = args.eat()?.unwrap();
        }

        Ok(Self::new(Spanned { span: args.span, v: key }, init))
    }

    /// Retrieves the value of the state at the current location.
    ///
    /// This is equivalent to `{state.at(here())}`.
    #[typst_macros::time(name = "state.get", span = span)]
    #[func(contextual)]
    pub fn get(
        &self,
        engine: &mut Engine,
        context: Tracked<Context>,
        span: Span,
    ) -> SourceResult<Value> {
        let loc = context.location().at(span)?;
        self.at_loc(engine, loc)
    }

    /// Retrieves the value of the state at the given selector's unique match.
    ///
    /// The `selector` must match exactly one element in the document. The most
    /// useful kinds of selectors for this are [labels]($label) and
    /// [locations]($location).
    #[typst_macros::time(name = "state.at", span = span)]
    #[func(contextual)]
    pub fn at(
        &self,
        engine: &mut Engine,
        context: Tracked<Context>,
        span: Span,
        /// The place at which the state's value should be retrieved.
        selector: LocatableSelector,
    ) -> SourceResult<Value> {
        let loc = selector.resolve_unique(engine.introspector, context).at(span)?;
        self.at_loc(engine, loc)
    }

    /// Retrieves the value of the state at the end of the document.
    #[func(contextual)]
    pub fn final_(
        &self,
        engine: &mut Engine,
        context: Tracked<Context>,
        span: Span,
    ) -> SourceResult<Value> {
        context.introspect().at(span)?;
        let sequence = self.sequence(engine)?;
        Ok(sequence.last().unwrap().clone())
    }

    /// Updates the value of the state.
    ///
    /// The update will be in effect at the position where the returned content
    /// is inserted into the document. If you don't put the output into the
    /// document, nothing happens! This would be the case, for example, if you
    /// write `{let _ = my-state.update(7)}`. State updates are always
    /// applied in layout order and in that case, Typst wouldn't know when to
    /// update the state.
    ///
    /// In contrast to [`get`]($state.get), [`at`]($state.at), and
    /// [`final`]($state.final), this function does not require [context].
    #[func]
    pub fn update(
        self,
        span: Span,
        /// A value to update to or a function to update with.
        ///
        /// - If given a non-function value, sets the state to that value.
        /// - If given a function, that function receives the state's previous
        ///   value and has to return the state's new value.
        ///
        /// When updating the state based on its previous value, you should
        /// prefer the function form instead of retrieving the previous value
        /// from the [context]($context). This allows the compiler to resolve
        /// the final state efficiently, minimizing the number of
        /// [layout iterations]($context/#compiler-iterations) required.
        ///
        /// In the following example, `{fill.update(f => not f)}` will paint odd
        /// [items in the bullet list]($list.item) as expected. However, if it's
        /// replaced with `{context fill.update(not fill.get())}`, then layout
        /// will not converge within 5 attempts, as each update will take one
        /// additional iteration to propagate.
        ///
        /// ```example
        /// #let fill = state("fill", false)
        ///
        /// #show list.item: it => {
        ///   fill.update(f => not f)
        ///   context {
        ///     set text(fill: fuchsia) if fill.get()
        ///     it
        ///   }
        /// }
        ///
        /// #lorem(5).split().map(list.item).join()
        /// ```
        update: StateUpdate,
    ) -> Content {
        StateUpdateElem::new(self.key, update).pack().spanned(span)
    }
}

impl Repr for State {
    fn repr(&self) -> EcoString {
        eco_format!("state({}, {})", self.key.v.repr(), self.init.repr())
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

cast! {
    StateUpdate,
    v: Func => Self::Func(v),
    v: Value => Self::Set(v),
}

/// Executes a display of a state.
#[elem(Construct, Locatable)]
pub struct StateUpdateElem {
    /// The key that identifies the state.
    #[required]
    key: Spanned<Str>,

    /// The update to perform on the state.
    #[required]
    #[internal]
    update: StateUpdate,
}

impl Construct for StateUpdateElem {
    fn construct(_: &mut Engine, args: &mut Args) -> SourceResult<Content> {
        bail!(args.span, "cannot be constructed manually");
    }
}
