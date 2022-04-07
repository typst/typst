use std::str::FromStr;

use crate::library::prelude::*;

/// Create an RGB(A) color.
pub fn rgb(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    Ok(Value::from(
        if let Some(string) = args.find::<Spanned<EcoString>>()? {
            match RgbaColor::from_str(&string.v) {
                Ok(color) => color,
                Err(msg) => bail!(string.span, msg),
            }
        } else {
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

            let Component(r) = args.expect("red component")?;
            let Component(g) = args.expect("green component")?;
            let Component(b) = args.expect("blue component")?;
            let Component(a) = args.eat()?.unwrap_or(Component(255));
            RgbaColor::new(r, g, b, a)
        },
    ))
}

/// Create a CMYK color.
pub fn cmyk(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    struct Component(u8);

    castable! {
        Component,
        Expected: "ratio",
        Value::Ratio(v) => if (0.0 ..= 1.0).contains(&v.get()) {
            Self((v.get() * 255.0).round() as u8)
        } else {
            Err("must be between 0% and 100%")?
        },
    }

    let Component(c) = args.expect("cyan component")?;
    let Component(m) = args.expect("magenta component")?;
    let Component(y) = args.expect("yellow component")?;
    let Component(k) = args.expect("key component")?;
    Ok(Value::Color(CmykColor::new(c, m, y, k).into()))
}
