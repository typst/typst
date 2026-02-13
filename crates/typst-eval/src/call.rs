use comemo::{Tracked, TrackedMut};
use ecow::{EcoString, EcoVec, eco_format};
use typst_library::World;
use typst_library::diag::{
    At, HintedStrResult, HintedString, SourceResult, Trace, Tracepoint, bail, error,
};
use typst_library::engine::{Engine, Sink, Traced};
use typst_library::foundations::{
    Arg, Args, Binding, Capturer, Closure, ClosureNode, Content, Context, Func,
    NativeElement, Scope, Scopes, SequenceElem, SymbolElem, Value,
};
use typst_library::introspection::Introspector;
use typst_library::math::LrElem;
use typst_library::routines::Routines;
use typst_syntax::ast::{self, AstNode};
use typst_syntax::{Span, Spanned, SyntaxNode};
use typst_utils::{LazyHash, Protected};

use crate::{
    Access, Eval, FlowEvent, Route, Vm, call_method_mut, hint_if_shadowed_std,
    is_mutating_method,
};

impl Eval for ast::FuncCall<'_> {
    type Output = Value;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let span = self.span();
        let callee = self.callee();
        let callee_span = callee.span();
        let args = self.args();

        vm.engine.route.check_call_depth().at(span)?;

        // Try to evaluate as a call to an associated function or field.
        if let ast::Expr::FieldAccess(access) = callee {
            let target_expr = access.target();
            let field = access.field();
            let (target, maybe_args) = if is_mutating_method(field.as_str()) {
                match maybe_resolve_mutating(vm, target_expr, field, args, span)? {
                    Ok(value) => return Ok(value),
                    Err((target, args)) => (target, Some(args)),
                }
            } else {
                (target_expr.eval(vm)?, None)
            };
            match eval_field_callee(vm, target, field, callee_span)? {
                FieldCallee::Func(func) => {
                    let args = match maybe_args {
                        Some(args) => args,
                        None => args.eval(vm)?.spanned(span),
                    };
                    call_func(vm, func, args, span)
                }
                FieldCallee::Method(func, target) => {
                    let mut args = match maybe_args {
                        Some(args) => args,
                        None => args.eval(vm)?.spanned(span),
                    };
                    // Method calls pass the target as the first argument.
                    args.insert(0, target_expr.span(), target);
                    call_func(vm, func, args, span)
                }
                FieldCallee::NonFunc(_, mut err) => {
                    // Give a custom hint if the field would not have been a function.
                    if args.eval(vm).is_ok_and(|args| args.items.is_empty()) {
                        err.hint(eco_format!(
                            "try removing the parentheses: `{}`",
                            access.to_untyped().clone().into_text(),
                        ));
                    } else {
                        err.hint(eco_format!(
                            "try adding a space before the parentheses: `{} {}`",
                            access.to_untyped().clone().into_text(),
                            args.to_untyped().clone().into_text(),
                        ));
                    }
                    Err(err).at(callee_span)
                }
                FieldCallee::DictFunc => bail!(
                    callee_span,
                    "cannot directly call a function stored in a dictionary";
                    hint: "to call the function, wrap the field access in parentheses: \
                        `({})(..)`",
                        callee.to_untyped().clone().into_text();
                ),
            }
        } else {
            // Function call order: we evaluate the callee before the arguments.
            let func = callee
                .eval(vm)?
                .cast::<Func>()
                .map_err(|err| hint_if_shadowed_std(vm, &callee, err))
                .at(callee_span)?;
            let args = args.eval(vm)?.spanned(span);
            call_func(vm, func, args, span)
        }
    }
}

impl Eval for ast::MathCall<'_> {
    type Output = Value;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let span = self.span();
        let callee = self.callee();
        let callee_span = callee.span();
        let mut target_span = Span::detached();

        vm.engine.route.check_call_depth().at(span)?;

        let math_call_result = match callee {
            ast::MathCallee::MathIdent(ident) => {
                let callee_value = crate::math::eval_math_ident(vm, ident, true)?;
                match callee_value.clone().cast::<Func>() {
                    Ok(func) => FieldCallee::Func(func),
                    Err(err) => FieldCallee::NonFunc(callee_value, err),
                }
            }
            ast::MathCallee::FieldAccess(access) => {
                let target_expr = access.target();
                target_span = target_expr.span();
                let field = access.field();
                let (was_math, target) = crate::code::eval_field_target(vm, target_expr)?;
                assert!(was_math);
                if is_mutating_method(field.as_str())
                    && matches!(target, Value::Array(_) | Value::Dict(_))
                {
                    // FUTURE: This is probably worth allowing once we nail down
                    // mutable method semantics.
                    //
                    // Mutable methods have always produced an error in math
                    // because `Access` was never implemented for `MathIdent`,
                    // so this explicit error is just nicer. And while we could
                    // start to implement `Access`, making mutable methods work
                    // in math still requires deeper changes because math mode
                    // needs to know whether the target is actually a function
                    // before evaluating arguments.
                    bail!(
                        span,
                        "cannot call mutating methods in math";
                        hint: "try using code mode to call the method: `#{}`",
                            self.to_untyped().clone().into_text();
                    );
                }
                eval_field_callee(vm, target, field, callee_span)?
            }
        };

        let args = self.args();
        match math_call_result {
            FieldCallee::Func(func) => {
                let args = args.eval(vm)?.spanned(span);
                call_func(vm, func, args, span)
            }
            FieldCallee::Method(func, target) => {
                let mut args = args.eval(vm)?.spanned(span);
                // Method calls pass the target as the first argument.
                args.insert(0, target_span, target);
                call_func(vm, func, args, span)
            }
            FieldCallee::NonFunc(callee_value, _) => {
                let parens = unparse_math_args(vm, args, callee)?;
                Ok(Value::Content(callee_value.display().spanned(callee.span()) + parens))
            }
            FieldCallee::DictFunc => bail!(
                callee_span,
                "cannot directly call a function stored in a dictionary";
                hint: "to call the function, use code mode and wrap the field \
                    access in parentheses: `#({})(..)`",
                    callee.to_untyped().clone().into_text();
            ),
        }
    }
}

/// Call a function.
fn call_func(vm: &mut Vm, func: Func, args: Args, span: Span) -> SourceResult<Value> {
    let func = func.spanned(span);
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

/// Attempt to resolve a mutating method call by evaluating args and then
/// attmempting to access the target mutably. If the target's type doesn't
/// support mutating methods (only Array/Dict actually do), returns the
/// evaluated value and arguments.
///
/// This currently causes a number of bad errors due to limitations of the
/// [`Access`] trait used for mutation.
fn maybe_resolve_mutating(
    vm: &mut Vm,
    target: ast::Expr,
    field: ast::Ident,
    args: ast::Args,
    span: Span,
) -> SourceResult<Result<Value, (Value, Args)>> {
    // We evaluate the arguments first because `target_expr.access(vm)` mutably
    // borrows `vm`, so we won't be able to call `args.eval(vm)` afterwards.
    let args = args.eval(vm)?.spanned(span);
    match target.access(vm)? {
        // Only arrays and dictionaries have mutable methods.
        target @ (Value::Array(_) | Value::Dict(_)) => {
            let value = call_method_mut(target, &field, args, span);
            let point = || Tracepoint::Call(Some(field.get().clone()));
            Ok(Ok(value.trace(vm.world(), point, span)?))
        }
        target => Ok(Err((target.clone(), args))),
    }
}

/// The kind of callee in a field-access function call.
enum FieldCallee {
    /// A method on a type or on content, with the target value to be added as
    /// the first argument of the call.
    Method(Func, Value),
    /// A plain function to call.
    Func(Func),
    /// The field access doesn't actually produce a function. This will error in
    /// code, but not in math.
    NonFunc(Value, HintedString),
    /// A non-method call from a dictionary field access.
    ///
    /// We give slightly different errors for code vs. math.
    DictFunc,
}

/// Evaluate a field-access callee, prioritizing associated functions of the
/// value's type, "methods", over fields on the specific value.
///
/// Calls to fields of a value are only allowed for functions (`assert.eq`),
/// types (`str.to-unicode`, `table.cell`), modules (`pdf.attach`), and symbols
/// (`arrow.l`).
///
/// In particular, calls to a field function are not allowed for dictionaries
/// because it would be ambiguous. If we did allow it, we would either have to
/// prioritize methods or field functions, but both choices are bad:
/// - Prioritizing methods would make all new method additions breaking changes.
/// - Prioritizing field functions would break methods for certain dictionaries,
///   e.g. `(at: x => ...).at(key)`.
fn eval_field_callee<'a, 'b>(
    vm: &'a mut Vm<'b>,
    target: Value,
    field: ast::Ident,
    callee_span: Span,
) -> SourceResult<FieldCallee> {
    let field_span = field.span();
    let sink = (&mut vm.engine, field_span);
    let mut is_method_call = false;
    let callee = if let Some(method) = target.ty().scope().get(&field) {
        is_method_call = true;
        method.read_checked(sink).clone()
    } else if let Value::Content(content) = &target
        && let Some(method) = content.elem().scope().get(&field)
    {
        is_method_call = true;
        method.read_checked(sink).clone()
    } else {
        match target.field(&field, sink) {
            Ok(callee) => callee,
            Err(missing_field) => match target {
                Value::Symbol(_) | Value::Func(_) | Value::Type(_) | Value::Module(_) => {
                    // Use the default missing field error for these types.
                    return Err(missing_field).at(field_span);
                }
                Value::Content(content) => bail!(
                    callee_span,
                    "element {} has no method `{}`",
                    content.elem().name(),
                    field.as_str(),
                ),
                _ => bail!(
                    callee_span,
                    "type {} has no method `{}`",
                    target.ty(),
                    field.as_str(),
                ),
            },
        }
    };

    if vm.inspected == Some(callee_span) {
        vm.trace(callee.clone());
    }

    match callee.clone().cast::<Func>() {
        Ok(func) if is_method_call => Ok(FieldCallee::Method(func, target)),
        Ok(_) if matches!(target, Value::Dict(_)) => Ok(FieldCallee::DictFunc),
        Ok(func) => Ok(FieldCallee::Func(func)),
        Err(err) => Ok(FieldCallee::NonFunc(callee, err)),
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

impl Eval for ast::MathArgs<'_> {
    type Output = Args;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        // Math args need to fully separate named/pos to handle two-dimensional
        // args correctly, for example: `mat(a, delim:"[", b; c, d)`.
        let mut named = EcoVec::new();
        let mut pos = Vec::new();
        let mut two_dim_start: Option<usize> = None;

        /// Optimize two-dimensional args by using `pos` as the sole container
        /// while iterating and only group into an array when we encounter a
        /// semicolon.
        fn drain_into_array(pos: &mut Vec<Arg>, start: usize, span: Span) {
            let array = pos.drain(start..).map(|arg| arg.value.v).collect();
            pos.push(Arg {
                span,
                name: None,
                value: Spanned::new(Value::Array(array), span),
            });
        }

        for ast::MathArg { arg, ends_in_semicolon } in self.arg_items() {
            let span = arg.span();
            match arg {
                ast::Arg::Pos(expr) => {
                    pos.push(Arg {
                        span,
                        name: None,
                        value: Spanned::new(expr.eval(vm)?, expr.span()),
                    });
                }
                ast::Arg::Named(named_arg) => {
                    let expr = named_arg.expr();
                    named.push(Arg {
                        span,
                        name: Some(named_arg.name().get().clone().into()),
                        value: Spanned::new(expr.eval(vm)?, expr.span()),
                    });
                }
                ast::Arg::Spread(spread) => match spread.expr().eval(vm)? {
                    Value::None => {}
                    Value::Array(array) => {
                        pos.extend(array.into_iter().map(|value| Arg {
                            span,
                            name: None,
                            value: Spanned::new(value, span),
                        }));
                    }
                    Value::Dict(dict) => {
                        named.extend(dict.into_iter().map(|(key, value)| Arg {
                            span,
                            name: Some(key),
                            value: Spanned::new(value, span),
                        }));
                    }
                    Value::Args(args) => {
                        for arg in args.items {
                            if arg.name.is_none() {
                                pos.push(arg);
                            } else {
                                named.push(arg);
                            }
                        }
                    }
                    v => bail!(spread.span(), "cannot spread {}", v.ty()),
                },
            }
            if ends_in_semicolon {
                let start = two_dim_start.unwrap_or(0);
                // There's not really a better span to use :/
                drain_into_array(&mut pos, start, self.span());
                two_dim_start = Some(pos.len());
            }
        }

        if let Some(start) = two_dim_start
            && start != pos.len()
        {
            drain_into_array(&mut pos, start, self.span());
        }

        named.extend(pos);
        Ok(Args { span: Span::detached(), items: named })
    }
}

/// For non-functions in math, we evaluate the arguments and punctuation as
/// content and wrap in an [`LrElem`].
fn unparse_math_args(
    vm: &mut Vm,
    args: ast::MathArgs,
    callee: ast::MathCallee,
) -> SourceResult<Content> {
    let mut body = Vec::new();
    let mut errors = EcoVec::new();
    for item in args.content_items() {
        match item {
            ast::MathArgItem::Space(space) => {
                body.push(space.eval(vm)?.spanned(space.span()));
            }
            ast::MathArgItem::Comma(c, node)
            | ast::MathArgItem::Semicolon(c, node)
            | ast::MathArgItem::LeftParen(c, node)
            | ast::MathArgItem::RightParen(c, node) => {
                body.push(SymbolElem::packed(c).spanned(node.span()));
            }
            ast::MathArgItem::Arg(ast::Arg::Pos(expr)) => {
                body.push(expr.eval(vm)?.display().spanned(expr.span()));
            }
            ast::MathArgItem::Arg(ast::Arg::Named(named)) => {
                let name = callee.to_untyped().clone().into_text();
                let fixed =
                    named.to_untyped().clone().into_text().replacen(":", "\\:", 1);
                errors.push(
                    error!(
                        named.span(),
                        "named-argument syntax can only be used with functions"
                    )
                    .with_spanned_hint(
                        eco_format!("`{name}` is not a function"),
                        callee.span(),
                    )
                    .with_hint(eco_format!(
                        "to render the colon as text, escape it: `{fixed}`"
                    )),
                );
            }
            ast::MathArgItem::Arg(ast::Arg::Spread(spread)) => {
                let name = callee.to_untyped().clone().into_text();
                let fixed =
                    spread.to_untyped().clone().into_text().replacen("..", ".. ", 1);
                errors.push(
                    error!(
                        spread.span(),
                        "spread-argument syntax can only be used with functions"
                    )
                    .with_spanned_hint(
                        eco_format!("`{name}` is not a function"),
                        callee.span(),
                    )
                    .with_hint(eco_format!(
                        "to render the dots as text, add a space: `{fixed}`"
                    )),
                );
            }
        }
    }
    if errors.is_empty() {
        let parens = LrElem::new(SequenceElem::new(body).pack())
            .pack()
            .spanned(args.span());
        Ok(parens)
    } else {
        Err(errors)
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
            node: ClosureNode::Closure(self.to_untyped().clone()),
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
    let (name, params, body) = match closure.node {
        ClosureNode::Closure(ref node) => {
            let closure =
                node.cast::<ast::Closure>().expect("node to be an `ast::Closure`");
            (closure.name(), closure.params(), closure.body())
        }
        ClosureNode::Context(ref node) => {
            (None, ast::Params::default(), node.cast().unwrap())
        }
    };

    // Don't leak the scopes from the call site. Instead, we use the scope
    // of captured variables we collected earlier.
    let mut scopes = Scopes::new(None);
    scopes.top = closure.captured.clone();

    // Prepare the engine.
    let introspector = Protected::from_raw(introspector);
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
        vm.define(name, func.clone());
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
        Some(FlowEvent::Return(_, Some(explicit), _)) => return Ok(explicit),
        Some(FlowEvent::Return(_, None, _)) => {}
        Some(flow) => bail!(flow.forbidden()),
        None => {}
    }

    Ok(output)
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
            Some(ast::Expr::Ident(ident)) => self.capture(ident.get(), Scopes::get),
            Some(ast::Expr::MathIdent(ident)) => {
                self.capture(ident.get(), Scopes::get_in_math)
            }

            // Code and content blocks create a scope.
            Some(ast::Expr::CodeBlock(_) | ast::Expr::ContentBlock(_)) => {
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
            Some(ast::Expr::LetBinding(expr)) => {
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
            Some(ast::Expr::ForLoop(expr)) => {
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
            Some(ast::Expr::ModuleImport(expr)) => {
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
        // The concrete value does not matter as we only use the scoping
        // mechanism of `Scopes`, not the values themselves.
        self.internal
            .top
            .bind(ident.get().clone(), Binding::detached(Value::None));
    }

    /// Capture a variable if it isn't internal.
    fn capture(
        &mut self,
        ident: &EcoString,
        getter: impl FnOnce(&'a Scopes<'a>, &str) -> HintedStrResult<&'a Binding>,
    ) {
        if self.internal.get(ident).is_ok() {
            return;
        }

        let binding = match self.external {
            Some(external) => match getter(external, ident) {
                Ok(binding) => binding.capture(self.capturer),
                Err(_) => return,
            },
            // The external scopes are only `None` when we are doing IDE capture
            // analysis, in which case the concrete value doesn't matter.
            None => Binding::detached(Value::None),
        };

        self.captures.bind(ident.clone(), binding);
    }
}

#[cfg(test)]
mod tests {
    use typst_syntax::parse;

    use super::*;

    #[track_caller]
    fn test(scopes: &Scopes, text: &str, result: &[&str]) {
        let mut visitor = CapturesVisitor::new(Some(scopes), Capturer::Function);
        let root = parse(text);
        visitor.visit(&root);

        let captures = visitor.finish();
        let mut names: Vec<_> = captures.iter().map(|(k, ..)| k).collect();
        names.sort();

        assert_eq!(names, result);
    }

    #[test]
    fn test_captures() {
        let mut scopes = Scopes::new(None);
        scopes.top.define("f", 0);
        scopes.top.define("x", 0);
        scopes.top.define("y", 0);
        scopes.top.define("z", 0);
        let s = &scopes;

        // Let binding and function definition.
        test(s, "#let x = x", &["x"]);
        test(s, "#let x; #(x + y)", &["y"]);
        test(s, "#let f(x, y) = x + y", &[]);
        test(s, "#let f(x, y) = f", &[]);
        test(s, "#let f = (x, y) => f", &["f"]);

        // Closure with different kinds of params.
        test(s, "#((x, y) => x + z)", &["z"]);
        test(s, "#((x: y, z) => x + z)", &["y"]);
        test(s, "#((..x) => x + y)", &["y"]);
        test(s, "#((x, y: x + z) => x + y)", &["x", "z"]);
        test(s, "#{x => x; x}", &["x"]);

        // Show rule.
        test(s, "#show y: x => x", &["y"]);
        test(s, "#show y: x => x + z", &["y", "z"]);
        test(s, "#show x: x => x", &["x"]);

        // For loop.
        test(s, "#for x in y { x + z }", &["y", "z"]);
        test(s, "#for (x, y) in y { x + y }", &["y"]);
        test(s, "#for x in y {} #x", &["x", "y"]);

        // Import.
        test(s, "#import z: x, y", &["z"]);
        test(s, "#import x + y: x, y, z", &["x", "y"]);

        // Blocks.
        test(s, "#{ let x = 1; { let y = 2; y }; x + y }", &["y"]);
        test(s, "#[#let x = 1]#x", &["x"]);

        // Field access.
        test(s, "#x.y.f(z)", &["x", "z"]);

        // Parenthesized expressions.
        test(s, "#f(x: 1)", &["f"]);
        test(s, "#(x: 1)", &[]);
        test(s, "#(x = 1)", &["x"]);
        test(s, "#(x += y)", &["x", "y"]);
        test(s, "#{ (x, z) = (y, 1) }", &["x", "y", "z"]);
        test(s, "#(x.at(y) = 5)", &["x", "y"]);
    }

    #[test]
    fn test_captures_in_math() {
        let mut scopes = Scopes::new(None);
        scopes.top.define("f", 0);
        scopes.top.define("x", 0);
        scopes.top.define("y", 0);
        scopes.top.define("z", 0);
        // Multi-letter variables are required for math.
        scopes.top.define("foo", 0);
        scopes.top.define("bar", 0);
        scopes.top.define("x-bar", 0);
        scopes.top.define("x_bar", 0);
        let s = &scopes;

        // Basic math identifier differences.
        test(s, "$ x f(z) $", &[]); // single letters not captured.
        test(s, "$ #x #f(z) $", &["f", "x", "z"]);
        test(s, "$ foo f(bar) $", &["bar", "foo"]);
        test(s, "$ #foo[#$bar$] $", &["bar", "foo"]);
        test(s, "$ #let foo = x; foo $", &["x"]);

        // Math idents don't have dashes/underscores
        test(s, "$ x-y x_y foo-x x_bar $", &["bar", "foo"]);
        test(s, "$ #x-bar #x_bar $", &["x-bar", "x_bar"]);

        // Named-params.
        test(s, "$ foo(bar: y) $", &["foo"]);
        test(s, "$ foo(x-y: 1, bar-z: 2) $", &["foo"]);

        // Field access in math.
        test(s, "$ foo.bar $", &["foo"]);
        test(s, "$ foo.x $", &["foo"]);
        test(s, "$ x.foo $", &["foo"]);
        test(s, "$ foo . bar $", &["bar", "foo"]);
        test(s, "$ foo.x.y.bar(z) $", &["foo"]);
        test(s, "$ foo.x-bar $", &["bar", "foo"]);
        test(s, "$ foo.x_bar $", &["bar", "foo"]);
        test(s, "$ #x_bar.x-bar $", &["x_bar"]);
    }
}
