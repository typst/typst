//! The standard library for the _Typst_ language.

use crate::func::prelude::*;
use toddle::query::FontClass;

pub_use_mod!(align);
pub_use_mod!(boxed);

mod keys;
use keys::*;

/// Create a scope with all standard functions.
pub fn std() -> Scope {
    let mut std = Scope::new();

    std.add::<Align>("align");
    std.add::<Boxed>("box");
    std.add::<PageSize>("page.size");
    std.add::<PageMargins>("page.margins");

    std.add::<LineBreak>("n");
    std.add::<LineBreak>("line.break");
    std.add::<ParBreak>("par.break");
    std.add::<PageBreak>("page.break");

    std.add_with_metadata::<Spacing, Option<AxisKey>>("spacing", None);

    for (name, key) in &[("h", AxisKey::Horizontal), ("v", AxisKey::Vertical)] {
        std.add_with_metadata::<Spacing, Option<AxisKey>>(name, Some(*key));
    }

    for (name, class) in &[
        ("bold", FontClass::Bold),
        ("italic", FontClass::Italic),
        ("mono", FontClass::Monospace),
    ] {
        std.add_with_metadata::<StyleChange, FontClass>(name, class.clone());
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
        width: Option<Size>,
        height: Option<Size>,
    }

    parse(args, body) {
        parse!(forbidden: body);
        PageSize {
            width: args.get_key_opt::<Size>("width")?.map(|s| s.v),
            height: args.get_key_opt::<Size>("height")?.map(|s| s.v),
        }
    }

    layout(self, ctx) {
        let mut style = ctx.style.page;
        if let Some(width) = self.width { style.dimensions.x = width; }
        if let Some(height) = self.height { style.dimensions.y = height; }
        vec![SetPageStyle(style)]
    }
}

function! {
    /// `page.margins`: Set the margins of pages.
    #[derive(Debug, PartialEq)]
    pub struct PageMargins {
        map: ConsistentMap<PaddingKey<AxisKey>, Size>,
    }

    parse(args, body) {
        let mut map = ConsistentMap::new();
        map.add_opt_span(PaddingKey::All, args.get_pos_opt::<Size>()?)?;

        for arg in args.keys() {
            let key = PaddingKey::from_ident(&arg.v.key)?;
            let size = Size::from_expr(arg.v.value)?;

            map.add(key, size)?;
        }

        parse!(forbidden: body);
        PageMargins { map }
    }

    layout(self, ctx) {
        use PaddingKey::*;

        let axes = ctx.axes;
        let map = self.map.dedup(|key, val| {
            Ok((match key {
                All => All,
                Axis(axis) => Axis(axis.specific(axes)),
                AxisAligned(axis, alignment) => {
                    let axis = axis.specific(axes);
                    AxisAligned(axis, alignment.specific(axes, axis))
                }
            }, val))
        })?;

        let mut style = ctx.style.page;
        let padding = &mut style.margins;

        map.with(All, |&val| padding.set_all(val));
        map.with(Axis(SpecificAxisKind::Horizontal), |&val| padding.set_horizontal(val));
        map.with(Axis(SpecificAxisKind::Vertical), |&val| padding.set_vertical(val));

        for (key, &val) in map.iter() {
            if let AxisAligned(_, alignment) = key {
                match alignment {
                    AlignmentKey::Left => padding.left = val,
                    AlignmentKey::Right => padding.right = val,
                    AlignmentKey::Top => padding.top = val,
                    AlignmentKey::Bottom => padding.bottom = val,
                    _ => {},
                }
            }
        }

        vec![SetPageStyle(style)]
    }
}

function! {
    /// `spacing`, `h`, `v`: Add spacing along an axis.
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
        let axis = self.axis.generic(ctx.axes);
        let spacing = match self.spacing {
            FSize::Absolute(size) => size,
            FSize::Scaled(scale) => scale * ctx.style.text.font_size,
        };

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
