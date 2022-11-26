use typst::model::Regex;

use crate::prelude::*;
use crate::shared::NumberingKind;

/// The string representation of a value.
pub fn repr(_: &Vm, args: &mut Args) -> SourceResult<Value> {
    Ok(args.expect::<Value>("value")?.repr().into())
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

/// Create a label from a string.
pub fn label(_: &Vm, args: &mut Args) -> SourceResult<Value> {
    Ok(Value::Label(Label(args.expect("string")?)))
}

/// Create blind text.
pub fn lorem(_: &Vm, args: &mut Args) -> SourceResult<Value> {
    let words: usize = args.expect("number of words")?;
    Ok(Value::Str(lipsum::lipsum(words).into()))
}

/// Create a regular expression.
pub fn regex(_: &Vm, args: &mut Args) -> SourceResult<Value> {
    let Spanned { v, span } = args.expect::<Spanned<EcoString>>("regular expression")?;
    Ok(Regex::new(&v).at(span)?.into())
}

/// Converts an integer into one or multiple letters.
pub fn letter(_: &Vm, args: &mut Args) -> SourceResult<Value> {
    numbered(NumberingKind::Letter, args)
}

/// Converts an integer into a roman numeral.
pub fn roman(_: &Vm, args: &mut Args) -> SourceResult<Value> {
    numbered(NumberingKind::Roman, args)
}

/// Convert a number into a symbol.
pub fn symbol(_: &Vm, args: &mut Args) -> SourceResult<Value> {
    numbered(NumberingKind::Symbol, args)
}

fn numbered(numbering: NumberingKind, args: &mut Args) -> SourceResult<Value> {
    let n = args.expect::<usize>("non-negative integer")?;
    Ok(Value::Str(numbering.apply(n).into()))
}
