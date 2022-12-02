use crate::layout::{GridNode, TrackSizing, TrackSizings};
use crate::prelude::*;

/// A table of items.
#[derive(Debug, Hash)]
pub struct TableNode {
    /// Defines sizing for content rows and columns.
    pub tracks: Axes<Vec<TrackSizing>>,
    /// Defines sizing of gutter rows and columns between content.
    pub gutter: Axes<Vec<TrackSizing>>,
    /// The content to be arranged in the table.
    pub cells: Vec<Content>,
}

#[node(Layout)]
impl TableNode {
    /// How to fill the cells.
    #[property(referenced)]
    pub const FILL: Celled<Option<Paint>> = Celled::Value(None);
    /// How to stroke the cells.
    #[property(resolve, fold)]
    pub const STROKE: Option<PartialStroke> = Some(PartialStroke::default());
    /// How much to pad the cells's content.
    pub const PADDING: Rel<Length> = Abs::pt(5.0).into();

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let TrackSizings(columns) = args.named("columns")?.unwrap_or_default();
        let TrackSizings(rows) = args.named("rows")?.unwrap_or_default();
        let TrackSizings(base_gutter) = args.named("gutter")?.unwrap_or_default();
        let column_gutter = args.named("column-gutter")?.map(|TrackSizings(v)| v);
        let row_gutter = args.named("row-gutter")?.map(|TrackSizings(v)| v);
        Ok(Self {
            tracks: Axes::new(columns, rows),
            gutter: Axes::new(
                column_gutter.unwrap_or_else(|| base_gutter.clone()),
                row_gutter.unwrap_or(base_gutter),
            ),
            cells: args.all()?,
        }
        .pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "cells" => Some(Value::Array(
                self.cells.iter().cloned().map(Value::Content).collect(),
            )),
            _ => None,
        }
    }
}

impl Layout for TableNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: &Regions,
    ) -> SourceResult<Fragment> {
        let fill = styles.get(Self::FILL);
        let stroke = styles.get(Self::STROKE).map(PartialStroke::unwrap_or_default);
        let padding = styles.get(Self::PADDING);

        let cols = self.tracks.x.len().max(1);
        let cells = self
            .cells
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, child)| {
                let mut child = child.padded(Sides::splat(padding));

                if let Some(stroke) = stroke {
                    child = child.stroked(stroke);
                }

                let x = i % cols;
                let y = i / cols;
                if let Some(fill) = fill.resolve(vt, x, y)? {
                    child = child.filled(fill);
                }

                Ok(child)
            })
            .collect::<SourceResult<_>>()?;

        GridNode {
            tracks: self.tracks.clone(),
            gutter: self.gutter.clone(),
            cells,
        }
        .layout(vt, styles, regions)
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
    pub fn resolve(&self, vt: &Vt, x: usize, y: usize) -> SourceResult<T> {
        Ok(match self {
            Self::Value(value) => value.clone(),
            Self::Func(func, span) => {
                let args = Args::new(*span, [Value::Int(x as i64), Value::Int(y as i64)]);
                func.call_detached(vt.world(), args)?.cast().at(*span)?
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
