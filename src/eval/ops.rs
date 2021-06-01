use std::cmp::Ordering::*;

use super::{TemplateNode, Value};
use Value::*;

/// Apply the plus operator to a value.
pub fn pos(value: Value) -> Value {
    match value {
        Int(v) => Int(v),
        Float(v) => Float(v),
        Length(v) => Length(v),
        Angle(v) => Angle(v),
        Relative(v) => Relative(v),
        Fractional(v) => Fractional(v),
        Linear(v) => Linear(v),
        _ => Error,
    }
}

/// Compute the negation of a value.
pub fn neg(value: Value) -> Value {
    match value {
        Int(v) => Int(-v),
        Float(v) => Float(-v),
        Length(v) => Length(-v),
        Angle(v) => Angle(-v),
        Relative(v) => Relative(-v),
        Fractional(v) => Fractional(-v),
        Linear(v) => Linear(-v),
        _ => Error,
    }
}

/// Compute the sum of two values.
pub fn add(lhs: Value, rhs: Value) -> Value {
    match (lhs, rhs) {
        // Math.
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
        (Fractional(a), Fractional(b)) => Fractional(a + b),
        (Linear(a), Length(b)) => Linear(a + b),
        (Linear(a), Relative(b)) => Linear(a + b),
        (Linear(a), Linear(b)) => Linear(a + b),

        // Collections.
        (Str(a), Str(b)) => Str(a + &b),
        (Array(a), Array(b)) => Array(concat(a, b)),
        (Dict(a), Dict(b)) => Dict(concat(a, b)),

        // Templates.
        (Template(a), Template(b)) => Template(concat(a, b)),
        (Template(a), None) => Template(a),
        (None, Template(b)) => Template(b),
        (Template(mut a), Str(b)) => Template({
            a.push(TemplateNode::Str(b));
            a
        }),
        (Str(a), Template(mut b)) => Template({
            b.insert(0, TemplateNode::Str(a));
            b
        }),

        _ => Error,
    }
}

/// Compute the difference of two values.
pub fn sub(lhs: Value, rhs: Value) -> Value {
    match (lhs, rhs) {
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
        (Fractional(a), Fractional(b)) => Fractional(a - b),
        (Linear(a), Length(b)) => Linear(a - b),
        (Linear(a), Relative(b)) => Linear(a - b),
        (Linear(a), Linear(b)) => Linear(a - b),
        _ => Error,
    }
}

/// Compute the product of two values.
pub fn mul(lhs: Value, rhs: Value) -> Value {
    match (lhs, rhs) {
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
        (Fractional(a), Fractional(b)) => Fractional(a * b.get()),
        (Fractional(a), Int(b)) => Fractional(a * b as f64),
        (Fractional(a), Float(b)) => Fractional(a * b),
        (Int(a), Relative(b)) => Relative(a as f64 * b),
        (Int(a), Fractional(b)) => Fractional(a as f64 * b),
        (Float(a), Relative(b)) => Relative(a * b),
        (Float(a), Fractional(b)) => Fractional(a * b),
        (Linear(a), Int(b)) => Linear(a * b as f64),
        (Linear(a), Float(b)) => Linear(a * b),
        (Int(a), Linear(b)) => Linear(a as f64 * b),
        (Float(a), Linear(b)) => Linear(a * b),
        _ => Error,
    }
}

/// Compute the quotient of two values.
pub fn div(lhs: Value, rhs: Value) -> Value {
    match (lhs, rhs) {
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
        (Fractional(a), Fractional(b)) => Float(a.get() / b.get()),
        (Fractional(a), Int(b)) => Fractional(a / b as f64),
        (Fractional(a), Float(b)) => Fractional(a / b),
        (Linear(a), Int(b)) => Linear(a / b as f64),
        (Linear(a), Float(b)) => Linear(a / b),
        _ => Error,
    }
}

/// Compute the logical "not" of a value.
pub fn not(value: Value) -> Value {
    match value {
        Bool(b) => Bool(!b),
        _ => Error,
    }
}

/// Compute the logical "and" of two values.
pub fn and(lhs: Value, rhs: Value) -> Value {
    match (lhs, rhs) {
        (Bool(a), Bool(b)) => Bool(a && b),
        _ => Error,
    }
}

/// Compute the logical "or" of two values.
pub fn or(lhs: Value, rhs: Value) -> Value {
    match (lhs, rhs) {
        (Bool(a), Bool(b)) => Bool(a || b),
        _ => Error,
    }
}

/// Compute whether two values are equal.
pub fn eq(lhs: Value, rhs: Value) -> Value {
    Bool(lhs.eq(&rhs))
}

/// Compute whether two values are equal.
pub fn neq(lhs: Value, rhs: Value) -> Value {
    Bool(!lhs.eq(&rhs))
}

macro_rules! comparison {
    ($name:ident, $($pat:tt)*) => {
        /// Compute how a value compares with another value.
        pub fn $name(lhs: Value, rhs: Value) -> Value {
            lhs.cmp(&rhs)
                .map_or(Value::Error, |x| Value::Bool(matches!(x, $($pat)*)))
        }
    };
}

comparison!(lt, Less);
comparison!(leq, Less | Equal);
comparison!(gt, Greater);
comparison!(geq, Greater | Equal);

/// Concatenate two collections.
fn concat<T, A>(mut a: T, b: T) -> T
where
    T: Extend<A> + IntoIterator<Item = A>,
{
    a.extend(b);
    a
}
