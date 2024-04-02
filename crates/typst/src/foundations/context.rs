use comemo::Track;

use crate::diag::{bail, Hint, HintedStrResult, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    elem, Args, Construct, Content, Func, Packed, Show, StyleChain, Value,
};
use crate::introspection::{Locatable, Location};

/// Data that is contextually made available to code.
///
/// _Contextual_ functions and expressions require the presence of certain
/// pieces of context to be evaluated. This includes things like `text.lang`,
/// `measure`, or `counter(heading).get()`.
#[derive(Debug, Default, Clone, Hash)]
pub struct Context<'a> {
    /// The location in the document.
    pub location: Option<Location>,
    /// The active styles.
    pub styles: Option<StyleChain<'a>>,
}

impl<'a> Context<'a> {
    /// An empty context.
    pub fn none() -> Self {
        Self::default()
    }

    /// Create a new context from its parts.
    pub fn new(location: Option<Location>, styles: Option<StyleChain<'a>>) -> Self {
        Self { location, styles }
    }
}

#[comemo::track]
impl<'a> Context<'a> {
    /// Try to extract the location.
    pub fn location(&self) -> HintedStrResult<Location> {
        require(self.location)
    }

    /// Try to extract the styles.
    pub fn styles(&self) -> HintedStrResult<StyleChain<'a>> {
        require(self.styles)
    }

    /// Guard access to the introspector by requiring at least some piece of context.
    pub fn introspect(&self) -> HintedStrResult<()> {
        require(self.location.map(|_| ()).or(self.styles.map(|_| ())))
    }
}

/// Extracts an optional piece of context, yielding an error with hints if
/// it isn't available.
fn require<T>(val: Option<T>) -> HintedStrResult<T> {
    val.ok_or("can only be used when context is known")
    .hint("try wrapping this in a `context` expression")
    .hint(
        "the `context` expression should wrap everything that depends on this function",
    )
}

/// Executes a `context` block.
#[elem(Construct, Locatable, Show)]
pub struct ContextElem {
    /// The function to call with the context.
    #[required]
    #[internal]
    func: Func,
}

impl Construct for ContextElem {
    fn construct(_: &mut Engine, args: &mut Args) -> SourceResult<Content> {
        bail!(args.span, "cannot be constructed manually");
    }
}

impl Show for Packed<ContextElem> {
    #[typst_macros::time(name = "context", span = self.span())]
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let loc = self.location().unwrap();
        let context = Context::new(Some(loc), Some(styles));
        Ok(self.func.call::<[Value; 0]>(engine, context.track(), [])?.display())
    }
}
