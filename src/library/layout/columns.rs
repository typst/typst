use crate::library::prelude::*;
use crate::library::text::ParNode;

/// Separate a region into multiple equally sized columns.
#[derive(Debug, Hash)]
pub struct ColumnsNode {
    /// How many columns there should be.
    pub columns: NonZeroUsize,
    /// The child to be layouted into the columns. Most likely, this should be a
    /// flow or stack node.
    pub child: LayoutNode,
}

#[node]
impl ColumnsNode {
    /// The size of the gutter space between each column.
    #[property(resolve)]
    pub const GUTTER: Relative<RawLength> = Ratio::new(0.04).into();

    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Content> {
        Ok(Content::block(Self {
            columns: args.expect("column count")?,
            child: args.expect("body")?,
        }))
    }
}

impl Layout for ColumnsNode {
    fn layout(
        &self,
        ctx: &mut Context,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Arc<Frame>>> {
        // Separating the infinite space into infinite columns does not make
        // much sense.
        if !regions.first.x.is_finite() {
            return self.child.layout(ctx, regions, styles);
        }

        // Determine the width of the gutter and each column.
        let columns = self.columns.get();
        let gutter = styles.get(Self::GUTTER).relative_to(regions.base.x);
        let width = (regions.first.x - gutter * (columns - 1) as f64) / columns as f64;

        // Create the pod regions.
        let pod = Regions {
            first: Size::new(width, regions.first.y),
            base: Size::new(width, regions.base.y),
            backlog: std::iter::once(&regions.first.y)
                .chain(regions.backlog.as_slice())
                .flat_map(|&height| std::iter::repeat(height).take(columns))
                .skip(1)
                .collect(),
            last: regions.last,
            expand: Spec::new(true, regions.expand.y),
        };

        // Layout the children.
        let mut frames = self.child.layout(ctx, &pod, styles)?.into_iter();

        let dir = styles.get(ParNode::DIR);
        let total_regions = (frames.len() as f32 / columns as f32).ceil() as usize;
        let mut finished = vec![];

        // Stitch together the columns for each region.
        for region in regions.iter().take(total_regions) {
            // The height should be the parent height if the node shall expand.
            // Otherwise its the maximum column height for the frame. In that
            // case, the frame is first created with zero height and then
            // resized.
            let height = if regions.expand.y { region.y } else { Length::zero() };
            let mut output = Frame::new(Size::new(regions.first.x, height));
            let mut cursor = Length::zero();

            for _ in 0 .. columns {
                let frame = match frames.next() {
                    Some(frame) => frame,
                    None => break,
                };

                if !regions.expand.y {
                    output.size.y.set_max(frame.size.y);
                }

                let width = frame.size.x;
                let x = if dir.is_positive() {
                    cursor
                } else {
                    regions.first.x - cursor - width
                };

                output.push_frame(Point::with_x(x), frame);
                cursor += width + gutter;
            }

            finished.push(Arc::new(output));
        }

        Ok(finished)
    }
}

/// A column break.
pub struct ColbreakNode;

#[node]
impl ColbreakNode {
    fn construct(_: &mut Context, _: &mut Args) -> TypResult<Content> {
        Ok(Content::Colbreak)
    }
}
