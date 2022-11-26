use std::str::FromStr;

use typst::model::Regex;

use crate::prelude::*;

/// Convert a value to an integer.
pub fn int(_: &Vm, args: &mut Args) -> SourceResult<Value> {
    let Spanned { v, span } = args.expect("value")?;
    Ok(Value::Int(match v {
        Value::Bool(v) => v as i64,
        Value::Int(v) => v,
        Value::Float(v) => v as i64,
        Value::Str(v) => match v.parse() {
            Ok(v) => v,
            Err(_) => bail!(span, "invalid integer"),
        },
        v => bail!(span, "cannot convert {} to integer", v.type_name()),
    }))
}

/// Convert a value to a float.
pub fn float(_: &Vm, args: &mut Args) -> SourceResult<Value> {
    let Spanned { v, span } = args.expect("value")?;
    Ok(Value::Float(match v {
        Value::Int(v) => v as f64,
        Value::Float(v) => v,
        Value::Str(v) => match v.parse() {
            Ok(v) => v,
            Err(_) => bail!(span, "invalid float"),
        },
        v => bail!(span, "cannot convert {} to float", v.type_name()),
    }))
}

/// Create a grayscale color.
pub fn luma(_: &Vm, args: &mut Args) -> SourceResult<Value> {
    let Component(luma) = args.expect("gray component")?;
    Ok(Value::Color(LumaColor::new(luma).into()))
}

/// Create an RGB(A) color.
pub fn rgb(_: &Vm, args: &mut Args) -> SourceResult<Value> {
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

/// Create a CMYK color.
pub fn cmyk(_: &Vm, args: &mut Args) -> SourceResult<Value> {
    let RatioComponent(c) = args.expect("cyan component")?;
    let RatioComponent(m) = args.expect("magenta component")?;
    let RatioComponent(y) = args.expect("yellow component")?;
    let RatioComponent(k) = args.expect("key component")?;
    Ok(Value::Color(CmykColor::new(c, m, y, k).into()))
}

/// An integer or ratio component.
struct Component(u8);

castable! {
    Component,
    Expected: "integer or ratio",
    Value::Int(v) => match v {
        0 ..= 255 => Self(v as u8),
        _ => Err("must be between 0 and 255")?,
    },
    Value::Ratio(v) => if (0.0 ..= 1.0).contains(&v.get()) {
        Self((v.get() * 255.0).round() as u8)
    } else {
        Err("must be between 0% and 100%")?
    },
}

/// A component that must be a ratio.
struct RatioComponent(u8);

castable! {
    RatioComponent,
    Expected: "ratio",
    Value::Ratio(v) => if (0.0 ..= 1.0).contains(&v.get()) {
        Self((v.get() * 255.0).round() as u8)
    } else {
        Err("must be between 0% and 100%")?
    },
}

/// Convert a value to a string.
pub fn str(_: &Vm, args: &mut Args) -> SourceResult<Value> {
    let Spanned { v, span } = args.expect("value")?;
    Ok(Value::Str(match v {
        Value::Int(v) => format_str!("{}", v),
        Value::Float(v) => format_str!("{}", v),
        Value::Label(label) => label.0.into(),
        Value::Str(v) => v,
        v => bail!(span, "cannot convert {} to string", v.type_name()),
    }))
}

/// Create a blind text string.
pub fn lorem(_: &Vm, args: &mut Args) -> SourceResult<Value> {
    let words: usize = args.expect("number of words")?;
    Ok(Value::Str(lipsum::lipsum(words).into()))
}

/// Create a label from a string.
pub fn label(_: &Vm, args: &mut Args) -> SourceResult<Value> {
    Ok(Value::Label(Label(args.expect("string")?)))
}

/// Create a regular expression from a string.
pub fn regex(_: &Vm, args: &mut Args) -> SourceResult<Value> {
    let Spanned { v, span } = args.expect::<Spanned<EcoString>>("regular expression")?;
    Ok(Regex::new(&v).at(span)?.into())
}

/// Create an array consisting of a sequence of numbers.
pub fn range(_: &Vm, args: &mut Args) -> SourceResult<Value> {
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
