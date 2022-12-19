use std::str::FromStr;

use typst::model::Regex;

use crate::prelude::*;

/// Convert a value to an integer.
///
/// # Parameters
/// - value: ToInt (positional, required)
///   The value that should be converted to an integer.
///
/// # Tags
/// - create
#[func]
pub fn int(args: &mut Args) -> SourceResult<Value> {
    Ok(Value::Int(args.expect::<ToInt>("value")?.0))
}

/// A value that can be cast to an integer.
struct ToInt(i64);

castable! {
    ToInt,
    v: bool => Self(v as i64),
    v: i64 => Self(v),
    v: f64 => Self(v as i64),
    v: EcoString => Self(v.parse().map_err(|_| "not a valid integer")?),
}

/// Convert a value to a float.
///
/// # Parameters
/// - value: ToFloat (positional, required)
///   The value that should be converted to a float.
///
/// # Tags
/// - create
#[func]
pub fn float(args: &mut Args) -> SourceResult<Value> {
    Ok(Value::Float(args.expect::<ToFloat>("value")?.0))
}

/// A value that can be cast to a float.
struct ToFloat(f64);

castable! {
    ToFloat,
    v: bool => Self(v as i64 as f64),
    v: i64 => Self(v as f64),
    v: f64 => Self(v),
    v: EcoString => Self(v.parse().map_err(|_| "not a valid float")?),
}

/// Create a grayscale color.
///
/// # Parameters
/// - gray: Component (positional, required)
///   The gray component.
///
/// # Tags
/// - create
#[func]
pub fn luma(args: &mut Args) -> SourceResult<Value> {
    let Component(luma) = args.expect("gray component")?;
    Ok(Value::Color(LumaColor::new(luma).into()))
}

/// Create an RGB(A) color.
///
/// # Parameters
/// - hex: EcoString (positional)
///   The color in hexademical notation.
///
///   Accepts three, four, six or eight hexadecimal digits and optionally
///   a leading hashtag.
///
///   If this string is given, the individual components should not be given.
///
///   # Example
///   ```
///   #let color = rgb("#239dad")
///   #text(16pt, color)[*Typst*]
///   ```
///
/// - red: Component (positional)
///   The red component.
///
/// - green: Component (positional)
///   The green component.
///
/// - blue: Component (positional)
///   The blue component.
///
/// - alpha: Component (positional)
///   The alpha component.
///
/// # Tags
/// - create
#[func]
pub fn rgb(args: &mut Args) -> SourceResult<Value> {
    Ok(Value::Color(if let Some(string) = args.find::<Spanned<EcoString>>()? {
        match RgbaColor::from_str(&string.v) {
            Ok(color) => color.into(),
            Err(msg) => bail!(string.span, msg),
        }
    } else {
        let Component(r) = args.expect("red component")?;
        let Component(g) = args.expect("green component")?;
        let Component(b) = args.expect("blue component")?;
        let Component(a) = args.eat()?.unwrap_or(Component(255));
        RgbaColor::new(r, g, b, a).into()
    }))
}

/// An integer or ratio component.
struct Component(u8);

castable! {
    Component,
    v: i64 => match v {
        0 ..= 255 => Self(v as u8),
        _ => Err("must be between 0 and 255")?,
    },
    v: Ratio => if (0.0 ..= 1.0).contains(&v.get()) {
        Self((v.get() * 255.0).round() as u8)
    } else {
        Err("must be between 0% and 100%")?
    },
}

/// Create a CMYK color.
///
/// # Parameters
/// - cyan: RatioComponent (positional, required)
///   The cyan component.
///
/// - magenta: RatioComponent (positional, required)
///   The magenta component.
///
/// - yellow: RatioComponent (positional, required)
///   The yellow component.
///
/// - key: RatioComponent (positional, required)
///   The key component.
///
/// # Tags
/// - create
#[func]
pub fn cmyk(args: &mut Args) -> SourceResult<Value> {
    let RatioComponent(c) = args.expect("cyan component")?;
    let RatioComponent(m) = args.expect("magenta component")?;
    let RatioComponent(y) = args.expect("yellow component")?;
    let RatioComponent(k) = args.expect("key component")?;
    Ok(Value::Color(CmykColor::new(c, m, y, k).into()))
}

/// A component that must be a ratio.
struct RatioComponent(u8);

castable! {
    RatioComponent,
    v: Ratio => if (0.0 ..= 1.0).contains(&v.get()) {
        Self((v.get() * 255.0).round() as u8)
    } else {
        Err("must be between 0% and 100%")?
    },
}

/// Convert a value to a string.
///
/// # Parameters
/// - value: ToStr (positional, required)
///   The value that should be converted to a string.
///
/// # Tags
/// - create
#[func]
pub fn str(args: &mut Args) -> SourceResult<Value> {
    Ok(Value::Str(args.expect::<ToStr>("value")?.0))
}

/// A value that can be cast to a string.
struct ToStr(Str);

castable! {
    ToStr,
    v: i64 => Self(format_str!("{}", v)),
    v: f64 => Self(format_str!("{}", v)),
    v: Label => Self(v.0.into()),
    v: Str => Self(v),
}

/// Create a label from a string.
///
/// # Parameters
/// - name: EcoString (positional, required)
///   The name of the label.
///
/// # Tags
/// - create
#[func]
pub fn label(args: &mut Args) -> SourceResult<Value> {
    Ok(Value::Label(Label(args.expect("string")?)))
}

/// Create a regular expression from a string.
///
/// # Parameters
/// - regex: EcoString (positional, required)
///   The regular expression.
///
/// # Tags
/// - create
#[func]
pub fn regex(args: &mut Args) -> SourceResult<Value> {
    let Spanned { v, span } = args.expect::<Spanned<EcoString>>("regular expression")?;
    Ok(Regex::new(&v).at(span)?.into())
}

/// Create an array consisting of a sequence of numbers.
///
/// # Parameters
/// - start: i64 (positional)
///   The start of the range (inclusive).
///
/// - end: i64 (positional, required)
///   The end of the range (exclusive).
///
/// - step: i64 (named)
///   The distance between the generated numbers.
///
/// # Tags
/// - create
#[func]
pub fn range(args: &mut Args) -> SourceResult<Value> {
    let first = args.expect::<i64>("end")?;
    let (start, end) = match args.eat::<i64>()? {
        Some(second) => (first, second),
        None => (0, first),
    };

    let step: i64 = match args.named("step")? {
        Some(Spanned { v: 0, span }) => bail!(span, "step must not be zero"),
        Some(Spanned { v, .. }) => v,
        None => 1,
    };

    let mut x = start;
    let mut seq = vec![];

    while x.cmp(&end) == 0.cmp(&step) {
        seq.push(Value::Int(x));
        x += step;
    }

    Ok(Value::Array(Array::from_vec(seq)))
}
