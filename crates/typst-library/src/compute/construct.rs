use std::num::NonZeroI64;
use std::str::FromStr;

use time::{Month, PrimitiveDateTime};

use typst::eval::{Bytes, Datetime, Module, Reflect, Regex};

use crate::prelude::*;

/// Converts a value to an integer.
///
/// - Booleans are converted to `0` or `1`.
/// - Floats are floored to the next 64-bit integer.
/// - Strings are parsed in base 10.
///
/// ## Example { #example }
/// ```example
/// #int(false) \
/// #int(true) \
/// #int(2.7) \
/// #{ int("27") + int("4") }
/// ```
///
/// Display: Integer
/// Category: construct
#[func]
pub fn int(
    /// The value that should be converted to an integer.
    value: ToInt,
) -> i64 {
    value.0
}

/// A value that can be cast to an integer.
pub struct ToInt(i64);

cast! {
    ToInt,
    v: bool => Self(v as i64),
    v: f64 => Self(v as i64),
    v: EcoString => Self(v.parse().map_err(|_| eco_format!("invalid integer: {}", v))?),
    v: i64 => Self(v),
}

/// Converts a value to a float.
///
/// - Booleans are converted to `0.0` or `1.0`.
/// - Integers are converted to the closest 64-bit float.
/// - Ratios are divided by 100%.
/// - Strings are parsed in base 10 to the closest 64-bit float.
///   Exponential notation is supported.
///
/// ## Example { #example }
/// ```example
/// #float(false) \
/// #float(true) \
/// #float(4) \
/// #float(40%) \
/// #float("2.7") \
/// #float("1e5")
/// ```
///
/// Display: Float
/// Category: construct
#[func]
pub fn float(
    /// The value that should be converted to a float.
    value: ToFloat,
) -> f64 {
    value.0
}

/// A value that can be cast to a float.
pub struct ToFloat(f64);

cast! {
    ToFloat,
    v: bool => Self(v as i64 as f64),
    v: i64 => Self(v as f64),
    v: Ratio => Self(v.get()),
    v: EcoString => Self(v.parse().map_err(|_| eco_format!("invalid float: {}", v))?),
    v: f64 => Self(v),
}

/// Creates a grayscale color.
///
/// ## Example { #example }
/// ```example
/// #for x in range(250, step: 50) {
///   box(square(fill: luma(x)))
/// }
/// ```
///
/// Display: Luma
/// Category: construct
#[func]
pub fn luma(
    /// The gray component.
    gray: Component,
) -> Color {
    LumaColor::new(gray.0).into()
}

/// Creates an RGB(A) color.
///
/// The color is specified in the sRGB color space.
///
/// ## Example { #example }
/// ```example
/// #square(fill: rgb("#b1f2eb"))
/// #square(fill: rgb(87, 127, 230))
/// #square(fill: rgb(25%, 13%, 65%))
/// ```
///
/// Display: RGB
/// Category: construct
#[func]
pub fn rgb(
    /// The color in hexadecimal notation.
    ///
    /// Accepts three, four, six or eight hexadecimal digits and optionally
    /// a leading hashtag.
    ///
    /// If this string is given, the individual components should not be given.
    ///
    /// ```example
    /// #text(16pt, rgb("#239dad"))[
    ///   *Typst*
    /// ]
    /// ```
    #[external]
    hex: EcoString,
    /// The red component.
    #[external]
    red: Component,
    /// The green component.
    #[external]
    green: Component,
    /// The blue component.
    #[external]
    blue: Component,
    /// The alpha component.
    #[external]
    alpha: Component,
    /// The arguments.
    args: Args,
) -> SourceResult<Color> {
    let mut args = args;
    Ok(if let Some(string) = args.find::<Spanned<EcoString>>()? {
        match RgbaColor::from_str(&string.v) {
            Ok(color) => color.into(),
            Err(msg) => bail!(string.span, "{msg}"),
        }
    } else {
        let Component(r) = args.expect("red component")?;
        let Component(g) = args.expect("green component")?;
        let Component(b) = args.expect("blue component")?;
        let Component(a) = args.eat()?.unwrap_or(Component(255));
        RgbaColor::new(r, g, b, a).into()
    })
}

/// An integer or ratio component.
pub struct Component(u8);

cast! {
    Component,
    v: i64 => match v {
        0 ..= 255 => Self(v as u8),
        _ => bail!("number must be between 0 and 255"),
    },
    v: Ratio => if (0.0 ..= 1.0).contains(&v.get()) {
        Self((v.get() * 255.0).round() as u8)
    } else {
        bail!("ratio must be between 0% and 100%");
    },
}

/// Creates a new datetime.
///
/// You can specify the [datetime]($type/datetime) using a year, month, day,
/// hour, minute, and second. You can also get the current date with
/// [`datetime.today`]($func/datetime.today).
///
/// ## Example
/// ```example
/// #let date = datetime(
///   year: 2012,
///   month: 8,
///   day: 3,
/// )
///
/// #date.display() \
/// #date.display(
///   "[day].[month].[year]"
/// )
/// ```
///
/// ## Format
/// _Note_: Depending on which components of the datetime you specify, Typst
/// will store it in one of the following three ways:
/// * If you specify year, month and day, Typst will store just a date.
/// * If you specify hour, minute and second, Typst will store just a time.
/// * If you specify all of year, month, day, hour, minute and second, Typst
///   will store a full datetime.
///
/// Depending on how it is stored, the [`display`]($type/datetime.display)
/// method will choose a different formatting by default.
///
/// Display: Datetime
/// Category: construct
#[func]
#[scope(
    scope.define("today", datetime_today_func());
    scope
)]
pub fn datetime(
    /// The year of the datetime.
    #[named]
    year: Option<YearComponent>,
    /// The month of the datetime.
    #[named]
    month: Option<MonthComponent>,
    /// The day of the datetime.
    #[named]
    day: Option<DayComponent>,
    /// The hour of the datetime.
    #[named]
    hour: Option<HourComponent>,
    /// The minute of the datetime.
    #[named]
    minute: Option<MinuteComponent>,
    /// The second of the datetime.
    #[named]
    second: Option<SecondComponent>,
) -> StrResult<Datetime> {
    let time = match (hour, minute, second) {
        (Some(hour), Some(minute), Some(second)) => {
            match time::Time::from_hms(hour.0, minute.0, second.0) {
                Ok(time) => Some(time),
                Err(_) => bail!("time is invalid"),
            }
        }
        (None, None, None) => None,
        _ => bail!("time is incomplete"),
    };

    let date = match (year, month, day) {
        (Some(year), Some(month), Some(day)) => {
            match time::Date::from_calendar_date(year.0, month.0, day.0) {
                Ok(date) => Some(date),
                Err(_) => bail!("date is invalid"),
            }
        }
        (None, None, None) => None,
        _ => bail!("date is incomplete"),
    };

    Ok(match (date, time) {
        (Some(date), Some(time)) => {
            Datetime::Datetime(PrimitiveDateTime::new(date, time))
        }
        (Some(date), None) => Datetime::Date(date),
        (None, Some(time)) => Datetime::Time(time),
        (None, None) => {
            bail!("at least one of date or time must be fully specified")
        }
    })
}

pub struct YearComponent(i32);
pub struct MonthComponent(Month);
pub struct DayComponent(u8);
pub struct HourComponent(u8);
pub struct MinuteComponent(u8);
pub struct SecondComponent(u8);

cast! {
    YearComponent,
    v: i32 => Self(v),
}

cast! {
    MonthComponent,
    v: u8 => Self(Month::try_from(v).map_err(|_| "month is invalid")?)
}

cast! {
    DayComponent,
    v: u8 => Self(v),
}

cast! {
    HourComponent,
    v: u8 => Self(v),
}

cast! {
    MinuteComponent,
    v: u8 => Self(v),
}

cast! {
    SecondComponent,
    v: u8 => Self(v),
}

/// Returns the current date.
///
/// Refer to the documentation of the [`display`]($type/datetime.display) method
/// for details on how to affect the formatting of the date.
///
/// ## Example
/// ```example
/// Today's date is
/// #datetime.today().display().
/// ```
///
/// Display: Today
/// Category: construct
#[func]
pub fn datetime_today(
    /// An offset to apply to the current UTC date. If set to `{auto}`, the
    /// offset will be the local offset.
    #[named]
    #[default]
    offset: Smart<i64>,
    /// The virtual machine.
    vt: &mut Vt,
) -> StrResult<Datetime> {
    Ok(vt
        .world
        .today(offset.as_custom())
        .ok_or("unable to get the current date")?)
}

/// Creates a CMYK color.
///
/// This is useful if you want to target a specific printer. The conversion
/// to RGB for display preview might differ from how your printer reproduces
/// the color.
///
/// ## Example { #example }
/// ```example
/// #square(
///   fill: cmyk(27%, 0%, 3%, 5%)
/// )
/// ````
///
/// Display: CMYK
/// Category: construct
#[func]
pub fn cmyk(
    /// The cyan component.
    cyan: RatioComponent,
    /// The magenta component.
    magenta: RatioComponent,
    /// The yellow component.
    yellow: RatioComponent,
    /// The key component.
    key: RatioComponent,
) -> Color {
    CmykColor::new(cyan.0, magenta.0, yellow.0, key.0).into()
}

/// A component that must be a ratio.
pub struct RatioComponent(u8);

cast! {
    RatioComponent,
    v: Ratio => if (0.0 ..= 1.0).contains(&v.get()) {
        Self((v.get() * 255.0).round() as u8)
    } else {
        bail!("ratio must be between 0% and 100%");
    },
}

/// A module with functions operating on colors.
pub fn color_module() -> Module {
    let mut scope = Scope::new();
    scope.define("mix", mix_func());
    Module::new("color").with_scope(scope)
}

/// Create a color by mixing two or more colors.
///
/// ## Example
/// ```example
/// #color.mix(red, green)
/// #color.mix(red, green, white)
/// #color.mix(red, green, space: "srgb")
/// #color.mix((red, 30%), (green, 70%))
/// ````
///
/// _Note:_ This function must be specified as `color.mix`, not just `mix`.
/// Currently, `color` is a module, but it is designed to be forward compatible
/// with a future `color` type.
///
/// Display: Mix
/// Category: construct
#[func]
pub fn mix(
    /// The colors, optionally with weights, specified as a pair (array of
    /// length two) of color and weight (float or ratio).
    #[variadic]
    colors: Vec<WeightedColor>,
    /// The color space to mix in. By default, this happens in a perceptual
    /// color space (Oklab).
    #[named]
    #[default(ColorSpace::Oklab)]
    space: ColorSpace,
) -> StrResult<Color> {
    Color::mix(colors, space)
}

/// Creates a custom symbol with modifiers.
///
/// ## Example { #example }
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
/// Display: Symbol
/// Category: construct
#[func]
pub fn symbol(
    /// The variants of the symbol.
    ///
    /// Can be a just a string consisting of a single character for the
    /// modifierless variant or an array with two strings specifying the modifiers
    /// and the symbol. Individual modifiers should be separated by dots. When
    /// displaying a symbol, Typst selects the first from the variants that have
    /// all attached modifiers and the minimum number of other modifiers.
    #[variadic]
    variants: Vec<Spanned<Variant>>,
    /// The callsite span.
    span: Span,
) -> SourceResult<Symbol> {
    let mut list = Vec::new();
    if variants.is_empty() {
        bail!(span, "expected at least one variant");
    }
    for Spanned { v, span } in variants {
        if list.iter().any(|(prev, _)| &v.0 == prev) {
            bail!(span, "duplicate variant");
        }
        list.push((v.0, v.1));
    }
    Ok(Symbol::runtime(list.into_boxed_slice()))
}

/// A value that can be cast to a symbol.
pub struct Variant(EcoString, char);

cast! {
    Variant,
    c: char => Self(EcoString::new(), c),
    array: Array => {
        let mut iter = array.into_iter();
        match (iter.next(), iter.next(), iter.next()) {
            (Some(a), Some(b), None) => Self(a.cast()?, b.cast()?),
            _ => bail!("point array must contain exactly two entries"),
        }
    },
}

/// Converts a value to a string.
///
/// - Integers are formatted in base 10. This can be overridden with the
///   optional `base` parameter.
/// - Floats are formatted in base 10 and never in exponential notation.
/// - From labels the name is extracted.
/// - Bytes are decoded as UTF-8.
///
/// If you wish to convert from and to Unicode code points, see
/// [`str.to-unicode`]($func/str.to-unicode) and
/// [`str.from-unicode`]($func/str.from-unicode).
///
/// ## Example { #example }
/// ```example
/// #str(10) \
/// #str(4000, base: 16) \
/// #str(2.7) \
/// #str(1e8) \
/// #str(<intro>)
/// ```
///
/// Display: String
/// Category: construct
#[func]
#[scope(
    scope.define("to-unicode", str_to_unicode_func());
    scope.define("from-unicode", str_from_unicode_func());
    scope
)]
pub fn str(
    /// The value that should be converted to a string.
    value: ToStr,
    /// The base (radix) to display integers in, between 2 and 36.
    #[named]
    #[default(Spanned::new(10, Span::detached()))]
    base: Spanned<i64>,
) -> SourceResult<Str> {
    Ok(match value {
        ToStr::Str(s) => {
            if base.v != 10 {
                bail!(base.span, "base is only supported for integers");
            }
            s
        }
        ToStr::Int(n) => {
            if base.v < 2 || base.v > 36 {
                bail!(base.span, "base must be between 2 and 36");
            }
            int_to_base(n, base.v).into()
        }
    })
}

/// A value that can be cast to a string.
pub enum ToStr {
    /// A string value ready to be used as-is.
    Str(Str),
    /// An integer about to be formatted in a given base.
    Int(i64),
}

cast! {
    ToStr,
    v: i64 => Self::Int(v),
    v: f64 => Self::Str(format_str!("{}", v)),
    v: Label => Self::Str(v.0.into()),
    v: Bytes => Self::Str(
        std::str::from_utf8(&v)
            .map_err(|_| "bytes are not valid utf-8")?
            .into()
    ),
    v: Str => Self::Str(v),
}

/// Format an integer in a base.
fn int_to_base(mut n: i64, base: i64) -> EcoString {
    if n == 0 {
        return "0".into();
    }

    // In Rust, `format!("{:x}", -14i64)` is not `-e` but `fffffffffffffff2`.
    // So we can only use the built-in for decimal, not bin/oct/hex.
    if base == 10 {
        return eco_format!("{n}");
    }

    // The largest output is `to_base(i64::MIN, 2)`, which is 65 chars long.
    const SIZE: usize = 65;
    let mut digits = [b'\0'; SIZE];
    let mut i = SIZE;

    // It's tempting to take the absolute value, but this will fail for i64::MIN.
    // Instead, we turn n negative, as -i64::MAX is perfectly representable.
    let negative = n < 0;
    if n > 0 {
        n = -n;
    }

    while n != 0 {
        let digit = char::from_digit(-(n % base) as u32, base as u32);
        i -= 1;
        digits[i] = digit.unwrap_or('?') as u8;
        n /= base;
    }

    if negative {
        i -= 1;
        digits[i] = b'-';
    }

    std::str::from_utf8(&digits[i..]).unwrap_or_default().into()
}

/// Converts a character into its corresponding code point.
///
/// ## Example
/// ```example
/// #str.to-unicode("a") \
/// #"a\u{0300}".codepoints().map(str.to-unicode)
/// ```
///
/// Display: String To Unicode
/// Category: construct
#[func]
pub fn str_to_unicode(
    /// The character that should be converted.
    value: char,
) -> u32 {
    value.into()
}

/// Converts a Unicode code point into its corresponding string.
///
/// ```example
/// #str.from-unicode(97)
/// ```
///
/// Display: String From Unicode
/// Category: construct
#[func]
pub fn str_from_unicode(
    /// The code point that should be converted.
    value: CodePoint,
) -> Str {
    format_str!("{}", value.0)
}

/// The numeric representation of a single unicode code point.
pub struct CodePoint(char);

cast! {
    CodePoint,
    v: i64 => {
        Self(v.try_into().ok().and_then(|v: u32| v.try_into().ok()).ok_or_else(
            || eco_format!("{:#x} is not a valid codepoint", v),
        )?)
    },
}

/// Creates a regular expression from a string.
///
/// The result can be used as a
/// [show rule selector]($styling/#show-rules) and with
/// [string methods]($type/string) like `find`, `split`, and `replace`.
///
/// See [the specification of the supported syntax](https://docs.rs/regex/latest/regex/#syntax).
///
/// ## Example { #example }
/// ```example
/// // Works with show rules.
/// #show regex("\d+"): set text(red)
///
/// The numbers 1 to 10.
///
/// // Works with string methods.
/// #("a,b;c"
///     .split(regex("[,;]")))
/// ```
///
/// Display: Regex
/// Category: construct
#[func]
pub fn regex(
    /// The regular expression as a string.
    ///
    /// Most regex escape sequences just work because they are not valid Typst
    /// escape sequences. To produce regex escape sequences that are also valid in
    /// Typst (e.g. `[\\]`), you need to escape twice. Thus, to match a verbatim
    /// backslash, you would need to write `{regex("\\\\")}`.
    ///
    /// If you need many escape sequences, you can also create a raw element
    /// and extract its text to use it for your regular expressions:
    /// ```{regex(`\d+\.\d+\.\d+`.text)}```.
    regex: Spanned<EcoString>,
) -> SourceResult<Regex> {
    Regex::new(&regex.v).at(regex.span)
}

/// Converts a value to bytes.
///
/// - Strings are encoded in UTF-8.
/// - Arrays of integers between `{0}` and `{255}` are converted directly. The
///   dedicated byte representation is much more efficient than the array
///   representation and thus typically used for large byte buffers (e.g. image
///   data).
///
/// ```example
/// #bytes("Hello 😃") \
/// #bytes((123, 160, 22, 0))
/// ```
///
/// Display: Bytes
/// Category: construct
#[func]
pub fn bytes(
    /// The value that should be converted to a string.
    value: ToBytes,
) -> Bytes {
    value.0
}

/// A value that can be cast to bytes.
pub struct ToBytes(Bytes);

cast! {
    ToBytes,
    v: Str => Self(v.as_bytes().into()),
    v: Array => Self(v.iter()
        .map(|v| match v {
            Value::Int(byte @ 0..=255) => Ok(*byte as u8),
            Value::Int(_) => bail!("number must be between 0 and 255"),
            value => Err(<u8 as Reflect>::error(value)),
        })
        .collect::<Result<Vec<u8>, _>>()?
        .into()
    ),
    v: Bytes => Self(v),
}

/// Creates a label from a string.
///
/// Inserting a label into content attaches it to the closest previous element
/// that is not a space. Then, the element can be [referenced]($func/ref) and
/// styled through the label.
///
/// ## Example { #example }
/// ```example
/// #show <a>: set text(blue)
/// #show label("b"): set text(red)
///
/// = Heading <a>
/// *Strong* #label("b")
/// ```
///
/// ## Syntax { #syntax }
/// This function also has dedicated syntax: You can create a label by enclosing
/// its name in angle brackets. This works both in markup and code.
///
/// Display: Label
/// Category: construct
#[func]
pub fn label(
    /// The name of the label.
    name: EcoString,
) -> Label {
    Label(name)
}

/// Converts a value to an array.
///
/// Note that this function is only intended for conversion of a collection-like
/// value to an array, not for creation of an array from individual items. Use
/// the array syntax `(1, 2, 3)` (or `(1,)` for a single-element array) instead.
///
/// ```example
/// #let hi = "Hello 😃"
/// #array(bytes(hi))
/// ```
///
/// Display: Array
/// Category: construct
#[func]
pub fn array(
    /// The value that should be converted to an array.
    value: ToArray,
) -> Array {
    value.0
}

/// A value that can be cast to bytes.
pub struct ToArray(Array);

cast! {
    ToArray,
    v: Bytes => Self(v.iter().map(|&b| Value::Int(b as i64)).collect()),
    v: Array => Self(v),
}

/// Creates an array consisting of consecutive integers.
///
/// If you pass just one positional parameter, it is interpreted as the `end` of
/// the range. If you pass two, they describe the `start` and `end` of the
/// range.
///
/// ## Example { #example }
/// ```example
/// #range(5) \
/// #range(2, 5) \
/// #range(20, step: 4) \
/// #range(21, step: 4) \
/// #range(5, 2, step: -1)
/// ```
///
/// Display: Range
/// Category: construct
#[func]
pub fn range(
    /// The start of the range (inclusive).
    #[external]
    #[default]
    start: i64,
    /// The end of the range (exclusive).
    #[external]
    end: i64,
    /// The distance between the generated numbers.
    #[named]
    #[default(NonZeroI64::new(1).unwrap())]
    step: NonZeroI64,
    /// The arguments.
    args: Args,
) -> SourceResult<Array> {
    let mut args = args;
    let first = args.expect::<i64>("end")?;
    let (start, end) = match args.eat::<i64>()? {
        Some(second) => (first, second),
        None => (0, first),
    };

    let step = step.get();

    let mut x = start;
    let mut array = Array::new();

    while x.cmp(&end) == 0.cmp(&step) {
        array.push(Value::Int(x));
        x += step;
    }

    Ok(array)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_base() {
        assert_eq!(&int_to_base(0, 10), "0");
        assert_eq!(&int_to_base(0, 16), "0");
        assert_eq!(&int_to_base(0, 36), "0");
        assert_eq!(
            &int_to_base(i64::MAX, 2),
            "111111111111111111111111111111111111111111111111111111111111111"
        );
        assert_eq!(
            &int_to_base(i64::MIN, 2),
            "-1000000000000000000000000000000000000000000000000000000000000000"
        );
        assert_eq!(&int_to_base(i64::MAX, 10), "9223372036854775807");
        assert_eq!(&int_to_base(i64::MIN, 10), "-9223372036854775808");
        assert_eq!(&int_to_base(i64::MAX, 16), "7fffffffffffffff");
        assert_eq!(&int_to_base(i64::MIN, 16), "-8000000000000000");
        assert_eq!(&int_to_base(i64::MAX, 36), "1y2p0ij32e8e7");
        assert_eq!(&int_to_base(i64::MIN, 36), "-1y2p0ij32e8e8");
    }
}
