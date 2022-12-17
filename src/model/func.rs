use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use comemo::{Track, Tracked};

use super::{
    Args, CastInfo, Dict, Eval, Flow, Node, NodeId, Route, Scope, Scopes, Selector,
    StyleMap, Value, Vm,
};
use crate::diag::{bail, SourceResult, StrResult};
use crate::syntax::ast::{self, AstNode, Expr};
use crate::syntax::{SourceId, SyntaxNode};
use crate::util::EcoString;
use crate::World;

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
    /// Create a new function from a type that can be turned into a function.
    pub fn from_type<T: FuncType>(name: &'static str) -> Self {
        T::create_func(name)
    }

    /// Create a new function from a native rust function.
    pub fn from_fn(
        name: &'static str,
        func: fn(&Vm, &mut Args) -> SourceResult<Value>,
        info: FuncInfo,
    ) -> Self {
        Self(Arc::new(Repr::Native(Native { name, func, set: None, node: None, info })))
    }

    /// Create a new function from a native rust node.
    pub fn from_node<T: Node>(name: &'static str, mut info: FuncInfo) -> Self {
        info.params.extend(T::properties());
        Self(Arc::new(Repr::Native(Native {
            name,
            func: |ctx, args| {
                let styles = T::set(args, true)?;
                let content = T::construct(ctx, args)?;
                Ok(Value::Content(content.styled_with_map(styles.scoped())))
            },
            set: Some(|args| T::set(args, false)),
            node: Some(NodeId::of::<T>()),
            info,
        })))
    }

    /// Create a new function from a closure.
    pub(super) fn from_closure(closure: Closure) -> Self {
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

    /// Extract details the function.
    pub fn info(&self) -> Option<&FuncInfo> {
        match self.0.as_ref() {
            Repr::Native(native) => Some(&native.info),
            Repr::With(func, _) => func.info(),
            _ => None,
        }
    }

    /// The number of positional arguments this function takes, if known.
    pub fn argc(&self) -> Option<usize> {
        match self.0.as_ref() {
            Repr::Closure(closure) => closure.argc(),
            Repr::With(wrapped, applied) => Some(wrapped.argc()?.saturating_sub(
                applied.items.iter().filter(|arg| arg.name.is_none()).count(),
            )),
            _ => None,
        }
    }

    /// Call the function with the given arguments.
    pub fn call(&self, vm: &Vm, mut args: Args) -> SourceResult<Value> {
        let value = match self.0.as_ref() {
            Repr::Native(native) => (native.func)(vm, &mut args)?,
            Repr::Closure(closure) => closure.call(vm, &mut args)?,
            Repr::With(wrapped, applied) => {
                args.items.splice(..0, applied.items.iter().cloned());
                return wrapped.call(vm, args);
            }
        };
        args.finish()?;
        Ok(value)
    }

    /// Call the function without an existing virtual machine.
    pub fn call_detached(
        &self,
        world: Tracked<dyn World>,
        args: Args,
    ) -> SourceResult<Value> {
        let route = Route::default();
        let id = SourceId::detached();
        let scopes = Scopes::new(None);
        let vm = Vm::new(world, route.track(), id, scopes);
        self.call(&vm, args)
    }

    /// Apply the given arguments to the function.
    pub fn with(self, args: Args) -> Self {
        Self(Arc::new(Repr::With(self, args)))
    }

    /// Create a selector for this function's node type, filtering by node's
    /// whose [fields](super::Content::field) match the given arguments.
    pub fn where_(self, args: &mut Args) -> StrResult<Selector> {
        let fields = args.to_named();
        args.items.retain(|arg| arg.name.is_none());
        self.select(Some(fields))
    }

    /// Execute the function's set rule and return the resulting style map.
    pub fn set(&self, mut args: Args) -> SourceResult<StyleMap> {
        Ok(match self.0.as_ref() {
            Repr::Native(Native { set: Some(set), .. }) => {
                let styles = set(&mut args)?;
                args.finish()?;
                styles
            }
            _ => StyleMap::new(),
        })
    }

    /// Create a selector for this function's node type.
    pub fn select(&self, fields: Option<Dict>) -> StrResult<Selector> {
        match self.0.as_ref() {
            &Repr::Native(Native { node: Some(id), .. }) => {
                if id == item!(text_id) {
                    Err("to select text, please use a string or regex instead")?;
                }

                Ok(Selector::Node(id, fields))
            }
            _ => Err("this function is not selectable")?,
        }
    }
}

impl Debug for Func {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self.name() {
            Some(name) => write!(f, "<function {name}>"),
            None => f.write_str("<function>"),
        }
    }
}

impl PartialEq for Func {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// Types that can be turned into functions.
pub trait FuncType {
    /// Create a function with the given name from this type.
    fn create_func(name: &'static str) -> Func;
}

/// A function defined by a native rust function or node.
struct Native {
    /// The name of the function.
    name: &'static str,
    /// The function pointer.
    func: fn(&Vm, &mut Args) -> SourceResult<Value>,
    /// The set rule.
    set: Option<fn(&mut Args) -> SourceResult<StyleMap>>,
    /// The id of the node to customize with this function's show rule.
    node: Option<NodeId>,
    /// Documentation of the function.
    info: FuncInfo,
}

impl Hash for Native {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        (self.func as usize).hash(state);
        self.set.map(|set| set as usize).hash(state);
        self.node.hash(state);
    }
}

/// Details about a function.
#[derive(Debug, Clone)]
pub struct FuncInfo {
    /// The function's name.
    pub name: &'static str,
    /// Tags that categorize the function.
    pub tags: &'static [&'static str],
    /// Documentation for the function.
    pub docs: &'static str,
    /// Details about the function's parameters.
    pub params: Vec<ParamInfo>,
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
    /// Is the parameter positional?
    pub positional: bool,
    /// Is the parameter named?
    ///
    /// Can be true even if `positional` is true if the parameter can be given
    /// in both variants.
    pub named: bool,
    /// Is the parameter required?
    pub required: bool,
    /// Can the parameter be given any number of times?
    pub variadic: bool,
    /// Is the parameter settable with a set rule?
    pub settable: bool,
}

/// A user-defined closure.
#[derive(Hash)]
pub(super) struct Closure {
    /// The source file where the closure was defined.
    pub location: SourceId,
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
    fn call(&self, vm: &Vm, args: &mut Args) -> SourceResult<Value> {
        // Don't leak the scopes from the call site. Instead, we use the scope
        // of captured variables we collected earlier.
        let mut scopes = Scopes::new(None);
        scopes.top = self.captured.clone();

        // Parse the arguments according to the parameter list.
        for (param, default) in &self.params {
            scopes.top.define(
                param.clone(),
                match default {
                    Some(default) => {
                        args.named::<Value>(param)?.unwrap_or_else(|| default.clone())
                    }
                    None => args.expect::<Value>(param)?,
                },
            );
        }

        // Put the remaining arguments into the sink.
        if let Some(sink) = &self.sink {
            scopes.top.define(sink.clone(), args.take());
        }

        // Determine the route inside the closure.
        let detached = vm.location.is_detached();
        let fresh = Route::new(self.location);
        let route = if detached { fresh.track() } else { vm.route };

        // Evaluate the body.
        let mut sub = Vm::new(vm.world, route, self.location, scopes);
        let result = self.body.eval(&mut sub);

        // Handle control flow.
        match sub.flow {
            Some(Flow::Return(_, Some(explicit))) => return Ok(explicit),
            Some(Flow::Return(_, None)) => {}
            Some(flow) => bail!(flow.forbidden()),
            None => {}
        }

        result
    }

    /// The number of positional arguments this function takes, if known.
    fn argc(&self) -> Option<usize> {
        if self.sink.is_some() {
            return None;
        }

        Some(self.params.iter().filter(|(_, default)| default.is_none()).count())
    }
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
    pub fn visit(&mut self, node: &SyntaxNode) {
        match node.cast() {
            // Every identifier is a potential variable that we need to capture.
            // Identifiers that shouldn't count as captures because they
            // actually bind a new name are handled below (individually through
            // the expressions that contain them).
            Some(ast::Expr::Ident(ident)) => self.capture(ident),

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
                for param in expr.params() {
                    if let ast::Param::Named(named) = param {
                        self.visit(named.expr().as_untyped());
                    }
                }

                for param in expr.params() {
                    match param {
                        ast::Param::Pos(ident) => self.bind(ident),
                        ast::Param::Named(named) => self.bind(named.name()),
                        ast::Param::Sink(ident) => self.bind(ident),
                    }
                }

                self.visit(expr.body().as_untyped());
            }

            // A let expression contains a binding, but that binding is only
            // active after the body is evaluated.
            Some(ast::Expr::Let(expr)) => {
                if let Some(init) = expr.init() {
                    self.visit(init.as_untyped());
                }
                self.bind(expr.binding());
            }

            // A for loop contains one or two bindings in its pattern. These are
            // active after the iterable is evaluated but before the body is
            // evaluated.
            Some(ast::Expr::For(expr)) => {
                self.visit(expr.iter().as_untyped());
                self.internal.enter();
                let pattern = expr.pattern();
                if let Some(key) = pattern.key() {
                    self.bind(key);
                }
                self.bind(pattern.value());
                self.visit(expr.body().as_untyped());
                self.internal.exit();
            }

            // An import contains items, but these are active only after the
            // path is evaluated.
            Some(ast::Expr::Import(expr)) => {
                self.visit(expr.path().as_untyped());
                if let ast::Imports::Items(items) = expr.imports() {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::parse;

    #[track_caller]
    fn test(text: &str, result: &[&str]) {
        let mut scopes = Scopes::new(None);
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
        test("#let x; {x + y}", &["y"]);
        test("#let f(x, y) = x + y", &[]);

        // Closure with different kinds of params.
        test("{(x, y) => x + z}", &["z"]);
        test("{(x: y, z) => x + z}", &["y"]);
        test("{(..x) => x + y}", &["y"]);
        test("{(x, y: x + z) => x + y}", &["x", "z"]);

        // Show rule.
        test("#show y: x => x", &["y"]);
        test("#show y: x => x + z", &["y", "z"]);
        test("#show x: x => x", &["x"]);

        // For loop.
        test("#for x in y { x + z }", &["y", "z"]);
        test("#for x, y in y { x + y }", &["y"]);
        test("#for x in y {} #x", &["x", "y"]);

        // Import.
        test("#import x, y from z", &["z"]);
        test("#import x, y, z from x + y", &["x", "y"]);

        // Blocks.
        test("{ let x = 1; { let y = 2; y }; x + y }", &["y"]);
        test("[#let x = 1]#x", &["x"]);
    }
}
