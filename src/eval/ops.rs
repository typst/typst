use super::*;
use Value::*;

/// Apply the plus operator to a value.
pub fn pos(value: Value) -> Value {
    match value {
        Int(v) => Int(v),
        Float(v) => Float(v),
        Length(v) => Length(v),
        Angle(v) => Angle(v),
        Relative(v) => Relative(v),
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
        Linear(v) => Linear(-v),
        _ => Error,
    }
}

/// Compute the sum of two values.
pub fn add(lhs: Value, rhs: Value) -> Value {
    match (lhs, rhs) {
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
        (Str(a), Str(b)) => Str(a + &b),
        (Array(a), Array(b)) => Array(concat(a, b)),
        (Dict(a), Dict(b)) => Dict(concat(a, b)),
        (Template(a), Template(b)) => Template(concat(a, b)),
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
        (Int(a), Relative(b)) => Relative(a as f64 * b),
        (Float(a), Relative(b)) => Relative(a * b),
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
    Bool(value_eq(&lhs, &rhs))
}

/// Compute whether two values are equal.
pub fn neq(lhs: Value, rhs: Value) -> Value {
    Bool(!value_eq(&lhs, &rhs))
}

/// Recursively compute whether two values are equal.
fn value_eq(lhs: &Value, rhs: &Value) -> bool {
    match (lhs, rhs) {
        (&Int(a), &Float(b)) => a as f64 == b,
        (&Float(a), &Int(b)) => a == b as f64,
        (&Length(a), &Linear(b)) => a == b.abs && b.rel.is_zero(),
        (&Relative(a), &Linear(b)) => a == b.rel && b.abs.is_zero(),
        (&Linear(a), &Length(b)) => a.abs == b && a.rel.is_zero(),
        (&Linear(a), &Relative(b)) => a.rel == b && a.abs.is_zero(),
        (Array(a), Array(b)) => array_eq(a, b),
        (Dict(a), Dict(b)) => dict_eq(a, b),
        (Template(a), Template(b)) => Span::without_cmp(|| a == b),
        (a, b) => a == b,
    }
}

/// Compute whether two arrays are equal.
fn array_eq(a: &ValueArray, b: &ValueArray) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| value_eq(x, y))
}

/// Compute whether two dictionaries are equal.
fn dict_eq(a: &ValueDict, b: &ValueDict) -> bool {
    a.len() == b.len()
        && a.iter().all(|(k, x)| b.get(k).map_or(false, |y| value_eq(x, y)))
}

macro_rules! comparison {
    ($name:ident, $op:tt) => {
        /// Compute how a value compares with another value.
        pub fn $name(lhs: Value, rhs: Value) -> Value {
            match (lhs, rhs) {
                (Int(a), Int(b)) => Bool(a $op b),
                (Int(a), Float(b)) => Bool((a as f64) $op b),
                (Float(a), Int(b)) => Bool(a $op b as f64),
                (Float(a), Float(b)) => Bool(a $op b),
                (Angle(a), Angle(b)) => Bool(a $op b),
                (Length(a), Length(b)) => Bool(a $op b),
                _ => Error,
            }
        }
    };
}

comparison!(lt, <);
comparison!(leq, <=);
comparison!(gt, >);
comparison!(geq, >=);

/// Concatenate two collections.
fn concat<T, A>(mut a: T, b: T) -> T
where
    T: Extend<A> + IntoIterator<Item = A>,
{
    a.extend(b);
    a
}
