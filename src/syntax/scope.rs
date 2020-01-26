//! Scopes containing function parsers.

use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};

use crate::func::ParseFunc;
use super::func::FuncHeader;
use super::parsing::{ParseContext, Parsed};
use super::span::Spanned;
use super::Model;


/// A map from identifiers to function parsers.
pub struct Scope {
    parsers: HashMap<String, Box<Parser>>,
    fallback: Box<Parser>
}

impl Scope {
    /// Create a new empty scope with a fallback parser that is invoked when no
    /// match is found.
    pub fn new<F>() -> Scope
    where F: ParseFunc<Meta=()> + Model + 'static {
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
    where F: ParseFunc<Meta=()> + Model + 'static {
        self.add_with_meta::<F>(name, ());
    }

    /// Add a parseable type with additional metadata  that is given to the
    /// parser (other than the default of `()`).
    pub fn add_with_meta<F>(&mut self, name: &str, metadata: <F as ParseFunc>::Meta)
    where F: ParseFunc + Model + 'static {
        self.parsers.insert(
            name.to_owned(),
            parser::<F>(metadata),
        );
    }

    /// Return the parser with the given name if there is one.
    pub fn get_parser(&self, name: &str) -> Result<&Parser, &Parser> {
        self.parsers.get(name)
            .map(|x| &**x)
            .ok_or_else(|| &*self.fallback)
    }

    /// Return the fallback parser.
    pub fn get_fallback_parser(&self) -> &Parser {
        &*self.fallback
    }
}

impl Debug for Scope {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Scope ")?;
        write!(f, "{:?}", self.parsers.keys())
    }
}

/// A function which parses the source of a function into a model type which
/// implements [`Model`].
type Parser = dyn Fn(
    FuncHeader,
    Option<Spanned<&str>>,
    ParseContext,
) -> Parsed<Box<dyn Model>>;

fn parser<F>(metadata: <F as ParseFunc>::Meta) -> Box<Parser>
where F: ParseFunc + Model + 'static {
    Box::new(move |h, b, c| {
        F::parse(h, b, c, metadata.clone())
            .map(|model| Box::new(model) as Box<dyn Model>)
    })
}
