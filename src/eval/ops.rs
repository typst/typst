use std::cmp::Ordering;

use super::Value;
use crate::diag::StrResult;
use Value::*;

/// Bail with a type mismatch error.
macro_rules! mismatch {
    ($fmt:expr, $($value:expr),* $(,)?) => {
        return Err(format!($fmt, $($value.type_name()),*));
    };
}

/// Join a value with another value.
pub fn join(lhs: Value, rhs: Value) -> StrResult<Value> {
    Ok(match (lhs, rhs) {
        (a, None) => a,
        (None, b) => b,
        (Str(a), Str(b)) => Str(a + b),
        (Array(a), Array(b)) => Array(a + b),
        (Dict(a), Dict(b)) => Dict(a + b),
        (Template(a), Template(b)) => Template(a + b),
        (Template(a), Str(b)) => Template(a + b),
        (Str(a), Template(b)) => Template(a + b),
        (a, b) => mismatch!("cannot join {} with {}", a, b),
    })
}

/// Apply the plus operator to a value.
pub fn pos(value: Value) -> StrResult<Value> {
    Ok(match value {
        Int(v) => Int(v),
        Float(v) => Float(v),
        Length(v) => Length(v),
        Angle(v) => Angle(v),
        Relative(v) => Relative(v),
        Linear(v) => Linear(v),
        Fractional(v) => Fractional(v),
        v => mismatch!("cannot apply '+' to {}", v),
    })
}

/// Compute the negation of a value.
pub fn neg(value: Value) -> StrResult<Value> {
    Ok(match value {
        Int(v) => Int(-v),
        Float(v) => Float(-v),
        Length(v) => Length(-v),
        Angle(v) => Angle(-v),
        Relative(v) => Relative(-v),
        Linear(v) => Linear(-v),
        Fractional(v) => Fractional(-v),
        v => mismatch!("cannot apply '-' to {}", v),
    })
}

/// Compute the sum of two values.
pub fn add(lhs: Value, rhs: Value) -> StrResult<Value> {
    Ok(match (lhs, rhs) {
        (Int(a), Int(b)) => Int(a + b),
        (Int(a), Float(b)) => Float(a as f64 + b),
        (Float(a), Int(b)) => Float(a + b as f64),
        (Float(a), Float(b)) => Float(a + b),

        (Angle(a), Angle(b)) => Angle(a + b),

        (Length(a), Length(b)) => Length(a + b),
        (Length(a), Relative(b)) => Linear(a + b),
        (Length(a), Linear(b)) => Linear(a + b),

        (Relative(a), Length(b)) => Linear(a + b),
        (Relative(a), Relative(b)) => Relative(a + b),
        (Relative(a), Linear(b)) => Linear(a + b),

        (Linear(a), Length(b)) => Linear(a + b),
        (Linear(a), Relative(b)) => Linear(a + b),
        (Linear(a), Linear(b)) => Linear(a + b),

        (Fractional(a), Fractional(b)) => Fractional(a + b),

        (Str(a), Str(b)) => Str(a + b),
        (Array(a), Array(b)) => Array(a + b),
        (Dict(a), Dict(b)) => Dict(a + b),
        (Template(a), Template(b)) => Template(a + b),
        (Template(a), Str(b)) => Template(a + b),
        (Str(a), Template(b)) => Template(a + b),

        (a, b) => mismatch!("cannot add {} and {}", a, b),
    })
}

/// Compute the difference of two values.
pub fn sub(lhs: Value, rhs: Value) -> StrResult<Value> {
    Ok(match (lhs, rhs) {
        (Int(a), Int(b)) => Int(a - b),
        (Int(a), Float(b)) => Float(a as f64 - b),
        (Float(a), Int(b)) => Float(a - b as f64),
        (Float(a), Float(b)) => Float(a - b),

        (Angle(a), Angle(b)) => Angle(a - b),

        (Length(a), Length(b)) => Length(a - b),
        (Length(a), Relative(b)) => Linear(a - b),
        (Length(a), Linear(b)) => Linear(a - b),

        (Relative(a), Length(b)) => Linear(a - b),
        (Relative(a), Relative(b)) => Relative(a - b),
        (Relative(a), Linear(b)) => Linear(a - b),

        (Linear(a), Length(b)) => Linear(a - b),
        (Linear(a), Relative(b)) => Linear(a - b),
        (Linear(a), Linear(b)) => Linear(a - b),

        (Fractional(a), Fractional(b)) => Fractional(a - b),

        (a, b) => mismatch!("cannot subtract {1} from {0}", a, b),
    })
}

/// Compute the product of two values.
pub fn mul(lhs: Value, rhs: Value) -> StrResult<Value> {
    Ok(match (lhs, rhs) {
        (Int(a), Int(b)) => Int(a * b),
        (Int(a), Float(b)) => Float(a as f64 * b),
        (Float(a), Int(b)) => Float(a * b as f64),
        (Float(a), Float(b)) => Float(a * b),

        (Length(a), Int(b)) => Length(a * b as f64),
        (Length(a), Float(b)) => Length(a * b),
        (Int(a), Length(b)) => Length(a as f64 * b),
        (Float(a), Length(b)) => Length(a * b),

        (Angle(a), Int(b)) => Angle(a * b as f64),
        (Angle(a), Float(b)) => Angle(a * b),
        (Int(a), Angle(b)) => Angle(a as f64 * b),
        (Float(a), Angle(b)) => Angle(a * b),

        (Relative(a), Int(b)) => Relative(a * b as f64),
        (Relative(a), Float(b)) => Relative(a * b),
        (Float(a), Relative(b)) => Relative(a * b),
        (Int(a), Relative(b)) => Relative(a as f64 * b),

        (Linear(a), Int(b)) => Linear(a * b as f64),
        (Linear(a), Float(b)) => Linear(a * b),
        (Int(a), Linear(b)) => Linear(a as f64 * b),
        (Float(a), Linear(b)) => Linear(a * b),

        (Float(a), Fractional(b)) => Fractional(a * b),
        (Fractional(a), Int(b)) => Fractional(a * b as f64),
        (Fractional(a), Float(b)) => Fractional(a * b),
        (Int(a), Fractional(b)) => Fractional(a as f64 * b),

        (Str(a), Int(b)) => Str(a.repeat(b)?),
        (Int(a), Str(b)) => Str(b.repeat(a)?),
        (Array(a), Int(b)) => Array(a.repeat(b)?),
        (Int(a), Array(b)) => Array(b.repeat(a)?),
        (Template(a), Int(b)) => Template(a.repeat(b)?),
        (Int(a), Template(b)) => Template(b.repeat(a)?),

        (a, b) => mismatch!("cannot multiply {} with {}", a, b),
    })
}

/// Compute the quotient of two values.
pub fn div(lhs: Value, rhs: Value) -> StrResult<Value> {
    Ok(match (lhs, rhs) {
        (Int(a), Int(b)) => Float(a as f64 / b as f64),
        (Int(a), Float(b)) => Float(a as f64 / b),
        (Float(a), Int(b)) => Float(a / b as f64),
        (Float(a), Float(b)) => Float(a / b),

        (Length(a), Int(b)) => Length(a / b as f64),
        (Length(a), Float(b)) => Length(a / b),
        (Length(a), Length(b)) => Float(a / b),

        (Angle(a), Int(b)) => Angle(a / b as f64),
        (Angle(a), Float(b)) => Angle(a / b),
        (Angle(a), Angle(b)) => Float(a / b),

        (Relative(a), Int(b)) => Relative(a / b as f64),
        (Relative(a), Float(b)) => Relative(a / b),
        (Relative(a), Relative(b)) => Float(a / b),

        (Fractional(a), Int(b)) => Fractional(a / b as f64),
        (Fractional(a), Float(b)) => Fractional(a / b),
        (Fractional(a), Fractional(b)) => Float(a / b),

        (Linear(a), Int(b)) => Linear(a / b as f64),
        (Linear(a), Float(b)) => Linear(a / b),

        (a, b) => mismatch!("cannot divide {} by {}", a, b),
    })
}

/// Compute the logical "not" of a value.
pub fn not(value: Value) -> StrResult<Value> {
    match value {
        Bool(b) => Ok(Bool(!b)),
        v => mismatch!("cannot apply 'not' to {}", v),
    }
}

/// Compute the logical "and" of two values.
pub fn and(lhs: Value, rhs: Value) -> StrResult<Value> {
    match (lhs, rhs) {
        (Bool(a), Bool(b)) => Ok(Bool(a && b)),
        (a, b) => mismatch!("cannot apply 'and' to {} and {}", a, b),
    }
}

/// Compute the logical "or" of two values.
pub fn or(lhs: Value, rhs: Value) -> StrResult<Value> {
    match (lhs, rhs) {
        (Bool(a), Bool(b)) => Ok(Bool(a || b)),
        (a, b) => mismatch!("cannot apply 'or' to {} and {}", a, b),
    }
}

/// Compute whether two values are equal.
pub fn eq(lhs: Value, rhs: Value) -> StrResult<Value> {
    Ok(Bool(equal(&lhs, &rhs)))
}

/// Compute whether two values are equal.
pub fn neq(lhs: Value, rhs: Value) -> StrResult<Value> {
    Ok(Bool(!equal(&lhs, &rhs)))
}

macro_rules! comparison {
    ($name:ident, $op:tt, $($pat:tt)*) => {
        /// Compute how a value compares with another value.
        pub fn $name(lhs: Value, rhs: Value) -> StrResult<Value> {
            if let Some(ordering) = compare(&lhs, &rhs) {
                Ok(Bool(matches!(ordering, $($pat)*)))
            } else {
                mismatch!(concat!("cannot apply '", $op, "' to {} and {}"), lhs, rhs);
            }
        }
    };
}

comparison!(lt, "<", Ordering::Less);
comparison!(leq, "<=", Ordering::Less | Ordering::Equal);
comparison!(gt, ">", Ordering::Greater);
comparison!(geq, ">=", Ordering::Greater | Ordering::Equal);

/// Compute the range from `lhs` to `rhs`.
pub fn range(lhs: Value, rhs: Value) -> StrResult<Value> {
    match (lhs, rhs) {
        (Int(a), Int(b)) => Ok(Array((a ..= b).map(Int).collect())),
        (a, b) => mismatch!("cannot apply '..' to {} and {}", a, b),
    }
}

/// Determine whether two values are equal.
pub fn equal(lhs: &Value, rhs: &Value) -> bool {
    match (lhs, rhs) {
        // Compare reflexively.
        (None, None) => true,
        (Auto, Auto) => true,
        (Bool(a), Bool(b)) => a == b,
        (Int(a), Int(b)) => a == b,
        (Float(a), Float(b)) => a == b,
        (Length(a), Length(b)) => a == b,
        (Angle(a), Angle(b)) => a == b,
        (Relative(a), Relative(b)) => a == b,
        (Linear(a), Linear(b)) => a == b,
        (Fractional(a), Fractional(b)) => a == b,
        (Color(a), Color(b)) => a == b,
        (Str(a), Str(b)) => a == b,
        (Array(a), Array(b)) => a == b,
        (Dict(a), Dict(b)) => a == b,
        (Template(a), Template(b)) => a == b,
        (Func(a), Func(b)) => a == b,
        (Dyn(a), Dyn(b)) => a == b,

        // Some technically different things should compare equal.
        (&Int(a), &Float(b)) => a as f64 == b,
        (&Float(a), &Int(b)) => a == b as f64,
        (&Length(a), &Linear(b)) => a == b.abs && b.rel.is_zero(),
        (&Relative(a), &Linear(b)) => a == b.rel && b.abs.is_zero(),
        (&Linear(a), &Length(b)) => a.abs == b && a.rel.is_zero(),
        (&Linear(a), &Relative(b)) => a.rel == b && a.abs.is_zero(),

        _ => false,
    }
}

/// Compare two values.
pub fn compare(lhs: &Value, rhs: &Value) -> Option<Ordering> {
    match (lhs, rhs) {
        (Bool(a), Bool(b)) => a.partial_cmp(b),
        (Int(a), Int(b)) => a.partial_cmp(b),
        (Int(a), Float(b)) => (*a as f64).partial_cmp(b),
        (Float(a), Int(b)) => a.partial_cmp(&(*b as f64)),
        (Float(a), Float(b)) => a.partial_cmp(b),
        (Angle(a), Angle(b)) => a.partial_cmp(b),
        (Length(a), Length(b)) => a.partial_cmp(b),
        (Str(a), Str(b)) => a.partial_cmp(b),
        _ => Option::None,
    }
}
