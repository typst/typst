use super::prelude::*;
use super::ParNode;

/// `columns`: Stack children along an axis.
pub fn columns(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let columns = args.expect("column count")?;
    let gutter = args.named("gutter")?.unwrap_or(Relative::new(0.04).into());
    let body: Node = args.expect("body")?;
    Ok(Value::block(ColumnsNode {
        columns,
        gutter,
        child: body.into_block(),
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
    ) -> Vec<Constrained<Rc<Frame>>> {
        // Separating the infinite space into infinite columns does not make
        // much sense. Note that this line assumes that no infinitely wide
        // region will follow if the first region's width is finite.
        if regions.current.x.is_infinite() {
            return self.child.layout(ctx, regions);
        }

        // Gutter width for each region. (Can be different because the relative
        // component is calculated seperately for each region.)
        let mut gutters = vec![];

        // Sizes of all columns resulting from `region.current`,
        // `region.backlog` and `regions.last`.
        let mut sizes = vec![];

        let columns = self.columns.get();

        for (current, base) in regions
            .iter()
            .take(1 + regions.backlog.len() + if regions.last.is_some() { 1 } else { 0 })
        {
            let gutter = self.gutter.resolve(base.x);
            gutters.push(gutter);
            let size = Size::new(
                (current.x - gutter * (columns - 1) as f64) / columns as f64,
                current.y,
            );
            for _ in 0 .. columns {
                sizes.push(size);
            }
        }

        let first = sizes.remove(0);
        let mut pod = Regions::one(
            first,
            Size::new(first.x, regions.base.y),
            Spec::new(true, regions.expand.y),
        );

        // Retrieve elements for the last region from the vectors.
        let last_gutter = if regions.last.is_some() {
            let gutter = gutters.pop().unwrap();
            let size = sizes.drain(sizes.len() - columns ..).next().unwrap();
            pod.last = Some(size);
            Some(gutter)
        } else {
            None
        };

        pod.backlog = sizes.into_iter();

        let mut frames = self.child.layout(ctx, &pod).into_iter();

        let dir = ctx.styles.get(ParNode::DIR);

        let mut finished = vec![];
        let total_regions = (frames.len() as f32 / columns as f32).ceil() as usize;

        for ((current, base), gutter) in regions
            .iter()
            .take(total_regions)
            .zip(gutters.into_iter().chain(last_gutter.into_iter().cycle()))
        {
            // The height should be the parent height if the node shall expand.
            // Otherwise its the maximum column height for the frame. In that
            // case, the frame is first created with zero height and then
            // resized.
            let mut height = if regions.expand.y { current.y } else { Length::zero() };
            let mut frame = Frame::new(Spec::new(regions.current.x, height));

            let mut cursor = Length::zero();

            for _ in 0 .. columns {
                let child_frame = match frames.next() {
                    Some(frame) => frame.item,
                    None => break,
                };

                let width = child_frame.size.x;

                if !regions.expand.y {
                    height.set_max(child_frame.size.y);
                }

                frame.push_frame(
                    Point::with_x(if dir.is_positive() {
                        cursor
                    } else {
                        regions.current.x - cursor - width
                    }),
                    child_frame,
                );

                cursor += width + gutter;
            }

            frame.size.y = height;

            let mut cts = Constraints::new(regions.expand);
            cts.base = base.map(Some);
            cts.exact = current.map(Some);
            finished.push(frame.constrain(cts));
        }

        finished
    }
}
