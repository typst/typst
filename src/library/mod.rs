//! The standard library.

use toddle::query::FontClass;

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

    std.add::<Align>("align");
    std.add::<Boxed>("box");
    std.add::<DirectionChange>("direction");

    std.add::<LineBreak>("n");
    std.add::<LineBreak>("line.break");
    std.add::<ParBreak>("par.break");
    std.add::<PageBreak>("page.break");

    std.add_with_metadata::<ContentSpacing>("word.spacing", ContentKind::Word);
    std.add_with_metadata::<ContentSpacing>("line.spacing", ContentKind::Line);
    std.add_with_metadata::<ContentSpacing>("par.spacing", ContentKind::Paragraph);

    std.add::<PageSize>("page.size");
    std.add::<PageMargins>("page.margins");

    std.add_with_metadata::<Spacing>("spacing", None);
    std.add_with_metadata::<Spacing>("h", Some(Horizontal));
    std.add_with_metadata::<Spacing>("v", Some(Vertical));

    std.add_with_metadata::<StyleChange>("bold", FontClass::Bold);
    std.add_with_metadata::<StyleChange>("italic", FontClass::Italic);
    std.add_with_metadata::<StyleChange>("mono", FontClass::Monospace);

    std.add::<FontSize>("font.size");

    std
}

function! {
    /// `line.break`, `n`: Ends the current line.
    #[derive(Debug, Default, PartialEq)]
    pub struct LineBreak;

    parse(default)
    layout() { vec![FinishLine] }
}

function! {
    /// `par.break`: Ends the current paragraph.
    ///
    /// self has the same effect as two subsequent newlines.
    #[derive(Debug, Default, PartialEq)]
    pub struct ParBreak;

    parse(default)
    layout() { vec![BreakParagraph] }
}

function! {
    /// `page.break`: Ends the current page.
    #[derive(Debug, Default, PartialEq)]
    pub struct PageBreak;

    parse(default)
    layout() { vec![BreakPage] }
}

function! {
    /// `word.spacing`, `line.spacing`, `par.spacing`: The spacing between
    /// words, lines or paragraphs as a multiple of the font size.
    #[derive(Debug, PartialEq)]
    pub struct ContentSpacing {
        spacing: f32,
        content: ContentKind,
    }

    type Meta = ContentKind;

    parse(args, body, _, meta) {
        parse!(forbidden: body);
        ContentSpacing {
            spacing: args.get_pos::<f64>()? as f32,
            content: meta
        }
    }

    layout(self, mut ctx) {
        let mut style = ctx.style.text.clone();
        match self.content {
            ContentKind::Word => style.word_spacing_scale = self.spacing,
            ContentKind::Line => style.line_spacing_scale = self.spacing,
            ContentKind::Paragraph => style.paragraph_spacing_scale = self.spacing,
        }
        vec![SetTextStyle(style)]
    }
}

/// The different kinds of content that can be spaced.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ContentKind {
    Word,
    Line,
    Paragraph,
}

function! {
    /// `page.size`: Set the size of pages.
    #[derive(Debug, PartialEq)]
    pub enum PageSize {
        Paper(Paper, bool),
        Custom(ExtentMap<PSize>),
    }

    parse(args, body) {
        parse!(forbidden: body);

        if let Some(name) = args.get_pos_opt::<Ident>()? {
            let landscape = args.get_key_opt::<bool>("landscape")?
                .unwrap_or(false);
            PageSize::Paper(Paper::from_name(name.as_str())?, landscape)
        } else {
            PageSize::Custom(ExtentMap::new(&mut args, true)?)
        }
    }

    layout(self, ctx) {
        let mut style = ctx.style.page;

        match self {
            PageSize::Paper(paper, landscape) => {
                style.class = paper.class;
                style.dimensions = paper.dimensions;
                if *landscape {
                    style.dimensions.swap();
                }
            }

            PageSize::Custom(map) => {
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
    pub struct PageMargins {
        map: PaddingMap,
    }

    parse(args, body) {
        parse!(forbidden: body);
        PageMargins {
            map: PaddingMap::new(&mut args)?,
        }
    }

    layout(self, ctx) {
        let mut style = ctx.style.page;
        self.map.apply(ctx.axes, &mut style.margins)?;
        vec![SetPageStyle(style)]
    }
}

function! {
    /// `spacing`, `h`, `v`: Adds spacing along an axis.
    #[derive(Debug, PartialEq)]
    pub struct Spacing {
        axis: AxisKey,
        spacing: FSize,
    }

    type Meta = Option<SpecificAxis>;

    parse(args, body, _, meta) {
        parse!(forbidden: body);

        if let Some(axis) = meta {
            Spacing {
                axis: AxisKey::Specific(axis),
                spacing: FSize::from_expr(args.get_pos::<Spanned<Expression>>()?)?,
            }
        } else if let Some(arg) = args.get_key_next() {
            let axis = AxisKey::from_ident(&arg.v.key)
                .map_err(|_| error!(@unexpected_argument))?;

            let spacing = FSize::from_expr(arg.v.value)?;
            Spacing { axis, spacing }
        } else {
            error!("expected axis and spacing")
        }
    }

    layout(self, ctx) {
        let axis = self.axis.to_generic(ctx.axes);
        let spacing = self.spacing.scaled(ctx.style.text.font_size());
        vec![AddSpacing(spacing, SpacingKind::Hard, axis)]
    }
}

function! {
    /// `bold`, `italic`, `mono`: Sets text with a different style.
    #[derive(Debug, PartialEq)]
    pub struct StyleChange {
        body: Option<SyntaxTree>,
        class: FontClass,
    }

    type Meta = FontClass;

    parse(args, body, ctx, meta) {
        StyleChange {
            body: parse!(optional: body, ctx),
            class: meta,
        }
    }

    layout(self, ctx) {
        let mut style = ctx.style.text.clone();
        style.toggle_class(self.class.clone());
        styled(&self.body, &ctx, style)
    }
}

function! {
    /// `font.size`: Sets the font size.
    #[derive(Debug, PartialEq)]
    pub struct FontSize {
        body: Option<SyntaxTree>,
        size: ScaleSize,
    }

    parse(args, body, ctx) {
        FontSize {
            body: parse!(optional: body, ctx),
            size: args.get_pos::<ScaleSize>()?,
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
