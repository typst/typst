use std::fmt::Debug;

use ecow::{eco_vec, EcoVec};
use typst::eval::{Repr, Tracer};
use typst::model::DelayedErrors;

use crate::prelude::*;

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
/// value gives you a state value which exposes a few methods. The two most
/// important ones are `display` and `update`:
///
/// - The `display` method shows the current value of the state. You can
///   optionally give it a function that receives the value and formats it in
///   some way.
///
/// - The `update` method modifies the state. You can give it any value. If
///   given a non-function value, it sets the state to that value. If given a
///   function, that function receives the previous state and has to return the
///   new state.
///
/// Our initial example would now look like this:
///
/// ```example
/// #let s = state("x", 0)
/// #let compute(expr) = [
///   #s.update(x =>
///     eval(expr.replace("x", str(x)))
///   )
///   New value is #s.display().
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
/// As a result, we can now also store some of the computations in
/// variables, but they still show the correct results:
///
/// ```example
/// >>> #let s = state("x", 0)
/// >>> #let compute(expr) = [
/// >>>   #s.update(x =>
/// >>>     eval(expr.replace("x", str(x)))
/// >>>   )
/// >>>   New value is #s.display().
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
/// capabilities! By combining the state system with [`locate`]($locate) and
/// [`query`]($query), we can find out what the value of the state will be at
/// any position in the document from anywhere else. In particular, the `at`
/// method gives us the value of the state at any location and the `final`
/// methods gives us the value of the state at the end of the document.
///
/// ```example
/// >>> #let s = state("x", 0)
/// >>> #let compute(expr) = [
/// >>>   #s.update(x => {
/// >>>     eval(expr.replace("x", str(x)))
/// >>>   })
/// >>>   New value is #s.display().
/// >>> ]
/// <<< ...
///
/// Value at `<here>` is
/// #locate(loc => s.at(
///   query(<here>, loc)
///     .first()
///     .location()
/// ))
///
/// #compute("10") \
/// #compute("x + 3") \
/// *Here.* <here> \
/// #compute("x * 2") \
/// #compute("x - 5")
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
/// should be `3`, and so on. This example display `4` because Typst simply
/// gives up after a few attempts.
///
/// ```example
/// #let s = state("x", 1)
/// #locate(loc => {
///   s.update(s.final(loc) + 1)
/// })
/// #s.display()
/// ```
///
/// In general, you should _typically_ not generate state updates from within
/// `locate` calls or `display` calls of state or counters. Instead, pass a
/// function to `update` that determines the value of the state based on its
/// previous value.
#[ty(scope)]
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct State {
    /// The key that identifies the state.
    key: Str,
    /// The initial value of the state.
    init: Value,
}

impl State {
    /// Create a new state identified by a key.
    pub fn new(key: Str, init: Value) -> State {
        Self { key, init }
    }

    /// Produce the whole sequence of states.
    ///
    /// This has to happen just once for all states, cutting down the number
    /// of state updates from quadratic to linear.
    fn sequence(&self, vt: &mut Vt) -> SourceResult<EcoVec<Value>> {
        self.sequence_impl(
            vt.world,
            vt.introspector,
            vt.locator.track(),
            TrackedMut::reborrow_mut(&mut vt.delayed),
            TrackedMut::reborrow_mut(&mut vt.tracer),
        )
    }

    /// Memoized implementation of `sequence`.
    #[comemo::memoize]
    fn sequence_impl(
        &self,
        world: Tracked<dyn World + '_>,
        introspector: Tracked<Introspector>,
        locator: Tracked<Locator>,
        delayed: TrackedMut<DelayedErrors>,
        tracer: TrackedMut<Tracer>,
    ) -> SourceResult<EcoVec<Value>> {
        let mut locator = Locator::chained(locator);
        let mut vt = Vt {
            world,
            introspector,
            locator: &mut locator,
            delayed,
            tracer,
        };
        let mut state = self.init.clone();
        let mut stops = eco_vec![state.clone()];

        for elem in introspector.query(&self.selector()) {
            let elem = elem.to::<UpdateElem>().unwrap();
            match elem.update() {
                StateUpdate::Set(value) => state = value,
                StateUpdate::Func(func) => state = func.call_vt(&mut vt, [state])?,
            }
            stops.push(state.clone());
        }

        Ok(stops)
    }

    /// The selector for this state's updates.
    fn selector(&self) -> Selector {
        Selector::Elem(UpdateElem::elem(), Some(dict! { "key" => self.key.clone() }))
    }
}

#[scope]
impl State {
    /// Create a new state identified by a key.
    #[func(constructor)]
    pub fn construct(
        /// The key that identifies this state.
        key: Str,
        /// The initial value of the state.
        #[default]
        init: Value,
    ) -> State {
        Self::new(key, init)
    }

    /// Displays the current value of the state.
    #[func]
    pub fn display(
        self,
        /// A function which receives the value of the state and can return
        /// arbitrary content which is then displayed. If this is omitted, the
        /// value is directly displayed.
        #[default]
        func: Option<Func>,
    ) -> Content {
        DisplayElem::new(self, func).pack()
    }

    /// Update the value of the state.
    ///
    /// The update will be in effect at the position where the returned content
    /// is inserted into the document. If you don't put the output into the
    /// document, nothing happens! This would be the case, for example, if you
    /// write `{let _ = state("key").update(7)}`. State updates are always
    /// applied in layout order and in that case, Typst wouldn't know when to
    /// update the state.
    #[func]
    pub fn update(
        self,
        /// If given a non function-value, sets the state to that value. If
        /// given a function, that function receives the previous state and has
        /// to return the new state.
        update: StateUpdate,
    ) -> Content {
        UpdateElem::new(self.key, update).pack()
    }

    /// Get the value of the state at the given location.
    #[func]
    pub fn at(
        &self,
        /// The virtual typesetter.
        vt: &mut Vt,
        /// The location at which the state's value should be retrieved. A
        /// suitable location can be retrieved from [`locate`]($locate) or
        /// [`query`]($query).
        location: Location,
    ) -> SourceResult<Value> {
        let sequence = self.sequence(vt)?;
        let offset = vt
            .introspector
            .query(&self.selector().before(location.into(), true))
            .len();
        Ok(sequence[offset].clone())
    }

    /// Get the value of the state at the end of the document.
    #[func]
    pub fn final_(
        &self,
        /// The virtual typesetter.
        vt: &mut Vt,
        /// Can be an arbitrary location, as its value is irrelevant for the
        /// method's return value. Why is it required then? As noted before,
        /// Typst has to evaluate parts of your code multiple times to determine
        /// the values of all state. By only allowing this method within
        /// [`locate`]($locate) calls, the amount of code that can depend on the
        /// method's result is reduced. If you could call `final` directly at
        /// the top level of a module, the evaluation of the whole module and
        /// its exports could depend on the state's value.
        location: Location,
    ) -> SourceResult<Value> {
        let _ = location;
        let sequence = self.sequence(vt)?;
        Ok(sequence.last().unwrap().clone())
    }
}

impl Repr for State {
    fn repr(&self) -> EcoString {
        eco_format!("state({}, {})", self.key.repr(), self.init.repr())
    }
}

cast! {
    type State,
}

/// An update to perform on a state.
#[ty]
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum StateUpdate {
    /// Set the state to the specified value.
    Set(Value),
    /// Apply the given function to the state.
    Func(Func),
}

impl Repr for StateUpdate {
    fn repr(&self) -> EcoString {
        "..".into()
    }
}

cast! {
    type StateUpdate,
    v: Func => Self::Func(v),
    v: Value => Self::Set(v),
}

/// Executes a display of a state.
#[elem(Locatable, Show)]
struct DisplayElem {
    /// The state.
    #[required]
    state: State,

    /// The function to display the state with.
    #[required]
    func: Option<Func>,
}

impl Show for DisplayElem {
    #[tracing::instrument(name = "DisplayElem::show", skip(self, vt))]
    fn show(&self, vt: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        Ok(vt.delayed(|vt| {
            let location = self.0.location().unwrap();
            let value = self.state().at(vt, location)?;
            Ok(match self.func() {
                Some(func) => func.call_vt(vt, [value])?.display(),
                None => value.display(),
            })
        }))
    }
}

/// Executes a display of a state.
#[elem(Locatable, Show)]
struct UpdateElem {
    /// The key that identifies the state.
    #[required]
    key: Str,

    /// The update to perform on the state.
    #[required]
    update: StateUpdate,
}

impl Show for UpdateElem {
    #[tracing::instrument(name = "UpdateElem::show")]
    fn show(&self, _: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        Ok(Content::empty())
    }
}
