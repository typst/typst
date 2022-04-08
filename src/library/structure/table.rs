use crate::library::layout::{GridNode, TrackSizing};
use crate::library::prelude::*;

/// A table of items.
#[derive(Debug, Hash)]
pub struct TableNode {
    /// Defines sizing for content rows and columns.
    pub tracks: Spec<Vec<TrackSizing>>,
    /// Defines sizing of gutter rows and columns between content.
    pub gutter: Spec<Vec<TrackSizing>>,
    /// The nodes to be arranged in the table.
    pub children: Vec<Content>,
}

#[node(showable)]
impl TableNode {
    /// The primary cell fill color.
    pub const PRIMARY: Option<Paint> = None;
    /// The secondary cell fill color.
    pub const SECONDARY: Option<Paint> = None;
    /// How to stroke the cells.
    #[property(resolve, fold)]
    pub const STROKE: Option<RawStroke> = Some(RawStroke::default());
    /// How much to pad the cells's content.
    pub const PADDING: Relative<RawLength> = Length::pt(5.0).into();

    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Content> {
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
            children: args.all()?,
        }))
    }

    fn set(args: &mut Args) -> TypResult<StyleMap> {
        let mut styles = StyleMap::new();
        let fill = args.named("fill")?;
        styles.set_opt(Self::PRIMARY, args.named("primary")?.or(fill));
        styles.set_opt(Self::SECONDARY, args.named("secondary")?.or(fill));
        styles.set_opt(Self::STROKE, args.named("stroke")?);
        styles.set_opt(Self::PADDING, args.named("padding")?);
        Ok(styles)
    }
}

impl Show for TableNode {
    fn show(&self, ctx: &mut Context, styles: StyleChain) -> TypResult<Content> {
        let args = self.children.iter().map(|child| Value::Content(child.clone()));
        if let Some(content) = styles.show::<Self, _>(ctx, args)? {
            return Ok(content);
        }

        let primary = styles.get(Self::PRIMARY);
        let secondary = styles.get(Self::SECONDARY);
        let stroke = styles.get(Self::STROKE).map(RawStroke::unwrap_or_default);
        let padding = styles.get(Self::PADDING);

        let cols = self.tracks.x.len().max(1);
        let children = self
            .children
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
                if let Some(fill) = [primary, secondary][(x + y) % 2] {
                    child = child.filled(fill);
                }

                child
            })
            .collect();

        Ok(Content::block(GridNode {
            tracks: self.tracks.clone(),
            gutter: self.gutter.clone(),
            children,
        }))
    }
}
