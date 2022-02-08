//! Tabular container.

use super::prelude::*;
use super::{GridNode, TrackSizing};

/// A table of items.
#[derive(Debug, Hash)]
pub struct TableNode {
    /// Defines sizing for content rows and columns.
    pub tracks: Spec<Vec<TrackSizing>>,
    /// Defines sizing of gutter rows and columns between content.
    pub gutter: Spec<Vec<TrackSizing>>,
    /// The nodes to be arranged in the table.
    pub children: Vec<LayoutNode>,
}

#[class]
impl TableNode {
    /// The primary cell fill color.
    pub const PRIMARY: Option<Paint> = None;
    /// The secondary cell fill color.
    pub const SECONDARY: Option<Paint> = None;
    /// How the stroke the cells.
    pub const STROKE: Option<Paint> = Some(Color::BLACK.into());
    /// The stroke's thickness.
    pub const THICKNESS: Length = Length::pt(1.0);
    /// How much to pad the cells's content.
    pub const PADDING: Linear = Length::pt(5.0).into();

    fn construct(_: &mut EvalContext, args: &mut Args) -> TypResult<Template> {
        let columns = args.named("columns")?.unwrap_or_default();
        let rows = args.named("rows")?.unwrap_or_default();
        let base_gutter: Vec<TrackSizing> = args.named("gutter")?.unwrap_or_default();
        let column_gutter = args.named("column-gutter")?;
        let row_gutter = args.named("row-gutter")?;
        Ok(Template::show(Self {
            tracks: Spec::new(columns, rows),
            gutter: Spec::new(
                column_gutter.unwrap_or_else(|| base_gutter.clone()),
                row_gutter.unwrap_or(base_gutter),
            ),
            children: args.all().collect(),
        }))
    }

    fn set(args: &mut Args, styles: &mut StyleMap) -> TypResult<()> {
        let fill = args.named("fill")?;
        styles.set_opt(Self::PRIMARY, args.named("primary")?.or(fill));
        styles.set_opt(Self::SECONDARY, args.named("secondary")?.or(fill));
        styles.set_opt(Self::STROKE, args.named("stroke")?);
        styles.set_opt(Self::THICKNESS, args.named("thickness")?);
        styles.set_opt(Self::PADDING, args.named("padding")?);
        Ok(())
    }
}

impl Show for TableNode {
    fn show(&self, styles: StyleChain) -> Template {
        let primary = styles.get(Self::PRIMARY);
        let secondary = styles.get(Self::SECONDARY);
        let thickness = styles.get(Self::THICKNESS);
        let stroke = styles.get(Self::STROKE).map(|paint| Stroke { paint, thickness });
        let padding = styles.get(Self::PADDING);

        let cols = self.tracks.x.len().max(1);
        let children = self
            .children
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, mut child)| {
                child = child.padded(Sides::splat(padding));

                if let Some(stroke) = stroke {
                    child = child.stroked(stroke);
                }

                let x = i % cols;
                let y = i / cols;
                if let Some(fill) = [primary, secondary][(x + y) % 2] {
                    child = child.filled(fill);
                }

                child
            })
            .collect();

        Template::block(GridNode {
            tracks: self.tracks.clone(),
            gutter: self.gutter.clone(),
            children,
        })
    }
}
