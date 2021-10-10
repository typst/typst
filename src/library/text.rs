use super::*;
use crate::layout::{Decoration, LineDecoration, LineKind};

/// `font`: Configure the font.
pub fn font(ctx: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    struct FontDef(Rc<Vec<FontFamily>>);
    struct FamilyDef(Rc<Vec<String>>);

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

    let list = args.named("family")?.or_else(|| {
        let families: Vec<_> = args.all().collect();
        (!families.is_empty()).then(|| FontDef(Rc::new(families)))
    });

    let size = args.named::<Linear>("size")?.or_else(|| args.eat());
    let style = args.named("style")?;
    let weight = args.named("weight")?;
    let stretch = args.named("stretch")?;
    let fill = args.named("fill")?.or_else(|| args.eat());
    let top_edge = args.named("top-edge")?;
    let bottom_edge = args.named("bottom-edge")?;
    let serif = args.named("serif")?;
    let sans_serif = args.named("sans-serif")?;
    let monospace = args.named("monospace")?;
    let fallback = args.named("fallback")?;
    let body = args.eat::<Template>();

    let f = move |style_: &mut Style| {
        let text = style_.text_mut();

        if let Some(size) = size {
            text.size = size.resolve(text.size);
        }

        if let Some(style) = style {
            text.variant.style = style;
        }

        if let Some(weight) = weight {
            text.variant.weight = weight;
        }

        if let Some(stretch) = stretch {
            text.variant.stretch = stretch;
        }

        if let Some(top_edge) = top_edge {
            text.top_edge = top_edge;
        }

        if let Some(bottom_edge) = bottom_edge {
            text.bottom_edge = bottom_edge;
        }

        if let Some(fill) = fill {
            text.fill = Paint::Color(fill);
        }

        if let Some(FontDef(list)) = &list {
            text.families_mut().list = list.clone();
        }

        if let Some(FamilyDef(serif)) = &serif {
            text.families_mut().serif = serif.clone();
        }

        if let Some(FamilyDef(sans_serif)) = &sans_serif {
            text.families_mut().sans_serif = sans_serif.clone();
        }

        if let Some(FamilyDef(monospace)) = &monospace {
            text.families_mut().monospace = monospace.clone();
        }

        if let Some(fallback) = fallback {
            text.fallback = fallback;
        }
    };

    Ok(if let Some(body) = body {
        Value::Template(body.modified(f))
    } else {
        ctx.template.modify(f);
        Value::None
    })
}

/// `par`: Configure paragraphs.
pub fn par(ctx: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let par_spacing = args.named("spacing")?;
    let line_spacing = args.named("leading")?;

    ctx.template.modify(move |style| {
        let par = style.par_mut();

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
pub fn lang(ctx: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
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
        ctx.template.modify(move |style| style.dir = dir);
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

/// `strike`: Typeset striken-through text.
pub fn strike(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    line_impl(args, LineKind::Strikethrough)
}

/// `underline`: Typeset underlined text.
pub fn underline(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    line_impl(args, LineKind::Underline)
}

/// `overline`: Typeset text with an overline.
pub fn overline(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    line_impl(args, LineKind::Overline)
}

fn line_impl(args: &mut Args, kind: LineKind) -> TypResult<Value> {
    let stroke = args.named("stroke")?.or_else(|| args.eat());
    let thickness = args.named::<Linear>("thickness")?.or_else(|| args.eat());
    let offset = args.named("offset")?;
    let extent = args.named("extent")?.unwrap_or_default();

    let mut body: Template = args.expect("body")?;
    body.decorate(Decoration::Line(LineDecoration {
        kind,
        stroke: stroke.map(Paint::Color),
        thickness,
        offset,
        extent,
    }));

    Ok(Value::Template(body))
}

/// `link`: Typeset text as a link.
pub fn link(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let url = args.expect::<Str>("url")?;

    let mut body = args.eat().unwrap_or_else(|| {
        let mut template = Template::new();
        template.text(&url);
        template
    });

    body.decorate(Decoration::Link(url.into()));

    Ok(Value::Template(body))
}
