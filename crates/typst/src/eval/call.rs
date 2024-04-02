use comemo::{Tracked, TrackedMut};
use ecow::{eco_format, EcoVec};

use crate::diag::{bail, error, At, HintedStrResult, SourceResult, Trace, Tracepoint};
use crate::engine::Engine;
use crate::eval::{Access, Eval, FlowEvent, Route, Tracer, Vm};
use crate::foundations::{
    call_method_mut, is_mutating_method, Arg, Args, Bytes, Capturer, Closure, Content,
    Context, Func, IntoValue, NativeElement, Scope, Scopes, Value,
};
use crate::introspection::{Introspector, Locator};
use crate::math::{Accent, AccentElem, LrElem};
use crate::symbols::Symbol;
use crate::syntax::ast::{self, AstNode};
use crate::syntax::{Span, Spanned, SyntaxNode};
use crate::text::TextElem;
use crate::util::LazyHash;
use crate::World;

impl Eval for ast::FuncCall<'_> {
    type Output = Value;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let span = self.span();
        let callee = self.callee();
        let in_math = in_math(callee);
        let callee_span = callee.span();
        let args = self.args();
        let trailing_comma = args.trailing_comma();

        if !vm.engine.route.within(Route::MAX_CALL_DEPTH) {
            bail!(span, "maximum function call depth exceeded");
        }

        // Try to evaluate as a call to an associated function or field.
        let (callee, mut args) = if let ast::Expr::FieldAccess(access) = callee {
            let target = access.target();
            let target_span = target.span();
            let field = access.field();
            let field_span = field.span();

            let target = if is_mutating_method(&field) {
                let mut args = args.eval(vm)?.spanned(span);
                let target = target.access(vm)?;

                // Only arrays and dictionaries have mutable methods.
                if matches!(target, Value::Array(_) | Value::Dict(_)) {
                    args.span = span;
                    let point = || Tracepoint::Call(Some(field.get().clone()));
                    return call_method_mut(target, &field, args, span).trace(
                        vm.world(),
                        point,
                        span,
                    );
                }

                target.clone()
            } else {
                access.target().eval(vm)?
            };

            let mut args = args.eval(vm)?.spanned(span);

            // Handle plugins.
            if let Value::Plugin(plugin) = &target {
                let bytes = args.all::<Bytes>()?;
                args.finish()?;
                return Ok(plugin.call(&field, bytes).at(span)?.into_value());
            }

            // Prioritize associated functions on the value's type (i.e.,
            // methods) over its fields. A function call on a field is only
            // allowed for functions, types, modules (because they are scopes),
            // and symbols (because they have modifiers).
            //
            // For dictionaries, it is not allowed because it would be ambiguous
            // (prioritizing associated functions would make an addition of a
            // new associated function a breaking change and prioritizing fields
            // would break associated functions for certain dictionaries).
            if let Some(callee) = target.ty().scope().get(&field) {
                let this = Arg {
                    span: target_span,
                    name: None,
                    value: Spanned::new(target, target_span),
                };
                args.span = span;
                args.items.insert(0, this);
                (callee.clone(), args)
            } else if matches!(
                target,
                Value::Symbol(_) | Value::Func(_) | Value::Type(_) | Value::Module(_)
            ) {
                (target.field(&field).at(field_span)?, args)
            } else {
                let mut error = error!(
                    field_span,
                    "type {} has no method `{}`",
                    target.ty(),
                    field.as_str()
                );

                let mut field_hint = || {
                    if target.field(&field).is_ok() {
                        error.hint(eco_format!(
                            "did you mean to access the field `{}`?",
                            field.as_str()
                        ));
                    }
                };

                match target {
                    Value::Dict(ref dict) => {
                        if matches!(dict.get(&field), Ok(Value::Func(_))) {
                            error.hint(eco_format!(
                                "to call the function stored in the dictionary, surround \
                                 the field access with parentheses, e.g. `(dict.{})(..)`",
                               field.as_str(),
                            ));
                        } else {
                            field_hint();
                        }
                    }
                    _ => field_hint(),
                }

                bail!(error);
            }
        } else {
            (callee.eval(vm)?, args.eval(vm)?.spanned(span))
        };

        // Handle math special cases for non-functions:
        // Combining accent symbols apply themselves while everything else
        // simply displays the arguments verbatim.
        if in_math && !matches!(callee, Value::Func(_)) {
            if let Value::Symbol(sym) = &callee {
                let c = sym.get();
                if let Some(accent) = Symbol::combining_accent(c) {
                    let base = args.expect("base")?;
                    let size = args.named("size")?;
                    args.finish()?;
                    let mut accent = AccentElem::new(base, Accent::new(accent));
                    if let Some(size) = size {
                        accent = accent.with_size(size);
                    }
                    return Ok(Value::Content(accent.pack()));
                }
            }
            let mut body = Content::empty();
            for (i, arg) in args.all::<Content>()?.into_iter().enumerate() {
                if i > 0 {
                    body += TextElem::packed(',');
                }
                body += arg;
            }
            if trailing_comma {
                body += TextElem::packed(',');
            }
            return Ok(Value::Content(
                callee.display().spanned(callee_span)
                    + LrElem::new(TextElem::packed('(') + body + TextElem::packed(')'))
                        .pack(),
            ));
        }

        let callee = callee.cast::<Func>().at(callee_span)?;
        let point = || Tracepoint::Call(callee.name().map(Into::into));
        let f = || {
            callee
                .call(&mut vm.engine, vm.context, args)
                .trace(vm.world(), point, span)
        };

        // Stacker is broken on WASM.
        #[cfg(target_arch = "wasm32")]
        return f();

        #[cfg(not(target_arch = "wasm32"))]
        stacker::maybe_grow(32 * 1024, 2 * 1024 * 1024, f)
    }
}

impl Eval for ast::Args<'_> {
    type Output = Args;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let mut items = EcoVec::with_capacity(self.items().count());

        for arg in self.items() {
            let span = arg.span();
            match arg {
                ast::Arg::Pos(expr) => {
                    items.push(Arg {
                        span,
                        name: None,
                        value: Spanned::new(expr.eval(vm)?, expr.span()),
                    });
                }
                ast::Arg::Named(named) => {
                    let expr = named.expr();
                    items.push(Arg {
                        span,
                        name: Some(named.name().get().clone().into()),
                        value: Spanned::new(expr.eval(vm)?, expr.span()),
                    });
                }
                ast::Arg::Spread(spread) => match spread.expr().eval(vm)? {
                    Value::None => {}
                    Value::Array(array) => {
                        items.extend(array.into_iter().map(|value| Arg {
                            span,
                            name: None,
                            value: Spanned::new(value, span),
                        }));
                    }
                    Value::Dict(dict) => {
                        items.extend(dict.into_iter().map(|(key, value)| Arg {
                            span,
                            name: Some(key),
                            value: Spanned::new(value, span),
                        }));
                    }
                    Value::Args(args) => items.extend(args.items),
                    v => bail!(spread.span(), "cannot spread {}", v.ty()),
                },
            }
        }

        // We do *not* use the `self.span()` here because we want the callsite
        // span to be one level higher (the whole function call).
        Ok(Args { span: Span::detached(), items })
    }
}

impl Eval for ast::Closure<'_> {
    type Output = Value;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        // Evaluate default values of named parameters.
        let mut defaults = Vec::new();
        for param in self.params().children() {
            if let ast::Param::Named(named) = param {
                defaults.push(named.expr().eval(vm)?);
            }
        }

        // Collect captured variables.
        let captured = {
            let mut visitor = CapturesVisitor::new(Some(&vm.scopes), Capturer::Function);
            visitor.visit(self.to_untyped());
            visitor.finish()
        };

        // Define the closure.
        let closure = Closure {
            node: self.to_untyped().clone(),
            defaults,
            captured,
            num_pos_params: self
                .params()
                .children()
                .filter(|p| matches!(p, ast::Param::Pos(_)))
                .count(),
        };

        Ok(Value::Func(Func::from(closure).spanned(self.params().span())))
    }
}

/// Call the function in the context with the arguments.
#[comemo::memoize]
#[allow(clippy::too_many_arguments)]
pub(crate) fn call_closure(
    func: &Func,
    closure: &LazyHash<Closure>,
    world: Tracked<dyn World + '_>,
    introspector: Tracked<Introspector>,
    route: Tracked<Route>,
    locator: Tracked<Locator>,
    tracer: TrackedMut<Tracer>,
    context: Tracked<Context>,
    mut args: Args,
) -> SourceResult<Value> {
    let (name, params, body) = match closure.node.cast::<ast::Closure>() {
        Some(node) => (node.name(), node.params(), node.body()),
        None => (None, ast::Params::default(), closure.node.cast().unwrap()),
    };

    // Don't leak the scopes from the call site. Instead, we use the scope
    // of captured variables we collected earlier.
    let mut scopes = Scopes::new(None);
    scopes.top = closure.captured.clone();

    // Prepare the engine.
    let mut locator = Locator::chained(locator);
    let engine = Engine {
        world,
        introspector,
        route: Route::extend(route),
        locator: &mut locator,
        tracer,
    };

    // Prepare VM.
    let mut vm = Vm::new(engine, context, scopes, body.span());

    // Provide the closure itself for recursive calls.
    if let Some(name) = name {
        vm.define(name, Value::Func(func.clone()));
    }

    let num_pos_args = args.to_pos().len();
    let sink_size = num_pos_args.checked_sub(closure.num_pos_params);

    let mut sink = None;
    let mut sink_pos_values = None;
    let mut defaults = closure.defaults.iter();
    for p in params.children() {
        match p {
            ast::Param::Pos(pattern) => match pattern {
                ast::Pattern::Normal(ast::Expr::Ident(ident)) => {
                    vm.define(ident, args.expect::<Value>(&ident)?)
                }
                pattern => {
                    crate::eval::destructure(
                        &mut vm,
                        pattern,
                        args.expect::<Value>("pattern parameter")?,
                    )?;
                }
            },
            ast::Param::Spread(spread) => {
                sink = Some(spread.sink_ident());
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
        // Remaining args are captured regardless of whether the sink is named.
        let mut remaining_args = args.take();
        if let Some(sink_name) = sink {
            if let Some(sink_pos_values) = sink_pos_values {
                remaining_args.items.extend(sink_pos_values);
            }
            vm.define(sink_name, remaining_args);
        }
    }

    // Ensure all arguments have been used.
    args.finish()?;

    // Handle control flow.
    let output = body.eval(&mut vm)?;
    match vm.flow {
        Some(FlowEvent::Return(_, Some(explicit))) => return Ok(explicit),
        Some(FlowEvent::Return(_, None)) => {}
        Some(flow) => bail!(flow.forbidden()),
        None => {}
    }

    Ok(output)
}

fn in_math(expr: ast::Expr) -> bool {
    match expr {
        ast::Expr::MathIdent(_) => true,
        ast::Expr::FieldAccess(access) => in_math(access.target()),
        _ => false,
    }
}

/// A visitor that determines which variables to capture for a closure.
pub struct CapturesVisitor<'a> {
    external: Option<&'a Scopes<'a>>,
    internal: Scopes<'a>,
    captures: Scope,
    capturer: Capturer,
}

impl<'a> CapturesVisitor<'a> {
    /// Create a new visitor for the given external scopes.
    pub fn new(external: Option<&'a Scopes<'a>>, capturer: Capturer) -> Self {
        Self {
            external,
            internal: Scopes::new(None),
            captures: Scope::new(),
            capturer,
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
                            for ident in pattern.bindings() {
                                self.bind(ident);
                            }
                        }
                        ast::Param::Named(named) => self.bind(named.name()),
                        ast::Param::Spread(spread) => {
                            if let Some(ident) = spread.sink_ident() {
                                self.bind(ident);
                            }
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

                for ident in expr.kind().bindings() {
                    self.bind(ident);
                }
            }

            // A for loop contains one or two bindings in its pattern. These are
            // active after the iterable is evaluated but before the body is
            // evaluated.
            Some(ast::Expr::For(expr)) => {
                self.visit(expr.iterable().to_untyped());
                self.internal.enter();

                let pattern = expr.pattern();
                for ident in pattern.bindings() {
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
    fn capture(
        &mut self,
        ident: &str,
        getter: impl FnOnce(&'a Scopes<'a>, &str) -> HintedStrResult<&'a Value>,
    ) {
        if self.internal.get(ident).is_err() {
            let Some(value) = self
                .external
                .map(|external| getter(external, ident).ok())
                .unwrap_or(Some(&Value::None))
            else {
                return;
            };

            self.captures.define_captured(ident, value.clone(), self.capturer);
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

        let mut visitor = CapturesVisitor::new(Some(&scopes), Capturer::Function);
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
