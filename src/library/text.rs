use crate::exec::{FontState, LineState};
use crate::font::{FontStretch, FontStyle, FontWeight};
use crate::layout::Fill;

use super::*;

/// `font`: Configure the font.
pub fn font(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let list = args.named(ctx, "family");
    let size = args.named::<Linear>(ctx, "size");
    let style = args.named(ctx, "style");
    let weight = args.named(ctx, "weight");
    let stretch = args.named(ctx, "stretch");
    let top_edge = args.named(ctx, "top-edge");
    let bottom_edge = args.named(ctx, "bottom-edge");
    let color = args.named(ctx, "color");
    let serif = args.named(ctx, "serif");
    let sans_serif = args.named(ctx, "sans-serif");
    let monospace = args.named(ctx, "monospace");
    let body = args.eat::<TemplateValue>(ctx);

    Value::template(move |ctx| {
        let snapshot = ctx.state.clone();
        let font = ctx.state.font_mut();

        if let Some(linear) = size {
            font.size = linear.resolve(font.size);
        }

        if let Some(FontDef(list)) = &list {
            font.families_mut().list = list.clone();
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

        if let Some(color) = color {
            font.fill = Fill::Color(color);
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

        if let Some(body) = &body {
            body.exec(ctx);
            ctx.state = snapshot;
        }
    })
}

#[derive(Debug)]
struct FontDef(Vec<FontFamily>);

castable! {
    FontDef: "font family or array of font families",
    Value::Str(string) => Self(vec![FontFamily::Named(string.to_lowercase())]),
    Value::Array(values) => Self(values
        .into_iter()
        .filter_map(|v| v.cast().ok())
        .collect()
    ),
    #(family: FontFamily) => Self(vec![family]),
}

#[derive(Debug)]
struct FamilyDef(Vec<String>);

castable! {
    FamilyDef: "string or array of strings",
    Value::Str(string) => Self(vec![string.to_lowercase()]),
    Value::Array(values) => Self(values
        .into_iter()
        .filter_map(|v| v.cast().ok())
        .map(|string: String| string.to_lowercase())
        .collect()
    ),
}

castable! {
    FontFamily: "font family",
    Value::Str(string) => Self::Named(string.to_lowercase())
}

castable! {
    FontStyle: "font style",
}

castable! {
    FontWeight: "font weight",
    Value::Int(number) => {
        let [min, max] = [Self::THIN, Self::BLACK];
        let message = || format!(
            "should be between {} and {}",
            min.to_number(),
            max.to_number(),
        );

        return if number < i64::from(min.to_number()) {
            CastResult::Warn(min, message())
        } else if number > i64::from(max.to_number()) {
            CastResult::Warn(max, message())
        } else {
            CastResult::Ok(Self::from_number(number as u16))
        };
    },
}

castable! {
    FontStretch: "font stretch",
    Value::Relative(relative) => {
        let [min, max] = [Self::ULTRA_CONDENSED, Self::ULTRA_EXPANDED];
        let message = || format!(
            "should be between {} and {}",
            Relative::new(min.to_ratio() as f64),
            Relative::new(max.to_ratio() as f64),
        );

        let ratio = relative.get() as f32;
        let value = Self::from_ratio(ratio);

        return if ratio < min.to_ratio() || ratio > max.to_ratio() {
            CastResult::Warn(value, message())
        } else {
            CastResult::Ok(value)
        };
    },
}

castable! {
    VerticalFontMetric: "vertical font metric",
}

/// `par`: Configure paragraphs.
pub fn par(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let spacing = args.named(ctx, "spacing");
    let leading = args.named(ctx, "leading");
    let word_spacing = args.named(ctx, "word-spacing");

    Value::template(move |ctx| {
        if let Some(spacing) = spacing {
            ctx.state.par.spacing = spacing;
        }

        if let Some(leading) = leading {
            ctx.state.par.leading = leading;
        }

        if let Some(word_spacing) = word_spacing {
            ctx.state.par.word_spacing = word_spacing;
        }

        ctx.parbreak();
    })
}

/// `lang`: Configure the language.
pub fn lang(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let iso = args.eat::<String>(ctx).map(|s| lang_dir(&s));
    let dir = match args.named::<Spanned<Dir>>(ctx, "dir") {
        Some(dir) if dir.v.axis() == SpecAxis::Horizontal => Some(dir.v),
        Some(dir) => {
            ctx.diag(error!(dir.span, "must be horizontal"));
            None
        }
        None => None,
    };

    Value::template(move |ctx| {
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

/// `strike`: Enable striken-through text.
pub fn strike(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    line_impl(ctx, args, |font| &mut font.strikethrough)
}

/// `underline`: Enable underlined text.
pub fn underline(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    line_impl(ctx, args, |font| &mut font.underline)
}

/// `overline`: Add an overline above text.
pub fn overline(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    line_impl(ctx, args, |font| &mut font.overline)
}

fn line_impl(
    ctx: &mut EvalContext,
    args: &mut FuncArgs,
    substate: fn(&mut FontState) -> &mut Option<Rc<LineState>>,
) -> Value {
    let color = args.named(ctx, "color");
    let position = args.named(ctx, "position");
    let strength = args.named::<Linear>(ctx, "strength");
    let extent = args.named(ctx, "extent").unwrap_or_default();
    let body = args.eat::<TemplateValue>(ctx);

    // Suppress any existing strikethrough if strength is explicitly zero.
    let state = strength.map_or(true, |s| !s.is_zero()).then(|| {
        Rc::new(LineState {
            strength,
            position,
            extent,
            fill: color.map(Fill::Color),
        })
    });

    Value::template(move |ctx| {
        let snapshot = ctx.state.clone();

        *substate(ctx.state.font_mut()) = state.clone();

        if let Some(body) = &body {
            body.exec(ctx);
            ctx.state = snapshot;
        }
    })
}
