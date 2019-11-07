use crate::func::prelude::*;
use Command::*;

/// 📜 `page.break`: Ends the current page.
#[derive(Debug, PartialEq)]
pub struct Pagebreak;

function! {
    data: Pagebreak,
    parse: plain,

    layout(_, _) {
        Ok(commands![FinishLayout])
    }
}

/// 🔙 `line.break`, `n`: Ends the current line.
#[derive(Debug, PartialEq)]
pub struct Linebreak;

function! {
    data: Linebreak,
    parse: plain,

    layout(_, _) {
        Ok(commands![FinishFlexRun])
    }
}

/// 📐 `align`: Aligns content in different ways.
///
/// **Positional arguments:**
/// - `left`, `right` or `center` _(required)_.
#[derive(Debug, PartialEq)]
pub struct Align {
    body: Option<SyntaxTree>,
    alignment: Alignment,
}

function! {
    data: Align,

    parse(args, body, ctx) {
        let body = parse!(optional: body, ctx);
        let arg = args.get_pos::<ArgIdent>()?;
        let alignment = match arg.val {
            "left" => Alignment::Left,
            "right" => Alignment::Right,
            "center" => Alignment::Center,
            s => err!("invalid alignment specifier: {}", s),
        };
        args.done()?;

        Ok(Align {
            body,
            alignment,
        })
    }

    layout(this, ctx) {
        Ok(commands![match &this.body {
            Some(body) => {
                AddMany(layout_tree(body, LayoutContext {
                    alignment: this.alignment,
                    .. ctx
                })?)
            }
            None => SetAlignment(this.alignment)
        }])
    }
}

/// 📦 `box`: Layouts content into a box.
///
/// **Positional arguments:** None.
///
/// **Keyword arguments:**
/// - flow: either `horizontal` or `vertical` _(optional)_.
#[derive(Debug, PartialEq)]
pub struct Boxed {
    body: SyntaxTree,
    flow: Flow,
}

function! {
    data: Boxed,

    parse(args, body, ctx) {
        let body = parse!(required: body, ctx);

        let mut flow = Flow::Vertical;
        if let Some(ident) = args.get_key_opt::<ArgIdent>("flow")? {
            flow = match ident.val {
                "vertical" => Flow::Vertical,
                "horizontal" => Flow::Horizontal,
                f => err!("invalid flow specifier: {}", f),
            };
        }
        args.done()?;

        Ok(Boxed {
            body,
            flow,
        })
    }

    layout(this, ctx) {
        Ok(commands![
            AddMany(layout_tree(&this.body, LayoutContext {
                flow: this.flow,
                .. ctx
            })?)
        ])
    }
}

macro_rules! spacefunc {
    ($ident:ident, $doc:expr, $var:ident => $command:expr) => (
        #[doc = $doc]
        ///
        /// **Positional arguments:**
        /// - Spacing as a size or number, which is interpreted as a multiple
        ///   of the font size _(required)_.
        #[derive(Debug, PartialEq)]
        pub struct $ident(Spacing);

        function! {
            data: $ident,

            parse(args, body, _ctx) {
                parse!(forbidden: body);

                let arg = args.get_pos::<ArgExpr>()?;
                let spacing = match arg.val {
                    Expression::Size(s) => Spacing::Absolute(*s),
                    Expression::Num(f) => Spacing::Relative(*f as f32),
                    _ => err!("invalid spacing, expected size or number"),
                };

                Ok($ident(spacing))
            }

            layout(this, ctx) {
                let $var = match this.0 {
                    Spacing::Absolute(s) => s,
                    Spacing::Relative(f) => Size::pt(f * ctx.style.font_size),
                };

                Ok(commands![$command])
            }
        }
    );
}

/// Absolute or font-relative spacing.
#[derive(Debug, PartialEq)]
enum Spacing {
    Absolute(Size),
    Relative(f32),
}

spacefunc!(HorizontalSpace, "📖 `h`: Adds horizontal whitespace.",
    space => AddFlex(Layout::empty(space, Size::zero())));

spacefunc!(VerticalSpace, "📑 `v`: Adds vertical whitespace.",
    space => Add(Layout::empty(Size::zero(), space)));
