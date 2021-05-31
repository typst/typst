use super::*;

/// `lang`: Configure the language.
///
/// # Positional parameters
/// - Language: of type `string`. Has to be a valid ISO 639-1 code.
///
/// # Named parameters
/// - Text direction: `dir`, of type `direction`, must be horizontal.
///
/// # Return value
/// A template that configures language properties.
///
/// # Relevant types and constants
/// - Type `direction`
///   - `ltr`
///   - `rtl`
pub fn lang(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let iso = args.eat::<String>(ctx).map(|s| lang_dir(&s));
    let dir = match args.eat_named::<Spanned<Dir>>(ctx, "dir") {
        Some(dir) if dir.v.axis() == SpecAxis::Horizontal => Some(dir.v),
        Some(dir) => {
            ctx.diag(error!(dir.span, "must be horizontal"));
            None
        }
        None => None,
    };

    Value::template("lang", move |ctx| {
        if let Some(dir) = dir.or(iso) {
            ctx.state.lang.dir = dir;
        }

        ctx.parbreak();
    })
}

/// The default direction for the language identified by `iso`.
fn lang_dir(iso: &str) -> Dir {
    match iso.to_ascii_lowercase().as_str() {
        "ar" | "he" | "fa" | "ur" | "ps" | "yi" => Dir::RTL,
        "en" | "fr" | "de" | _ => Dir::LTR,
    }
}
