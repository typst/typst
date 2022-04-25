use lipsum::lipsum_from_seed;

use crate::library::prelude::*;

/// Create blind text.
pub fn lipsum(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    let words: usize = args.expect("number of words")?;
    Ok(Value::Str(lipsum_from_seed(words, 97).into()))
}
