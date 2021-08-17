use crate::eval::{FontState, LineState};
use crate::layout::Paint;

use super::*;

/// `font`: Configure the font.
pub fn font(ctx: &mut EvalContext, args: &mut Arguments) -> TypResult<Value> {
    let list = args.named("family")?.or_else(|| {
        let families: Vec<_> = args.all().collect();
        (!families.is_empty()).then(|| FontDef(Rc::new(families)))
    });

    let size = args.named::<Linear>("size")?.or_else(|| args.eat());
    let style = args.named("style")?;
    let weight = args.named("weight")?;
    let stretch = args.named("stretch")?;
    let top_edge = args.named("top-edge")?;
    let bottom_edge = args.named("bottom-edge")?;
    let fill = args.named("fill")?;
    let serif = args.named("serif")?;
    let sans_serif = args.named("sans-serif")?;
    let monospace = args.named("monospace")?;

    ctx.template.modify(move |state| {
        let font = state.font_mut();

        if let Some(size) = size {
            font.size = size.resolve(font.size);
        }

        if let Some(style) = style {
            font.variant.style = style;
        }

        if let Some(weight) = weight {
            font.variant.weight = weight;
        }

        if let Some(stretch) = stretch {
            font.variant.stretch = stretch;
        }

        if let Some(top_edge) = top_edge {
            font.top_edge = top_edge;
        }

        if let Some(bottom_edge) = bottom_edge {
            font.bottom_edge = bottom_edge;
        }

        if let Some(fill) = fill {
            font.fill = Paint::Color(fill);
        }

        if let Some(FontDef(list)) = &list {
            font.families_mut().list = list.clone();
        }

        if let Some(FamilyDef(serif)) = &serif {
            font.families_mut().serif = serif.clone();
        }

        if let Some(FamilyDef(sans_serif)) = &sans_serif {
            font.families_mut().sans_serif = sans_serif.clone();
        }

        if let Some(FamilyDef(monospace)) = &monospace {
            font.families_mut().monospace = monospace.clone();
        }
    });

    Ok(Value::None)
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
            .map(|string: Str| string.to_lowercase())
            .collect()
    )),
}

/// `par`: Configure paragraphs.
pub fn par(ctx: &mut EvalContext, args: &mut Arguments) -> TypResult<Value> {
    let par_spacing = args.named("spacing")?;
    let line_spacing = args.named("leading")?;

    ctx.template.modify(move |state| {
        let par = state.par_mut();

        if let Some(par_spacing) = par_spacing {
            par.par_spacing = par_spacing;
        }

        if let Some(line_spacing) = line_spacing {
            par.line_spacing = line_spacing;
        }
    });

    ctx.template.parbreak();

    Ok(Value::None)
}

/// `lang`: Configure the language.
pub fn lang(ctx: &mut EvalContext, args: &mut Arguments) -> TypResult<Value> {
    let iso = args.eat::<Str>();
    let dir = if let Some(dir) = args.named::<Spanned<Dir>>("dir")? {
        if dir.v.axis() == SpecAxis::Horizontal {
            Some(dir.v)
        } else {
            bail!(dir.span, "must be horizontal");
        }
    } else {
        iso.as_deref().map(lang_dir)
    };

    if let Some(dir) = dir {
        ctx.template.modify(move |state| state.dirs.cross = dir);
    }

    ctx.template.parbreak();

    Ok(Value::None)
}

/// The default direction for the language identified by the given `iso` code.
fn lang_dir(iso: &str) -> Dir {
    match iso.to_ascii_lowercase().as_str() {
        "ar" | "he" | "fa" | "ur" | "ps" | "yi" => Dir::RTL,
        "en" | "fr" | "de" => Dir::LTR,
        _ => Dir::LTR,
    }
}

/// `strike`: Set striken-through text.
pub fn strike(ctx: &mut EvalContext, args: &mut Arguments) -> TypResult<Value> {
    line_impl(ctx, args, |font| &mut font.strikethrough)
}

/// `underline`: Set underlined text.
pub fn underline(ctx: &mut EvalContext, args: &mut Arguments) -> TypResult<Value> {
    line_impl(ctx, args, |font| &mut font.underline)
}

/// `overline`: Set text with an overline.
pub fn overline(ctx: &mut EvalContext, args: &mut Arguments) -> TypResult<Value> {
    line_impl(ctx, args, |font| &mut font.overline)
}

fn line_impl(
    _: &mut EvalContext,
    args: &mut Arguments,
    substate: fn(&mut FontState) -> &mut Option<Rc<LineState>>,
) -> TypResult<Value> {
    let stroke = args.named("stroke")?.or_else(|| args.eat());
    let thickness = args.named::<Linear>("thickness")?.or_else(|| args.eat());
    let offset = args.named("offset")?;
    let extent = args.named("extent")?.unwrap_or_default();
    let body = args.expect("body")?;

    // Suppress any existing strikethrough if strength is explicitly zero.
    let line = thickness.map_or(true, |s| !s.is_zero()).then(|| {
        Rc::new(LineState {
            stroke: stroke.map(Paint::Color),
            thickness,
            offset,
            extent,
        })
    });

    let mut template = Template::new();
    template.save();
    template.modify(move |state| *substate(state.font_mut()) = line.clone());
    template += body;
    template.restore();

    Ok(Value::Template(template))
}
