use crate::exec::{FontState, LineState};
use crate::layout::Paint;

use super::*;

/// `font`: Configure the font.
pub fn font(_: &mut EvalContext, args: &mut FuncArgs) -> TypResult<Value> {
    let size = args.named::<Linear>("size")?.or_else(|| args.eat());
    let style = args.named("style")?;
    let weight = args.named("weight")?;
    let stretch = args.named("stretch")?;
    let top_edge = args.named("top-edge")?;
    let bottom_edge = args.named("bottom-edge")?;
    let fill = args.named("fill")?;

    let list = args.named("family")?.or_else(|| {
        let families: Vec<_> = args.all().collect();
        (!families.is_empty()).then(|| FontDef(Rc::new(families)))
    });

    let serif = args.named("serif")?;
    let sans_serif = args.named("sans-serif")?;
    let monospace = args.named("monospace")?;

    let body = args.expect::<Template>("body")?;

    Ok(Value::template(move |ctx| {
        let state = ctx.state.font_mut();

        if let Some(size) = size {
            state.size = size.resolve(state.size);
        }

        if let Some(style) = style {
            state.variant.style = style;
        }

        if let Some(weight) = weight {
            state.variant.weight = weight;
        }

        if let Some(stretch) = stretch {
            state.variant.stretch = stretch;
        }

        if let Some(top_edge) = top_edge {
            state.top_edge = top_edge;
        }

        if let Some(bottom_edge) = bottom_edge {
            state.bottom_edge = bottom_edge;
        }

        if let Some(fill) = fill {
            state.fill = Paint::Color(fill);
        }

        if let Some(FontDef(list)) = &list {
            state.families_mut().list = list.clone();
        }

        if let Some(FamilyDef(serif)) = &serif {
            state.families_mut().serif = serif.clone();
        }

        if let Some(FamilyDef(sans_serif)) = &sans_serif {
            state.families_mut().sans_serif = sans_serif.clone();
        }

        if let Some(FamilyDef(monospace)) = &monospace {
            state.families_mut().monospace = monospace.clone();
        }

        body.exec(ctx);
    }))
}

struct FontDef(Rc<Vec<FontFamily>>);

castable! {
    FontDef: "font family or array of font families",
    Value::Str(string) => Self(Rc::new(vec![FontFamily::Named(string.to_lowercase())])),
    Value::Array(values) => Self(Rc::new(
        values
            .into_iter()
            .filter_map(|v| v.cast().ok())
            .collect()
    )),
    @family: FontFamily => Self(Rc::new(vec![family.clone()])),
}

struct FamilyDef(Rc<Vec<String>>);

castable! {
    FamilyDef: "string or array of strings",
    Value::Str(string) => Self(Rc::new(vec![string.to_lowercase()])),
    Value::Array(values) => Self(Rc::new(
        values
            .into_iter()
            .filter_map(|v| v.cast().ok())
            .map(|string: EcoString| string.to_lowercase())
            .collect()
    )),
}

/// `par`: Configure paragraphs.
pub fn par(_: &mut EvalContext, args: &mut FuncArgs) -> TypResult<Value> {
    let par_spacing = args.named("spacing")?;
    let line_spacing = args.named("leading")?;
    let body = args.expect::<Template>("body")?;

    Ok(Value::template(move |ctx| {
        let state = ctx.state.par_mut();

        if let Some(par_spacing) = par_spacing {
            state.par_spacing = par_spacing;
        }

        if let Some(line_spacing) = line_spacing {
            state.line_spacing = line_spacing;
        }

        ctx.parbreak();
        body.exec(ctx);
    }))
}

/// `lang`: Configure the language.
pub fn lang(_: &mut EvalContext, args: &mut FuncArgs) -> TypResult<Value> {
    let iso = args.eat::<EcoString>();
    let dir = if let Some(dir) = args.named::<Spanned<Dir>>("dir")? {
        if dir.v.axis() == SpecAxis::Horizontal {
            Some(dir.v)
        } else {
            bail!(args.file, dir.span, "must be horizontal");
        }
    } else {
        iso.as_deref().map(lang_dir)
    };

    let body = args.expect::<Template>("body")?;

    Ok(Value::template(move |ctx| {
        if let Some(dir) = dir {
            ctx.state.dirs.cross = dir;
        }

        ctx.parbreak();
        body.exec(ctx);
    }))
}

/// The default direction for the language identified by the given `iso` code.
fn lang_dir(iso: &str) -> Dir {
    match iso.to_ascii_lowercase().as_str() {
        "ar" | "he" | "fa" | "ur" | "ps" | "yi" => Dir::RTL,
        "en" | "fr" | "de" => Dir::LTR,
        _ => Dir::LTR,
    }
}

/// `strike`: Enable striken-through text.
pub fn strike(_: &mut EvalContext, args: &mut FuncArgs) -> TypResult<Value> {
    line_impl(args, |font| &mut font.strikethrough)
}

/// `underline`: Enable underlined text.
pub fn underline(_: &mut EvalContext, args: &mut FuncArgs) -> TypResult<Value> {
    line_impl(args, |font| &mut font.underline)
}

/// `overline`: Add an overline above text.
pub fn overline(_: &mut EvalContext, args: &mut FuncArgs) -> TypResult<Value> {
    line_impl(args, |font| &mut font.overline)
}

fn line_impl(
    args: &mut FuncArgs,
    substate: fn(&mut FontState) -> &mut Option<Rc<LineState>>,
) -> TypResult<Value> {
    let stroke = args.named("stroke")?.or_else(|| args.eat());
    let thickness = args.named::<Linear>("thickness")?.or_else(|| args.eat());
    let offset = args.named("offset")?;
    let extent = args.named("extent")?.unwrap_or_default();
    let body = args.expect::<Template>("body")?;

    // Suppress any existing strikethrough if strength is explicitly zero.
    let state = thickness.map_or(true, |s| !s.is_zero()).then(|| {
        Rc::new(LineState {
            stroke: stroke.map(Paint::Color),
            thickness,
            offset,
            extent,
        })
    });

    Ok(Value::template(move |ctx| {
        *substate(ctx.state.font_mut()) = state.clone();
        body.exec(ctx);
    }))
}
