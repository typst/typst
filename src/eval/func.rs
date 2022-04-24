use std::fmt::{self, Debug, Formatter, Write};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use super::{Args, Control, Eval, Scope, Scopes, Value};
use crate::diag::{StrResult, TypResult};
use crate::model::{Content, StyleMap};
use crate::syntax::ast::Expr;
use crate::syntax::Span;
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
    pub fn from_fn(
        name: &'static str,
        func: fn(&mut Context, &mut Args) -> TypResult<Value>,
    ) -> Self {
        Self(Arc::new(Repr::Native(Native {
            name,
            func,
            set: None,
            show: None,
        })))
    }

    /// Create a new function from a native rust node.
    pub fn from_node<T: Node>(name: &'static str) -> Self {
        Self(Arc::new(Repr::Native(Native {
            name,
            func: |ctx, args| {
                let styles = T::set(args)?;
                let content = T::construct(ctx, args)?;
                Ok(Value::Content(content.styled_with_map(styles.scoped())))
            },
            set: Some(T::set),
            show: if T::SHOWABLE {
                Some(|recipe, span| {
                    let mut styles = StyleMap::new();
                    styles.set_recipe::<T>(recipe, span);
                    styles
                })
            } else {
                None
            },
        })))
    }

    /// Create a new function from a closure.
    pub fn from_closure(closure: Closure) -> Self {
        Self(Arc::new(Repr::Closure(closure)))
    }

    /// Apply the given arguments to the function.
    pub fn with(self, args: Args) -> Self {
        Self(Arc::new(Repr::With(self, args)))
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

    /// Execute the function's set rule.
    pub fn set(&self, mut args: Args) -> TypResult<StyleMap> {
        let styles = match self.0.as_ref() {
            Repr::Native(Native { set: Some(set), .. }) => set(&mut args)?,
            _ => StyleMap::new(),
        };
        args.finish()?;
        Ok(styles)
    }

    /// Execute the function's show rule.
    pub fn show(&self, recipe: Func, span: Span) -> StrResult<StyleMap> {
        match self.0.as_ref() {
            Repr::Native(Native { show: Some(show), .. }) => Ok(show(recipe, span)),
            _ => Err("this function cannot be customized with show")?,
        }
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
    /// The set rule.
    pub set: Option<fn(&mut Args) -> TypResult<StyleMap>>,
    /// The show rule.
    pub show: Option<fn(Func, Span) -> StyleMap>,
}

impl Hash for Native {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        (self.func as usize).hash(state);
        self.set.map(|set| set as usize).hash(state);
        self.show.map(|show| show as usize).hash(state);
    }
}

/// A constructable, stylable content node.
pub trait Node: 'static {
    /// Whether this node can be customized through a show rule.
    const SHOWABLE: bool;

    /// Construct a node from the arguments.
    ///
    /// This is passed only the arguments that remain after execution of the
    /// node's set rule.
    fn construct(ctx: &mut Context, args: &mut Args) -> TypResult<Content>;

    /// Parse the arguments into style properties for this node.
    fn set(args: &mut Args) -> TypResult<StyleMap>;
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
            Err(Control::Return(value, _, _)) => value,
            other => other?,
        };

        Ok(value)
    }
}
