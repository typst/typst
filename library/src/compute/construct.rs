use std::num::NonZeroI64;
use std::str::FromStr;

use time::{Month, PrimitiveDateTime};

use typst::eval::{Datetime, Dynamic, Regex};

use crate::prelude::*;

/// Convert a value to an integer.
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
/// Returns: integer
#[func]
pub fn int(
    /// The value that should be converted to an integer.
    value: ToInt,
) -> Value {
    Value::Int(value.0)
}

/// A value that can be cast to an integer.
struct ToInt(i64);

cast_from_value! {
    ToInt,
    v: bool => Self(v as i64),
    v: i64 => Self(v),
    v: f64 => Self(v as i64),
    v: EcoString => Self(v.parse().map_err(|_| eco_format!("invalid integer: {}", v))?),
}

/// Convert a value to a float.
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
/// Returns: float
#[func]
pub fn float(
    /// The value that should be converted to a float.
    value: ToFloat,
) -> Value {
    Value::Float(value.0)
}

/// A value that can be cast to a float.
struct ToFloat(f64);

cast_from_value! {
    ToFloat,
    v: bool => Self(v as i64 as f64),
    v: i64 => Self(v as f64),
    v: f64 => Self(v),
    v: Ratio => Self(v.get()),
    v: EcoString => Self(v.parse().map_err(|_| eco_format!("invalid float: {}", v))?),
}

/// Create a grayscale color.
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
/// Returns: color
#[func]
pub fn luma(
    /// The gray component.
    gray: Component,
) -> Value {
    Value::Color(LumaColor::new(gray.0).into())
}

/// Create an RGB(A) color.
///
/// The color is specified in the sRGB color space.
///
/// _Note:_ While you can specify transparent colors and Typst's preview will
/// render them correctly, the PDF export does not handle them properly at the
/// moment. This will be fixed in the future.
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
/// Returns: color
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
) -> Value {
    Value::Color(if let Some(string) = args.find::<Spanned<EcoString>>()? {
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
    })
}

/// An integer or ratio component.
struct Component(u8);

cast_from_value! {
    Component,
    v: i64 => match v {
        0 ..= 255 => Self(v as u8),
        _ => Err("number must be between 0 and 255")?,
    },
    v: Ratio => if (0.0 ..= 1.0).contains(&v.get()) {
        Self((v.get() * 255.0).round() as u8)
    } else {
        Err("ratio must be between 0% and 100%")?
    },
}

/// Create a new datetime.
///
/// You can specify the [datetime]($type/datetime) using a year, month, day,
/// hour, minute, and second.
///
/// ## Example
/// ```example
/// #datetime(
///   year: 2012,
///   month: 8,
///   day: 3,
/// ).display()
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
/// Returns: datetime
#[func]
#[scope(
    scope.define("today", datetime_today);
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
) -> Value {
    let time = match (hour, minute, second) {
        (Some(hour), Some(minute), Some(second)) => {
            match time::Time::from_hms(hour.0, minute.0, second.0) {
                Ok(time) => Some(time),
                Err(_) => bail!(args.span, "time is invalid"),
            }
        }
        (None, None, None) => None,
        _ => bail!(args.span, "time is incomplete"),
    };

    let date = match (year, month, day) {
        (Some(year), Some(month), Some(day)) => {
            match time::Date::from_calendar_date(year.0, month.0, day.0) {
                Ok(date) => Some(date),
                Err(_) => bail!(args.span, "date is invalid"),
            }
        }
        (None, None, None) => None,
        _ => bail!(args.span, "date is incomplete"),
    };

    match (date, time) {
        (Some(date), Some(time)) => Value::Dyn(Dynamic::new(Datetime::Datetime(
            PrimitiveDateTime::new(date, time),
        ))),
        (Some(date), None) => Value::Dyn(Dynamic::new(Datetime::Date(date))),
        (None, Some(time)) => Value::Dyn(Dynamic::new(Datetime::Time(time))),
        (None, None) => {
            bail!(args.span, "at least one of date or time must be fully specified")
        }
    }
}

struct YearComponent(i32);
struct MonthComponent(Month);
struct DayComponent(u8);
struct HourComponent(u8);
struct MinuteComponent(u8);
struct SecondComponent(u8);

cast_from_value!(
    YearComponent,
    v: i64 => match i32::try_from(v) {
        Ok(n) => Self(n),
        _ => Err("year is invalid")?
    }
);

cast_from_value!(
    MonthComponent,
    v: i64 => match u8::try_from(v).ok().and_then(|n1| Month::try_from(n1).ok()).map(Self) {
        Some(m) => m,
        _ => Err("month is invalid")?
    }
);

cast_from_value!(
    DayComponent,
    v: i64 => match u8::try_from(v) {
        Ok(n) => Self(n),
        _ => Err("day is invalid")?
    }
);

cast_from_value!(
    HourComponent,
    v: i64 => match u8::try_from(v) {
        Ok(n) => Self(n),
        _ => Err("hour is invalid")?
    }
);

cast_from_value!(
    MinuteComponent,
    v: i64 => match u8::try_from(v) {
        Ok(n) => Self(n),
        _ => Err("minute is invalid")?
    }
);

cast_from_value!(
    SecondComponent,
    v: i64 => match u8::try_from(v) {
        Ok(n) => Self(n),
        _ => Err("second is invalid")?
    }
);

/// Returns the current date.
///
/// ## Example
/// ```example
/// Today's date is
/// #datetime.today().display().
/// ```
///
/// Display: Today
/// Category: construct
/// Returns: datetime
#[func]
pub fn datetime_today(
    /// An offset to apply to the current UTC date. If set to `{auto}`, the
    /// offset will be the local offset.
    #[named]
    #[default]
    offset: Smart<i64>,
) -> Value {
    let current_date = match vm.vt.world.today(offset.as_custom()) {
        Some(d) => d,
        None => bail!(args.span, "unable to get the current date"),
    };

    Value::Dyn(Dynamic::new(current_date))
}

/// Create a CMYK color.
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
/// Returns: color
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
) -> Value {
    Value::Color(CmykColor::new(cyan.0, magenta.0, yellow.0, key.0).into())
}

/// A component that must be a ratio.
struct RatioComponent(u8);

cast_from_value! {
    RatioComponent,
    v: Ratio => if (0.0 ..= 1.0).contains(&v.get()) {
        Self((v.get() * 255.0).round() as u8)
    } else {
        Err("ratio must be between 0% and 100%")?
    },
}

/// Create a custom symbol with modifiers.
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
/// Returns: symbol
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
) -> Value {
    let mut list = Vec::new();
    if variants.is_empty() {
        bail!(args.span, "expected at least one variant");
    }
    for Spanned { v, span } in variants {
        if list.iter().any(|(prev, _)| &v.0 == prev) {
            bail!(span, "duplicate variant");
        }
        list.push((v.0, v.1));
    }
    Value::Symbol(Symbol::runtime(list.into_boxed_slice()))
}

/// A value that can be cast to a symbol.
struct Variant(EcoString, char);

cast_from_value! {
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

/// Convert a value to a string.
///
/// - Integers are formatted in base 10.
/// - Floats are formatted in base 10 and never in exponential notation.
/// - From labels the name is extracted.
///
/// ## Example { #example }
/// ```example
/// #str(10) \
/// #str(2.7) \
/// #str(1e8) \
/// #str(<intro>)
/// ```
///
/// Display: String
/// Category: construct
/// Returns: string
#[func]
pub fn str(
    /// The value that should be converted to a string.
    value: ToStr,
) -> Value {
    Value::Str(value.0)
}

/// A value that can be cast to a string.
struct ToStr(Str);

cast_from_value! {
    ToStr,
    v: i64 => Self(format_str!("{}", v)),
    v: f64 => Self(format_str!("{}", v)),
    v: Label => Self(v.0.into()),
    v: Str => Self(v),
}

/// Converts a unicode codepoint value into it's corresponding string and vice versa.
///
/// ## Example
/// ```example
/// #unicode("a") \
/// #unicode(97) \
/// #"a\u{0300}".codepoints().map(unicode)
/// ```
///
/// Display: Unicode
/// Category: construct
/// Returns: any
#[func]
pub fn unicode(
    /// The value that should be converted.
    value: CharOrInt,
) -> Value {
    match value {
        CharOrInt::Char(c) => Value::Int(From::<u32>::from(c.into())),
        CharOrInt::Int(i) => Value::Str(format_str!("{}", i)),
    }
}

/// A value that is either a single unicdoe code point or it's numeric representation.
enum CharOrInt {
    Char(char),
    Int(char),
}

cast_from_value! {
    CharOrInt,
    v: i64 => {
        if let Some(c) = v.try_into().ok().and_then(|v: u32| v.try_into().ok()) {
            Self::Int(c)
        } else {
            Err(eco_format!("{:#x} is not inside the valid code point range", v))?
        }
    },
    v: Str => {
        match v.chars().next() {
            Some(c) if c.len_utf8() == v.len() as usize => Self::Char(c),
            _ => Err(eco_format!(
                "string must contain exactly one code point, contained {}",
                v.chars().count(),
            ))?,
        }
    },
}

/// Create a label from a string.
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
/// Returns: label
#[func]
pub fn label(
    /// The name of the label.
    name: EcoString,
) -> Value {
    Value::Label(Label(name))
}

/// Create a regular expression from a string.
///
/// The result can be used as a
/// [show rule selector]($styling/#show-rules) and with
/// [string methods]($type/string) like `find`, `split`, and `replace`.
///
/// [See here](https://docs.rs/regex/latest/regex/#syntax) for a specification
/// of the supported syntax.
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
/// Returns: regex
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
) -> Value {
    Regex::new(&regex.v).at(regex.span)?.into()
}

/// Create an array consisting of a sequence of numbers.
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
/// Returns: array
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
) -> Value {
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

    Value::Array(array)
}
