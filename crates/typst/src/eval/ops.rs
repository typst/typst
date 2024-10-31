//! Operations on values.

use std::cmp::Ordering;

use ecow::eco_format;

use crate::diag::{bail, At, HintedStrResult, SourceResult, StrResult};
use crate::eval::{access_dict, Access, Eval, Vm};
use crate::foundations::{format_str, Datetime, IntoValue, Regex, Repr, Value};
use crate::layout::{Alignment, Length, Rel};
use crate::syntax::ast::{self, AstNode};
use crate::text::TextElem;
use crate::utils::Numeric;
use crate::visualize::Stroke;

impl Eval for ast::Unary<'_> {
    type Output = Value;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let value = self.expr().eval(vm)?;
        let result = match self.op() {
            ast::UnOp::Pos => pos(value),
            ast::UnOp::Neg => neg(value),
            ast::UnOp::Not => not(value),
        };
        result.at(self.span())
    }
}

impl Eval for ast::Binary<'_> {
    type Output = Value;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        match self.op() {
            ast::BinOp::Add => apply_binary(self, vm, add),
            ast::BinOp::Sub => apply_binary(self, vm, sub),
            ast::BinOp::Mul => apply_binary(self, vm, mul),
            ast::BinOp::Div => apply_binary(self, vm, div),
            ast::BinOp::And => apply_binary(self, vm, and),
            ast::BinOp::Or => apply_binary(self, vm, or),
            ast::BinOp::Eq => apply_binary(self, vm, eq),
            ast::BinOp::Neq => apply_binary(self, vm, neq),
            ast::BinOp::Lt => apply_binary(self, vm, lt),
            ast::BinOp::Leq => apply_binary(self, vm, leq),
            ast::BinOp::Gt => apply_binary(self, vm, gt),
            ast::BinOp::Geq => apply_binary(self, vm, geq),
            ast::BinOp::In => apply_binary(self, vm, in_),
            ast::BinOp::NotIn => apply_binary(self, vm, not_in),
            ast::BinOp::Assign => apply_assignment(self, vm, |_, b| Ok(b)),
            ast::BinOp::AddAssign => apply_assignment(self, vm, add),
            ast::BinOp::SubAssign => apply_assignment(self, vm, sub),
            ast::BinOp::MulAssign => apply_assignment(self, vm, mul),
            ast::BinOp::DivAssign => apply_assignment(self, vm, div),
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

/// Bail with a type mismatch error.
macro_rules! mismatch {
    ($fmt:expr, $($value:expr),* $(,)?) => {
        return Err(eco_format!($fmt, $($value.ty()),*).into())
    };
}

/// Join a value with another value.
pub fn join(lhs: Value, rhs: Value) -> StrResult<Value> {
    use Value::*;
    Ok(match (lhs, rhs) {
        (a, None) => a,
        (None, b) => b,
        (Symbol(a), Symbol(b)) => Str(format_str!("{a}{b}")),
        (Str(a), Str(b)) => Str(a + b),
        (Str(a), Symbol(b)) => Str(format_str!("{a}{b}")),
        (Symbol(a), Str(b)) => Str(format_str!("{a}{b}")),
        (Bytes(a), Bytes(b)) => Bytes(a + b),
        (Content(a), Content(b)) => Content(a + b),
        (Content(a), Symbol(b)) => Content(a + TextElem::packed(b.get())),
        (Content(a), Str(b)) => Content(a + TextElem::packed(b)),
        (Str(a), Content(b)) => Content(TextElem::packed(a) + b),
        (Symbol(a), Content(b)) => Content(TextElem::packed(a.get()) + b),
        (Array(a), Array(b)) => Array(a + b),
        (Dict(a), Dict(b)) => Dict(a + b),

        // Type compatibility.
        (Type(a), Str(b)) => Str(format_str!("{a}{b}")),
        (Str(a), Type(b)) => Str(format_str!("{a}{b}")),

        (a, b) => mismatch!("cannot join {} with {}", a, b),
    })
}

/// Apply the unary plus operator to a value.
pub fn pos(value: Value) -> HintedStrResult<Value> {
    use Value::*;
    Ok(match value {
        Int(v) => Int(v),
        Float(v) => Float(v),
        Decimal(v) => Decimal(v),
        Length(v) => Length(v),
        Angle(v) => Angle(v),
        Ratio(v) => Ratio(v),
        Relative(v) => Relative(v),
        Fraction(v) => Fraction(v),
        Symbol(_) | Str(_) | Bytes(_) | Content(_) | Array(_) | Dict(_) | Datetime(_) => {
            mismatch!("cannot apply unary '+' to {}", value)
        }
        Dyn(d) => {
            if d.is::<Alignment>() {
                mismatch!("cannot apply unary '+' to {}", d)
            } else {
                mismatch!("cannot apply '+' to {}", d)
            }
        }
        v => mismatch!("cannot apply '+' to {}", v),
    })
}

/// Compute the negation of a value.
pub fn neg(value: Value) -> HintedStrResult<Value> {
    use Value::*;
    Ok(match value {
        Int(v) => Int(v.checked_neg().ok_or_else(too_large)?),
        Float(v) => Float(-v),
        Decimal(v) => Decimal(-v),
        Length(v) => Length(-v),
        Angle(v) => Angle(-v),
        Ratio(v) => Ratio(-v),
        Relative(v) => Relative(-v),
        Fraction(v) => Fraction(-v),
        Duration(v) => Duration(-v),
        Datetime(_) => mismatch!("cannot apply unary '-' to {}", value),
        v => mismatch!("cannot apply '-' to {}", v),
    })
}

/// Compute the sum of two values.
pub fn add(lhs: Value, rhs: Value) -> HintedStrResult<Value> {
    use Value::*;
    Ok(match (lhs, rhs) {
        (a, None) => a,
        (None, b) => b,

        (Int(a), Int(b)) => Int(a.checked_add(b).ok_or_else(too_large)?),
        (Int(a), Float(b)) => Float(a as f64 + b),
        (Float(a), Int(b)) => Float(a + b as f64),
        (Float(a), Float(b)) => Float(a + b),

        (Decimal(a), Decimal(b)) => Decimal(a.checked_add(b).ok_or_else(too_large)?),
        (Decimal(a), Int(b)) => Decimal(
            a.checked_add(crate::foundations::Decimal::from(b))
                .ok_or_else(too_large)?,
        ),
        (Int(a), Decimal(b)) => Decimal(
            crate::foundations::Decimal::from(a)
                .checked_add(b)
                .ok_or_else(too_large)?,
        ),

        (Angle(a), Angle(b)) => Angle(a + b),

        (Length(a), Length(b)) => Length(a + b),
        (Length(a), Ratio(b)) => Relative(b + a),
        (Length(a), Relative(b)) => Relative(b + a),

        (Ratio(a), Length(b)) => Relative(a + b),
        (Ratio(a), Ratio(b)) => Ratio(a + b),
        (Ratio(a), Relative(b)) => Relative(b + a),

        (Relative(a), Length(b)) => Relative(a + b),
        (Relative(a), Ratio(b)) => Relative(a + b),
        (Relative(a), Relative(b)) => Relative(a + b),

        (Fraction(a), Fraction(b)) => Fraction(a + b),

        (Symbol(a), Symbol(b)) => Str(format_str!("{a}{b}")),
        (Str(a), Str(b)) => Str(a + b),
        (Str(a), Symbol(b)) => Str(format_str!("{a}{b}")),
        (Symbol(a), Str(b)) => Str(format_str!("{a}{b}")),
        (Bytes(a), Bytes(b)) => Bytes(a + b),
        (Content(a), Content(b)) => Content(a + b),
        (Content(a), Symbol(b)) => Content(a + TextElem::packed(b.get())),
        (Content(a), Str(b)) => Content(a + TextElem::packed(b)),
        (Str(a), Content(b)) => Content(TextElem::packed(a) + b),
        (Symbol(a), Content(b)) => Content(TextElem::packed(a.get()) + b),

        (Array(a), Array(b)) => Array(a + b),
        (Dict(a), Dict(b)) => Dict(a + b),

        (Color(color), Length(thickness)) | (Length(thickness), Color(color)) => {
            Stroke::from_pair(color, thickness).into_value()
        }
        (Gradient(gradient), Length(thickness))
        | (Length(thickness), Gradient(gradient)) => {
            Stroke::from_pair(gradient, thickness).into_value()
        }
        (Pattern(pattern), Length(thickness)) | (Length(thickness), Pattern(pattern)) => {
            Stroke::from_pair(pattern, thickness).into_value()
        }

        (Duration(a), Duration(b)) => Duration(a + b),
        (Datetime(a), Duration(b)) => Datetime(a + b),
        (Duration(a), Datetime(b)) => Datetime(b + a),

        // Type compatibility.
        (Type(a), Str(b)) => Str(format_str!("{a}{b}")),
        (Str(a), Type(b)) => Str(format_str!("{a}{b}")),

        (Dyn(a), Dyn(b)) => {
            // Alignments can be summed.
            if let (Some(&a), Some(&b)) =
                (a.downcast::<Alignment>(), b.downcast::<Alignment>())
            {
                return Ok((a + b)?.into_value());
            }

            mismatch!("cannot add {} and {}", a, b);
        }

        (a, b) => mismatch!("cannot add {} and {}", a, b),
    })
}

/// Compute the difference of two values.
pub fn sub(lhs: Value, rhs: Value) -> HintedStrResult<Value> {
    use Value::*;
    Ok(match (lhs, rhs) {
        (Int(a), Int(b)) => Int(a.checked_sub(b).ok_or_else(too_large)?),
        (Int(a), Float(b)) => Float(a as f64 - b),
        (Float(a), Int(b)) => Float(a - b as f64),
        (Float(a), Float(b)) => Float(a - b),

        (Decimal(a), Decimal(b)) => Decimal(a.checked_sub(b).ok_or_else(too_large)?),
        (Decimal(a), Int(b)) => Decimal(
            a.checked_sub(crate::foundations::Decimal::from(b))
                .ok_or_else(too_large)?,
        ),
        (Int(a), Decimal(b)) => Decimal(
            crate::foundations::Decimal::from(a)
                .checked_sub(b)
                .ok_or_else(too_large)?,
        ),

        (Angle(a), Angle(b)) => Angle(a - b),

        (Length(a), Length(b)) => Length(a - b),
        (Length(a), Ratio(b)) => Relative(-b + a),
        (Length(a), Relative(b)) => Relative(-b + a),

        (Ratio(a), Length(b)) => Relative(a + -b),
        (Ratio(a), Ratio(b)) => Ratio(a - b),
        (Ratio(a), Relative(b)) => Relative(-b + a),

        (Relative(a), Length(b)) => Relative(a + -b),
        (Relative(a), Ratio(b)) => Relative(a + -b),
        (Relative(a), Relative(b)) => Relative(a - b),

        (Fraction(a), Fraction(b)) => Fraction(a - b),

        (Duration(a), Duration(b)) => Duration(a - b),
        (Datetime(a), Duration(b)) => Datetime(a - b),
        (Datetime(a), Datetime(b)) => Duration((a - b)?),

        (a, b) => mismatch!("cannot subtract {1} from {0}", a, b),
    })
}

/// Compute the product of two values.
pub fn mul(lhs: Value, rhs: Value) -> HintedStrResult<Value> {
    use Value::*;
    Ok(match (lhs, rhs) {
        (Int(a), Int(b)) => Int(a.checked_mul(b).ok_or_else(too_large)?),
        (Int(a), Float(b)) => Float(a as f64 * b),
        (Float(a), Int(b)) => Float(a * b as f64),
        (Float(a), Float(b)) => Float(a * b),

        (Decimal(a), Decimal(b)) => Decimal(a.checked_mul(b).ok_or_else(too_large)?),
        (Decimal(a), Int(b)) => Decimal(
            a.checked_mul(crate::foundations::Decimal::from(b))
                .ok_or_else(too_large)?,
        ),
        (Int(a), Decimal(b)) => Decimal(
            crate::foundations::Decimal::from(a)
                .checked_mul(b)
                .ok_or_else(too_large)?,
        ),

        (Length(a), Int(b)) => Length(a * b as f64),
        (Length(a), Float(b)) => Length(a * b),
        (Length(a), Ratio(b)) => Length(a * b.get()),
        (Int(a), Length(b)) => Length(b * a as f64),
        (Float(a), Length(b)) => Length(b * a),
        (Ratio(a), Length(b)) => Length(b * a.get()),

        (Angle(a), Int(b)) => Angle(a * b as f64),
        (Angle(a), Float(b)) => Angle(a * b),
        (Angle(a), Ratio(b)) => Angle(a * b.get()),
        (Int(a), Angle(b)) => Angle(a as f64 * b),
        (Float(a), Angle(b)) => Angle(a * b),
        (Ratio(a), Angle(b)) => Angle(a.get() * b),

        (Ratio(a), Ratio(b)) => Ratio(a * b),
        (Ratio(a), Int(b)) => Ratio(a * b as f64),
        (Ratio(a), Float(b)) => Ratio(a * b),
        (Int(a), Ratio(b)) => Ratio(a as f64 * b),
        (Float(a), Ratio(b)) => Ratio(a * b),

        (Relative(a), Int(b)) => Relative(a * b as f64),
        (Relative(a), Float(b)) => Relative(a * b),
        (Relative(a), Ratio(b)) => Relative(a * b.get()),
        (Int(a), Relative(b)) => Relative(a as f64 * b),
        (Float(a), Relative(b)) => Relative(a * b),
        (Ratio(a), Relative(b)) => Relative(a.get() * b),

        (Fraction(a), Int(b)) => Fraction(a * b as f64),
        (Fraction(a), Float(b)) => Fraction(a * b),
        (Fraction(a), Ratio(b)) => Fraction(a * b.get()),
        (Int(a), Fraction(b)) => Fraction(a as f64 * b),
        (Float(a), Fraction(b)) => Fraction(a * b),
        (Ratio(a), Fraction(b)) => Fraction(a.get() * b),

        (Str(a), Int(b)) => Str(a.repeat(Value::Int(b).cast()?)?),
        (Int(a), Str(b)) => Str(b.repeat(Value::Int(a).cast()?)?),
        (Array(a), Int(b)) => Array(a.repeat(Value::Int(b).cast()?)?),
        (Int(a), Array(b)) => Array(b.repeat(Value::Int(a).cast()?)?),
        (Content(a), b @ Int(_)) => Content(a.repeat(b.cast()?)),
        (a @ Int(_), Content(b)) => Content(b.repeat(a.cast()?)),

        (Int(a), Duration(b)) => Duration(b * (a as f64)),
        (Float(a), Duration(b)) => Duration(b * a),
        (Duration(a), Int(b)) => Duration(a * (b as f64)),
        (Duration(a), Float(b)) => Duration(a * b),

        (a, b) => mismatch!("cannot multiply {} with {}", a, b),
    })
}

/// Compute the quotient of two values.
pub fn div(lhs: Value, rhs: Value) -> HintedStrResult<Value> {
    use Value::*;
    if is_zero(&rhs) {
        bail!("cannot divide by zero");
    }

    Ok(match (lhs, rhs) {
        (Int(a), Int(b)) => Float(a as f64 / b as f64),
        (Int(a), Float(b)) => Float(a as f64 / b),
        (Float(a), Int(b)) => Float(a / b as f64),
        (Float(a), Float(b)) => Float(a / b),

        (Decimal(a), Decimal(b)) => Decimal(a.checked_div(b).ok_or_else(too_large)?),
        (Decimal(a), Int(b)) => Decimal(
            a.checked_div(crate::foundations::Decimal::from(b))
                .ok_or_else(too_large)?,
        ),
        (Int(a), Decimal(b)) => Decimal(
            crate::foundations::Decimal::from(a)
                .checked_div(b)
                .ok_or_else(too_large)?,
        ),

        (Length(a), Int(b)) => Length(a / b as f64),
        (Length(a), Float(b)) => Length(a / b),
        (Length(a), Length(b)) => Float(try_div_length(a, b)?),
        (Length(a), Relative(b)) if b.rel.is_zero() => Float(try_div_length(a, b.abs)?),

        (Angle(a), Int(b)) => Angle(a / b as f64),
        (Angle(a), Float(b)) => Angle(a / b),
        (Angle(a), Angle(b)) => Float(a / b),

        (Ratio(a), Int(b)) => Ratio(a / b as f64),
        (Ratio(a), Float(b)) => Ratio(a / b),
        (Ratio(a), Ratio(b)) => Float(a / b),
        (Ratio(a), Relative(b)) if b.abs.is_zero() => Float(a / b.rel),

        (Relative(a), Int(b)) => Relative(a / b as f64),
        (Relative(a), Float(b)) => Relative(a / b),
        (Relative(a), Length(b)) if a.rel.is_zero() => Float(try_div_length(a.abs, b)?),
        (Relative(a), Ratio(b)) if a.abs.is_zero() => Float(a.rel / b),
        (Relative(a), Relative(b)) => Float(try_div_relative(a, b)?),

        (Fraction(a), Int(b)) => Fraction(a / b as f64),
        (Fraction(a), Float(b)) => Fraction(a / b),
        (Fraction(a), Fraction(b)) => Float(a / b),

        (Duration(a), Int(b)) => Duration(a / (b as f64)),
        (Duration(a), Float(b)) => Duration(a / b),
        (Duration(a), Duration(b)) => Float(a / b),

        (a, b) => mismatch!("cannot divide {} by {}", a, b),
    })
}

/// Whether a value is a numeric zero.
fn is_zero(v: &Value) -> bool {
    use Value::*;
    match *v {
        Int(v) => v == 0,
        Float(v) => v == 0.0,
        Decimal(v) => v.is_zero(),
        Length(v) => v.is_zero(),
        Angle(v) => v.is_zero(),
        Ratio(v) => v.is_zero(),
        Relative(v) => v.is_zero(),
        Fraction(v) => v.is_zero(),
        Duration(v) => v.is_zero(),
        _ => false,
    }
}

/// Try to divide two lengths.
fn try_div_length(a: Length, b: Length) -> StrResult<f64> {
    a.try_div(b).ok_or_else(|| "cannot divide these two lengths".into())
}

/// Try to divide two relative lengths.
fn try_div_relative(a: Rel<Length>, b: Rel<Length>) -> StrResult<f64> {
    a.try_div(b)
        .ok_or_else(|| "cannot divide these two relative lengths".into())
}

/// Compute the logical "not" of a value.
pub fn not(value: Value) -> HintedStrResult<Value> {
    match value {
        Value::Bool(b) => Ok(Value::Bool(!b)),
        v => mismatch!("cannot apply 'not' to {}", v),
    }
}

/// Compute the logical "and" of two values.
pub fn and(lhs: Value, rhs: Value) -> HintedStrResult<Value> {
    match (lhs, rhs) {
        (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a && b)),
        (a, b) => mismatch!("cannot apply 'and' to {} and {}", a, b),
    }
}

/// Compute the logical "or" of two values.
pub fn or(lhs: Value, rhs: Value) -> HintedStrResult<Value> {
    match (lhs, rhs) {
        (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a || b)),
        (a, b) => mismatch!("cannot apply 'or' to {} and {}", a, b),
    }
}

/// Compute whether two values are equal.
pub fn eq(lhs: Value, rhs: Value) -> HintedStrResult<Value> {
    Ok(Value::Bool(equal(&lhs, &rhs)))
}

/// Compute whether two values are unequal.
pub fn neq(lhs: Value, rhs: Value) -> HintedStrResult<Value> {
    Ok(Value::Bool(!equal(&lhs, &rhs)))
}

macro_rules! comparison {
    ($name:ident, $op:tt, $($pat:tt)*) => {
        /// Compute how a value compares with another value.
        pub fn $name(lhs: Value, rhs: Value) -> HintedStrResult<Value> {
            let ordering = compare(&lhs, &rhs)?;
            Ok(Value::Bool(matches!(ordering, $($pat)*)))
        }
    };
}

comparison!(lt, "<", Ordering::Less);
comparison!(leq, "<=", Ordering::Less | Ordering::Equal);
comparison!(gt, ">", Ordering::Greater);
comparison!(geq, ">=", Ordering::Greater | Ordering::Equal);

/// Determine whether two values are equal.
pub fn equal(lhs: &Value, rhs: &Value) -> bool {
    use Value::*;
    match (lhs, rhs) {
        // Compare reflexively.
        (None, None) => true,
        (Auto, Auto) => true,
        (Bool(a), Bool(b)) => a == b,
        (Int(a), Int(b)) => a == b,
        (Float(a), Float(b)) => a == b,
        (Decimal(a), Decimal(b)) => a == b,
        (Length(a), Length(b)) => a == b,
        (Angle(a), Angle(b)) => a == b,
        (Ratio(a), Ratio(b)) => a == b,
        (Relative(a), Relative(b)) => a == b,
        (Fraction(a), Fraction(b)) => a == b,
        (Color(a), Color(b)) => a == b,
        (Symbol(a), Symbol(b)) => a == b,
        (Version(a), Version(b)) => a == b,
        (Str(a), Str(b)) => a == b,
        (Bytes(a), Bytes(b)) => a == b,
        (Label(a), Label(b)) => a == b,
        (Content(a), Content(b)) => a == b,
        (Array(a), Array(b)) => a == b,
        (Dict(a), Dict(b)) => a == b,
        (Func(a), Func(b)) => a == b,
        (Args(a), Args(b)) => a == b,
        (Type(a), Type(b)) => a == b,
        (Module(a), Module(b)) => a == b,
        (Plugin(a), Plugin(b)) => a == b,
        (Datetime(a), Datetime(b)) => a == b,
        (Duration(a), Duration(b)) => a == b,
        (Dyn(a), Dyn(b)) => a == b,

        // Some technically different things should compare equal.
        (&Int(i), &Float(f)) | (&Float(f), &Int(i)) => i as f64 == f,
        (&Int(i), &Decimal(d)) | (&Decimal(d), &Int(i)) => {
            crate::foundations::Decimal::from(i) == d
        }
        (&Length(len), &Relative(rel)) | (&Relative(rel), &Length(len)) => {
            len == rel.abs && rel.rel.is_zero()
        }
        (&Ratio(rat), &Relative(rel)) | (&Relative(rel), &Ratio(rat)) => {
            rat == rel.rel && rel.abs.is_zero()
        }

        // Type compatibility.
        (Type(ty), Str(str)) | (Str(str), Type(ty)) => ty.compat_name() == str.as_str(),

        _ => false,
    }
}

/// Compare two values.
pub fn compare(lhs: &Value, rhs: &Value) -> StrResult<Ordering> {
    use Value::*;
    Ok(match (lhs, rhs) {
        (Bool(a), Bool(b)) => a.cmp(b),
        (Int(a), Int(b)) => a.cmp(b),
        (Float(a), Float(b)) => try_cmp_values(a, b)?,
        (Decimal(a), Decimal(b)) => a.cmp(b),
        (Length(a), Length(b)) => try_cmp_values(a, b)?,
        (Angle(a), Angle(b)) => a.cmp(b),
        (Ratio(a), Ratio(b)) => a.cmp(b),
        (Relative(a), Relative(b)) => try_cmp_values(a, b)?,
        (Fraction(a), Fraction(b)) => a.cmp(b),
        (Version(a), Version(b)) => a.cmp(b),
        (Str(a), Str(b)) => a.cmp(b),

        // Some technically different things should be comparable.
        (Int(a), Float(b)) => try_cmp_values(&(*a as f64), b)?,
        (Float(a), Int(b)) => try_cmp_values(a, &(*b as f64))?,
        (Int(a), Decimal(b)) => crate::foundations::Decimal::from(*a).cmp(b),
        (Decimal(a), Int(b)) => a.cmp(&crate::foundations::Decimal::from(*b)),
        (Length(a), Relative(b)) if b.rel.is_zero() => try_cmp_values(a, &b.abs)?,
        (Ratio(a), Relative(b)) if b.abs.is_zero() => a.cmp(&b.rel),
        (Relative(a), Length(b)) if a.rel.is_zero() => try_cmp_values(&a.abs, b)?,
        (Relative(a), Ratio(b)) if a.abs.is_zero() => a.rel.cmp(b),

        (Duration(a), Duration(b)) => a.cmp(b),
        (Datetime(a), Datetime(b)) => try_cmp_datetimes(a, b)?,
        (Array(a), Array(b)) => try_cmp_arrays(a.as_slice(), b.as_slice())?,

        _ => mismatch!("cannot compare {} and {}", lhs, rhs),
    })
}

/// Try to compare two values.
fn try_cmp_values<T: PartialOrd + Repr>(a: &T, b: &T) -> StrResult<Ordering> {
    a.partial_cmp(b)
        .ok_or_else(|| eco_format!("cannot compare {} with {}", a.repr(), b.repr()))
}

/// Try to compare two datetimes.
fn try_cmp_datetimes(a: &Datetime, b: &Datetime) -> StrResult<Ordering> {
    a.partial_cmp(b)
        .ok_or_else(|| eco_format!("cannot compare {} and {}", a.kind(), b.kind()))
}

/// Try to compare arrays of values lexicographically.
fn try_cmp_arrays(a: &[Value], b: &[Value]) -> StrResult<Ordering> {
    a.iter()
        .zip(b.iter())
        .find_map(|(first, second)| {
            match compare(first, second) {
                // Keep searching for a pair of elements that isn't equal.
                Ok(Ordering::Equal) => None,
                // Found a pair which either is not equal or not comparable, so
                // we stop searching.
                result => Some(result),
            }
        })
        .unwrap_or_else(|| {
            // The two arrays are equal up to the shortest array's extent,
            // so compare their lengths instead.
            Ok(a.len().cmp(&b.len()))
        })
}

/// Test whether one value is "in" another one.
pub fn in_(lhs: Value, rhs: Value) -> HintedStrResult<Value> {
    if let Some(b) = contains(&lhs, &rhs) {
        Ok(Value::Bool(b))
    } else {
        mismatch!("cannot apply 'in' to {} and {}", lhs, rhs)
    }
}

/// Test whether one value is "not in" another one.
pub fn not_in(lhs: Value, rhs: Value) -> HintedStrResult<Value> {
    if let Some(b) = contains(&lhs, &rhs) {
        Ok(Value::Bool(!b))
    } else {
        mismatch!("cannot apply 'not in' to {} and {}", lhs, rhs)
    }
}

/// Test for containment.
pub fn contains(lhs: &Value, rhs: &Value) -> Option<bool> {
    use Value::*;
    match (lhs, rhs) {
        (Str(a), Str(b)) => Some(b.as_str().contains(a.as_str())),
        (Dyn(a), Str(b)) => a.downcast::<Regex>().map(|regex| regex.is_match(b)),
        (Str(a), Dict(b)) => Some(b.contains(a)),
        (a, Array(b)) => Some(b.contains(a.clone())),

        // Type compatibility.
        (Type(a), Str(b)) => Some(b.as_str().contains(a.compat_name())),
        (Type(a), Dict(b)) => Some(b.contains(a.compat_name())),

        _ => Option::None,
    }
}

#[cold]
fn too_large() -> &'static str {
    "value is too large"
}
