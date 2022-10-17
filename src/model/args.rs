use std::fmt::{self, Debug, Formatter, Write};

use super::{Array, Cast, Dict, Str, Value};
use crate::diag::{At, SourceResult};
use crate::syntax::{Span, Spanned};

/// Evaluated arguments to a function.
#[derive(Clone, PartialEq, Hash)]
pub struct Args {
    /// The span of the whole argument list.
    pub span: Span,
    /// The positional and named arguments.
    pub items: Vec<Arg>,
}

/// An argument to a function call: `12` or `draw: false`.
#[derive(Clone, PartialEq, Hash)]
pub struct Arg {
    /// The span of the whole argument.
    pub span: Span,
    /// The name of the argument (`None` for positional arguments).
    pub name: Option<Str>,
    /// The value of the argument.
    pub value: Spanned<Value>,
}

impl Args {
    /// Create positional arguments from a span and values.
    pub fn new(span: Span, values: impl IntoIterator<Item = Value>) -> Self {
        let items = values
            .into_iter()
            .map(|value| Arg {
                span,
                name: None,
                value: Spanned::new(value, span),
            })
            .collect();
        Self { span, items }
    }

    /// Push a positional argument.
    pub fn push(&mut self, span: Span, value: Value) {
        self.items.push(Arg {
            span: self.span,
            name: None,
            value: Spanned::new(value, span),
        })
    }

    /// Consume and cast the first positional argument if there is one.
    pub fn eat<T>(&mut self) -> SourceResult<Option<T>>
    where
        T: Cast<Spanned<Value>>,
    {
        for (i, slot) in self.items.iter().enumerate() {
            if slot.name.is_none() {
                let value = self.items.remove(i).value;
                let span = value.span;
                return T::cast(value).at(span).map(Some);
            }
        }
        Ok(None)
    }

    /// Consume and cast the first positional argument.
    ///
    /// Returns a `missing argument: {what}` error if no positional argument is
    /// left.
    pub fn expect<T>(&mut self, what: &str) -> SourceResult<T>
    where
        T: Cast<Spanned<Value>>,
    {
        match self.eat()? {
            Some(v) => Ok(v),
            None => bail!(self.span, "missing argument: {}", what),
        }
    }

    /// Find and consume the first castable positional argument.
    pub fn find<T>(&mut self) -> SourceResult<Option<T>>
    where
        T: Cast<Spanned<Value>>,
    {
        for (i, slot) in self.items.iter().enumerate() {
            if slot.name.is_none() && T::is(&slot.value) {
                let value = self.items.remove(i).value;
                let span = value.span;
                return T::cast(value).at(span).map(Some);
            }
        }
        Ok(None)
    }

    /// Find and consume all castable positional arguments.
    pub fn all<T>(&mut self) -> SourceResult<Vec<T>>
    where
        T: Cast<Spanned<Value>>,
    {
        let mut list = vec![];
        while let Some(value) = self.find()? {
            list.push(value);
        }
        Ok(list)
    }

    /// Cast and remove the value for the given named argument, returning an
    /// error if the conversion fails.
    pub fn named<T>(&mut self, name: &str) -> SourceResult<Option<T>>
    where
        T: Cast<Spanned<Value>>,
    {
        // We don't quit once we have a match because when multiple matches
        // exist, we want to remove all of them and use the last one.
        let mut i = 0;
        let mut found = None;
        while i < self.items.len() {
            if self.items[i].name.as_deref() == Some(name) {
                let value = self.items.remove(i).value;
                let span = value.span;
                found = Some(T::cast(value).at(span)?);
            } else {
                i += 1;
            }
        }
        Ok(found)
    }

    /// Same as named, but with fallback to find.
    pub fn named_or_find<T>(&mut self, name: &str) -> SourceResult<Option<T>>
    where
        T: Cast<Spanned<Value>>,
    {
        match self.named(name)? {
            Some(value) => Ok(Some(value)),
            None => self.find(),
        }
    }

    /// Take out all arguments into a new instance.
    pub fn take(&mut self) -> Self {
        Self {
            span: self.span,
            items: std::mem::take(&mut self.items),
        }
    }

    /// Return an "unexpected argument" error if there is any remaining
    /// argument.
    pub fn finish(self) -> SourceResult<()> {
        if let Some(arg) = self.items.first() {
            bail!(arg.span, "unexpected argument");
        }
        Ok(())
    }

    /// Extract the positional arguments as an array.
    pub fn to_positional(&self) -> Array {
        self.items
            .iter()
            .filter(|item| item.name.is_none())
            .map(|item| item.value.v.clone())
            .collect()
    }

    /// Extract the named arguments as a dictionary.
    pub fn to_named(&self) -> Dict {
        self.items
            .iter()
            .filter_map(|item| item.name.clone().map(|name| (name, item.value.v.clone())))
            .collect()
    }

    /// Reinterpret these arguments as actually being an array index.
    pub fn into_index(self) -> SourceResult<i64> {
        self.into_castable("index")
    }

    /// Reinterpret these arguments as actually being a dictionary key.
    pub fn into_key(self) -> SourceResult<Str> {
        self.into_castable("key")
    }

    /// Reinterpret these arguments as actually being a single castable thing.
    fn into_castable<T: Cast>(self, what: &str) -> SourceResult<T> {
        let mut iter = self.items.into_iter();
        let value = match iter.next() {
            Some(Arg { name: None, value, .. }) => value.v.cast().at(value.span)?,
            None => {
                bail!(self.span, "missing {}", what);
            }
            Some(Arg { name: Some(_), span, .. }) => {
                bail!(span, "named pair is not allowed here");
            }
        };

        if let Some(arg) = iter.next() {
            bail!(arg.span, "only one {} is allowed", what);
        }

        Ok(value)
    }
}

impl Debug for Args {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_char('(')?;
        for (i, arg) in self.items.iter().enumerate() {
            arg.fmt(f)?;
            if i + 1 < self.items.len() {
                f.write_str(", ")?;
            }
        }
        f.write_char(')')
    }
}

impl Debug for Arg {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if let Some(name) = &self.name {
            f.write_str(name)?;
            f.write_str(": ")?;
        }
        Debug::fmt(&self.value.v, f)
    }
}
