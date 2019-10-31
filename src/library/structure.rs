use crate::func::prelude::*;

/// Ends the current page.
#[derive(Debug, PartialEq)]
pub struct Pagebreak;

function! {
    data: Pagebreak,
    parse: plain,

    layout(_, _) {
        Ok(commands![Command::FinishLayout])
    }
}

/// Ends the current line.
#[derive(Debug, PartialEq)]
pub struct Linebreak;

function! {
    data: Linebreak,
    parse: plain,

    layout(_, _) {
        Ok(commands![Command::FinishFlexRun])
    }
}

/// Aligns content in different ways.
#[derive(Debug, PartialEq)]
pub struct Align {
    body: Option<SyntaxTree>,
    alignment: Alignment,
}

function! {
    data: Align,

    parse(args, body, ctx) {
        let body = parse!(optional: body, ctx);
        let arg = args.get_ident()?;
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
                Command::AddMany(layout_tree(body, LayoutContext {
                    alignment: this.alignment,
                    .. ctx
                })?)
            }
            None => Command::SetAlignment(this.alignment)
        }])
    }
}

/// Layouts content into a box.
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
        if let Some(ident) = args.get_ident_if_present()? {
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
            Command::AddMany(layout_tree(&this.body, LayoutContext {
                flow: this.flow,
                .. ctx
            })?)
        ])
    }
}

macro_rules! spacefunc {
    ($ident:ident, $name:expr, $var:ident => $command:expr) => (
        /// Adds whitespace.
        #[derive(Debug, PartialEq)]
        pub struct $ident(Spacing);

        function! {
            data: $ident,

            parse(args, body, _ctx) {
                parse!(forbidden: body);

                let arg = args.get_expr()?;
                let spacing = match arg.val {
                    Expression::Size(s) => Spacing::Absolute(s),
                    Expression::Number(f) => Spacing::Relative(f as f32),
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

spacefunc!(HorizontalSpace, "h", space => Command::AddFlex(Layout::empty(space, Size::zero())));
spacefunc!(VerticalSpace, "v", space => Command::Add(Layout::empty(Size::zero(), space)));
