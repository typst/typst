//! The standard library.

use toddle::query::{FontWeight, FontStyle};

use crate::func::prelude::*;
use crate::style::{Paper, PaperClass};
use self::maps::{ExtentMap, PaddingMap, AxisKey};

pub mod maps;

pub_use_mod!(align);
pub_use_mod!(boxed);
pub_use_mod!(direction);


/// Create a scope with all standard functions.
pub fn std() -> Scope {
    let mut std = Scope::new();

    // Font setup
    std.add::<FontFamilyFunc>("font.family");
    std.add::<FontStyleFunc>("font.style");
    std.add::<FontWeightFunc>("font.weight");
    std.add::<FontSizeFunc>("font.size");

    // Layout
    std.add::<AlignFunc>("align");
    std.add::<DirectionFunc>("direction");
    std.add_with_metadata::<ContentSpacingFunc>("par.spacing", ContentKind::Paragraph);
    std.add_with_metadata::<ContentSpacingFunc>("word.spacing", ContentKind::Word);
    std.add_with_metadata::<ContentSpacingFunc>("line.spacing", ContentKind::Line);
    std.add::<BoxFunc>("box");

    // Spacing
    std.add::<LineBreakFunc>("n");
    std.add::<LineBreakFunc>("line.break");
    std.add::<ParBreakFunc>("par.break");
    std.add::<PageBreakFunc>("page.break");
    std.add_with_metadata::<SpacingFunc>("spacing", None);
    std.add_with_metadata::<SpacingFunc>("h", Some(Horizontal));
    std.add_with_metadata::<SpacingFunc>("v", Some(Vertical));

    // Page setup
    std.add::<PageSizeFunc>("page.size");
    std.add::<PageMarginsFunc>("page.margins");

    std
}

// -------------------------------------------------------------------------- //
// Font setup

function! {
    /// `font.family`: Set the font family.
    #[derive(Debug, PartialEq)]
    pub struct FontFamilyFunc {
        body: Option<SyntaxTree>,
        list: Vec<String>,
    }

    parse(header, body, ctx) {
        FontFamilyFunc {
            body: parse!(optional: body, ctx),
            list: {
                header.args.iter_pos().map(|arg| match arg.v {
                    Expr::Str(s) |
                    Expr::Ident(Ident(s)) => Ok(s.to_lowercase()),
                    _ => error!("expected identifier or string"),
                }).collect::<LayoutResult<Vec<_>>>()?
            }
        }
    }

    layout(self, ctx) {
        let mut style = ctx.style.text.clone();
        style.fallback.list = self.list.clone();
        styled(&self.body, &ctx, style)
    }
}

function! {
    /// `font.style`: Set the font style (normal / italic).
    #[derive(Debug, PartialEq)]
    pub struct FontStyleFunc {
        body: Option<SyntaxTree>,
        style: FontStyle,
    }

    parse(header, body, ctx) {
        FontStyleFunc {
            body: parse!(optional: body, ctx),
            style: {
                let s = header.args.get_pos::<String>()?;
                match FontStyle::from_str(&s) {
                    Some(style) => style,
                    None => error!("invalid font style: `{}`", s),
                }
            }
        }
    }

    layout(self, ctx) {
        let mut style = ctx.style.text.clone();
        style.variant.style = self.style;
        styled(&self.body, &ctx, style)
    }
}

function! {
    /// `font.weight`: Set text with a given weight.
    #[derive(Debug, PartialEq)]
    pub struct FontWeightFunc {
        body: Option<SyntaxTree>,
        weight: FontWeight,
    }

    parse(header, body, ctx) {
        FontWeightFunc {
            body: parse!(optional: body, ctx),
            weight: match header.args.get_pos::<Expr>()? {
                Expr::Number(weight) => {
                    let weight = weight.round() as i16;
                    FontWeight(
                        if weight < 100 { 100 }
                        else if weight <= 900 { weight }
                        else { 900 }
                    )
                }
                Expr::Ident(Ident(s)) => {
                    match FontWeight::from_str(&s) {
                        Some(weight) => weight,
                        None => error!("invalid font weight: `{}`", s),
                    }
                }
                _ => error!("expected identifier or number"),
            },
        }
    }

    layout(self, ctx) {
        let mut style = ctx.style.text.clone();
        style.variant.weight = self.weight;
        styled(&self.body, &ctx, style)
    }
}

function! {
    /// `font.size`: Sets the font size.
    #[derive(Debug, PartialEq)]
    pub struct FontSizeFunc {
        body: Option<SyntaxTree>,
        size: ScaleSize,
    }

    parse(header, body, ctx) {
        FontSizeFunc {
            body: parse!(optional: body, ctx),
            size: header.args.get_pos::<ScaleSize>()?,
        }
    }

    layout(self, ctx) {
        let mut style = ctx.style.text.clone();
        match self.size {
            ScaleSize::Absolute(size) => {
                style.base_font_size = size;
                style.font_scale = 1.0;
            }
            ScaleSize::Scaled(scale) => style.font_scale = scale,
        }
        styled(&self.body, &ctx, style)
    }
}

// -------------------------------------------------------------------------- //
// Layout

function! {
    /// `word.spacing`, `line.spacing`, `par.spacing`: The spacing between
    /// words, lines or paragraphs as a multiple of the font size.
    #[derive(Debug, PartialEq)]
    pub struct ContentSpacingFunc {
        body: Option<SyntaxTree>,
        content: ContentKind,
        spacing: f32,
    }

    type Meta = ContentKind;

    parse(header, body, ctx, meta) {
        ContentSpacingFunc {
            body: parse!(optional: body, ctx),
            content: meta,
            spacing: header.args.get_pos::<f64>()? as f32,
        }
    }

    layout(self, ctx) {
        let mut style = ctx.style.text.clone();
        match self.content {
            ContentKind::Word => style.word_spacing_scale = self.spacing,
            ContentKind::Line => style.line_spacing_scale = self.spacing,
            ContentKind::Paragraph => style.paragraph_spacing_scale = self.spacing,
        }
        styled(&self.body, &ctx, style)
    }
}

/// The different kinds of content that can be spaced.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ContentKind {
    Word,
    Line,
    Paragraph,
}

// -------------------------------------------------------------------------- //
// Spacing

function! {
    /// `line.break`, `n`: Ends the current line.
    #[derive(Debug, Default, PartialEq)]
    pub struct LineBreakFunc;

    parse(default)
    layout() { vec![FinishLine] }
}

function! {
    /// `par.break`: Ends the current paragraph.
    ///
    /// self has the same effect as two subsequent newlines.
    #[derive(Debug, Default, PartialEq)]
    pub struct ParBreakFunc;

    parse(default)
    layout() { vec![BreakParagraph] }
}

function! {
    /// `page.break`: Ends the current page.
    #[derive(Debug, Default, PartialEq)]
    pub struct PageBreakFunc;

    parse(default)
    layout() { vec![BreakPage] }
}

function! {
    /// `spacing`, `h`, `v`: Adds spacing along an axis.
    #[derive(Debug, PartialEq)]
    pub struct SpacingFunc {
        axis: AxisKey,
        spacing: FSize,
    }

    type Meta = Option<SpecificAxis>;

    parse(header, body, _, meta) {
        parse!(forbidden: body);

        if let Some(axis) = meta {
            SpacingFunc {
                axis: AxisKey::Specific(axis),
                spacing: FSize::from_expr(
                    header.args.get_pos::<Spanned<Expr>>()?
                )?,
            }
        } else {
            for arg in header.args.iter_keys() {
                let axis = AxisKey::from_ident(&arg.key)
                    .map_err(|_| error!(@unexpected_argument))?;

                let spacing = FSize::from_expr(arg.value)?;
                return Ok(SpacingFunc { axis, spacing });
            }

            error!("expected axis and spacing")
        }
    }

    layout(self, ctx) {
        let axis = self.axis.to_generic(ctx.axes);
        let spacing = self.spacing.scaled(ctx.style.text.font_size());
        vec![SpacingFunc(spacing, SpacingKind::Hard, axis)]
    }
}

// -------------------------------------------------------------------------- //
// Page setup

function! {
    /// `page.size`: Set the size of pages.
    #[derive(Debug, PartialEq)]
    pub enum PageSizeFunc {
        Paper(Paper, bool),
        Custom(ExtentMap<PSize>),
    }

    parse(header, body) {
        parse!(forbidden: body);

        if let Some(name) = header.args.get_pos_opt::<Ident>()? {
            let flip = header.args.get_key_opt::<bool>("flip")?.unwrap_or(false);
            let paper = Paper::from_name(name.as_str())
                .ok_or_else(|| error!(@"invalid paper name: `{}`", name))?;
            PageSizeFunc::Paper(paper, flip)
        } else {
            PageSizeFunc::Custom(ExtentMap::new(&mut header.args, true)?)
        }
    }

    layout(self, ctx) {
        let mut style = ctx.style.page;

        match self {
            PageSizeFunc::Paper(paper, flip) => {
                style.class = paper.class;
                style.dimensions = paper.dimensions;
                if *flip {
                    style.dimensions.swap();
                }
            }

            PageSizeFunc::Custom(map) => {
                style.class = PaperClass::Custom;

                let map = map.dedup(ctx.axes)?;
                let dims = &mut style.dimensions;
                map.with(Horizontal, |&psize| dims.x = psize.scaled(dims.x));
                map.with(Vertical, |&psize| dims.y = psize.scaled(dims.y));
            }
        }

        vec![SetPageStyle(style)]
    }
}

function! {
    /// `page.margins`: Sets the page margins.
    #[derive(Debug, PartialEq)]
    pub struct PageMarginsFunc {
        map: PaddingMap,
    }

    parse(header, body) {
        parse!(forbidden: body);
        PageMarginsFunc {
            map: PaddingMap::new(&mut header.args)?,
        }
    }

    layout(self, ctx) {
        let mut style = ctx.style.page;
        self.map.apply(ctx.axes, &mut style.margins)?;
        vec![SetPageStyle(style)]
    }
}

// -------------------------------------------------------------------------- //
// Helpers

/// Layout the body with the style or update the style if there is no body.
fn styled<'a>(
    body: &'a Option<SyntaxTree>,
    ctx: &LayoutContext,
    style: TextStyle
) -> Commands<'a> {
    match &body {
        Some(body) => vec![
            SetTextStyle(style),
            LayoutTree(body),
            SetTextStyle(ctx.style.text.clone()),
        ],
        None => vec![SetTextStyle(style)]
    }
}
