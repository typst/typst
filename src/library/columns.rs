//! Multi-column layouts.

use super::prelude::*;
use super::ParNode;

/// `columns`: Set content into multiple columns.
pub fn columns(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    Ok(Value::block(ColumnsNode {
        columns: args.expect("column count")?,
        gutter: args.named("gutter")?.unwrap_or(Relative::new(0.04).into()),
        child: args.expect("body")?,
    }))
}

/// `colbreak`: Start a new column.
pub fn colbreak(_: &mut EvalContext, _: &mut Args) -> TypResult<Value> {
    Ok(Value::Node(Node::Colbreak))
}

/// A node that separates a region into multiple equally sized columns.
#[derive(Debug, Hash)]
pub struct ColumnsNode {
    /// How many columns there should be.
    pub columns: NonZeroUsize,
    /// The size of the gutter space between each column.
    pub gutter: Linear,
    /// The child to be layouted into the columns. Most likely, this should be a
    /// flow or stack node.
    pub child: PackedNode,
}

impl Layout for ColumnsNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
        styles: StyleChain,
    ) -> Vec<Constrained<Rc<Frame>>> {
        let columns = self.columns.get();

        // Separating the infinite space into infinite columns does not make
        // much sense. Note that this line assumes that no infinitely wide
        // region will follow if the first region's width is finite.
        if regions.current.x.is_infinite() {
            return self.child.layout(ctx, regions, styles);
        }

        // Gutter width for each region. (Can be different because the relative
        // component is calculated seperately for each region.)
        let mut gutters = vec![];

        // Sizes of all columns resulting from `region.current`,
        // `region.backlog` and `regions.last`.
        let mut sizes = vec![];

        for (current, base) in regions
            .iter()
            .take(1 + regions.backlog.len() + regions.last.iter().len())
        {
            let gutter = self.gutter.resolve(base.x);
            let width = (current.x - gutter * (columns - 1) as f64) / columns as f64;
            let size = Size::new(width, current.y);
            gutters.push(gutter);
            sizes.extend(std::iter::repeat(size).take(columns));
        }

        let first = sizes.remove(0);
        let mut pod = Regions::one(
            first,
            Size::new(first.x, regions.base.y),
            Spec::new(true, regions.expand.y),
        );

        // Retrieve elements for the last region from the vectors.
        let last_gutter = regions.last.map(|_| {
            let gutter = gutters.pop().unwrap();
            let size = sizes.drain(sizes.len() - columns ..).next().unwrap();
            pod.last = Some(size);
            gutter
        });

        pod.backlog = sizes.into_iter();

        let mut finished = vec![];
        let mut frames = self.child.layout(ctx, &pod, styles).into_iter();

        let dir = styles.get(ParNode::DIR);
        let total_regions = (frames.len() as f32 / columns as f32).ceil() as usize;

        // Stitch together the columns for each region.
        for ((current, base), gutter) in regions
            .iter()
            .take(total_regions)
            .zip(gutters.into_iter().chain(last_gutter.into_iter().cycle()))
        {
            // The height should be the parent height if the node shall expand.
            // Otherwise its the maximum column height for the frame. In that
            // case, the frame is first created with zero height and then
            // resized.
            let height = if regions.expand.y { current.y } else { Length::zero() };
            let mut output = Frame::new(Spec::new(regions.current.x, height));
            let mut cursor = Length::zero();

            for _ in 0 .. columns {
                let frame = match frames.next() {
                    Some(frame) => frame.item,
                    None => break,
                };

                if !regions.expand.y {
                    output.size.y.set_max(frame.size.y);
                }

                let width = frame.size.x;
                let x = if dir.is_positive() {
                    cursor
                } else {
                    regions.current.x - cursor - width
                };

                output.push_frame(Point::with_x(x), frame);
                cursor += width + gutter;
            }

            let mut cts = Constraints::new(regions.expand);
            cts.base = base.map(Some);
            cts.exact = current.map(Some);
            finished.push(output.constrain(cts));
        }

        finished
    }
}
