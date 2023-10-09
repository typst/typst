use std::fmt::Debug;
use std::sync::Arc;

use comemo::{Prehashed, Tracked, TrackedMut};
use ecow::EcoString;
use once_cell::sync::Lazy;

use super::{
    cast, scope, ty, Args, CastInfo, Eval, FlowEvent, IntoValue, Route, Scope, Scopes,
    Tracer, Type, Value, Vm,
};
use crate::diag::{bail, SourceResult, StrResult};
use crate::model::{
    Content, DelayedErrors, Element, Introspector, Locator, Selector, Vt,
};
use crate::syntax::ast::{self, AstNode};
use crate::syntax::{FileId, Span, SyntaxNode};
use crate::util::Static;
use crate::World;

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
/// contain positional parameters, named parameters with default values and
/// [argument sinks]($arguments). The right-hand side of the binding can be a
/// block or any other expression. It defines the function's return value and
/// can depend on the parameters.
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
/// # Notable fact
/// In Typst, all functions are _pure._ This means that for the same
/// arguments, they always return the same result. They cannot "remember" things to
/// produce another value when they are called a second time.
///
/// The only exception are built-in methods like
/// [`array.push(value)`]($array.push). These can modify the values they are
/// called on.
#[ty(scope, name = "function")]
#[derive(Debug, Clone, Hash)]
#[allow(clippy::derived_hash_with_manual_eq)]
pub struct Func {
    /// The internal representation.
    repr: Repr,
    /// The span with which errors are reported when this function is called.
    span: Span,
}

/// The different kinds of function representations.
#[derive(Debug, Clone, PartialEq, Hash)]
enum Repr {
    /// A native Rust function.
    Native(Static<NativeFuncData>),
    /// A function for an element.
    Element(Element),
    /// A user-defined closure.
    Closure(Arc<Prehashed<Closure>>),
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

    /// Call the function with the given arguments.
    pub fn call_vm(&self, vm: &mut Vm, mut args: Args) -> SourceResult<Value> {
        let _span = tracing::info_span!(
            "call",
            name = self.name().unwrap_or("<anon>"),
            file = 0,
        );

        match &self.repr {
            Repr::Native(native) => {
                let value = (native.function)(vm, &mut args)?;
                args.finish()?;
                Ok(value)
            }
            Repr::Element(func) => {
                let value = func.construct(vm, &mut args)?;
                args.finish()?;
                Ok(Value::Content(value))
            }
            Repr::Closure(closure) => {
                // Determine the route inside the closure.
                let fresh = Route::new(closure.file);
                let route = if vm.file.is_none() { fresh.track() } else { vm.route };

                Closure::call(
                    self,
                    vm.world(),
                    route,
                    vm.vt.introspector,
                    vm.vt.locator.track(),
                    TrackedMut::reborrow_mut(&mut vm.vt.delayed),
                    TrackedMut::reborrow_mut(&mut vm.vt.tracer),
                    vm.depth + 1,
                    args,
                )
            }
            Repr::With(with) => {
                args.items = with.1.items.iter().cloned().chain(args.items).collect();
                with.0.call_vm(vm, args)
            }
        }
    }

    /// Call the function with a Vt.
    #[tracing::instrument(skip_all)]
    pub fn call_vt<T: IntoValue>(
        &self,
        vt: &mut Vt,
        args: impl IntoIterator<Item = T>,
    ) -> SourceResult<Value> {
        let route = Route::default();
        let scopes = Scopes::new(None);
        let mut locator = Locator::chained(vt.locator.track());
        let vt = Vt {
            world: vt.world,
            introspector: vt.introspector,
            locator: &mut locator,
            delayed: TrackedMut::reborrow_mut(&mut vt.delayed),
            tracer: TrackedMut::reborrow_mut(&mut vt.tracer),
        };
        let mut vm = Vm::new(vt, route.track(), None, scopes);
        let args = Args::new(self.span(), args);
        self.call_vm(&mut vm, args)
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
        args: Args,
        /// The arguments to apply to the function.
        #[external]
        #[variadic]
        arguments: Vec<Args>,
    ) -> Func {
        let span = self.span;
        Self { repr: Repr::With(Arc::new((self, args))), span }
    }

    /// Returns a selector that filters for elements belonging to this function
    /// whose fields have the values of the given arguments.
    #[func]
    pub fn where_(
        self,
        /// The real arguments (the other argument is just for the docs).
        /// The docs argument cannot be called `args`.
        args: Args,
        /// The fields to filter for.
        #[variadic]
        #[external]
        fields: Vec<Args>,
    ) -> StrResult<Selector> {
        let mut args = args;
        let fields = args.to_named();
        args.items.retain(|arg| arg.name.is_none());
        Ok(self
            .element()
            .ok_or("`where()` can only be called on element functions")?
            .where_(fields))
    }
}

impl super::Repr for Func {
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
    pub function: fn(&mut Vm, &mut Args) -> SourceResult<Value>,
    pub name: &'static str,
    pub title: &'static str,
    pub docs: &'static str,
    pub keywords: &'static [&'static str],
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
pub(super) struct Closure {
    /// The closure's syntax node. Must be castable to `ast::Closure`.
    pub node: SyntaxNode,
    /// The source file where the closure was defined.
    pub file: Option<FileId>,
    /// Default values of named parameters.
    pub defaults: Vec<Value>,
    /// Captured values from outer scopes.
    pub captured: Scope,
}

impl Closure {
    /// The name of the closure.
    pub fn name(&self) -> Option<&str> {
        self.node
            .cast::<ast::Closure>()
            .unwrap()
            .name()
            .map(|ident| ident.as_str())
    }

    /// Call the function in the context with the arguments.
    #[comemo::memoize]
    #[tracing::instrument(skip_all)]
    #[allow(clippy::too_many_arguments)]
    fn call(
        func: &Func,
        world: Tracked<dyn World + '_>,
        route: Tracked<Route>,
        introspector: Tracked<Introspector>,
        locator: Tracked<Locator>,
        delayed: TrackedMut<DelayedErrors>,
        tracer: TrackedMut<Tracer>,
        depth: usize,
        mut args: Args,
    ) -> SourceResult<Value> {
        let Repr::Closure(this) = &func.repr else {
            panic!("`this` must be a closure");
        };
        let closure = this.node.cast::<ast::Closure>().unwrap();

        // Don't leak the scopes from the call site. Instead, we use the scope
        // of captured variables we collected earlier.
        let mut scopes = Scopes::new(None);
        scopes.top = this.captured.clone();

        // Prepare VT.
        let mut locator = Locator::chained(locator);
        let vt = Vt {
            world,
            introspector,
            locator: &mut locator,
            delayed,
            tracer,
        };

        // Prepare VM.
        let mut vm = Vm::new(vt, route, this.file, scopes);
        vm.depth = depth;

        // Provide the closure itself for recursive calls.
        if let Some(name) = closure.name() {
            vm.define(name, Value::Func(func.clone()));
        }

        // Parse the arguments according to the parameter list.
        let num_pos_params = closure
            .params()
            .children()
            .filter(|p| matches!(p, ast::Param::Pos(_)))
            .count();
        let num_pos_args = args.to_pos().len();
        let sink_size = num_pos_args.checked_sub(num_pos_params);

        let mut sink = None;
        let mut sink_pos_values = None;
        let mut defaults = this.defaults.iter();
        for p in closure.params().children() {
            match p {
                ast::Param::Pos(pattern) => match pattern {
                    ast::Pattern::Normal(ast::Expr::Ident(ident)) => {
                        vm.define(ident, args.expect::<Value>(&ident)?)
                    }
                    ast::Pattern::Normal(_) => unreachable!(),
                    pattern => {
                        super::define_pattern(
                            &mut vm,
                            pattern,
                            args.expect::<Value>("pattern parameter")?,
                        )?;
                    }
                },
                ast::Param::Sink(ident) => {
                    sink = ident.name();
                    if let Some(sink_size) = sink_size {
                        sink_pos_values = Some(args.consume(sink_size)?);
                    }
                }
                ast::Param::Named(named) => {
                    let name = named.name();
                    let default = defaults.next().unwrap();
                    let value =
                        args.named::<Value>(&name)?.unwrap_or_else(|| default.clone());
                    vm.define(name, value);
                }
            }
        }

        if let Some(sink) = sink {
            let mut remaining_args = args.take();
            if let Some(sink_pos_values) = sink_pos_values {
                remaining_args.items.extend(sink_pos_values);
            }
            vm.define(sink, remaining_args);
        }

        // Ensure all arguments have been used.
        args.finish()?;

        // Handle control flow.
        let output = closure.body().eval(&mut vm)?;
        match vm.flow {
            Some(FlowEvent::Return(_, Some(explicit))) => return Ok(explicit),
            Some(FlowEvent::Return(_, None)) => {}
            Some(flow) => bail!(flow.forbidden()),
            None => {}
        }

        Ok(output)
    }
}

impl From<Closure> for Func {
    fn from(closure: Closure) -> Self {
        Repr::Closure(Arc::new(Prehashed::new(closure))).into()
    }
}

cast! {
    Closure,
    self => Value::Func(self.into()),
}

/// A visitor that determines which variables to capture for a closure.
pub struct CapturesVisitor<'a> {
    external: Option<&'a Scopes<'a>>,
    internal: Scopes<'a>,
    captures: Scope,
}

impl<'a> CapturesVisitor<'a> {
    /// Create a new visitor for the given external scopes.
    pub fn new(external: Option<&'a Scopes<'a>>) -> Self {
        Self {
            external,
            internal: Scopes::new(None),
            captures: Scope::new(),
        }
    }

    /// Return the scope of captured variables.
    pub fn finish(self) -> Scope {
        self.captures
    }

    /// Visit any node and collect all captured variables.
    #[tracing::instrument(skip_all)]
    pub fn visit(&mut self, node: &SyntaxNode) {
        match node.cast() {
            // Every identifier is a potential variable that we need to capture.
            // Identifiers that shouldn't count as captures because they
            // actually bind a new name are handled below (individually through
            // the expressions that contain them).
            Some(ast::Expr::Ident(ident)) => self.capture(&ident, Scopes::get),
            Some(ast::Expr::MathIdent(ident)) => {
                self.capture(&ident, Scopes::get_in_math)
            }

            // Code and content blocks create a scope.
            Some(ast::Expr::Code(_) | ast::Expr::Content(_)) => {
                self.internal.enter();
                for child in node.children() {
                    self.visit(child);
                }
                self.internal.exit();
            }

            // Don't capture the field of a field access.
            Some(ast::Expr::FieldAccess(access)) => {
                self.visit(access.target().to_untyped());
            }

            // A closure contains parameter bindings, which are bound before the
            // body is evaluated. Care must be taken so that the default values
            // of named parameters cannot access previous parameter bindings.
            Some(ast::Expr::Closure(expr)) => {
                for param in expr.params().children() {
                    if let ast::Param::Named(named) = param {
                        self.visit(named.expr().to_untyped());
                    }
                }

                self.internal.enter();
                if let Some(name) = expr.name() {
                    self.bind(name);
                }

                for param in expr.params().children() {
                    match param {
                        ast::Param::Pos(pattern) => {
                            for ident in pattern.idents() {
                                self.bind(ident);
                            }
                        }
                        ast::Param::Named(named) => self.bind(named.name()),
                        ast::Param::Sink(spread) => {
                            self.bind(spread.name().unwrap_or_default())
                        }
                    }
                }

                self.visit(expr.body().to_untyped());
                self.internal.exit();
            }

            // A let expression contains a binding, but that binding is only
            // active after the body is evaluated.
            Some(ast::Expr::Let(expr)) => {
                if let Some(init) = expr.init() {
                    self.visit(init.to_untyped());
                }

                for ident in expr.kind().idents() {
                    self.bind(ident);
                }
            }

            // A for loop contains one or two bindings in its pattern. These are
            // active after the iterable is evaluated but before the body is
            // evaluated.
            Some(ast::Expr::For(expr)) => {
                self.visit(expr.iter().to_untyped());
                self.internal.enter();

                let pattern = expr.pattern();
                for ident in pattern.idents() {
                    self.bind(ident);
                }

                self.visit(expr.body().to_untyped());
                self.internal.exit();
            }

            // An import contains items, but these are active only after the
            // path is evaluated.
            Some(ast::Expr::Import(expr)) => {
                self.visit(expr.source().to_untyped());
                if let Some(ast::Imports::Items(items)) = expr.imports() {
                    for item in items.iter() {
                        self.bind(item.bound_name());
                    }
                }
            }

            _ => {
                // Never capture the name part of a named pair.
                if let Some(named) = node.cast::<ast::Named>() {
                    self.visit(named.expr().to_untyped());
                    return;
                }

                // Everything else is traversed from left to right.
                for child in node.children() {
                    self.visit(child);
                }
            }
        }
    }

    /// Bind a new internal variable.
    fn bind(&mut self, ident: ast::Ident) {
        self.internal.top.define(ident.get().clone(), Value::None);
    }

    /// Capture a variable if it isn't internal.
    #[inline]
    fn capture(
        &mut self,
        ident: &str,
        getter: impl FnOnce(&'a Scopes<'a>, &str) -> StrResult<&'a Value>,
    ) {
        if self.internal.get(ident).is_err() {
            let Some(value) = self
                .external
                .map(|external| getter(external, ident).ok())
                .unwrap_or(Some(&Value::None))
            else {
                return;
            };

            self.captures.define_captured(ident, value.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::parse;

    #[track_caller]
    fn test(text: &str, result: &[&str]) {
        let mut scopes = Scopes::new(None);
        scopes.top.define("f", 0);
        scopes.top.define("x", 0);
        scopes.top.define("y", 0);
        scopes.top.define("z", 0);

        let mut visitor = CapturesVisitor::new(Some(&scopes));
        let root = parse(text);
        visitor.visit(&root);

        let captures = visitor.finish();
        let mut names: Vec<_> = captures.iter().map(|(k, _)| k).collect();
        names.sort();

        assert_eq!(names, result);
    }

    #[test]
    fn test_captures() {
        // Let binding and function definition.
        test("#let x = x", &["x"]);
        test("#let x; #(x + y)", &["y"]);
        test("#let f(x, y) = x + y", &[]);
        test("#let f(x, y) = f", &[]);
        test("#let f = (x, y) => f", &["f"]);

        // Closure with different kinds of params.
        test("#((x, y) => x + z)", &["z"]);
        test("#((x: y, z) => x + z)", &["y"]);
        test("#((..x) => x + y)", &["y"]);
        test("#((x, y: x + z) => x + y)", &["x", "z"]);
        test("#{x => x; x}", &["x"]);

        // Show rule.
        test("#show y: x => x", &["y"]);
        test("#show y: x => x + z", &["y", "z"]);
        test("#show x: x => x", &["x"]);

        // For loop.
        test("#for x in y { x + z }", &["y", "z"]);
        test("#for (x, y) in y { x + y }", &["y"]);
        test("#for x in y {} #x", &["x", "y"]);

        // Import.
        test("#import z: x, y", &["z"]);
        test("#import x + y: x, y, z", &["x", "y"]);

        // Blocks.
        test("#{ let x = 1; { let y = 2; y }; x + y }", &["y"]);
        test("#[#let x = 1]#x", &["x"]);

        // Field access.
        test("#foo(body: 1)", &[]);
        test("#(body: 1)", &[]);
        test("#(body = 1)", &[]);
        test("#(body += y)", &["y"]);
        test("#{ (body, a) = (y, 1) }", &["y"]);
        test("#(x.at(y) = 5)", &["x", "y"])
    }
}
