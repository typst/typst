use crate::eval::Regex;
use crate::library::prelude::*;

/// The string representation of a value.
pub fn repr(_: &mut Machine, args: &mut Args) -> TypResult<Value> {
    Ok(args.expect::<Value>("value")?.repr().into())
}

/// Cconvert a value to a string.
pub fn str(_: &mut Machine, args: &mut Args) -> TypResult<Value> {
    let Spanned { v, span } = args.expect("value")?;
    Ok(Value::Str(match v {
        Value::Int(v) => format_eco!("{}", v),
        Value::Float(v) => format_eco!("{}", v),
        Value::Str(v) => v,
        v => bail!(span, "cannot convert {} to string", v.type_name()),
    }))
}

/// Create blind text.
pub fn lorem(_: &mut Machine, args: &mut Args) -> TypResult<Value> {
    let words: usize = args.expect("number of words")?;
    Ok(Value::Str(lipsum::lipsum(words).into()))
}

/// Create a regular expression.
pub fn regex(_: &mut Machine, args: &mut Args) -> TypResult<Value> {
    let Spanned { v, span } = args.expect::<Spanned<EcoString>>("regular expression")?;
    Ok(Regex::new(&v).at(span)?.into())
}

/// Converts an integer into one or multiple letters.
pub fn letter(_: &mut Machine, args: &mut Args) -> TypResult<Value> {
    convert(Numbering::Letter, args)
}

/// Converts an integer into a roman numeral.
pub fn roman(_: &mut Machine, args: &mut Args) -> TypResult<Value> {
    convert(Numbering::Roman, args)
}

/// Convert a number into a symbol.
pub fn symbol(_: &mut Machine, args: &mut Args) -> TypResult<Value> {
    convert(Numbering::Symbol, args)
}

fn convert(numbering: Numbering, args: &mut Args) -> TypResult<Value> {
    let n = args.expect::<usize>("non-negative integer")?;
    Ok(Value::Str(numbering.apply(n)))
}

/// Allows to convert a number into letters, roman numerals and symbols.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Numbering {
    Arabic,
    Letter,
    Roman,
    Symbol,
}

impl Numbering {
    /// Apply the numbering to the given number.
    pub fn apply(self, mut n: usize) -> EcoString {
        match self {
            Self::Arabic => {
                format_eco!("{}", n)
            }
            Self::Letter => {
                if n == 0 {
                    return '-'.into();
                }

                n -= 1;

                let mut letters = vec![];
                loop {
                    letters.push(b'a' + (n % 26) as u8);
                    n /= 26;
                    if n == 0 {
                        break;
                    }
                }

                letters.reverse();
                String::from_utf8(letters).unwrap().into()
            }
            Self::Roman => {
                if n == 0 {
                    return 'N'.into();
                }

                // Adapted from Yann Villessuzanne's roman.rs under the Unlicense, at
                // https://github.com/linfir/roman.rs/
                let mut fmt = EcoString::new();
                for &(name, value) in ROMANS {
                    while n >= value {
                        n -= value;
                        fmt.push_str(name);
                    }
                }

                fmt
            }
            Self::Symbol => {
                if n == 0 {
                    return '-'.into();
                }

                let symbol = SYMBOLS[(n - 1) % SYMBOLS.len()];
                let amount = ((n - 1) / SYMBOLS.len()) + 1;
                std::iter::repeat(symbol).take(amount).collect()
            }
        }
    }
}

const ROMANS: &[(&str, usize)] = &[
    ("M̅", 1000000),
    ("D̅", 500000),
    ("C̅", 100000),
    ("L̅", 50000),
    ("X̅", 10000),
    ("V̅", 5000),
    ("I̅V̅", 4000),
    ("M", 1000),
    ("CM", 900),
    ("D", 500),
    ("CD", 400),
    ("C", 100),
    ("XC", 90),
    ("L", 50),
    ("XL", 40),
    ("X", 10),
    ("IX", 9),
    ("V", 5),
    ("IV", 4),
    ("I", 1),
];

const SYMBOLS: &[char] = &['*', '†', '‡', '§', '‖', '¶'];
