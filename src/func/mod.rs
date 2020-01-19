//! Dynamic typesetting functions.

use std::any::Any;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};

use self::prelude::*;

#[macro_use]
mod macros;

/// Useful imports for creating your own functions.
pub mod prelude {
    pub use super::{Scope, Parse, Command, Commands};
    pub use crate::layout::prelude::*;
    pub use crate::syntax::prelude::*;
    pub use crate::size::{Size, Size2D, SizeBox, ValueBox, ScaleSize, FSize, PSize};
    pub use crate::style::{LayoutStyle, PageStyle, TextStyle};
    pub use Command::*;
}

/// Parse a function from source code.
pub trait Parse {
    type Meta: Clone;

    /// Parse the header and body into this function given a context.
    fn parse(
        header: FuncHeader,
        body: Option<Spanned<&str>>,
        ctx: ParseContext,
        metadata: Self::Meta,
    ) -> Parsed<Self> where Self: Sized;
}

/// A function which parses the source of a function into a model type which
/// implements [`Model`].
type Parser = dyn Fn(
    FuncHeader,
    Option<Spanned<&str>>,
    ParseContext,
) -> Parsed<Box<dyn Model>>;

/// A sequence of layouting commands.
pub type Commands<'a> = Vec<Command<'a>>;

/// Layouting commands from functions to the typesetting engine.
#[derive(Debug)]
pub enum Command<'a> {
    LayoutSyntaxModel(&'a SyntaxModel),

    Add(Layout),
    AddMultiple(MultiLayout),
    SpacingFunc(Size, SpacingKind, GenericAxis),

    FinishLine,
    FinishSpace,
    BreakParagraph,
    BreakPage,

    SetTextStyle(TextStyle),
    SetPageStyle(PageStyle),
    SetAlignment(LayoutAlignment),
    SetAxes(LayoutAxes),
}

/// A map from identifiers to function parsers.
pub struct Scope {
    parsers: HashMap<String, Box<Parser>>,
    fallback: Box<Parser>
}

impl Scope {
    /// Create a new empty scope with a fallback parser that is invoked when no
    /// match is found.
    pub fn new<F>() -> Scope
    where F: Parse<Meta=()> + Model + 'static {
        Scope {
            parsers: HashMap::new(),
            fallback: parser::<F>(()),
        }
    }

    /// Create a new scope with the standard functions contained.
    pub fn with_std() -> Scope {
        crate::library::std()
    }

    /// Associate the given name with a type that is parseable into a function.
    pub fn add<F>(&mut self, name: &str)
    where F: Parse<Meta=()> + Model + 'static {
        self.add_with_metadata::<F>(name, ());
    }

    /// Add a parseable type with additional metadata  that is given to the
    /// parser (other than the default of `()`).
    pub fn add_with_metadata<F>(&mut self, name: &str, metadata: <F as Parse>::Meta)
    where F: Parse + Model + 'static {
        self.parsers.insert(
            name.to_owned(),
            parser::<F>(metadata),
        );
    }

    /// Return the parser with the given name if there is one.
    pub(crate) fn get_parser(&self, name: &str) -> Result<&Parser, &Parser> {
        self.parsers.get(name)
            .map(|x| &**x)
            .ok_or_else(|| &*self.fallback)
    }
}

impl Debug for Scope {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Scope ")?;
        write!(f, "{:?}", self.parsers.keys())
    }
}

fn parser<F>(metadata: <F as Parse>::Meta) -> Box<Parser> where F: Parse + Model + 'static {
    Box::new(move |h, b, c| {
        F::parse(h, b, c, metadata.clone())
            .map(|model| Box::new(model) as Box<dyn Model>)
    })
}
