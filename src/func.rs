//! Dynamic typesetting functions.

use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;

use crate::syntax::{FuncHeader, Expression};
use crate::parsing::{ParseTokens, ParseResult};


/// Parser functions.
pub type ParseFunc = dyn Fn(ParseContext) -> ParseResult<Box<dyn Function>>;

/// Types that act as functions.
///
/// These types have to be able to parse tokens into themselves and store the
/// relevant information from the parsing to do their role in typesetting later.
///
/// The trait `FunctionBounds` is automatically implemented for types which can be
/// used as functions, that is they fulfill the bounds  `Debug + PartialEq + 'static`.
pub trait Function: FunctionBounds {
    /// Parse the function.
    fn parse(context: ParseContext) -> ParseResult<Self> where Self: Sized;

    /// Execute the function and optionally yield a return value.
    fn typeset(&self, header: &FuncHeader) -> Option<Expression>;
}

/// A map from identifiers to functions.
pub struct Scope {
    parsers: HashMap<String, Box<ParseFunc>>,
}

impl Scope {
    /// Create a new empty scope.
    pub fn new() -> Scope {
        Scope { parsers: HashMap::new() }
    }

    /// Add a function type to the scope with a given name.
    pub fn add<F: Function + 'static>(&mut self, name: &str) {
        self.parsers.insert(
            name.to_owned(),
            Box::new(|context| match F::parse(context) {
                Ok(func) => Ok(Box::new(func)),
                Err(err) => Err(err),
            })
        );
    }

    /// Return the parser with the given name if there is one.
    pub fn get_parser(&self, name: &str) -> Option<&ParseFunc> {
        self.parsers.get(name).map(|x| &**x)
    }
}

/// The context for parsing a function.
pub struct ParseContext<'s, 't> {
    /// The header of the function to be parsed.
    pub header: &'s FuncHeader,
    /// Tokens if the function has a body, otherwise nothing.
    pub tokens: Option<&'s mut ParseTokens<'t>>,
    /// The current scope containing function definitions.
    pub scope: &'s Scope,
}

/// A helper trait that describes requirements for types that can implement [`Function`].
///
/// Automatically implemented for all types which fulfill to the bounds
/// `Debug + PartialEq + 'static`. There should be no need to implement this manually.
pub trait FunctionBounds: Debug {
    /// Cast self into `Any`.
    fn help_cast_as_any(&self) -> &dyn Any;

    /// Compare self with another function.
    fn help_eq(&self, other: &dyn Function) -> bool;
}

impl<T> FunctionBounds for T where T: Debug + PartialEq + 'static {
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

impl PartialEq for dyn Function {
    fn eq(&self, other: &dyn Function) -> bool {
        self.help_eq(other)
    }
}
