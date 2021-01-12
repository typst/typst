use super::*;

/// Apply plus operator to a value.
pub fn pos(ctx: &mut EvalContext, span: Span, value: Value) -> Value {
    if value.is_numeric() {
        value
    } else {
        ctx.diag(error!(
            span,
            "cannot apply plus operator to {}",
            value.type_name()
        ));
        Value::Error
    }
}

/// Compute the negation of a value.
pub fn neg(ctx: &mut EvalContext, span: Span, value: Value) -> Value {
    use Value::*;
    match value {
        Int(v) => Int(-v),
        Float(v) => Float(-v),
        Length(v) => Length(-v),
        Angle(v) => Angle(-v),
        Relative(v) => Relative(-v),
        Linear(v) => Linear(-v),
        v => {
            ctx.diag(error!(span, "cannot negate {}", v.type_name()));
            Value::Error
        }
    }
}

/// Compute the sum of two values.
pub fn add(ctx: &mut EvalContext, span: Span, lhs: Value, rhs: Value) -> Value {
    use Value::*;
    match (lhs, rhs) {
        // Numeric types to themselves.
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

        // Complex data types to themselves.
        (Str(a), Str(b)) => Str(a + &b),
        (Array(a), Array(b)) => Array(concat(a, b)),
        (Dict(a), Dict(b)) => Dict(concat(a, b)),
        (Template(a), Template(b)) => Template(concat(a, b)),

        (a, b) => {
            ctx.diag(error!(
                span,
                "cannot add {} and {}",
                a.type_name(),
                b.type_name()
            ));
            Value::Error
        }
    }
}

/// Compute the difference of two values.
pub fn sub(ctx: &mut EvalContext, span: Span, lhs: Value, rhs: Value) -> Value {
    use Value::*;
    match (lhs, rhs) {
        // Numbers from themselves.
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

        (a, b) => {
            ctx.diag(error!(
                span,
                "cannot subtract {1} from {0}",
                a.type_name(),
                b.type_name()
            ));
            Value::Error
        }
    }
}

/// Compute the product of two values.
pub fn mul(ctx: &mut EvalContext, span: Span, lhs: Value, rhs: Value) -> Value {
    use Value::*;
    match (lhs, rhs) {
        // Numeric types with numbers.
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

        // Integers with strings.
        (Int(a), Str(b)) => Str(b.repeat(0.max(a) as usize)),
        (Str(a), Int(b)) => Str(a.repeat(0.max(b) as usize)),

        (a, b) => {
            ctx.diag(error!(
                span,
                "cannot multiply {} with {}",
                a.type_name(),
                b.type_name()
            ));
            Value::Error
        }
    }
}

/// Compute the quotient of two values.
pub fn div(ctx: &mut EvalContext, span: Span, lhs: Value, rhs: Value) -> Value {
    use Value::*;
    match (lhs, rhs) {
        // Numeric types by numbers.
        (Int(a), Int(b)) => Float(a as f64 / b as f64),
        (Int(a), Float(b)) => Float(a as f64 / b),
        (Float(a), Int(b)) => Float(a / b as f64),
        (Float(a), Float(b)) => Float(a / b),
        (Length(a), Int(b)) => Length(a / b as f64),
        (Length(a), Float(b)) => Length(a / b),
        (Angle(a), Int(b)) => Angle(a / b as f64),
        (Angle(a), Float(b)) => Angle(a / b),
        (Relative(a), Int(b)) => Relative(a / b as f64),
        (Relative(a), Float(b)) => Relative(a / b),
        (Linear(a), Int(b)) => Linear(a / b as f64),
        (Linear(a), Float(b)) => Linear(a / b),

        (a, b) => {
            ctx.diag(error!(
                span,
                "cannot divide {} by {}",
                a.type_name(),
                b.type_name()
            ));
            Value::Error
        }
    }
}

/// Concatenate two collections.
fn concat<T, A>(mut a: T, b: T) -> T
where
    T: Extend<A> + IntoIterator<Item = A>,
{
    a.extend(b);
    a
}
