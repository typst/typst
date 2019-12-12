//! The standard library.

use toddle::query::FontClass;

use crate::func::prelude::*;
use self::keys::*;
use self::maps::*;

pub mod maps;
pub mod keys;

pub_use_mod!(align);
pub_use_mod!(boxed);
pub_use_mod!(direction);

/// Create a scope with all standard functions.
pub fn std() -> Scope {
    let mut std = Scope::new();

    std.add::<Align>("align");
    std.add::<Boxed>("box");
    std.add::<DirectionChange>("direction");
    std.add::<PageSize>("page.size");
    std.add::<PageMargins>("page.margins");

    std.add::<LineBreak>("n");
    std.add::<LineBreak>("line.break");
    std.add::<ParBreak>("par.break");
    std.add::<PageBreak>("page.break");

    std.add::<FontSize>("font.size");

    std.add_with_metadata::<Spacing>("spacing", None);

    for (name, key) in &[("h", AxisKey::Horizontal), ("v", AxisKey::Vertical)] {
        std.add_with_metadata::<Spacing>(name, Some(*key));
    }

    for (name, class) in &[
        ("bold", FontClass::Bold),
        ("italic", FontClass::Italic),
        ("mono", FontClass::Monospace),
    ] {
        std.add_with_metadata::<StyleChange>(name, class.clone());
    }

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
    layout() { vec![FinishSpace] }
}

function! {
    /// `page.size`: Set the size of pages.
    #[derive(Debug, PartialEq)]
    pub struct PageSize {
        map: ExtentMap<Size>,
    }

    parse(args, body) {
        parse!(forbidden: body);
        PageSize {
            map: ExtentMap::new(&mut args, true)?,
        }
    }

    layout(self, ctx) {
        let mut style = ctx.style.page;
        self.map.apply(ctx.axes, &mut style.dimensions, |&s| s)?;
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
            map: PaddingMap::new(&mut args, true)?,
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

    type Meta = Option<AxisKey>;

    parse(args, body, _, meta) {
        let spacing = if let Some(axis) = meta {
            Spacing {
                axis,
                spacing: FSize::from_expr(args.get_pos::<Expression>()?)?,
            }
        } else {
            if let Some(arg) = args.get_key_next() {
                let axis = AxisKey::from_ident(&arg.v.key)
                    .map_err(|_| error!(@unexpected_argument))?;

                let spacing = FSize::from_expr(arg.v.value)?;
                Spacing { axis, spacing }
            } else {
                error!("expected axis and expression")
            }
        };

        parse!(forbidden: body);
        spacing
    }

    layout(self, ctx) {
        let axis = self.axis.to_generic(ctx.axes);
        let spacing = self.spacing.concretize(ctx.style.text.font_size);
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

        match &self.body {
            Some(body) => vec![
                SetTextStyle(style),
                LayoutTree(body),
                SetTextStyle(ctx.style.text.clone()),
            ],
            None => vec![SetTextStyle(style)]
        }
    }
}

function! {
    /// `font.size`: Sets the font size.
    #[derive(Debug, PartialEq)]
    pub struct FontSize {
        body: Option<SyntaxTree>,
        size: Size,
    }

    parse(args, body, ctx) {
        FontSize {
            body: parse!(optional: body, ctx),
            size: args.get_pos::<Size>()?.v,
        }
    }

    layout(self, ctx) {
        let mut style = ctx.style.text.clone();
        style.font_size = self.size;

        match &self.body {
            Some(body) => vec![
                SetTextStyle(style),
                LayoutTree(body),
                SetTextStyle(ctx.style.text.clone()),
            ],
            None => vec![SetTextStyle(style)]
        }
    }
}
