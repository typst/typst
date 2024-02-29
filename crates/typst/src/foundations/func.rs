use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use comemo::TrackedMut;
use ecow::{eco_format, EcoString};
use once_cell::sync::Lazy;

use crate::diag::{bail, SourceResult, StrResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, repr, scope, ty, Args, CastInfo, Content, Context, Element, IntoArgs, Scope,
    Selector, Type, Value,
};
use crate::syntax::{ast, Span, SyntaxNode};
use crate::util::{LazyHash, Static};

#[doc(inline)]
pub use typst_macros::func;

/// A mapping from argument values to a return value.
///
/// You can call a function by writing a comma-separated list of function
/// _arguments_ enclosed in parentheses directly after the function name.
/// Additionally, you can pass any number of trailing content blocks arguments
/// to a function _after_ the normal argument list. If the normal argument list
/// would become empty, it can be omitted. Typst supports positional and named
/// arguments. The former are identified by position and type, while the later
/// are written as `name: value`.
///
/// Within math mode, function calls have special behaviour. See the
/// [math documentation]($category/math) for more details.
///
/// # Example
/// ```example
/// // Call a function.
/// #list([A], [B])
///
/// // Named arguments and trailing
/// // content blocks.
/// #enum(start: 2)[A][B]
///
/// // Version without parentheses.
/// #list[A][B]
/// ```
///
/// Functions are a fundamental building block of Typst. Typst provides
/// functions for a variety of typesetting tasks. Moreover, the markup you write
/// is backed by functions and all styling happens through functions. This
/// reference lists all available functions and how you can use them. Please
/// also refer to the documentation about [set]($styling/#set-rules) and
/// [show]($styling/#show-rules) rules to learn about additional ways you can
/// work with functions in Typst.
///
/// # Element functions
/// Some functions are associated with _elements_ like [headings]($heading) or
/// [tables]($table). When called, these create an element of their respective
/// kind. In contrast to normal functions, they can further be used in [set
/// rules]($styling/#set-rules), [show rules]($styling/#show-rules), and
/// [selectors]($selector).
///
/// # Function scopes
/// Functions can hold related definitions in their own scope, similar to a
/// [module]($scripting/#modules). Examples of this are
/// [`assert.eq`]($assert.eq) or [`list.item`]($list.item). However, this
/// feature is currently only available for built-in functions.
///
/// # Defining functions
/// You can define your own function with a [let binding]($scripting/#bindings)
/// that has a parameter list after the binding's name. The parameter list can
/// contain mandatory positional parameters, named parameters with default
/// values and [argument sinks]($arguments).
///
/// The right-hand side of a function binding is the function body, which can be
/// a block or any other expression. It defines the function's return value and
/// can depend on the parameters. If the function body is a [code
/// block]($scripting/#blocks), the return value is the result of joining the
/// values of each expression in the block.
///
/// Within a function body, the `return` keyword can be used to exit early and
/// optionally specify a return value. If no explicit return value is given, the
/// body evaluates to the result of joining all expressions preceding the
/// `return`.
///
/// ```example
/// #let alert(body, fill: red) = {
///   set text(white)
///   set align(center)
///   rect(
///     fill: fill,
///     inset: 8pt,
///     radius: 4pt,
///     [*Warning:\ #body*],
///   )
/// }
///
/// #alert[
///   Danger is imminent!
/// ]
///
/// #alert(fill: blue)[
///   KEEP OFF TRACKS
/// ]
/// ```
///
/// # Unnamed functions { #unnamed }
/// You can also created an unnamed function without creating a binding by
/// specifying a parameter list followed by `=>` and the function body. If your
/// function has just one parameter, the parentheses around the parameter list
/// are optional. Unnamed functions are mainly useful for show rules, but also
/// for settable properties that take functions like the page function's
/// [`footer`]($page.footer) property.
///
/// ```example
/// #show "once?": it => [#it #it]
/// once?
/// ```
///
/// # Note on function purity
/// In Typst, all functions are _pure._ This means that for the same
/// arguments, they always return the same result. They cannot "remember" things to
/// produce another value when they are called a second time.
///
/// The only exception are built-in methods like
/// [`array.push(value)`]($array.push). These can modify the values they are
/// called on.
#[ty(scope, cast, name = "function")]
#[derive(Clone, Hash)]
#[allow(clippy::derived_hash_with_manual_eq)]
pub struct Func {
    /// The internal representation.
    repr: Repr,
    /// The span with which errors are reported when this function is called.
    span: Span,
}

/// The different kinds of function representations.
#[derive(Clone, PartialEq, Hash)]
enum Repr {
    /// A native Rust function.
    Native(Static<NativeFuncData>),
    /// A function for an element.
    Element(Element),
    /// A user-defined closure.
    Closure(Arc<LazyHash<Closure>>),
    /// A nested function with pre-applied arguments.
    With(Arc<(Func, Args)>),
}

impl Func {
    /// The function's name (e.g. `min`).
    ///
    /// Returns `None` if this is an anonymous closure.
    pub fn name(&self) -> Option<&str> {
        match &self.repr {
            Repr::Native(native) => Some(native.name),
            Repr::Element(elem) => Some(elem.name()),
            Repr::Closure(closure) => closure.name(),
            Repr::With(with) => with.0.name(),
        }
    }

    /// The function's title case name, for use in documentation (e.g. `Minimum`).
    ///
    /// Returns `None` if this is a closure.
    pub fn title(&self) -> Option<&'static str> {
        match &self.repr {
            Repr::Native(native) => Some(native.title),
            Repr::Element(elem) => Some(elem.title()),
            Repr::Closure(_) => None,
            Repr::With(with) => with.0.title(),
        }
    }

    /// Documentation for the function (as Markdown).
    pub fn docs(&self) -> Option<&'static str> {
        match &self.repr {
            Repr::Native(native) => Some(native.docs),
            Repr::Element(elem) => Some(elem.docs()),
            Repr::Closure(_) => None,
            Repr::With(with) => with.0.docs(),
        }
    }

    /// Whether the function is known to be contextual.
    pub fn contextual(&self) -> Option<bool> {
        match &self.repr {
            Repr::Native(native) => Some(native.contextual),
            _ => None,
        }
    }

    /// Get details about this function's parameters if available.
    pub fn params(&self) -> Option<&'static [ParamInfo]> {
        match &self.repr {
            Repr::Native(native) => Some(&native.0.params),
            Repr::Element(elem) => Some(elem.params()),
            Repr::Closure(_) => None,
            Repr::With(with) => with.0.params(),
        }
    }

    /// Get the parameter info for a parameter with the given name if it exist.
    pub fn param(&self, name: &str) -> Option<&'static ParamInfo> {
        self.params()?.iter().find(|param| param.name == name)
    }

    /// Get details about the function's return type.
    pub fn returns(&self) -> Option<&'static CastInfo> {
        static CONTENT: Lazy<CastInfo> =
            Lazy::new(|| CastInfo::Type(Type::of::<Content>()));
        match &self.repr {
            Repr::Native(native) => Some(&native.0.returns),
            Repr::Element(_) => Some(&CONTENT),
            Repr::Closure(_) => None,
            Repr::With(with) => with.0.returns(),
        }
    }

    /// Search keywords for the function.
    pub fn keywords(&self) -> &'static [&'static str] {
        match &self.repr {
            Repr::Native(native) => native.keywords,
            Repr::Element(elem) => elem.keywords(),
            Repr::Closure(_) => &[],
            Repr::With(with) => with.0.keywords(),
        }
    }

    /// The function's associated scope of sub-definition.
    pub fn scope(&self) -> Option<&'static Scope> {
        match &self.repr {
            Repr::Native(native) => Some(&native.0.scope),
            Repr::Element(elem) => Some(elem.scope()),
            Repr::Closure(_) => None,
            Repr::With(with) => with.0.scope(),
        }
    }

    /// Get a field from this function's scope, if possible.
    pub fn field(&self, field: &str) -> StrResult<&'static Value> {
        let scope =
            self.scope().ok_or("cannot access fields on user-defined functions")?;
        match scope.get(field) {
            Some(field) => Ok(field),
            None => match self.name() {
                Some(name) => bail!("function `{name}` does not contain field `{field}`"),
                None => bail!("function does not contain field `{field}`"),
            },
        }
    }

    /// Extract the element function, if it is one.
    pub fn element(&self) -> Option<Element> {
        match self.repr {
            Repr::Element(func) => Some(func),
            _ => None,
        }
    }

    /// Call the function with the given context and arguments.
    pub fn call<A: IntoArgs>(
        &self,
        engine: &mut Engine,
        context: &Context,
        args: A,
    ) -> SourceResult<Value> {
        self.call_impl(engine, context, args.into_args(self.span))
    }

    /// Non-generic implementation of `call`.
    #[typst_macros::time(name = "func call", span = self.span())]
    fn call_impl(
        &self,
        engine: &mut Engine,
        context: &Context,
        mut args: Args,
    ) -> SourceResult<Value> {
        match &self.repr {
            Repr::Native(native) => {
                let value = (native.function)(engine, context, &mut args)?;
                args.finish()?;
                Ok(value)
            }
            Repr::Element(func) => {
                let value = func.construct(engine, &mut args)?;
                args.finish()?;
                Ok(Value::Content(value))
            }
            Repr::Closure(closure) => crate::eval::call_closure(
                self,
                closure,
                engine.world,
                engine.introspector,
                engine.route.track(),
                engine.locator.track(),
                TrackedMut::reborrow_mut(&mut engine.tracer),
                context,
                args,
            ),
            Repr::With(with) => {
                args.items = with.1.items.iter().cloned().chain(args.items).collect();
                with.0.call(engine, context, args)
            }
        }
    }

    /// The function's span.
    pub fn span(&self) -> Span {
        self.span
    }

    /// Attach a span to this function if it doesn't already have one.
    pub fn spanned(mut self, span: Span) -> Self {
        if self.span.is_detached() {
            self.span = span;
        }
        self
    }
}

#[scope]
impl Func {
    /// Returns a new function that has the given arguments pre-applied.
    #[func]
    pub fn with(
        self,
        /// The real arguments (the other argument is just for the docs).
        /// The docs argument cannot be called `args`.
        args: &mut Args,
        /// The arguments to apply to the function.
        #[external]
        #[variadic]
        arguments: Vec<Value>,
    ) -> Func {
        let span = self.span;
        Self {
            repr: Repr::With(Arc::new((self, args.take()))),
            span,
        }
    }

    /// Returns a selector that filters for elements belonging to this function
    /// whose fields have the values of the given arguments.
    #[func]
    pub fn where_(
        self,
        /// The real arguments (the other argument is just for the docs).
        /// The docs argument cannot be called `args`.
        args: &mut Args,
        /// The fields to filter for.
        #[variadic]
        #[external]
        fields: Vec<Value>,
    ) -> StrResult<Selector> {
        let fields = args.to_named();
        args.items.retain(|arg| arg.name.is_none());

        let element = self
            .element()
            .ok_or("`where()` can only be called on element functions")?;

        let fields = fields
            .into_iter()
            .map(|(key, value)| {
                element.field_id(&key).map(|id| (id, value)).ok_or_else(|| {
                    eco_format!(
                        "element `{}` does not have field `{}`",
                        element.name(),
                        key
                    )
                })
            })
            .collect::<StrResult<smallvec::SmallVec<_>>>()?;

        Ok(element.where_(fields))
    }
}

impl Debug for Func {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Func({})", self.name().unwrap_or(".."))
    }
}

impl repr::Repr for Func {
    fn repr(&self) -> EcoString {
        match self.name() {
            Some(name) => name.into(),
            None => "(..) => ..".into(),
        }
    }
}

impl PartialEq for Func {
    fn eq(&self, other: &Self) -> bool {
        self.repr == other.repr
    }
}

impl PartialEq<&NativeFuncData> for Func {
    fn eq(&self, other: &&NativeFuncData) -> bool {
        match &self.repr {
            Repr::Native(native) => native.function == other.function,
            _ => false,
        }
    }
}

impl From<Repr> for Func {
    fn from(repr: Repr) -> Self {
        Self { repr, span: Span::detached() }
    }
}

impl From<Element> for Func {
    fn from(func: Element) -> Self {
        Repr::Element(func).into()
    }
}

/// A Typst function that is defined by a native Rust type that shadows a
/// native Rust function.
pub trait NativeFunc {
    /// Get the function for the native Rust type.
    fn func() -> Func {
        Func::from(Self::data())
    }

    /// Get the function data for the native Rust type.
    fn data() -> &'static NativeFuncData;
}

/// Defines a native function.
#[derive(Debug)]
pub struct NativeFuncData {
    pub function: fn(&mut Engine, &Context, &mut Args) -> SourceResult<Value>,
    pub name: &'static str,
    pub title: &'static str,
    pub docs: &'static str,
    pub keywords: &'static [&'static str],
    pub contextual: bool,
    pub scope: Lazy<Scope>,
    pub params: Lazy<Vec<ParamInfo>>,
    pub returns: Lazy<CastInfo>,
}

impl From<&'static NativeFuncData> for Func {
    fn from(data: &'static NativeFuncData) -> Self {
        Repr::Native(Static(data)).into()
    }
}

cast! {
    &'static NativeFuncData,
    self => Func::from(self).into_value(),
}

/// Describes a function parameter.
#[derive(Debug, Clone)]
pub struct ParamInfo {
    /// The parameter's name.
    pub name: &'static str,
    /// Documentation for the parameter.
    pub docs: &'static str,
    /// Describe what values this parameter accepts.
    pub input: CastInfo,
    /// Creates an instance of the parameter's default value.
    pub default: Option<fn() -> Value>,
    /// Is the parameter positional?
    pub positional: bool,
    /// Is the parameter named?
    ///
    /// Can be true even if `positional` is true if the parameter can be given
    /// in both variants.
    pub named: bool,
    /// Can the parameter be given any number of times?
    pub variadic: bool,
    /// Is the parameter required?
    pub required: bool,
    /// Is the parameter settable with a set rule?
    pub settable: bool,
}

/// A user-defined closure.
#[derive(Debug, Hash)]
pub struct Closure {
    /// The closure's syntax node. Must be either castable to `ast::Closure` or
    /// `ast::Expr`. In the latter case, this is a synthesized closure without
    /// any parameters (used by `context` expressions).
    pub node: SyntaxNode,
    /// Default values of named parameters.
    pub defaults: Vec<Value>,
    /// Captured values from outer scopes.
    pub captured: Scope,
    /// The number of positional parameters in the closure.
    pub num_pos_params: usize,
}

impl Closure {
    /// The name of the closure.
    pub fn name(&self) -> Option<&str> {
        self.node.cast::<ast::Closure>()?.name().map(|ident| ident.as_str())
    }
}

impl From<Closure> for Func {
    fn from(closure: Closure) -> Self {
        Repr::Closure(Arc::new(LazyHash::new(closure))).into()
    }
}

cast! {
    Closure,
    self => Value::Func(self.into()),
}
