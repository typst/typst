use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;
use std::rc::Rc;

use super::{Cast, EvalContext, Value};
use crate::diag::{At, TypResult};
use crate::syntax::{Span, Spanned};
use crate::util::EcoString;

/// An evaluatable function.
#[derive(Clone)]
pub struct Function(Rc<Repr<Func>>);

/// The unsized representation behind the [`Rc`].
struct Repr<T: ?Sized> {
    name: Option<EcoString>,
    func: T,
}

type Func = dyn Fn(&mut EvalContext, &mut FuncArgs) -> TypResult<Value>;

impl Function {
    /// Create a new function from a rust closure.
    pub fn new<F>(name: Option<EcoString>, func: F) -> Self
    where
        F: Fn(&mut EvalContext, &mut FuncArgs) -> TypResult<Value> + 'static,
    {
        Self(Rc::new(Repr { name, func }))
    }

    /// The name of the function.
    pub fn name(&self) -> Option<&EcoString> {
        self.0.name.as_ref()
    }
}

impl Deref for Function {
    type Target = Func;

    fn deref(&self) -> &Self::Target {
        &self.0.func
    }
}

impl Debug for Function {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("ValueFunc").field("name", &self.0.name).finish()
    }
}

impl PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        // We cast to thin pointers for comparison.
        Rc::as_ptr(&self.0) as *const () == Rc::as_ptr(&other.0) as *const ()
    }
}

/// Evaluated arguments to a function.
#[derive(Debug, Clone, PartialEq)]
pub struct FuncArgs {
    /// The span of the whole argument list.
    pub span: Span,
    /// The positional and named arguments.
    pub items: Vec<FuncArg>,
}

/// An argument to a function call: `12` or `draw: false`.
#[derive(Debug, Clone, PartialEq)]
pub struct FuncArg {
    /// The span of the whole argument.
    pub span: Span,
    /// The name of the argument (`None` for positional arguments).
    pub name: Option<EcoString>,
    /// The value of the argument.
    pub value: Spanned<Value>,
}

impl FuncArgs {
    /// Find and consume the first castable positional argument.
    pub fn eat<T>(&mut self) -> Option<T>
    where
        T: Cast<Spanned<Value>>,
    {
        for (i, slot) in self.items.iter().enumerate() {
            if slot.name.is_none() {
                if T::is(&slot.value) {
                    let value = self.items.remove(i).value;
                    return T::cast(value).ok();
                }
            }
        }
        None
    }

    /// Find and consume the first castable positional argument, returning a
    /// `missing argument: {what}` error if no match was found.
    pub fn expect<T>(&mut self, what: &str) -> TypResult<T>
    where
        T: Cast<Spanned<Value>>,
    {
        match self.eat() {
            Some(found) => Ok(found),
            None => bail!(self.span, "missing argument: {}", what),
        }
    }

    /// Find and consume all castable positional arguments.
    pub fn all<T>(&mut self) -> impl Iterator<Item = T> + '_
    where
        T: Cast<Spanned<Value>>,
    {
        std::iter::from_fn(move || self.eat())
    }

    /// Cast and remove the value for the given named argument, returning an
    /// error if the conversion fails.
    pub fn named<T>(&mut self, name: &str) -> TypResult<Option<T>>
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

    /// Take out all arguments into a new instance.
    pub fn take(&mut self) -> Self {
        Self {
            span: self.span,
            items: std::mem::take(&mut self.items),
        }
    }

    /// Return an "unexpected argument" error if there is any remaining
    /// argument.
    pub fn finish(self) -> TypResult<()> {
        if let Some(arg) = self.items.first() {
            bail!(arg.span, "unexpected argument");
        }
        Ok(())
    }
}

impl FuncArgs {
    /// Reinterpret these arguments as actually being an array index.
    pub fn into_index(self) -> TypResult<i64> {
        self.into_castable("index")
    }

    /// Reinterpret these arguments as actually being a dictionary key.
    pub fn into_key(self) -> TypResult<EcoString> {
        self.into_castable("key")
    }

    /// Reinterpret these arguments as actually being a single castable thing.
    fn into_castable<T>(self, what: &str) -> TypResult<T>
    where
        T: Cast<Value>,
    {
        let mut iter = self.items.into_iter();
        let value = match iter.next() {
            Some(FuncArg { name: None, value, .. }) => value.v.cast().at(value.span)?,
            None => {
                bail!(self.span, "missing {}", what);
            }
            Some(FuncArg { name: Some(_), span, .. }) => {
                bail!(span, "named pair is not allowed here");
            }
        };

        if let Some(arg) = iter.next() {
            bail!(arg.span, "only one {} is allowed", what);
        }

        Ok(value)
    }
}
