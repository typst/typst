use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;
use std::rc::Rc;

use super::{Cast, EvalContext, Value};
use crate::diag::{Error, TypResult};
use crate::loading::FileId;
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
    /// The file in which the function was called.
    pub file: FileId,
    /// The span of the whole argument list.
    pub span: Span,
    /// The positional arguments.
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
            None => bail!(self.file, self.span, "missing argument: {}", what),
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
        let index = match self
            .items
            .iter()
            .filter_map(|arg| arg.name.as_deref())
            .position(|other| name == other)
        {
            Some(index) => index,
            None => return Ok(None),
        };

        let value = self.items.remove(index).value;
        let span = value.span;

        T::cast(value).map(Some).map_err(Error::partial(self.file, span))
    }

    /// Return an "unexpected argument" error if there is any remaining
    /// argument.
    pub fn finish(self) -> TypResult<()> {
        if let Some(arg) = self.items.first() {
            bail!(self.file, arg.span, "unexpected argument");
        }
        Ok(())
    }
}
