use typst_library::diag::{At, HintedStrResult, SourceResult};
use typst_library::foundations::{ops, IntoValue, Value};
use typst_syntax::ast::{self, AstNode};

use crate::{access_dict, Access, Eval, Vm};

impl Eval for ast::Unary<'_> {
    type Output = Value;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let value = self.expr().eval(vm)?;
        let result = match self.op() {
            ast::UnOp::Pos => ops::pos(value),
            ast::UnOp::Neg => ops::neg(value),
            ast::UnOp::Not => ops::not(value),
        };
        result.at(self.span())
    }
}

impl Eval for ast::Binary<'_> {
    type Output = Value;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        match self.op() {
            ast::BinOp::Add => apply_binary(self, vm, ops::add),
            ast::BinOp::Sub => apply_binary(self, vm, ops::sub),
            ast::BinOp::Mul => apply_binary(self, vm, ops::mul),
            ast::BinOp::Div => apply_binary(self, vm, ops::div),
            ast::BinOp::And => apply_binary(self, vm, ops::and),
            ast::BinOp::Or => apply_binary(self, vm, ops::or),
            ast::BinOp::Eq => apply_binary(self, vm, ops::eq),
            ast::BinOp::Neq => apply_binary(self, vm, ops::neq),
            ast::BinOp::Lt => apply_binary(self, vm, ops::lt),
            ast::BinOp::Leq => apply_binary(self, vm, ops::leq),
            ast::BinOp::Gt => apply_binary(self, vm, ops::gt),
            ast::BinOp::Geq => apply_binary(self, vm, ops::geq),
            ast::BinOp::In => apply_binary(self, vm, ops::in_),
            ast::BinOp::NotIn => apply_binary(self, vm, ops::not_in),
            ast::BinOp::Assign => apply_assignment(self, vm, |_, b| Ok(b)),
            ast::BinOp::AddAssign => apply_assignment(self, vm, ops::add),
            ast::BinOp::SubAssign => apply_assignment(self, vm, ops::sub),
            ast::BinOp::MulAssign => apply_assignment(self, vm, ops::mul),
            ast::BinOp::DivAssign => apply_assignment(self, vm, ops::div),
        }
    }
}

/// Apply a basic binary operation.
fn apply_binary(
    binary: ast::Binary,
    vm: &mut Vm,
    op: fn(Value, Value) -> HintedStrResult<Value>,
) -> SourceResult<Value> {
    let lhs = binary.lhs().eval(vm)?;

    // Short-circuit boolean operations.
    if (binary.op() == ast::BinOp::And && lhs == false.into_value())
        || (binary.op() == ast::BinOp::Or && lhs == true.into_value())
    {
        return Ok(lhs);
    }

    let rhs = binary.rhs().eval(vm)?;
    op(lhs, rhs).at(binary.span())
}

/// Apply an assignment operation.
fn apply_assignment(
    binary: ast::Binary,
    vm: &mut Vm,
    op: fn(Value, Value) -> HintedStrResult<Value>,
) -> SourceResult<Value> {
    let rhs = binary.rhs().eval(vm)?;
    let lhs = binary.lhs();

    // An assignment to a dictionary field is different from a normal access
    // since it can create the field instead of just modifying it.
    if binary.op() == ast::BinOp::Assign {
        if let ast::Expr::FieldAccess(access) = lhs {
            let dict = access_dict(vm, access)?;
            dict.insert(access.field().get().clone().into(), rhs);
            return Ok(Value::None);
        }
    }

    let location = binary.lhs().access(vm)?;
    let lhs = std::mem::take(&mut *location);
    *location = op(lhs, rhs).at(binary.span())?;
    Ok(Value::None)
}
