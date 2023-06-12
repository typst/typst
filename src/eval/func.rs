use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use comemo::{Prehashed, Tracked, TrackedMut};
use ecow::eco_format;
use once_cell::sync::Lazy;

use super::{
    cast, Args, CastInfo, Eval, FlowEvent, IntoValue, Route, Scope, Scopes, Tracer,
    Value, Vm,
};
use crate::diag::{bail, SourceResult, StrResult};
use crate::model::{DelayedErrors, ElemFunc, Introspector, Locator, Vt};
use crate::syntax::ast::{self, AstNode, Expr, Ident};
use crate::syntax::{SourceId, Span, SyntaxNode};
use crate::World;

/// An evaluatable function.
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
    Native(&'static NativeFunc),
    /// A function for an element.
    Elem(ElemFunc),
    /// A user-defined closure.
    Closure(Arc<Prehashed<Closure>>),
    /// A nested function with pre-applied arguments.
    With(Arc<(Func, Args)>),
}

impl Func {
    /// The name of the function.
    pub fn name(&self) -> Option<&str> {
        match &self.repr {
            Repr::Native(native) => Some(native.info.name),
            Repr::Elem(func) => Some(func.info().name),
            Repr::Closure(closure) => closure.name.as_deref(),
            Repr::With(arc) => arc.0.name(),
        }
    }

    /// Extract details the function.
    pub fn info(&self) -> Option<&FuncInfo> {
        match &self.repr {
            Repr::Native(native) => Some(&native.info),
            Repr::Elem(func) => Some(func.info()),
            Repr::Closure(_) => None,
            Repr::With(arc) => arc.0.info(),
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

    /// Call the function with the given arguments.
    pub fn call_vm(&self, vm: &mut Vm, mut args: Args) -> SourceResult<Value> {
        let _span = tracing::info_span!(
            "call",
            name = self.name().unwrap_or("<anon>"),
            file = 0,
        );

        match &self.repr {
            Repr::Native(native) => {
                let value = (native.func)(vm, &mut args)?;
                args.finish()?;
                Ok(value)
            }
            Repr::Elem(func) => {
                let value = func.construct(vm, &mut args)?;
                args.finish()?;
                Ok(Value::Content(value))
            }
            Repr::Closure(closure) => {
                // Determine the route inside the closure.
                let fresh = Route::new(closure.location);
                let route =
                    if vm.location.is_detached() { fresh.track() } else { vm.route };

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
            Repr::With(arc) => {
                args.items = arc.1.items.iter().cloned().chain(args.items).collect();
                arc.0.call_vm(vm, args)
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
        let id = SourceId::detached();
        let scopes = Scopes::new(None);
        let mut locator = Locator::chained(vt.locator.track());
        let vt = Vt {
            world: vt.world,
            introspector: vt.introspector,
            locator: &mut locator,
            delayed: TrackedMut::reborrow_mut(&mut vt.delayed),
            tracer: TrackedMut::reborrow_mut(&mut vt.tracer),
        };
        let mut vm = Vm::new(vt, route.track(), id, scopes);
        let args = Args::new(self.span(), args);
        self.call_vm(&mut vm, args)
    }

    /// Apply the given arguments to the function.
    pub fn with(self, args: Args) -> Self {
        let span = self.span;
        Self { repr: Repr::With(Arc::new((self, args))), span }
    }

    /// Extract the element function, if it is one.
    pub fn element(&self) -> Option<ElemFunc> {
        match self.repr {
            Repr::Elem(func) => Some(func),
            _ => None,
        }
    }

    /// Get a field from this function's scope, if possible.
    pub fn get(&self, field: &str) -> StrResult<&Value> {
        match &self.repr {
            Repr::Native(func) => func.info.scope.get(field).ok_or_else(|| {
                eco_format!(
                    "function `{}` does not contain field `{}`",
                    func.info.name,
                    field
                )
            }),
            Repr::Elem(func) => func.info().scope.get(field).ok_or_else(|| {
                eco_format!(
                    "function `{}` does not contain field `{}`",
                    func.name(),
                    field
                )
            }),
            Repr::Closure(_) => {
                Err(eco_format!("cannot access fields on user-defined functions"))
            }
            Repr::With(arc) => arc.0.get(field),
        }
    }
}

impl Debug for Func {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self.name() {
            Some(name) => write!(f, "{name}"),
            None => f.write_str("(..) => .."),
        }
    }
}

impl PartialEq for Func {
    fn eq(&self, other: &Self) -> bool {
        self.repr == other.repr
    }
}

impl From<Repr> for Func {
    fn from(repr: Repr) -> Self {
        Self { repr, span: Span::detached() }
    }
}

impl From<ElemFunc> for Func {
    fn from(func: ElemFunc) -> Self {
        Repr::Elem(func).into()
    }
}

/// A Typst function defined by a native Rust function.
pub struct NativeFunc {
    /// The function's implementation.
    pub func: fn(&mut Vm, &mut Args) -> SourceResult<Value>,
    /// Details about the function.
    pub info: Lazy<FuncInfo>,
}

impl PartialEq for NativeFunc {
    fn eq(&self, other: &Self) -> bool {
        self.func as usize == other.func as usize
    }
}

impl Eq for NativeFunc {}

impl Hash for NativeFunc {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.func as usize).hash(state);
    }
}

impl From<&'static NativeFunc> for Func {
    fn from(native: &'static NativeFunc) -> Self {
        Repr::Native(native).into()
    }
}

cast! {
    &'static NativeFunc,
    self => Value::Func(self.into()),
}

/// Details about a function.
#[derive(Debug, Clone)]
pub struct FuncInfo {
    /// The function's name.
    pub name: &'static str,
    /// The display name of the function.
    pub display: &'static str,
    /// A string of search keywords.
    pub keywords: Option<&'static str>,
    /// Which category the function is part of.
    pub category: &'static str,
    /// Documentation for the function.
    pub docs: &'static str,
    /// Details about the function's parameters.
    pub params: Vec<ParamInfo>,
    /// Valid values for the return value.
    pub returns: CastInfo,
    /// The function's own scope of fields and sub-functions.
    pub scope: Scope,
}

impl FuncInfo {
    /// Get the parameter info for a parameter with the given name
    pub fn param(&self, name: &str) -> Option<&ParamInfo> {
        self.params.iter().find(|param| param.name == name)
    }
}

/// Describes a named parameter.
#[derive(Debug, Clone)]
pub struct ParamInfo {
    /// The parameter's name.
    pub name: &'static str,
    /// Documentation for the parameter.
    pub docs: &'static str,
    /// Valid values for the parameter.
    pub cast: CastInfo,
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
#[derive(Hash)]
pub(super) struct Closure {
    /// The source file where the closure was defined.
    pub location: SourceId,
    /// The name of the closure.
    pub name: Option<Ident>,
    /// Captured values from outer scopes.
    pub captured: Scope,
    /// The list of parameters.
    pub params: Vec<Param>,
    /// The expression the closure should evaluate to.
    pub body: Expr,
}

/// A closure parameter.
#[derive(Hash)]
pub enum Param {
    /// A positional parameter: `x`.
    Pos(ast::Pattern),
    /// A named parameter with a default value: `draw: false`.
    Named(Ident, Value),
    /// An argument sink: `..args`.
    Sink(Option<Ident>),
}

impl Closure {
    /// Call the function in the context with the arguments.
    #[comemo::memoize]
    #[tracing::instrument(skip_all)]
    #[allow(clippy::too_many_arguments)]
    fn call(
        this: &Func,
        world: Tracked<dyn World + '_>,
        route: Tracked<Route>,
        introspector: Tracked<Introspector>,
        locator: Tracked<Locator>,
        delayed: TrackedMut<DelayedErrors>,
        tracer: TrackedMut<Tracer>,
        depth: usize,
        mut args: Args,
    ) -> SourceResult<Value> {
        let closure = match &this.repr {
            Repr::Closure(closure) => closure,
            _ => panic!("`this` must be a closure"),
        };

        // Don't leak the scopes from the call site. Instead, we use the scope
        // of captured variables we collected earlier.
        let mut scopes = Scopes::new(None);
        scopes.top = closure.captured.clone();

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
        let mut vm = Vm::new(vt, route, closure.location, scopes);
        vm.depth = depth;

        // Provide the closure itself for recursive calls.
        if let Some(name) = &closure.name {
            vm.define(name.clone(), Value::Func(this.clone()));
        }

        // Parse the arguments according to the parameter list.
        let num_pos_params =
            closure.params.iter().filter(|p| matches!(p, Param::Pos(_))).count();
        let num_pos_args = args.to_pos().len();
        let sink_size = num_pos_args.checked_sub(num_pos_params);

        let mut sink = None;
        let mut sink_pos_values = None;
        for p in &closure.params {
            match p {
                Param::Pos(pattern) => match pattern {
                    ast::Pattern::Normal(ast::Expr::Ident(ident)) => {
                        vm.define(ident.clone(), args.expect::<Value>(ident)?)
                    }
                    ast::Pattern::Normal(_) => unreachable!(),
                    _ => {
                        pattern.define(
                            &mut vm,
                            args.expect::<Value>("pattern parameter")?,
                        )?;
                    }
                },
                Param::Sink(ident) => {
                    sink = ident.clone();
                    if let Some(sink_size) = sink_size {
                        sink_pos_values = Some(args.consume(sink_size)?);
                    }
                }
                Param::Named(ident, default) => {
                    let value =
                        args.named::<Value>(ident)?.unwrap_or_else(|| default.clone());
                    vm.define(ident.clone(), value);
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
        let result = closure.body.eval(&mut vm);
        match vm.flow {
            Some(FlowEvent::Return(_, Some(explicit))) => return Ok(explicit),
            Some(FlowEvent::Return(_, None)) => {}
            Some(flow) => bail!(flow.forbidden()),
            None => {}
        }

        result
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
pub(super) struct CapturesVisitor<'a> {
    external: &'a Scopes<'a>,
    internal: Scopes<'a>,
    captures: Scope,
}

impl<'a> CapturesVisitor<'a> {
    /// Create a new visitor for the given external scopes.
    pub fn new(external: &'a Scopes) -> Self {
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
            Some(ast::Expr::Ident(ident)) => self.capture(ident),
            Some(ast::Expr::MathIdent(ident)) => self.capture_in_math(ident),

            // Code and content blocks create a scope.
            Some(ast::Expr::Code(_) | ast::Expr::Content(_)) => {
                self.internal.enter();
                for child in node.children() {
                    self.visit(child);
                }
                self.internal.exit();
            }

            // A closure contains parameter bindings, which are bound before the
            // body is evaluated. Care must be taken so that the default values
            // of named parameters cannot access previous parameter bindings.
            Some(ast::Expr::Closure(expr)) => {
                for param in expr.params().children() {
                    if let ast::Param::Named(named) = param {
                        self.visit(named.expr().as_untyped());
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

                self.visit(expr.body().as_untyped());
                self.internal.exit();
            }

            // A let expression contains a binding, but that binding is only
            // active after the body is evaluated.
            Some(ast::Expr::Let(expr)) => {
                if let Some(init) = expr.init() {
                    self.visit(init.as_untyped());
                }

                for ident in expr.kind().idents() {
                    self.bind(ident);
                }
            }

            // A for loop contains one or two bindings in its pattern. These are
            // active after the iterable is evaluated but before the body is
            // evaluated.
            Some(ast::Expr::For(expr)) => {
                self.visit(expr.iter().as_untyped());
                self.internal.enter();

                let pattern = expr.pattern();
                for ident in pattern.idents() {
                    self.bind(ident);
                }

                self.visit(expr.body().as_untyped());
                self.internal.exit();
            }

            // An import contains items, but these are active only after the
            // path is evaluated.
            Some(ast::Expr::Import(expr)) => {
                self.visit(expr.source().as_untyped());
                if let Some(ast::Imports::Items(items)) = expr.imports() {
                    for item in items {
                        self.bind(item);
                    }
                }
            }

            // Everything else is traversed from left to right.
            _ => {
                for child in node.children() {
                    self.visit(child);
                }
            }
        }
    }

    /// Bind a new internal variable.
    fn bind(&mut self, ident: ast::Ident) {
        self.internal.top.define(ident.take(), Value::None);
    }

    /// Capture a variable if it isn't internal.
    fn capture(&mut self, ident: ast::Ident) {
        if self.internal.get(&ident).is_err() {
            if let Ok(value) = self.external.get(&ident) {
                self.captures.define_captured(ident.take(), value.clone());
            }
        }
    }

    /// Capture a variable in math mode if it isn't internal.
    fn capture_in_math(&mut self, ident: ast::MathIdent) {
        if self.internal.get(&ident).is_err() {
            if let Ok(value) = self.external.get_in_math(&ident) {
                self.captures.define_captured(ident.take(), value.clone());
            }
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

        let mut visitor = CapturesVisitor::new(&scopes);
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
    }
}
