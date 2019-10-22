use std::marker::PhantomData;
use super::prelude::*;
use crate::size::Size;

/// Adds vertical space.
pub type VerticalSpaceFunc = SpaceFunc<SpaceVertical>;

/// Adds horizontal space.
pub type HorizontalSpaceFunc = SpaceFunc<SpaceHorizontal>;

/// Adds generic space.
#[derive(Debug, PartialEq)]
pub struct SpaceFunc<F: SpaceFlow> {
    spacing: Spacing,
    _phantom: PhantomData<F>,
}

/// Absolute or font-relative spacing.
#[derive(Debug, PartialEq)]
enum Spacing {
    Absolute(Size),
    Relative(f32),
}

impl<F: SpaceFlow> Function for SpaceFunc<F> {
    fn parse(header: &FuncHeader, body: Option<&str>, _: ParseContext) -> ParseResult<Self>
    where Self: Sized {
        if header.args.len() != 1 || !header.kwargs.is_empty() {
            return err("align: expected exactly one positional argument");
        }

        let spacing = match header.args[0] {
            Expression::Size(s) => Spacing::Absolute(s),
            Expression::Number(f) => Spacing::Relative(f as f32),
            _ => return err("space: expected size or number"),
        };

        if body.is_some() {
            return err("space: expected no body");
        }

        Ok(SpaceFunc {
            spacing,
            _phantom: PhantomData,
        })
    }

    fn layout(&self, ctx: LayoutContext) -> LayoutResult<CommandList> {
        let space = match self.spacing {
            Spacing::Absolute(s) => s,
            Spacing::Relative(f) => Size::pt(f * ctx.style.font_size),
        };

        Ok(commands![F::cmd(space)])
    }
}

pub trait SpaceFlow: std::fmt::Debug + PartialEq + 'static {
    fn cmd(space: Size) -> Command<'static>;
}

#[derive(Debug, PartialEq)]
pub struct SpaceVertical;
impl SpaceFlow for SpaceVertical {
    fn cmd(space: Size) -> Command<'static> {
        Command::Add(Layout::empty(Size::zero(), space))
    }
}

#[derive(Debug, PartialEq)]
pub struct SpaceHorizontal;
impl SpaceFlow for SpaceHorizontal {
    fn cmd(space: Size) -> Command<'static> {
        Command::AddFlex(Layout::empty(space, Size::zero()))
    }
}
