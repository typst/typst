//! Dynamic typesetting functions.

use std::any::Any;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use async_trait::async_trait;

use self::prelude::*;

#[macro_use]
mod macros;

/// Useful imports for creating your own functions.
pub mod prelude {
    pub use crate::func::{Scope, ParseFunc, LayoutFunc, Command, Commands};
    pub use crate::layout::prelude::*;
    pub use crate::syntax::{
        ParseContext, ParseResult,
        SyntaxTree, FuncCall, FuncArgs, PosArg, KeyArg,
        Expression, Ident, ExpressionKind,
        Spanned, Span
    };
    pub use crate::size::{Size, Size2D, SizeBox, ValueBox, ScaleSize, FSize, PSize};
    pub use crate::style::{LayoutStyle, PageStyle, TextStyle};
    pub use Command::*;
}

/// Types representing functions that are parsed from source code.
pub trait ParseFunc {
    type Meta: Clone;

    /// Parse the header and body into this function given a context.
    fn parse(
        args: FuncArgs,
        body: Option<&str>,
        ctx: ParseContext,
        metadata: Self::Meta,
    ) -> ParseResult<Self> where Self: Sized;
}

/// Function types which can be laid out in a layout context.
///
/// This trait is a supertrait of `[LayoutFuncBounds]` for technical reasons.
/// The trait `[LayoutFuncBounds]` is automatically implemented for types which
/// can be used as functions, that is, all types which fulfill the bounds `Debug
/// + PartialEq + 'static`.
#[async_trait(?Send)]
pub trait LayoutFunc: LayoutFuncBounds {
    /// Layout this function in a given context.
    ///
    /// Returns a sequence of layouting commands which describe what the
    /// function is doing.
    async fn layout<'a>(&'a self, ctx: LayoutContext<'_, '_>) -> LayoutResult<Commands<'a>>;
}

impl dyn LayoutFunc {
    /// Downcast a function trait object to a concrete function type.
    pub fn downcast<F>(&self) -> Option<&F> where F: LayoutFunc + 'static {
        self.help_cast_as_any().downcast_ref::<F>()
    }
}

impl PartialEq for dyn LayoutFunc {
    fn eq(&self, other: &dyn LayoutFunc) -> bool {
        self.help_eq(other)
    }
}

/// A helper trait that describes requirements for types that can implement
/// [`Function`].
///
/// Automatically implemented for all types which fulfill to the bounds `Debug +
/// PartialEq + 'static`. There should be no need to implement this manually.
pub trait LayoutFuncBounds: Debug {
    /// Cast self into `Any`.
    fn help_cast_as_any(&self) -> &dyn Any;

    /// Compare self with another function trait object.
    fn help_eq(&self, other: &dyn LayoutFunc) -> bool;
}

impl<T> LayoutFuncBounds for T where T: Debug + PartialEq + 'static {
    fn help_cast_as_any(&self) -> &dyn Any {
        self
    }

    fn help_eq(&self, other: &dyn LayoutFunc) -> bool {
        if let Some(other) = other.help_cast_as_any().downcast_ref::<Self>() {
            self == other
        } else {
            false
        }
    }
}

/// A sequence of layouting commands.
pub type Commands<'a> = Vec<Command<'a>>;

/// Layouting commands from functions to the typesetting engine.
#[derive(Debug)]
pub enum Command<'a> {
    LayoutTree(&'a SyntaxTree),

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
}

/// A function which parses the source of a function into a function type which
/// implements [`LayoutFunc`].
type Parser = dyn Fn(
    FuncArgs,
    Option<&str>,
    ParseContext
) -> ParseResult<Box<dyn LayoutFunc>>;

impl Scope {
    /// Create a new empty scope.
    pub fn new() -> Scope {
        Scope {
            parsers: HashMap::new(),
        }
    }

    /// Create a new scope with the standard functions contained.
    pub fn with_std() -> Scope {
        crate::library::std()
    }

    /// Associate the given name with a type that is parseable into a function.
    pub fn add<F>(&mut self, name: &str)
    where F: ParseFunc<Meta=()> + LayoutFunc + 'static {
        self.add_with_metadata::<F>(name, ());
    }

    /// Add a parseable type with additional metadata  that is given to the
    /// parser (other than the default of `()`).
    pub fn add_with_metadata<F>(&mut self, name: &str, metadata: <F as ParseFunc>::Meta)
    where F: ParseFunc + LayoutFunc + 'static {
        self.parsers.insert(
            name.to_owned(),
            Box::new(move |a, b, c| {
                F::parse(a, b, c, metadata.clone())
                    .map(|f| Box::new(f) as Box<dyn LayoutFunc>)
            })
        );
    }

    /// Return the parser with the given name if there is one.
    pub(crate) fn get_parser(&self, name: &str) -> Option<&Parser> {
        self.parsers.get(name).map(|x| &**x)
    }
}

impl Debug for Scope {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Scope ")?;
        write!(f, "{:?}", self.parsers.keys())
    }
}
