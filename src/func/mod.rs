//! Dynamic typesetting functions.

use std::any::Any;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};

use self::prelude::*;

#[macro_use]
pub mod helpers;

/// Useful imports for creating your own functions.
pub mod prelude {
    pub use crate::func::{Command, CommandList, Function};
    pub use crate::layout::{layout_tree, Layout, MultiLayout, LayoutContext};
    pub use crate::layout::{LayoutSpace, LayoutSpaces, SpacingKind};
    pub use crate::layout::{LayoutAxes, Axis, AxisKind, LayoutAlignment, Alignment};
    pub use crate::layout::{LayoutError, LayoutResult};
    pub use crate::syntax::{SyntaxTree, FuncHeader, FuncArgs, Expression, Spanned, Span};
    pub use crate::syntax::{parse, ParseContext, ParseError, ParseResult};
    pub use crate::size::{Size, Size2D, SizeBox};
    pub use crate::style::{PageStyle, TextStyle};
    pub use super::helpers::*;
    pub use Command::*;
}

/// Typesetting function types.
///
/// These types have to be able to parse themselves from a string and build
/// a list of layouting commands corresponding to the parsed source.
///
/// This trait is a supertrait of `FunctionBounds` for technical reasons.  The
/// trait `FunctionBounds` is automatically implemented for types which can
/// be used as functions, that is, all types which fulfill the bounds `Debug + PartialEq +
/// 'static`.
pub trait Function: FunctionBounds {
    /// Parse the header and body into this function given a context.
    fn parse(header: &FuncHeader, body: Option<&str>, ctx: ParseContext) -> ParseResult<Self>
    where Self: Sized;

    /// Layout this function given a context.
    ///
    /// Returns optionally the resulting layout and a new context if changes to
    /// the context should be made.
    fn layout(&self, ctx: LayoutContext) -> LayoutResult<CommandList>;
}

impl dyn Function {
    /// Downcast a dynamic function to a concrete function type.
    pub fn downcast<F>(&self) -> Option<&F> where F: Function + 'static {
        self.help_cast_as_any().downcast_ref::<F>()
    }
}

impl PartialEq for dyn Function {
    fn eq(&self, other: &dyn Function) -> bool {
        self.help_eq(other)
    }
}

/// A helper trait that describes requirements for types that can implement
/// [`Function`].
///
/// Automatically implemented for all types which fulfill to the bounds `Debug +
/// PartialEq + 'static`. There should be no need to implement this manually.
pub trait FunctionBounds: Debug {
    /// Cast self into `Any`.
    fn help_cast_as_any(&self) -> &dyn Any;

    /// Compare self with another function.
    fn help_eq(&self, other: &dyn Function) -> bool;
}

impl<T> FunctionBounds for T
where T: Debug + PartialEq + 'static
{
    fn help_cast_as_any(&self) -> &dyn Any {
        self
    }

    fn help_eq(&self, other: &dyn Function) -> bool {
        if let Some(other) = other.help_cast_as_any().downcast_ref::<Self>() {
            self == other
        } else {
            false
        }
    }
}

/// A sequence of commands requested for execution by a function.
pub type CommandList<'a> = Vec<Command<'a>>;

/// Commands requested for execution by functions.
#[derive(Debug)]
pub enum Command<'a> {
    LayoutTree(&'a SyntaxTree),

    Add(Layout),
    AddMultiple(MultiLayout),
    AddSpacing(Size, SpacingKind, AxisKind),

    FinishLine,
    FinishRun,
    FinishSpace,
    BreakParagraph,

    SetTextStyle(TextStyle),
    SetPageStyle(PageStyle),
    SetAlignment(LayoutAlignment),
    SetAxes(LayoutAxes),
}

/// A map from identifiers to functions.
pub struct Scope {
    parsers: HashMap<String, Box<ParseFunc>>,
}

/// A function which parses a function invocation into a function type.
type ParseFunc = dyn Fn(&FuncHeader, Option<&str>, ParseContext) -> ParseResult<Box<dyn Function>>;

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

    /// Add a function type to the scope giving it a name.
    pub fn add<F: Function + 'static>(&mut self, name: &str) {
        self.parsers.insert(
            name.to_owned(),
            Box::new(|h, b, c| F::parse(h, b, c).map(|func| Box::new(func) as Box<dyn Function>)),
        );
    }

    /// Return the parser with the given name if there is one.
    pub(crate) fn get_parser(&self, name: &str) -> Option<&ParseFunc> {
        self.parsers.get(name).map(|x| &**x)
    }
}

impl Debug for Scope {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Scope ")?;
        write!(f, "{:?}", self.parsers.keys())
    }
}
