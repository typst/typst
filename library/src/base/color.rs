use std::str::FromStr;

use crate::prelude::*;

/// Create a grayscale color.
pub fn luma(_: &mut Vm, args: &mut Args) -> SourceResult<Value> {
    let Component(luma) = args.expect("gray component")?;
    Ok(Value::Color(LumaColor::new(luma).into()))
}

/// Create an RGB(A) color.
pub fn rgb(_: &mut Vm, args: &mut Args) -> SourceResult<Value> {
    Ok(Value::Color(
        if let Some(string) = args.find::<Spanned<EcoString>>()? {
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
        },
    ))
}

/// Create a CMYK color.
pub fn cmyk(_: &mut Vm, args: &mut Args) -> SourceResult<Value> {
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
