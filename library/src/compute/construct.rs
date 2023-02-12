use std::str::FromStr;

use typst::model::Regex;

use crate::prelude::*;

/// # Integer
/// Convert a value to an integer.
///
/// - Booleans are converted to `0` or `1`.
/// - Floats are floored to the next 64-bit integer.
/// - Strings are parsed in base 10.
///
/// ## Example
/// ```example
/// #int(false) \
/// #int(true) \
/// #int(2.7) \
/// #{ int("27") + int("4") }
/// ```
///
/// ## Parameters
/// - value: `ToInt` (positional, required)
///   The value that should be converted to an integer.
///
/// - returns: integer
///
/// ## Category
/// construct
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

/// # Float
/// Convert a value to a float.
///
/// - Booleans are converted to `0.0` or `1.0`.
/// - Integers are converted to the closest 64-bit float.
/// - Strings are parsed in base 10 to the closest 64-bit float.
///   Exponential notation is supported.
///
/// ## Example
/// ```example
/// #float(false) \
/// #float(true) \
/// #float(4) \
/// #float("2.7") \
/// #float("1e5")
/// ```
///
/// ## Parameters
/// - value: `ToFloat` (positional, required)
///   The value that should be converted to a float.
///
/// - returns: float
///
/// ## Category
/// construct
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

/// # Luma
/// Create a grayscale color.
///
/// ## Example
/// ```example
/// #for x in range(250, step: 50) {
///   box(square(fill: luma(x)))
/// }
/// ```
///
/// ## Parameters
/// - gray: `Component` (positional, required)
///   The gray component.
///
/// - returns: color
///
/// ## Category
/// construct
#[func]
pub fn luma(args: &mut Args) -> SourceResult<Value> {
    let Component(luma) = args.expect("gray component")?;
    Ok(Value::Color(LumaColor::new(luma).into()))
}

/// # RGBA
/// Create an RGB(A) color.
///
/// The color is specified in the sRGB color space.
///
/// _Note:_ While you can specify transparent colors and Typst's preview will
/// render them correctly, the PDF export does not handle them properly at the
/// moment. This will be fixed in the future.
///
/// ## Example
/// ```example
/// #square(fill: rgb("#b1f2eb"))
/// #square(fill: rgb(87, 127, 230))
/// #square(fill: rgb(25%, 13%, 65%))
/// ```
///
/// ## Parameters
/// - hex: `EcoString` (positional)
///   The color in hexadecimal notation.
///
///   Accepts three, four, six or eight hexadecimal digits and optionally
///   a leading hashtag.
///
///   If this string is given, the individual components should not be given.
///
///   ```example
///   #text(16pt, rgb("#239dad"))[
///     *Typst*
///   ]
///   ```
///
/// - red: `Component` (positional)
///   The red component.
///
/// - green: `Component` (positional)
///   The green component.
///
/// - blue: `Component` (positional)
///   The blue component.
///
/// - alpha: `Component` (positional)
///   The alpha component.
///
/// - returns: color
///
/// ## Category
/// construct
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

/// # CMYK
/// Create a CMYK color.
///
/// This is useful if you want to target a specific printer. The conversion
/// to RGB for display preview might differ from how your printer reproduces
/// the color.
///
/// ## Example
/// ```example
/// #square(
///   fill: cmyk(27%, 0%, 3%, 5%)
/// )
/// ````
///
/// ## Parameters
/// - cyan: `RatioComponent` (positional, required)
///   The cyan component.
///
/// - magenta: `RatioComponent` (positional, required)
///   The magenta component.
///
/// - yellow: `RatioComponent` (positional, required)
///   The yellow component.
///
/// - key: `RatioComponent` (positional, required)
///   The key component.
///
/// - returns: color
///
/// ## Category
/// construct
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

/// # Symbol
/// Create a custom symbol with modifiers.
///
/// ## Example
/// ```example
/// #let envelope = symbol(
///   "🖂",
///   ("stamped", "🖃"),
///   ("stamped.pen", "🖆"),
///   ("lightning", "🖄"),
///   ("fly", "🖅"),
/// )
///
/// #envelope
/// #envelope.stamped
/// #envelope.stamped.pen
/// #envelope.lightning
/// #envelope.fly
/// ```
///
/// ## Parameters
/// - variants: `Variant` (positional, variadic)
///   The variants of the symbol.
///
///   Can be a just a string consisting of a single character for the
///   modifierless variant or an array with two strings specifying the modifiers
///   and the symbol. Individual modifiers should be separated by dots. When
///   displaying a symbol, Typst selects the first from the variants that have
///   all attached modifiers and the minimum number of other modifiers.
///
/// - returns: symbol
///
/// ## Category
/// construct
#[func]
pub fn symbol(args: &mut Args) -> SourceResult<Value> {
    let mut list: Vec<(EcoString, char)> = vec![];
    for Spanned { v, span } in args.all::<Spanned<Variant>>()? {
        if list.iter().any(|(prev, _)| &v.0 == prev) {
            bail!(span, "duplicate variant");
        }
        list.push((v.0, v.1));
    }
    Ok(Value::Symbol(Symbol::runtime(list)))
}

/// A value that can be cast to a symbol.
struct Variant(EcoString, char);

castable! {
    Variant,
    c: char => Self(EcoString::new(), c),
    array: Array => {
        let mut iter = array.into_iter();
        match (iter.next(), iter.next(), iter.next()) {
            (Some(a), Some(b), None) => Self(a.cast()?, b.cast()?),
            _ => Err("point array must contain exactly two entries")?,
        }
    },
}

/// # String
/// Convert a value to a string.
///
/// - Integers are formatted in base 10.
/// - Floats are formatted in base 10 and never in exponential notation.
/// - From labels the name is extracted.
///
/// ## Example
/// ```example
/// #str(10) \
/// #str(2.7) \
/// #str(1e8) \
/// #str(<intro>)
/// ```
///
/// ## Parameters
/// - value: `ToStr` (positional, required)
///   The value that should be converted to a string.
///
/// - returns: string
///
/// ## Category
/// construct
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

/// # Label
/// Create a label from a string.
///
/// Inserting a label into content attaches it to the closest previous element
/// that is not a space. Then, the element can be [referenced]($func/ref) and
/// styled through the label.
///
/// ## Example
/// ```example
/// #show <a>: set text(blue)
/// #show label("b"): set text(red)
///
/// = Heading <a>
/// *Strong* #label("b")
/// ```
///
/// ## Syntax
/// This function also has dedicated syntax: You can create a label by enclosing
/// its name in angle brackets. This works both in markup and code.
///
/// ## Parameters
/// - name: `EcoString` (positional, required)
///   The name of the label.
///
/// - returns: label
///
/// ## Category
/// construct
#[func]
pub fn label(args: &mut Args) -> SourceResult<Value> {
    Ok(Value::Label(Label(args.expect("string")?)))
}

/// # Regex
/// Create a regular expression from a string.
///
/// The result can be used as a
/// [show rule selector]($styling/#show-rules) and with
/// [string methods]($type/string) like `find`, `split`, and `replace`.
///
/// [See here](https://docs.rs/regex/latest/regex/#syntax) for a specification
/// of the supported syntax.
///
/// ## Example
/// ```example
/// // Works with show rules.
/// #show regex("\d+"): set text(red)
///
/// The numbers 1 to 10.
///
/// // Works with string methods.
/// #{ "a,b;c"
///     .split(regex("[,;]")) }
/// ```
///
/// ## Parameters
/// - regex: `EcoString` (positional, required)
///   The regular expression as a string.
///
///   Most regex escape sequences just work because they are not valid Typst
///   escape sequences. To produce regex escape sequences that are also valid in
///   Typst (e.g. `[\\]`), you need to escape twice. Thus, to match a verbatim
///   backslash, you would need to write `{regex("\\\\")}`.
///
/// - returns: regex
///
/// ## Category
/// construct
#[func]
pub fn regex(args: &mut Args) -> SourceResult<Value> {
    let Spanned { v, span } = args.expect::<Spanned<EcoString>>("regular expression")?;
    Ok(Regex::new(&v).at(span)?.into())
}

/// # Range
/// Create an array consisting of a sequence of numbers.
///
/// If you pass just one positional parameter, it is interpreted as the `end` of
/// the range. If you pass two, they describe the `start` and `end` of the
/// range.
///
/// ## Example
/// ```example
/// #range(5) \
/// #range(2, 5) \
/// #range(20, step: 4) \
/// #range(21, step: 4) \
/// #range(5, 2, step: -1)
/// ```
///
/// ## Parameters
/// - start: `i64` (positional)
///   The start of the range (inclusive).
///
/// - end: `i64` (positional, required)
///   The end of the range (exclusive).
///
/// - step: `i64` (named)
///   The distance between the generated numbers.
///
/// - returns: array
///
/// ## Category
/// construct
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
