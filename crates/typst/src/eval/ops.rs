//! Operations on values.

use std::cmp::Ordering;
use std::fmt::Debug;

use ecow::eco_format;
use typst::eval::Duration;

use super::{format_str, Regex, Value};
use crate::diag::{bail, StrResult};
use crate::geom::{Axes, Axis, GenAlign, Length, Numeric, PartialStroke, Rel, Smart};
use Value::*;
use crate::eval::Dynamic;

macro_rules!expand {
	(pat($p:pat))=>{
		$p
	};

	(pat($($p:pat),+))=>{
		($($p),+)
	};

    ($x:tt) => {$x}
}

macro_rules! match_dyn {
    // Wrap the user-provided arguments in `@(...)`
    ($e:expr; $($rest:tt)*) => {
        match_dyn!{@($e; $($rest)*,)}
    };
    (;$e:expr; $($rest:tt)*) => {
        match_dyn!{@pat($e; $($rest)*,) @arms() @pats() @ifs() @casts()}
    };

    /* Match the pattern*/

    

   // Interpret one match pattern: (Dyn)
    (@pat($e:expr; Dyn($pn:ident: $pt:ty) => $($rest:tt)*) @arms($($arms:tt)*) @pats($($pats:pat),*) @ifs($($ifs:expr),*) @casts($($casts:stmt)*)) => {
        match_dyn!{
            // Next index, match argument, remaining match results
            @exec($e; $($rest)*)
            @arms($($arms)*)
            @pats(Dyn($pn))
            @ifs($pn.is::<$pt>())
            @casts(let $pn = *$pn.downcast::<$pt>().unwrap())
        }
    };

   // Interpret one match pattern: (Dyn, Dyn)
   (@pat($e:expr; (Dyn($an:ident: $at:ty), Dyn($bn:ident: $bt:ty)) => $($rest:tt)*) @arms($($arms:tt)*) @pats($($pats:pat),*) @ifs($($ifs:expr),*) @casts($($casts:stmt)*)) => {
        match_dyn!{
            // Next index, match argument, remaining match results
            @exec($e; $($rest)*)
            @arms($($arms)*)
            @pats(Dyn($an), Dyn($bn))
            @ifs($an.is::<$at>(), $bn.is::<$bt>())
            @casts(let $an = *$an.downcast::<$at>().unwrap() let $bn = *$bn.downcast::<$bt>().unwrap())
        }
    };


    // Interpret one match pattern: (Dyn, Regular)
    (@pat($e:expr; (Dyn($an:ident: $at:ty), $b:pat) => $($rest:tt)*) @arms($($arms:tt)*) @pats($($pats:pat),*) @ifs($($ifs:expr),*) @casts($($casts:stmt)*)) => {
        match_dyn!{
            // Next index, match argument, remaining match results
            @exec($e; $($rest)*)
            @arms($($arms)*)
            @pats(Dyn($an), $b)
            @ifs($an.is::<$at>())
            @casts(let $an = *$an.downcast::<$at>().unwrap())
        }
    };


    // Interpret one match pattern: (Regular, Dyn)
    (@pat($e:expr; ($a:pat, Dyn($bn:ident: $bt:ty)) => $($rest:tt)*) @arms($($arms:tt)*) @pats($($pats:pat),*) @ifs($($ifs:expr),*) @casts($($casts:stmt)*)) => {
        match_dyn!{
            // Next index, match argument, remaining match results
            @exec($e; $($rest)*)
            @arms($($arms)*)
            @pats($a, Dyn($bn))
            @ifs($bn.is::<$bt>())
            @casts(let $bn = *$bn.downcast::<$bt>().unwrap())
        }
    };

    // Interpret one match pattern: Regular
    (@pat($e:expr; $p:pat => $($rest:tt)*) @arms($($arms:tt)*) @pats($($pats:pat),*) @ifs($($ifs:expr),*) @casts($($casts:stmt)*)) => {
        match_dyn!{
            // Next index, match argument, remaining match results
            @exec($e; $($rest)*)
            @arms($($arms)*)
            @pats($p)
            @ifs()
            @casts()
        }
    };

    // Interpret one match thing => Dyn
   (@exec($e:expr; Dyn($cmd:expr), $($rest:tt)*) @arms($($arms:tt)*) @pats($($pats:pat),+) @ifs($($($ifs:expr),+)?) @casts($($casts:stmt)*))  => {
        match_dyn!{
            // Next index, match argument, remaining match results
            @pat($e; $($rest)*)
            @arms(
                $($arms)*
                ($($pats),*) $(if $($ifs)&&+)? => {
                    $($casts)*
                    Dyn(Dynamic::new($cmd))
                }
            )
            @pats()
            @ifs()
            @casts()
        }
    };

    // Interpret one match thing => cmd
   (@exec($e:expr; $cmd:expr, $($rest:tt)*) @arms($($arms:tt)*) @pats($($pats:pat),+) @ifs($($($ifs:expr),+)?) @casts($($casts:stmt)*))  => {
        match_dyn!{
            // Next index, match argument, remaining match results
            @pat($e; $($rest)*)
            @arms(
                $($arms)*
                ($($pats),*) $(if $($ifs)&&+)? => {
                    $($casts)*
                    $cmd
                }
            )
            @pats() @ifs() @casts()
        }
    };








 // No more match arms, produce final output
    (@pat($e:expr; $(,)?) @arms($($arms:tt)*) @pats($($pats:tt)*) @ifs($($ifs:tt)*) @casts($($casts:tt)*)) => {
        match $e {
            $($arms)*
        }
    };



   // Interpret one match arm: (Dyn) => Dyn
   (@($e:expr; Dyn($pn:ident: $pt:ty) => Dyn($cmd:expr), $($rest:tt)*) $($arms:tt)*) => {
        match_dyn!{
            // Next index, match argument, remaining match results
            @( $e; $($rest)*)
            $($arms)*
            Dyn($pn) if $pn.is::<$pt>() => {
                let $pn = *$pn.downcast::<$pt>().unwrap();
                Dyn(Dynamic::new($cmd))
            },
        }
    };

   // Interpret one match arm: (Dyn) => Regular
   (@($e:expr; Dyn($pn:ident: $pt:ty) => $cmd:expr, $($rest:tt)*) $($arms:tt)*) => {
        match_dyn!{
            // Next index, match argument, remaining match results
            @( $e; $($rest)*)
            $($arms)*
            Dyn($pn) if $pn.is::<$pt>() => {
                let $pn = *$pn.downcast::<$pt>().unwrap();
                $cmd
            },
        }
    };

   // Interpret one match arm: (Dyn, Dyn) => Dyn
   (@($e:expr; (Dyn($an:ident: $at:ty), Dyn($bn:ident: $bt:ty)) => Dyn($cmd:expr), $($rest:tt)*) $($arms:tt)*) => {
        match_dyn!{
            // Next index, match argument, remaining match results
            @( $e; $($rest)*)
            $($arms)*
            (Dyn($an), Dyn($bn)) if $an.is::<$at>() && $bn.is::<$bt>() => {
                let $an = *$an.downcast::<$at>().unwrap();
                let $bn = *$bn.downcast::<$bt>().unwrap();
                Dyn(Dynamic::new($cmd))
            },
        }
    };

   // Interpret one match arm: (Dyn, Dyn) => Regular
   (@($e:expr; (Dyn($an:ident: $at:ty), Dyn($bn:ident: $bt:ty)) => $cmd:expr, $($rest:tt)*) $($arms:tt)*) => {
        match_dyn!{
            // Next index, match argument, remaining match results
            @( $e; $($rest)*)
            $($arms)*
            (Dyn($an), Dyn($bn)) if $an.is::<$at>() && $bn.is::<$bt>() => {
                let $an = *$an.downcast::<$at>().unwrap();
                let $bn = *$bn.downcast::<$bt>().unwrap();
                $cmd
            },
        }
    };

    // Interpret one match arm: (Dyn, Regular) => Dyn
    (@($e:expr; (Dyn($an:ident: $at:ty), $b:pat) => Dyn($cmd:expr), $($rest:tt)*) $($arms:tt)*) => {
        match_dyn!{
            // Next index, match argument, remaining match results
            @( $e; $($rest)*)
            $($arms)*
            (Dyn($an), $b) if $an.is::<$at>() => {
                let $an = *$an.downcast::<$at>().unwrap();
                Dyn(Dynamic::new($cmd))
            },
        }
    };

    // Interpret one match arm: (Dyn, Regular) => Regular
    (@($e:expr; (Dyn($an:ident: $at:ty), $b:pat) => $cmd:expr, $($rest:tt)*) $($arms:tt)*) => {
        match_dyn!{
            // Next index, match argument, remaining match results
            @( $e; $($rest)*)
            $($arms)*
            (Dyn($an), $b) if $an.is::<$at>() => {
                let $an = *$an.downcast::<$at>().unwrap();
                $cmd
            },
        }
    };

    // Interpret one match arm: (Regular, Dyn) => Dyn
    (@($e:expr; ($a:pat, Dyn($bn:ident: $bt:ty)) => Dyn($cmd:expr), $($rest:tt)*) $($arms:tt)*) => {
        match_dyn!{
            // Next index, match argument, remaining match results
            @( $e; $($rest)*)
            $($arms)*
            ($a, Dyn($bn)) if $bn.is::<$bt>() => {
                let $bn = *$bn.downcast::<$bt>().unwrap();
                Dyn(Dynamic::new($cmd))
            },
        }
    };

    // Interpret one match arm: (Regular, Dyn) => Regular
    (@($e:expr; ($a:pat, Dyn($bn:ident: $bt:ty)) => $cmd:expr, $($rest:tt)*) $($arms:tt)*) => {
        match_dyn!{
            // Next index, match argument, remaining match results
            @( $e; $($rest)*)
            $($arms)*
            ($a, Dyn($bn)) if $bn.is::<$bt>() => {
                let $bn = *$bn.downcast::<$bt>().unwrap();
                $cmd
            },
        }
    };

    // Interpret one match arm: Regular => Dyn
    (@($e:expr; $p:pat => Dyn($cmd:expr), $($rest:tt)*) $($arms:tt)*) => {
        match_dyn!{
            // Next index, match argument, remaining match results
            @( $e; $($rest)*)
            $($arms)*
            $p => {
                Dyn(Dynamic::new($cmd))
            },
        }
    };

    // Interpret one match arm: Regular => Block
    (@($e:expr; $p:pat => $cmd:block $($rest:tt)*) $($arms:tt)*) => {
        match_dyn!{
            // Next index, match argument, remaining match results
            @( $e; $($rest)*)
            $($arms)*
            $p => {
                $cmd
            },
        }
    };

    // Interpret one match arm: Regular => Regular
    (@($e:expr; $p:pat $(if $cond:expr)? => $cmd:expr, $($rest:tt)*) $($arms:tt)*) => {
        match_dyn!{
            // Next index, match argument, remaining match results
            @( $e; $($rest)*)
            $($arms)*
            $p $(if $cond)?=> {
                $cmd
            },
        }
    };

    // No more match arms, produce final output
    (@($e:expr; $(,)?) $($arms:tt)* ) => {
        match $e {
            $($arms)*
        }
    };
}

fn test() -> Value {
    let x = (Int(1), Int(2));
    match_dyn!(;x;
        (Dyn(a: Duration), Dyn(b:Duration)) => Dyn(a+b),
        (Int(a), Dyn(b:Duration)) => Dyn(a+b),
       // (Int(a), Int(b)) => Dyn(a+b)
    )
}

/// Bail with a type mismatch error.
macro_rules! mismatch {
    ($fmt:expr, $($value:expr),* $(,)?) => {
        return Err(eco_format!($fmt, $($value.type_name()),*))
    };
}

/// Join a value with another value.
pub fn join(lhs: Value, rhs: Value) -> StrResult<Value> {
    Ok(match (lhs, rhs) {
        (a, None) => a,
        (None, b) => b,
        (Symbol(a), Symbol(b)) => Str(format_str!("{a}{b}")),
        (Str(a), Str(b)) => Str(a + b),
        (Str(a), Symbol(b)) => Str(format_str!("{a}{b}")),
        (Symbol(a), Str(b)) => Str(format_str!("{a}{b}")),
        (Content(a), Content(b)) => Content(a + b),
        (Content(a), Symbol(b)) => Content(a + item!(text)(b.get().into())),
        (Content(a), Str(b)) => Content(a + item!(text)(b.into())),
        (Str(a), Content(b)) => Content(item!(text)(a.into()) + b),
        (Symbol(a), Content(b)) => Content(item!(text)(a.get().into()) + b),
        (Array(a), Array(b)) => Array(a + b),
        (Dict(a), Dict(b)) => Dict(a + b),
        (a, b) => mismatch!("cannot join {} with {}", a, b),
    })
}

/// Apply the unary plus operator to a value.
pub fn pos(value: Value) -> StrResult<Value> {
    Ok(match value {
        Int(v) => Int(v),
        Float(v) => Float(v),
        Length(v) => Length(v),
        Angle(v) => Angle(v),
        Ratio(v) => Ratio(v),
        Relative(v) => Relative(v),
        Fraction(v) => Fraction(v),
        v => mismatch!("cannot apply '+' to {}", v),
    })
}

/// Compute the negation of a value.
pub fn neg(value: Value) -> StrResult<Value> {
    Ok(match_dyn!(value;
        Int(v) => Int(v.checked_neg().ok_or("value is too large")?),
        Float(v) => Float(-v),
        Length(v) => Length(-v),
        Angle(v) => Angle(-v),
        Ratio(v) => Ratio(-v),
        Relative(v) => Relative(-v),
        Fraction(v) => Fraction(-v),
        Dyn(v: Duration) => Dyn(-v),
        v => mismatch!("cannot apply '-' to {}", v),
    ))
}

/// Compute the sum of two values.
pub fn add(lhs: Value, rhs: Value) -> StrResult<Value> {
    Ok(match_dyn!((lhs, rhs);
        (a, None) => a,
        (None, b) => b,

        (Int(a), Int(b)) => Int(a.checked_add(b).ok_or("value is too large")?),
        (Int(a), Float(b)) => Float(a as f64 + b),
        (Float(a), Int(b)) => Float(a + b as f64),
        (Float(a), Float(b)) => Float(a + b),

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
        (Content(a), Content(b)) => Content(a + b),
        (Content(a), Symbol(b)) => Content(a + item!(text)(b.get().into())),
        (Content(a), Str(b)) => Content(a + item!(text)(b.into())),
        (Str(a), Content(b)) => Content(item!(text)(a.into()) + b),
        (Symbol(a), Content(b)) => Content(item!(text)(a.get().into()) + b),

        (Array(a), Array(b)) => Array(a + b),
        (Dict(a), Dict(b)) => Dict(a + b),

        (Color(color), Length(thickness)) | (Length(thickness), Color(color)) => {
            Value::dynamic(PartialStroke {
                paint: Smart::Custom(color.into()),
                thickness: Smart::Custom(thickness),
                ..PartialStroke::default()
            })
        }
        (Dyn(a: GenAlign), Dyn(b: GenAlign)) => Dyn({
            if a.axis() == b.axis() {
                return Err(eco_format!("cannot add two {:?} alignments", a.axis()));
            }

            match a.axis() {
                Axis::X => Axes { x: a, y: b },
                Axis::Y => Axes { x: b, y: a },
            }
        }),
        (Dyn(a: Duration), Dyn(b: Duration)) => Dyn(a+b),
        (a, b) => mismatch!("cannot add {} and {}", a, b),
    ))
}

/// Compute the difference of two values.
pub fn sub(lhs: Value, rhs: Value) -> StrResult<Value> {
    Ok(match_dyn!((lhs, rhs);
        (Int(a), Int(b)) => Int(a.checked_sub(b).ok_or("value is too large")?),
        (Int(a), Float(b)) => Float(a as f64 - b),
        (Float(a), Int(b)) => Float(a - b as f64),
        (Float(a), Float(b)) => Float(a - b),

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

        (Dyn(a:Duration), Dyn(b:Duration)) => Dyn(a-b),

        (a, b) => mismatch!("cannot subtract {1} from {0}", a, b),
    ))
}

/// Compute the product of two values.
pub fn mul(lhs: Value, rhs: Value) -> StrResult<Value> {
    Ok(match_dyn!((lhs, rhs);
        (Int(a), Int(b)) => Int(a.checked_mul(b).ok_or("value is too large")?),
        (Int(a), Float(b)) => Float(a as f64 * b),
        (Float(a), Int(b)) => Float(a * b as f64),
        (Float(a), Float(b)) => Float(a * b),

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

        (Str(a), Int(b)) => Str(a.repeat(b)?),
        (Int(a), Str(b)) => Str(b.repeat(a)?),
        (Array(a), Int(b)) => Array(a.repeat(b)?),
        (Int(a), Array(b)) => Array(b.repeat(a)?),
        (Content(a), b @ Int(_)) => Content(a.repeat(b.cast()?)),
        (a @ Int(_), Content(b)) => Content(b.repeat(a.cast()?)),

        (Int(a), Dyn(b: Duration)) => Dyn(b*(a as f64)),
        (Float(a), Dyn(b: Duration)) => Dyn(b*a),
        (Dyn(a: Duration), Int(b)) => Dyn(a*(b as f64)),
        (Dyn(a: Duration), Float(b)) => Dyn(a*b),

        (a, b) => mismatch!("cannot multiply {} with {}", a, b),
    ))
}

/// Compute the quotient of two values.
pub fn div(lhs: Value, rhs: Value) -> StrResult<Value> {
    if is_zero(&rhs) {
        bail!("cannot divide by zero");
    }

    Ok(match_dyn!((lhs, rhs);
        (Int(a), Int(b)) => Float(a as f64 / b as f64),
        (Int(a), Float(b)) => Float(a as f64 / b),
        (Float(a), Int(b)) => Float(a / b as f64),
        (Float(a), Float(b)) => Float(a / b),

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

        (Dyn(a: Duration), Int(b)) => Dyn(a/(b as f64)),
        (Dyn(a: Duration), Float(b)) => Dyn(a/b),
        (Dyn(a: Duration), Dyn(b: Duration)) => Float(a/b),

        (a, b) => mismatch!("cannot divide {} by {}", a, b),
    ))
}

/// Whether a value is a numeric zero.
fn is_zero(v: &Value) -> bool {
    match *v {
        Int(v) => v == 0,
        Float(v) => v == 0.0,
        Length(v) => v.is_zero(),
        Angle(v) => v.is_zero(),
        Ratio(v) => v.is_zero(),
        Relative(v) => v.is_zero(),
        Fraction(v) => v.is_zero(),
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

/// Compute whether two values are unequal.
pub fn neq(lhs: Value, rhs: Value) -> StrResult<Value> {
    Ok(Bool(!equal(&lhs, &rhs)))
}

macro_rules! comparison {
    ($name:ident, $op:tt, $($pat:tt)*) => {
        /// Compute how a value compares with another value.
        pub fn $name(lhs: Value, rhs: Value) -> StrResult<Value> {
            let ordering = compare(&lhs, &rhs)?;
            Ok(Bool(matches!(ordering, $($pat)*)))
        }
    };
}

comparison!(lt, "<", Ordering::Less);
comparison!(leq, "<=", Ordering::Less | Ordering::Equal);
comparison!(gt, ">", Ordering::Greater);
comparison!(geq, ">=", Ordering::Greater | Ordering::Equal);

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
        (Ratio(a), Ratio(b)) => a == b,
        (Relative(a), Relative(b)) => a == b,
        (Fraction(a), Fraction(b)) => a == b,
        (Color(a), Color(b)) => a == b,
        (Symbol(a), Symbol(b)) => a == b,
        (Str(a), Str(b)) => a == b,
        (Label(a), Label(b)) => a == b,
        (Content(a), Content(b)) => a == b,
        (Array(a), Array(b)) => a == b,
        (Dict(a), Dict(b)) => a == b,
        (Func(a), Func(b)) => a == b,
        (Args(a), Args(b)) => a == b,
        (Module(a), Module(b)) => a == b,
        (Dyn(a), Dyn(b)) => a == b,

        // Some technically different things should compare equal.
        (&Int(a), &Float(b)) => a as f64 == b,
        (&Float(a), &Int(b)) => a == b as f64,
        (&Length(a), &Relative(b)) => a == b.abs && b.rel.is_zero(),
        (&Ratio(a), &Relative(b)) => a == b.rel && b.abs.is_zero(),
        (&Relative(a), &Length(b)) => a.abs == b && a.rel.is_zero(),
        (&Relative(a), &Ratio(b)) => a.rel == b && a.abs.is_zero(),

        _ => false,
    }
}

/// Compare two values.
pub fn compare(lhs: &Value, rhs: &Value) -> StrResult<Ordering> {
    Ok(match (lhs, rhs) {
        (Bool(a), Bool(b)) => a.cmp(b),
        (Int(a), Int(b)) => a.cmp(b),
        (Float(a), Float(b)) => try_cmp_values(a, b)?,
        (Length(a), Length(b)) => try_cmp_values(a, b)?,
        (Angle(a), Angle(b)) => a.cmp(b),
        (Ratio(a), Ratio(b)) => a.cmp(b),
        (Relative(a), Relative(b)) => try_cmp_values(a, b)?,
        (Fraction(a), Fraction(b)) => a.cmp(b),
        (Str(a), Str(b)) => a.cmp(b),

        // Some technically different things should be comparable.
        (Int(a), Float(b)) => try_cmp_values(&(*a as f64), b)?,
        (Float(a), Int(b)) => try_cmp_values(a, &(*b as f64))?,
        (Length(a), Relative(b)) if b.rel.is_zero() => try_cmp_values(a, &b.abs)?,
        (Ratio(a), Relative(b)) if b.abs.is_zero() => a.cmp(&b.rel),
        (Relative(a), Length(b)) if a.rel.is_zero() => try_cmp_values(&a.abs, b)?,
        (Relative(a), Ratio(b)) if a.abs.is_zero() => a.rel.cmp(b),

        _ => mismatch!("cannot compare {} and {}", lhs, rhs),
    })
}

/// Try to compare two values.
fn try_cmp_values<T: PartialOrd + Debug>(a: &T, b: &T) -> StrResult<Ordering> {
    a.partial_cmp(b)
        .ok_or_else(|| eco_format!("cannot compare {:?} with {:?}", a, b))
}

/// Test whether one value is "in" another one.
pub fn in_(lhs: Value, rhs: Value) -> StrResult<Value> {
    if let Some(b) = contains(&lhs, &rhs) {
        Ok(Bool(b))
    } else {
        mismatch!("cannot apply 'in' to {} and {}", lhs, rhs)
    }
}

/// Test whether one value is "not in" another one.
pub fn not_in(lhs: Value, rhs: Value) -> StrResult<Value> {
    if let Some(b) = contains(&lhs, &rhs) {
        Ok(Bool(!b))
    } else {
        mismatch!("cannot apply 'not in' to {} and {}", lhs, rhs)
    }
}

/// Test for containment.
pub fn contains(lhs: &Value, rhs: &Value) -> Option<bool> {
    match (lhs, rhs) {
        (Str(a), Str(b)) => Some(b.as_str().contains(a.as_str())),
        (Dyn(a), Str(b)) => a.downcast::<Regex>().map(|regex| regex.is_match(b)),
        (Str(a), Dict(b)) => Some(b.contains(a)),
        (a, Array(b)) => Some(b.contains(a)),
        _ => Option::None,
    }
}
