use comemo::{Tracked, TrackedMut};
use ecow::{eco_format, EcoString, EcoVec};
use typst_library::diag::{
    bail, error, At, HintedStrResult, HintedString, SourceDiagnostic, SourceResult,
    Trace, Tracepoint,
};
use typst_library::engine::{Engine, Sink, Traced};
use typst_library::foundations::{
    Arg, Args, Bytes, Capturer, Closure, Content, Context, DictionaryKey, Func,
    IntoValue, NativeElement, Scope, Scopes, Str, Value,
};
use typst_library::introspection::Introspector;
use typst_library::math::LrElem;
use typst_library::routines::Routines;
use typst_library::text::TextElem;
use typst_library::World;
use typst_syntax::ast::{self, AstNode, Ident};
use typst_syntax::{Span, Spanned, SyntaxNode};
use typst_utils::LazyHash;

use crate::{call_method_mut, is_mutating_method, Access, Eval, FlowEvent, Route, Vm};

impl Eval for ast::FuncCall<'_> {
    type Output = Value;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let span = self.span();
        let callee = self.callee();
        let in_math = in_math(callee);
        let callee_span = callee.span();
        let args = self.args();
        let trailing_comma = args.trailing_comma();

        vm.engine.route.check_call_depth().at(span)?;

        // Try to evaluate as a call to an associated function or field.
        let (callee, args) = if let ast::Expr::FieldAccess(access) = callee {
            let target = access.target();
            let field = access.field();
            match eval_field_call(target, field, args, span, vm)? {
                FieldCall::Normal(callee, args) => (callee, args),
                FieldCall::Resolved(value) => return Ok(value),
            }
        } else {
            // Function call order: we evaluate the callee before the arguments.
            (callee.eval(vm)?, args.eval(vm)?.spanned(span))
        };

        let func_result = callee.clone().cast::<Func>();
        if in_math && func_result.is_err() {
            return wrap_args_in_math(callee, callee_span, args, trailing_comma);
        }

        let func = func_result
            .map_err(|err| hint_if_shadowed_std(vm, &self.callee(), err))
            .at(callee_span)?;

        let point = || Tracepoint::Call(func.name().map(Into::into));
        let f = || {
            func.call(&mut vm.engine, vm.context, args)
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
pub fn eval_closure(
    func: &Func,
    closure: &LazyHash<Closure>,
    routines: &Routines,
    world: Tracked<dyn World + '_>,
    introspector: Tracked<Introspector>,
    traced: Tracked<Traced>,
    sink: TrackedMut<Sink>,
    route: Tracked<Route>,
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
    let engine = Engine {
        routines,
        world,
        introspector,
        traced,
        sink,
        route: Route::extend(route),
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
                    crate::destructure(
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

/// This used only as the return value of `eval_field_call`.
/// - `Normal` means that we have a function to call and the arguments to call it with.
/// - `Resolved` means that we have already resolved the call and have the value.
enum FieldCall {
    Normal(Value, Args),
    Resolved(Value),
}

/// Evaluate a field call's callee and arguments.
///
/// This follows the normal function call order: we evaluate the callee before the
/// arguments.
///
/// Prioritize associated functions on the value's type (e.g., methods) over its fields.
/// A function call on a field is only allowed for functions, types, modules (because
/// they are scopes), and symbols (because they have modifiers or associated functions).
///
/// For dictionaries, it is not allowed because it would be ambiguous - prioritizing
/// associated functions would make an addition of a new associated function a breaking
/// change and prioritizing fields would break associated functions for certain
/// dictionaries.
fn eval_field_call(
    target_expr: ast::Expr,
    field: Ident,
    args: ast::Args,
    span: Span,
    vm: &mut Vm,
) -> SourceResult<FieldCall> {
    // Evaluate the field-call's target and overall arguments.
    let (target, mut args) = if is_mutating_method(&field) {
        // If `field` looks like a mutating method, we evaluate the arguments first,
        // because `target_expr.access(vm)` mutably borrows the `vm`, so that we can't
        // evaluate the arguments after it.
        let args = args.eval(vm)?.spanned(span);
        // However, this difference from the normal call order is not observable because
        // expressions like `(1, arr.len(), 2, 3).push(arr.pop())` evaluate the target to
        // a temporary which we disallow mutation on (returning an error).
        // Theoretically this could be observed if a method matching `is_mutating_method`
        // was added to some type in the future and we didn't update this function.
        match target_expr.access(vm)? {
            // Only arrays and dictionaries have mutable methods.
            target @ (Value::Array(_) | Value::Dict(_)) => {
                let value = call_method_mut(target, &field, args, span);
                let point = || Tracepoint::Call(Some(field.get().clone()));
                return Ok(FieldCall::Resolved(value.trace(vm.world(), point, span)?));
            }
            target => (target.clone(), args),
        }
    } else {
        let target = target_expr.eval(vm)?;
        let args = args.eval(vm)?.spanned(span);
        (target, args)
    };

    if let Value::Plugin(plugin) = &target {
        // Call plugins by converting args to bytes.
        let bytes = args.all::<Bytes>()?;
        args.finish()?;
        let value = plugin.call(&field, bytes).at(span)?.into_value();
        Ok(FieldCall::Resolved(value))
    } else if let Some(callee) = target.ty().scope().get(&field) {
        args.insert(0, target_expr.span(), target);
        Ok(FieldCall::Normal(callee.clone(), args))
    } else if matches!(
        target,
        Value::Symbol(_) | Value::Func(_) | Value::Type(_) | Value::Module(_)
    ) {
        // Certain value types may have their own ways to access method fields.
        // e.g. `$arrow.r(v)$`, `table.cell[..]`
        let value = target.field(&field).at(field.span())?;
        Ok(FieldCall::Normal(value, args))
    } else {
        // Otherwise we cannot call this field.
        bail!(missing_field_call_error(target, field))
    }
}

/// Produce an error when we cannot call the field.
fn missing_field_call_error(target: Value, field: Ident) -> SourceDiagnostic {
    let mut error =
        error!(field.span(), "type {} has no method `{}`", target.ty(), field.as_str());

    match target {
        Value::Dict(ref dict)
            if matches!(
                dict.get(&DictionaryKey::Name(Str::from(field.as_str()))),
                Ok(Value::Func(_))
            ) =>
        {
            error.hint(eco_format!(
                "to call the function stored in the dictionary, surround \
                the field access with parentheses, e.g. `(dict.{})(..)`",
                field.as_str(),
            ));
        }
        _ if target.field(&field).is_ok() => {
            error.hint(eco_format!(
                "did you mean to access the field `{}`?",
                field.as_str(),
            ));
        }
        _ => {}
    }
    error
}

/// Check if the expression is in a math context.
fn in_math(expr: ast::Expr) -> bool {
    match expr {
        ast::Expr::MathIdent(_) => true,
        ast::Expr::FieldAccess(access) => in_math(access.target()),
        _ => false,
    }
}

/// For non-functions in math, we wrap the arguments in parentheses.
fn wrap_args_in_math(
    callee: Value,
    callee_span: Span,
    mut args: Args,
    trailing_comma: bool,
) -> SourceResult<Value> {
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
    Ok(Value::Content(
        callee.display().spanned(callee_span)
            + LrElem::new(TextElem::packed('(') + body + TextElem::packed(')')).pack(),
    ))
}

/// Provide a hint if the callee is a shadowed standard library function.
fn hint_if_shadowed_std(
    vm: &mut Vm,
    callee: &ast::Expr,
    mut err: HintedString,
) -> HintedString {
    if let ast::Expr::Ident(ident) = callee {
        let ident = ident.get();
        if vm.scopes.check_std_shadowed(ident) {
            err.hint(eco_format!(
                "use `std.{ident}` to access the shadowed standard library function",
            ));
        }
    }
    err
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
            Some(ast::Expr::Ident(ident)) => {
                self.capture(ident.get(), ident.span(), Scopes::get)
            }
            Some(ast::Expr::MathIdent(ident)) => {
                self.capture(ident.get(), ident.span(), Scopes::get_in_math)
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
        self.internal.top.define_ident(ident, Value::None);
    }

    /// Capture a variable if it isn't internal.
    fn capture(
        &mut self,
        ident: &EcoString,
        span: Span,
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

            self.captures.define_captured(
                ident.clone(),
                value.clone(),
                self.capturer,
                span,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use typst_syntax::parse;

    use super::*;

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
        let mut names: Vec<_> = captures.iter().map(|(k, ..)| k).collect();
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
