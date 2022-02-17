//! Evaluation of markup into modules.

#[macro_use]
mod array;
#[macro_use]
mod dict;
#[macro_use]
mod value;
#[macro_use]
mod styles;
mod capture;
mod class;
mod collapse;
mod func;
mod ops;
mod scope;
mod show;
mod template;

pub use array::*;
pub use capture::*;
pub use class::*;
pub use collapse::*;
pub use dict::*;
pub use func::*;
pub use scope::*;
pub use show::*;
pub use styles::*;
pub use template::*;
pub use value::*;

use std::io;
use std::mem;

use unicode_segmentation::UnicodeSegmentation;

use crate::diag::{At, Error, StrResult, Trace, Tracepoint, TypResult};
use crate::geom::{Angle, Fractional, Length, Relative};
use crate::layout::Layout;
use crate::library::{self, ORDERED, UNORDERED};
use crate::syntax::ast::*;
use crate::syntax::{Span, Spanned};
use crate::util::EcoString;
use crate::Vm;

/// An evaluated module, ready for importing or conversion to a root layout
/// tree.
#[derive(Debug, Clone)]
pub struct Module {
    /// The top-level definitions that were bound in this module.
    pub scope: Scope,
    /// The module's layoutable contents.
    pub template: Template,
}

/// Evaluate an expression.
pub trait Eval {
    /// The output of evaluating the expression.
    type Output;

    /// Evaluate the expression to the output value.
    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output>;
}

impl Eval for Markup {
    type Output = Template;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        eval_markup(vm, &mut self.nodes())
    }
}

/// Evaluate a stream of markup nodes.
fn eval_markup(
    vm: &mut Vm,
    nodes: &mut impl Iterator<Item = MarkupNode>,
) -> TypResult<Template> {
    let mut seq = Vec::with_capacity(nodes.size_hint().1.unwrap_or_default());

    while let Some(node) = nodes.next() {
        seq.push(match node {
            MarkupNode::Expr(Expr::Set(set)) => {
                let class = set.class();
                let class = class.eval(vm)?.cast::<Class>().at(class.span())?;
                let args = set.args().eval(vm)?;
                let styles = class.set(args)?;
                let tail = eval_markup(vm, nodes)?;
                tail.styled_with_map(styles)
            }
            MarkupNode::Expr(Expr::Show(show)) => {
                return Err("show rules are not yet implemented").at(show.span());
            }
            MarkupNode::Expr(Expr::Wrap(wrap)) => {
                let tail = eval_markup(vm, nodes)?;
                vm.scopes.top.def_mut(wrap.binding().take(), tail);
                wrap.body().eval(vm)?.display()
            }
            _ => node.eval(vm)?,
        });
    }

    Ok(Template::sequence(seq))
}

impl Eval for MarkupNode {
    type Output = Template;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        Ok(match self {
            Self::Space => Template::Space,
            Self::Linebreak => Template::Linebreak,
            Self::Parbreak => Template::Parbreak,
            Self::Text(text) => Template::Text(text.clone()),
            Self::Strong(strong) => strong.eval(vm)?,
            Self::Emph(emph) => emph.eval(vm)?,
            Self::Raw(raw) => raw.eval(vm)?,
            Self::Math(math) => math.eval(vm)?,
            Self::Heading(heading) => heading.eval(vm)?,
            Self::List(list) => list.eval(vm)?,
            Self::Enum(enum_) => enum_.eval(vm)?,
            Self::Expr(expr) => expr.eval(vm)?.display(),
        })
    }
}

impl Eval for StrongNode {
    type Output = Template;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        Ok(Template::show(library::StrongNode(self.body().eval(vm)?)))
    }
}

impl Eval for EmphNode {
    type Output = Template;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        Ok(Template::show(library::EmphNode(self.body().eval(vm)?)))
    }
}

impl Eval for RawNode {
    type Output = Template;

    fn eval(&self, _: &mut Vm) -> TypResult<Self::Output> {
        let template = Template::show(library::RawNode {
            text: self.text.clone(),
            block: self.block,
        });
        Ok(match self.lang {
            Some(_) => template.styled(library::RawNode::LANG, self.lang.clone()),
            None => template,
        })
    }
}

impl Eval for MathNode {
    type Output = Template;

    fn eval(&self, _: &mut Vm) -> TypResult<Self::Output> {
        Ok(Template::show(library::MathNode {
            formula: self.formula.clone(),
            display: self.display,
        }))
    }
}

impl Eval for HeadingNode {
    type Output = Template;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        Ok(Template::show(library::HeadingNode {
            body: self.body().eval(vm)?,
            level: self.level(),
        }))
    }
}

impl Eval for ListNode {
    type Output = Template;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        Ok(Template::show(library::ListNode::<UNORDERED> {
            number: None,
            child: self.body().eval(vm)?.pack(),
        }))
    }
}

impl Eval for EnumNode {
    type Output = Template;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        Ok(Template::show(library::ListNode::<ORDERED> {
            number: self.number(),
            child: self.body().eval(vm)?.pack(),
        }))
    }
}

impl Eval for Expr {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        match self {
            Self::Lit(v) => v.eval(vm),
            Self::Ident(v) => v.eval(vm),
            Self::Array(v) => v.eval(vm).map(Value::Array),
            Self::Dict(v) => v.eval(vm).map(Value::Dict),
            Self::Template(v) => v.eval(vm).map(Value::Template),
            Self::Group(v) => v.eval(vm),
            Self::Block(v) => v.eval(vm),
            Self::Call(v) => v.eval(vm),
            Self::Closure(v) => v.eval(vm),
            Self::With(v) => v.eval(vm),
            Self::Unary(v) => v.eval(vm),
            Self::Binary(v) => v.eval(vm),
            Self::Let(v) => v.eval(vm),
            Self::Set(v) => v.eval(vm),
            Self::Show(v) => v.eval(vm),
            Self::Wrap(v) => v.eval(vm),
            Self::If(v) => v.eval(vm),
            Self::While(v) => v.eval(vm),
            Self::For(v) => v.eval(vm),
            Self::Import(v) => v.eval(vm),
            Self::Include(v) => v.eval(vm),
            Self::Break(v) => v.eval(vm),
            Self::Continue(v) => v.eval(vm),
            Self::Return(v) => v.eval(vm),
        }
    }
}

impl Eval for Lit {
    type Output = Value;

    fn eval(&self, _: &mut Vm) -> TypResult<Self::Output> {
        Ok(match self.kind() {
            LitKind::None => Value::None,
            LitKind::Auto => Value::Auto,
            LitKind::Bool(v) => Value::Bool(v),
            LitKind::Int(v) => Value::Int(v),
            LitKind::Float(v) => Value::Float(v),
            LitKind::Length(v, unit) => Value::Length(Length::with_unit(v, unit)),
            LitKind::Angle(v, unit) => Value::Angle(Angle::with_unit(v, unit)),
            LitKind::Percent(v) => Value::Relative(Relative::new(v / 100.0)),
            LitKind::Fractional(v) => Value::Fractional(Fractional::new(v)),
            LitKind::Str(ref v) => Value::Str(v.clone()),
        })
    }
}

impl Eval for Ident {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        match vm.scopes.get(self) {
            Some(slot) => Ok(slot.read().unwrap().clone()),
            None => bail!(self.span(), "unknown variable"),
        }
    }
}

impl Eval for ArrayExpr {
    type Output = Array;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        self.items().map(|expr| expr.eval(vm)).collect()
    }
}

impl Eval for DictExpr {
    type Output = Dict;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        self.items()
            .map(|x| Ok((x.name().take(), x.expr().eval(vm)?)))
            .collect()
    }
}

impl Eval for TemplateExpr {
    type Output = Template;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        vm.scopes.enter();
        let template = self.body().eval(vm)?;
        vm.scopes.exit();
        Ok(template)
    }
}

impl Eval for GroupExpr {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        self.expr().eval(vm)
    }
}

impl Eval for BlockExpr {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        vm.scopes.enter();

        let mut output = Value::None;
        for expr in self.exprs() {
            let value = expr.eval(vm)?;
            output = ops::join(output, value).at(expr.span())?;
        }

        vm.scopes.exit();

        Ok(output)
    }
}

impl Eval for UnaryExpr {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        let value = self.expr().eval(vm)?;
        let result = match self.op() {
            UnOp::Pos => ops::pos(value),
            UnOp::Neg => ops::neg(value),
            UnOp::Not => ops::not(value),
        };
        result.at(self.span())
    }
}

impl Eval for BinaryExpr {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        match self.op() {
            BinOp::Add => self.apply(vm, ops::add),
            BinOp::Sub => self.apply(vm, ops::sub),
            BinOp::Mul => self.apply(vm, ops::mul),
            BinOp::Div => self.apply(vm, ops::div),
            BinOp::And => self.apply(vm, ops::and),
            BinOp::Or => self.apply(vm, ops::or),
            BinOp::Eq => self.apply(vm, ops::eq),
            BinOp::Neq => self.apply(vm, ops::neq),
            BinOp::Lt => self.apply(vm, ops::lt),
            BinOp::Leq => self.apply(vm, ops::leq),
            BinOp::Gt => self.apply(vm, ops::gt),
            BinOp::Geq => self.apply(vm, ops::geq),
            BinOp::Assign => self.assign(vm, |_, b| Ok(b)),
            BinOp::AddAssign => self.assign(vm, ops::add),
            BinOp::SubAssign => self.assign(vm, ops::sub),
            BinOp::MulAssign => self.assign(vm, ops::mul),
            BinOp::DivAssign => self.assign(vm, ops::div),
        }
    }
}

impl BinaryExpr {
    /// Apply a basic binary operation.
    fn apply(
        &self,
        vm: &mut Vm,
        op: fn(Value, Value) -> StrResult<Value>,
    ) -> TypResult<Value> {
        let lhs = self.lhs().eval(vm)?;

        // Short-circuit boolean operations.
        if (self.op() == BinOp::And && lhs == Value::Bool(false))
            || (self.op() == BinOp::Or && lhs == Value::Bool(true))
        {
            return Ok(lhs);
        }

        let rhs = self.rhs().eval(vm)?;
        op(lhs, rhs).at(self.span())
    }

    /// Apply an assignment operation.
    fn assign(
        &self,
        vm: &mut Vm,
        op: fn(Value, Value) -> StrResult<Value>,
    ) -> TypResult<Value> {
        let rhs = self.rhs().eval(vm)?;
        self.lhs().access(
            vm,
            Box::new(|target| {
                let lhs = mem::take(&mut *target);
                *target = op(lhs, rhs).at(self.span())?;
                Ok(())
            }),
        )?;
        Ok(Value::None)
    }
}

impl Eval for CallExpr {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        let span = self.callee().span();
        let callee = self.callee().eval(vm)?;
        let args = self.args().eval(vm)?;

        match callee {
            Value::Array(array) => {
                array.get(args.into_index()?).map(Value::clone).at(self.span())
            }

            Value::Dict(dict) => {
                dict.get(args.into_key()?).map(Value::clone).at(self.span())
            }

            Value::Func(func) => {
                let point = || Tracepoint::Call(func.name().map(ToString::to_string));
                func.call(vm, args).trace(point, self.span())
            }

            Value::Class(class) => {
                let point = || Tracepoint::Call(Some(class.name().to_string()));
                class.construct(vm, args).trace(point, self.span())
            }

            v => bail!(
                span,
                "expected callable or collection, found {}",
                v.type_name(),
            ),
        }
    }
}

impl Eval for CallArgs {
    type Output = Args;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        let mut items = Vec::new();

        for arg in self.items() {
            let span = arg.span();
            match arg {
                CallArg::Pos(expr) => {
                    items.push(Arg {
                        span,
                        name: None,
                        value: Spanned::new(expr.eval(vm)?, expr.span()),
                    });
                }
                CallArg::Named(named) => {
                    items.push(Arg {
                        span,
                        name: Some(named.name().take()),
                        value: Spanned::new(named.expr().eval(vm)?, named.expr().span()),
                    });
                }
                CallArg::Spread(expr) => match expr.eval(vm)? {
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
                    v => bail!(expr.span(), "cannot spread {}", v.type_name()),
                },
            }
        }

        Ok(Args { span: self.span(), items })
    }
}

impl Eval for ClosureExpr {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        // The closure's name is defined by its let binding if there's one.
        let name = self.name().map(Ident::take);

        // Collect captured variables.
        let captured = {
            let mut visitor = CapturesVisitor::new(&vm.scopes);
            visitor.visit(self.as_red());
            visitor.finish()
        };

        let mut params = Vec::new();
        let mut sink = None;

        // Collect parameters and an optional sink parameter.
        for param in self.params() {
            match param {
                ClosureParam::Pos(name) => {
                    params.push((name.take(), None));
                }
                ClosureParam::Named(named) => {
                    params.push((named.name().take(), Some(named.expr().eval(vm)?)));
                }
                ClosureParam::Sink(name) => {
                    if sink.is_some() {
                        bail!(name.span(), "only one argument sink is allowed");
                    }
                    sink = Some(name.take());
                }
            }
        }

        // Define the actual function.
        Ok(Value::Func(Func::closure(Closure {
            name,
            captured,
            params,
            sink,
            body: self.body(),
        })))
    }
}

impl Eval for WithExpr {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        let callee = self.callee();
        let func = callee.eval(vm)?.cast::<Func>().at(callee.span())?;
        let args = self.args().eval(vm)?;
        Ok(Value::Func(func.with(args)))
    }
}

impl Eval for LetExpr {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        let value = match self.init() {
            Some(expr) => expr.eval(vm)?,
            None => Value::None,
        };
        vm.scopes.top.def_mut(self.binding().take(), value);
        Ok(Value::None)
    }
}

impl Eval for SetExpr {
    type Output = Value;

    fn eval(&self, _: &mut Vm) -> TypResult<Self::Output> {
        Err("set is only allowed directly in markup").at(self.span())
    }
}

impl Eval for ShowExpr {
    type Output = Value;

    fn eval(&self, _: &mut Vm) -> TypResult<Self::Output> {
        Err("show is only allowed directly in markup").at(self.span())
    }
}

impl Eval for WrapExpr {
    type Output = Value;

    fn eval(&self, _: &mut Vm) -> TypResult<Self::Output> {
        Err("wrap is only allowed directly in markup").at(self.span())
    }
}

impl Eval for IfExpr {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        let condition = self.condition();
        if condition.eval(vm)?.cast::<bool>().at(condition.span())? {
            self.if_body().eval(vm)
        } else if let Some(else_body) = self.else_body() {
            else_body.eval(vm)
        } else {
            Ok(Value::None)
        }
    }
}

impl Eval for WhileExpr {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        let mut output = Value::None;

        let condition = self.condition();
        while condition.eval(vm)?.cast::<bool>().at(condition.span())? {
            let body = self.body();
            let value = body.eval(vm)?;
            output = ops::join(output, value).at(body.span())?;
        }

        Ok(output)
    }
}

impl Eval for ForExpr {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        macro_rules! iter {
            (for ($($binding:ident => $value:ident),*) in $iter:expr) => {{
                let mut output = Value::None;
                vm.scopes.enter();

                #[allow(unused_parens)]
                for ($($value),*) in $iter {
                    $(vm.scopes.top.def_mut(&$binding, $value);)*

                    let value = self.body().eval(vm)?;
                    output = ops::join(output, value)
                        .at(self.body().span())?;
                }

                vm.scopes.exit();
                return Ok(output);
            }};
        }

        let iter = self.iter().eval(vm)?;
        let pattern = self.pattern();
        let key = pattern.key().map(Ident::take);
        let value = pattern.value().take();

        match (key, value, iter) {
            (None, v, Value::Str(string)) => {
                iter!(for (v => value) in string.graphemes(true));
            }
            (None, v, Value::Array(array)) => {
                iter!(for (v => value) in array.into_iter());
            }
            (Some(i), v, Value::Array(array)) => {
                iter!(for (i => idx, v => value) in array.into_iter().enumerate());
            }
            (None, v, Value::Dict(dict)) => {
                iter!(for (v => value) in dict.into_iter().map(|p| p.1));
            }
            (Some(k), v, Value::Dict(dict)) => {
                iter!(for (k => key, v => value) in dict.into_iter());
            }
            (None, v, Value::Args(args)) => {
                iter!(for (v => value) in args.items.into_iter()
                    .filter(|arg| arg.name.is_none())
                    .map(|arg| arg.value.v));
            }
            (Some(k), v, Value::Args(args)) => {
                iter!(for (k => key, v => value) in args.items.into_iter()
                    .map(|arg| (arg.name.map_or(Value::None, Value::Str), arg.value.v)));
            }
            (_, _, Value::Str(_)) => {
                bail!(pattern.span(), "mismatched pattern");
            }
            (_, _, iter) => {
                bail!(self.iter().span(), "cannot loop over {}", iter.type_name());
            }
        }
    }
}

impl Eval for ImportExpr {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        let span = self.path().span();
        let path = self.path().eval(vm)?.cast::<EcoString>().at(span)?;
        let module = import(vm, &path, span)?;

        match self.imports() {
            Imports::Wildcard => {
                for (var, slot) in module.scope.iter() {
                    vm.scopes.top.def_mut(var, slot.read().unwrap().clone());
                }
            }
            Imports::Items(idents) => {
                for ident in idents {
                    if let Some(slot) = module.scope.get(&ident) {
                        vm.scopes.top.def_mut(ident.take(), slot.read().unwrap().clone());
                    } else {
                        bail!(ident.span(), "unresolved import");
                    }
                }
            }
        }

        Ok(Value::None)
    }
}

impl Eval for IncludeExpr {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> TypResult<Self::Output> {
        let span = self.path().span();
        let path = self.path().eval(vm)?.cast::<EcoString>().at(span)?;
        let module = import(vm, &path, span)?;
        Ok(Value::Template(module.template.clone()))
    }
}

/// Process an import of a module relative to the current location.
fn import(vm: &mut Vm, path: &str, span: Span) -> TypResult<Module> {
    // Load the source file.
    let full = vm.resolve(path);
    let id = vm.sources.load(&full).map_err(|err| {
        Error::boxed(span, match err.kind() {
            io::ErrorKind::NotFound => "file not found".into(),
            _ => format!("failed to load source file ({})", err),
        })
    })?;

    // Prevent cyclic importing.
    if vm.route.contains(&id) {
        bail!(span, "cyclic import");
    }

    // Evaluate the file.
    vm.evaluate(id).trace(|| Tracepoint::Import, span)
}

impl Eval for BreakExpr {
    type Output = Value;

    fn eval(&self, _: &mut Vm) -> TypResult<Self::Output> {
        Err("break is not yet implemented").at(self.span())
    }
}

impl Eval for ContinueExpr {
    type Output = Value;

    fn eval(&self, _: &mut Vm) -> TypResult<Self::Output> {
        Err("continue is not yet implemented").at(self.span())
    }
}

impl Eval for ReturnExpr {
    type Output = Value;

    fn eval(&self, _: &mut Vm) -> TypResult<Self::Output> {
        Err("return is not yet implemented").at(self.span())
    }
}

/// Try to mutably access the value an expression points to.
///
/// This only works if the expression is a valid lvalue.
pub trait Access {
    /// Try to access the value.
    fn access(&self, vm: &mut Vm, f: Handler) -> TypResult<()>;
}

/// Process an accessed value.
type Handler<'a> = Box<dyn FnOnce(&mut Value) -> TypResult<()> + 'a>;

impl Access for Expr {
    fn access(&self, vm: &mut Vm, f: Handler) -> TypResult<()> {
        match self {
            Expr::Ident(ident) => ident.access(vm, f),
            Expr::Call(call) => call.access(vm, f),
            _ => bail!(self.span(), "cannot access this expression mutably"),
        }
    }
}

impl Access for Ident {
    fn access(&self, vm: &mut Vm, f: Handler) -> TypResult<()> {
        match vm.scopes.get(self) {
            Some(slot) => match slot.try_write() {
                Ok(mut guard) => f(&mut guard),
                Err(_) => bail!(self.span(), "cannot mutate a constant"),
            },
            None => bail!(self.span(), "unknown variable"),
        }
    }
}

impl Access for CallExpr {
    fn access(&self, vm: &mut Vm, f: Handler) -> TypResult<()> {
        let args = self.args().eval(vm)?;
        self.callee().access(
            vm,
            Box::new(|value| match value {
                Value::Array(array) => {
                    f(array.get_mut(args.into_index()?).at(self.span())?)
                }
                Value::Dict(dict) => f(dict.get_mut(args.into_key()?)),
                v => bail!(
                    self.callee().span(),
                    "expected collection, found {}",
                    v.type_name(),
                ),
            }),
        )
    }
}
