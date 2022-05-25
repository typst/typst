use crate::library::layout::{BlockSpacing, GridNode, TrackSizing};
use crate::library::prelude::*;

/// A table of items.
#[derive(Debug, Hash)]
pub struct TableNode {
    /// Defines sizing for content rows and columns.
    pub tracks: Spec<Vec<TrackSizing>>,
    /// Defines sizing of gutter rows and columns between content.
    pub gutter: Spec<Vec<TrackSizing>>,
    /// The nodes to be arranged in the table.
    pub cells: Vec<Content>,
}

#[node(showable)]
impl TableNode {
    /// How to fill the cells.
    #[property(referenced)]
    pub const FILL: Celled<Option<Paint>> = Celled::Value(None);
    /// How to stroke the cells.
    #[property(resolve, fold)]
    pub const STROKE: Option<RawStroke> = Some(RawStroke::default());
    /// How much to pad the cells's content.
    pub const PADDING: Relative<RawLength> = Length::pt(5.0).into();

    /// The spacing above the table.
    #[property(resolve, shorthand(around))]
    pub const ABOVE: Option<BlockSpacing> = Some(Ratio::one().into());
    /// The spacing below the table.
    #[property(resolve, shorthand(around))]
    pub const BELOW: Option<BlockSpacing> = Some(Ratio::one().into());

    fn construct(_: &mut Machine, args: &mut Args) -> TypResult<Content> {
        let columns = args.named("columns")?.unwrap_or_default();
        let rows = args.named("rows")?.unwrap_or_default();
        let base_gutter: Vec<TrackSizing> = args.named("gutter")?.unwrap_or_default();
        let column_gutter = args.named("column-gutter")?;
        let row_gutter = args.named("row-gutter")?;
        Ok(Content::show(Self {
            tracks: Spec::new(columns, rows),
            gutter: Spec::new(
                column_gutter.unwrap_or_else(|| base_gutter.clone()),
                row_gutter.unwrap_or(base_gutter),
            ),
            cells: args.all()?,
        }))
    }
}

impl Show for TableNode {
    fn unguard(&self, sel: Selector) -> ShowNode {
        Self {
            tracks: self.tracks.clone(),
            gutter: self.gutter.clone(),
            cells: self.cells.iter().map(|cell| cell.unguard(sel)).collect(),
        }
        .pack()
    }

    fn encode(&self, _: StyleChain) -> Dict {
        dict! {
            "cells" => Value::Array(
                self.cells
                    .iter()
                    .map(|cell| Value::Content(cell.clone()))
                    .collect()
            ),
        }
    }

    fn realize(&self, ctx: &mut Context, styles: StyleChain) -> TypResult<Content> {
        let fill = styles.get(Self::FILL);
        let stroke = styles.get(Self::STROKE).map(RawStroke::unwrap_or_default);
        let padding = styles.get(Self::PADDING);

        let cols = self.tracks.x.len().max(1);
        let cells = self
            .cells
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, child)| {
                let mut child = child.pack().padded(Sides::splat(padding));

                if let Some(stroke) = stroke {
                    child = child.stroked(stroke);
                }

                let x = i % cols;
                let y = i / cols;
                if let Some(fill) = fill.resolve(ctx, x, y)? {
                    child = child.filled(fill);
                }

                Ok(child)
            })
            .collect::<TypResult<_>>()?;

        Ok(Content::block(GridNode {
            tracks: self.tracks.clone(),
            gutter: self.gutter.clone(),
            cells,
        }))
    }

    fn finalize(
        &self,
        _: &mut Context,
        styles: StyleChain,
        realized: Content,
    ) -> TypResult<Content> {
        Ok(realized.spaced(styles.get(Self::ABOVE), styles.get(Self::BELOW)))
    }
}

/// A value that can be configured per cell.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Celled<T> {
    /// A bare value, the same for all cells.
    Value(T),
    /// A closure mapping from cell coordinates to a value.
    Func(Func, Span),
}

impl<T: Cast + Clone> Celled<T> {
    /// Resolve the value based on the cell position.
    pub fn resolve(&self, ctx: &mut Context, x: usize, y: usize) -> TypResult<T> {
        Ok(match self {
            Self::Value(value) => value.clone(),
            Self::Func(func, span) => {
                let args = Args::new(*span, [Value::Int(x as i64), Value::Int(y as i64)]);
                func.call_detached(ctx, args)?.cast().at(*span)?
            }
        })
    }
}

impl<T: Cast> Cast<Spanned<Value>> for Celled<T> {
    fn is(value: &Spanned<Value>) -> bool {
        matches!(&value.v, Value::Func(_)) || T::is(&value.v)
    }

    fn cast(value: Spanned<Value>) -> StrResult<Self> {
        match value.v {
            Value::Func(v) => Ok(Self::Func(v, value.span)),
            v => T::cast(v)
                .map(Self::Value)
                .map_err(|msg| with_alternative(msg, "function")),
        }
    }
}
