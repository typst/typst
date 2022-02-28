use std::fmt::{self, Debug, Formatter, Write};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use super::{Cast, Control, Eval, Scope, Scopes, Value};
use crate::diag::{At, TypResult};
use crate::syntax::ast::Expr;
use crate::syntax::{Span, Spanned};
use crate::util::EcoString;
use crate::Context;

/// An evaluatable function.
#[derive(Clone, Hash)]
pub struct Func(Arc<Repr>);

/// The different kinds of function representations.
#[derive(Hash)]
enum Repr {
    /// A native rust function.
    Native(Native),
    /// A user-defined closure.
    Closure(Closure),
    /// A nested function with pre-applied arguments.
    With(Func, Args),
}

impl Func {
    /// Create a new function from a native rust function.
    pub fn native(
        name: &'static str,
        func: fn(&mut Context, &mut Args) -> TypResult<Value>,
    ) -> Self {
        Self(Arc::new(Repr::Native(Native { name, func })))
    }

    /// Create a new function from a closure.
    pub fn closure(closure: Closure) -> Self {
        Self(Arc::new(Repr::Closure(closure)))
    }

    /// The name of the function.
    pub fn name(&self) -> Option<&str> {
        match self.0.as_ref() {
            Repr::Native(native) => Some(native.name),
            Repr::Closure(closure) => closure.name.as_deref(),
            Repr::With(func, _) => func.name(),
        }
    }

    /// Call the function with a virtual machine and arguments.
    pub fn call(&self, ctx: &mut Context, mut args: Args) -> TypResult<Value> {
        let value = match self.0.as_ref() {
            Repr::Native(native) => (native.func)(ctx, &mut args)?,
            Repr::Closure(closure) => closure.call(ctx, &mut args)?,
            Repr::With(wrapped, applied) => {
                args.items.splice(.. 0, applied.items.iter().cloned());
                return wrapped.call(ctx, args);
            }
        };
        args.finish()?;
        Ok(value)
    }

    /// Apply the given arguments to the function.
    pub fn with(self, args: Args) -> Self {
        Self(Arc::new(Repr::With(self, args)))
    }
}

impl Debug for Func {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("<function")?;
        if let Some(name) = self.name() {
            f.write_char(' ')?;
            f.write_str(name)?;
        }
        f.write_char('>')
    }
}

impl PartialEq for Func {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// A native rust function.
struct Native {
    /// The name of the function.
    pub name: &'static str,
    /// The function pointer.
    pub func: fn(&mut Context, &mut Args) -> TypResult<Value>,
}

impl Hash for Native {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.func as usize).hash(state);
    }
}

/// A user-defined closure.
#[derive(Hash)]
pub struct Closure {
    /// The name of the closure.
    pub name: Option<EcoString>,
    /// Captured values from outer scopes.
    pub captured: Scope,
    /// The parameter names and default values. Parameters with default value
    /// are named parameters.
    pub params: Vec<(EcoString, Option<Value>)>,
    /// The name of an argument sink where remaining arguments are placed.
    pub sink: Option<EcoString>,
    /// The expression the closure should evaluate to.
    pub body: Expr,
}

impl Closure {
    /// Call the function in the context with the arguments.
    pub fn call(&self, ctx: &mut Context, args: &mut Args) -> TypResult<Value> {
        // Don't leak the scopes from the call site. Instead, we use the
        // scope of captured variables we collected earlier.
        let mut scp = Scopes::new(None);
        scp.top = self.captured.clone();

        // Parse the arguments according to the parameter list.
        for (param, default) in &self.params {
            scp.top.def_mut(param, match default {
                None => args.expect::<Value>(param)?,
                Some(default) => {
                    args.named::<Value>(param)?.unwrap_or_else(|| default.clone())
                }
            });
        }

        // Put the remaining arguments into the sink.
        if let Some(sink) = &self.sink {
            scp.top.def_mut(sink, args.take());
        }

        // Evaluate the body.
        let value = match self.body.eval(ctx, &mut scp) {
            Err(Control::Return(value, _)) => value.unwrap_or_default(),
            other => other?,
        };

        Ok(value)
    }
}

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
    pub name: Option<EcoString>,
    /// The value of the argument.
    pub value: Spanned<Value>,
}

impl Args {
    /// Create positional arguments from a span and values.
    pub fn from_values(span: Span, values: impl IntoIterator<Item = Value>) -> Self {
        Self {
            span,
            items: values
                .into_iter()
                .map(|value| Arg {
                    span,
                    name: None,
                    value: Spanned::new(value, span),
                })
                .collect(),
        }
    }

    /// Consume and cast the first positional argument.
    ///
    /// Returns a `missing argument: {what}` error if no positional argument is
    /// left.
    pub fn expect<T>(&mut self, what: &str) -> TypResult<T>
    where
        T: Cast<Spanned<Value>>,
    {
        match self.eat()? {
            Some(v) => Ok(v),
            None => bail!(self.span, "missing argument: {}", what),
        }
    }

    /// Consume and cast the first positional argument if there is one.
    pub fn eat<T>(&mut self) -> TypResult<Option<T>>
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

    /// Find and consume the first castable positional argument.
    pub fn find<T>(&mut self) -> TypResult<Option<T>>
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
    pub fn all<T>(&mut self) -> TypResult<Vec<T>>
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

    /// Same as named, but with fallback to find.
    pub fn named_or_find<T>(&mut self, name: &str) -> TypResult<Option<T>>
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
    pub fn finish(self) -> TypResult<()> {
        if let Some(arg) = self.items.first() {
            bail!(arg.span, "unexpected argument");
        }
        Ok(())
    }

    /// Reinterpret these arguments as actually being an array index.
    pub fn into_index(self) -> TypResult<i64> {
        self.into_castable("index")
    }

    /// Reinterpret these arguments as actually being a dictionary key.
    pub fn into_key(self) -> TypResult<EcoString> {
        self.into_castable("key")
    }

    /// Reinterpret these arguments as actually being a single castable thing.
    fn into_castable<T: Cast>(self, what: &str) -> TypResult<T> {
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
