use std::fmt::{self, Debug, Formatter};

use ecow::{eco_format, EcoVec};

use super::{Array, Cast, Dict, Str, Value};
use crate::diag::{bail, At, SourceResult};
use crate::syntax::{Span, Spanned};
use crate::util::pretty_array_like;

/// Evaluated arguments to a function.
#[derive(Clone, PartialEq, Hash)]
pub struct Args {
    /// The span of the whole argument list.
    pub span: Span,
    /// The positional and named arguments.
    pub items: EcoVec<Arg>,
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
    #[must_use]
    pub fn new(span: Span, values: impl IntoIterator<Item = Value>) -> Self {
        let items = values
            .into_iter()
            .map(|value| Arg { span, name: None, value: Spanned::new(value, span) })
            .collect();
        Self { span, items }
    }

    /// Push a positional argument.
    pub fn push(&mut self, span: Span, value: Value) {
        self.items.push(Arg {
            span: self.span,
            name: None,
            value: Spanned::new(value, span),
        });
    }

    /// Consume and cast the first positional argument if there is one.
    ///
    /// # Errors
    ///
    /// If the positional argument cannot be casted to `T`.
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
    /// # Errors
    ///
    /// If no positional argument is left or if the positional argument cannot be casted to `T`.
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
    ///
    /// # Errors
    ///
    /// If a castable positional argument is found but fails to cast to `T`.
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
    ///
    /// # Errors
    ///
    /// If any castable positional argument fails to cast to `T`.
    pub fn all<T>(&mut self) -> SourceResult<Vec<T>>
    where
        T: Cast<Spanned<Value>>,
    {
        std::iter::from_fn(|| self.find::<T>().transpose()).collect()
    }

    /// Cast and remove the value for the given named argument.
    ///
    /// # Errors
    ///
    /// If the argument fails to cast to `T`.
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
    ///
    /// # Errors
    ///
    /// If the argument fails to cast to `T`.
    pub fn named_or_find<T>(&mut self, name: &str) -> SourceResult<Option<T>>
    where
        T: Cast<Spanned<Value>>,
    {
        Ok(if let Some(named) = self.named(name)? { Some(named) } else { self.find()? })
    }

    /// Take out all arguments into a new instance.
    #[must_use]
    pub fn take(&mut self) -> Self {
        Self {
            span: self.span,
            items: std::mem::take(&mut self.items),
        }
    }

    /// Return an "unexpected argument" error if there is any remaining
    /// argument.
    #[allow(clippy::missing_errors_doc /* false positive */)]
    pub fn finish(self) -> SourceResult<()> {
        if let Some(arg) = self.items.first() {
            bail!(arg.span, "unexpected argument");
        }
        Ok(())
    }

    /// Extract the positional arguments as an array.
    #[must_use]
    pub fn to_pos(&self) -> Array {
        self.items
            .iter()
            .filter(|item| item.name.is_none())
            .map(|item| item.value.v.clone())
            .collect()
    }

    /// Extract the named arguments as a dictionary.
    #[must_use]
    pub fn to_named(&self) -> Dict {
        self.items
            .iter()
            .filter_map(|item| item.name.clone().map(|name| (name, item.value.v.clone())))
            .collect()
    }
}

impl Debug for Args {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let pieces: Vec<_> =
            self.items.iter().map(|arg| eco_format!("{arg:?}")).collect();
        f.write_str(&pretty_array_like(&pieces, false))
    }
}

impl Debug for Arg {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(name) = &self.name {
            f.write_str(name)?;
            f.write_str(": ")?;
        }
        Debug::fmt(&self.value.v, f)
    }
}
