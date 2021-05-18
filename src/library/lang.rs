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
    let iso = args.eat::<String>(ctx).map(|s| s.to_ascii_lowercase());
    let dir = args.eat_named::<Spanned<Dir>>(ctx, "dir");

    Value::template("lang", move |ctx| {
        if let Some(iso) = &iso {
            ctx.state.lang.dir = lang_dir(iso);
        }

        if let Some(dir) = dir {
            if dir.v.axis() == SpecAxis::Horizontal {
                ctx.state.lang.dir = dir.v;
            } else {
                ctx.diag(error!(dir.span, "must be horizontal"));
            }
        }

        ctx.parbreak();
    })
}

/// The default direction for the language identified by `iso`.
fn lang_dir(iso: &str) -> Dir {
    match iso {
        "ar" | "he" | "fa" | "ur" | "ps" | "yi" => Dir::RTL,
        "en" | "fr" | "de" | _ => Dir::LTR,
    }
}
