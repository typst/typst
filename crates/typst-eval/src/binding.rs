use std::collections::HashSet;

use ecow::eco_format;
use typst_library::diag::{bail, error, At, SourceDiagnostic, SourceResult};
use typst_library::foundations::{Array, Dict, Value};
use typst_syntax::ast::{self, AstNode};

use crate::{Access, Eval, Vm};

impl Eval for ast::LetBinding<'_> {
    type Output = Value;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let value = match self.init() {
            Some(expr) => expr.eval(vm)?,
            None => Value::None,
        };
        if vm.flow.is_some() {
            return Ok(Value::None);
        }

        match self.kind() {
            ast::LetBindingKind::Normal(pattern) => destructure(vm, pattern, value)?,
            ast::LetBindingKind::Closure(ident) => vm.define(ident, value),
        }

        Ok(Value::None)
    }
}

impl Eval for ast::DestructAssignment<'_> {
    type Output = Value;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let value = self.value().eval(vm)?;
        destructure_impl(vm, self.pattern(), value, &mut |vm, expr, value| {
            let location = expr.access(vm)?;
            *location = value;
            Ok(())
        })?;
        Ok(Value::None)
    }
}

/// Destructures a value into a pattern.
pub(crate) fn destructure(
    vm: &mut Vm,
    pattern: ast::Pattern,
    value: Value,
) -> SourceResult<()> {
    destructure_impl(vm, pattern, value, &mut |vm, expr, value| match expr {
        ast::Expr::Ident(ident) => {
            vm.define(ident, value);
            Ok(())
        }
        _ => bail!(expr.span(), "cannot assign to this expression"),
    })
}

/// Destruct the given value into the pattern and apply the function to each binding.
fn destructure_impl<F>(
    vm: &mut Vm,
    pattern: ast::Pattern,
    value: Value,
    f: &mut F,
) -> SourceResult<()>
where
    F: Fn(&mut Vm, ast::Expr, Value) -> SourceResult<()>,
{
    match pattern {
        ast::Pattern::Normal(expr) => f(vm, expr, value)?,
        ast::Pattern::Placeholder(_) => {}
        ast::Pattern::Parenthesized(parenthesized) => {
            destructure_impl(vm, parenthesized.pattern(), value, f)?
        }
        ast::Pattern::Destructuring(destruct) => match value {
            Value::Array(value) => destructure_array(vm, destruct, value, f)?,
            Value::Dict(value) => destructure_dict(vm, destruct, value, f)?,
            _ => bail!(pattern.span(), "cannot destructure {}", value.ty()),
        },
    }
    Ok(())
}

fn destructure_array<F>(
    vm: &mut Vm,
    destruct: ast::Destructuring,
    value: Array,
    f: &mut F,
) -> SourceResult<()>
where
    F: Fn(&mut Vm, ast::Expr, Value) -> SourceResult<()>,
{
    let len = value.as_slice().len();
    let mut i = 0;

    for p in destruct.items() {
        match p {
            ast::DestructuringItem::Pattern(pattern) => {
                let Ok(v) = value.at(i as i64, None) else {
                    bail!(wrong_number_of_elements(destruct, len));
                };
                destructure_impl(vm, pattern, v, f)?;
                i += 1;
            }
            ast::DestructuringItem::Spread(spread) => {
                let sink_size = (1 + len).checked_sub(destruct.items().count());
                let sink = sink_size.and_then(|s| value.as_slice().get(i..i + s));
                let (Some(sink_size), Some(sink)) = (sink_size, sink) else {
                    bail!(wrong_number_of_elements(destruct, len));
                };
                if let Some(expr) = spread.sink_expr() {
                    f(vm, expr, Value::Array(sink.into()))?;
                }
                i += sink_size;
            }
            ast::DestructuringItem::Named(named) => {
                bail!(named.span(), "cannot destructure named pattern from an array")
            }
        }
    }

    if i < len {
        bail!(wrong_number_of_elements(destruct, len));
    }

    Ok(())
}

fn destructure_dict<F>(
    vm: &mut Vm,
    destruct: ast::Destructuring,
    dict: Dict,
    f: &mut F,
) -> SourceResult<()>
where
    F: Fn(&mut Vm, ast::Expr, Value) -> SourceResult<()>,
{
    let mut sink = None;
    let mut used = HashSet::new();

    for p in destruct.items() {
        match p {
            // Shorthand for a direct identifier.
            ast::DestructuringItem::Pattern(ast::Pattern::Normal(ast::Expr::Ident(
                ident,
            ))) => {
                let v = dict.get(&ident).at(ident.span())?;
                f(vm, ast::Expr::Ident(ident), v.clone())?;
                used.insert(ident.get().clone());
            }
            ast::DestructuringItem::Named(named) => {
                let name = named.name();
                let v = dict.get(&name).at(name.span())?;
                destructure_impl(vm, named.pattern(), v.clone(), f)?;
                used.insert(name.get().clone());
            }
            ast::DestructuringItem::Spread(spread) => sink = spread.sink_expr(),
            ast::DestructuringItem::Pattern(expr) => {
                bail!(expr.span(), "cannot destructure unnamed pattern from dictionary");
            }
        }
    }

    if let Some(expr) = sink {
        let mut sink = Dict::new();
        for (key, value) in dict {
            if !used.contains(key.as_str()) {
                sink.insert(key, value);
            }
        }
        f(vm, expr, Value::Dict(sink))?;
    }

    Ok(())
}

/// The error message when the number of elements of the destructuring and the
/// array is mismatched.
#[cold]
fn wrong_number_of_elements(
    destruct: ast::Destructuring,
    len: usize,
) -> SourceDiagnostic {
    let mut count = 0;
    let mut spread = false;

    for p in destruct.items() {
        match p {
            ast::DestructuringItem::Pattern(_) => count += 1,
            ast::DestructuringItem::Spread(_) => spread = true,
            ast::DestructuringItem::Named(_) => {}
        }
    }

    let quantifier = if len > count { "too many" } else { "not enough" };
    let expected = match (spread, count) {
        (true, 1) => "at least 1 element".into(),
        (true, c) => eco_format!("at least {c} elements"),
        (false, 0) => "an empty array".into(),
        (false, 1) => "a single element".into(),
        (false, c) => eco_format!("{c} elements",),
    };

    error!(
        destruct.span(), "{quantifier} elements to destructure";
        hint: "the provided array has a length of {len}, \
               but the pattern expects {expected}",
    )
}
