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
        // much sense.
        if regions.current.x.is_infinite() {
            return self.child.layout(ctx, regions);
        }

        // All gutters in the document. (Can be different because the relative
        // component is calculated seperately for each region.)
        let mut gutters = vec![];
        // Sizes of all columns resulting from `region.current` and
        // `region.backlog`.
        let mut sizes = vec![];

        let columns = self.columns.get();

        for (current, base) in std::iter::once((regions.current, regions.base))
            .chain(regions.backlog.as_slice().iter().map(|&s| (s, s)))
        {
            let gutter = self.gutter.resolve(base.x);
            gutters.push(gutter);
            let size = Spec::new(
                (current.x - gutter * (columns - 1) as f64) / columns as f64,
                current.y,
            );
            for _ in 0 .. columns {
                sizes.push(size);
            }
        }

        let first = sizes.remove(0);
        let mut pod =
            Regions::one(first, Spec::new(first.x, regions.base.y), regions.expand);
        pod.backlog = sizes.clone().into_iter();

        // We have to treat the last region separately.
        let last_column_gutter = regions.last.map(|last| {
            let gutter = self.gutter.resolve(last.x);
            let size = Spec::new(
                (last.x - gutter * (columns - 1) as f64) / columns as f64,
                last.y,
            );
            pod.last = Some(size);
            (size, gutter)
        });

        let frames = self.child.layout(ctx, &pod);
        let dir = ctx.styles.get(ParNode::DIR);

        // Dealing with infinite height areas here.
        let height = if regions.current.y.is_infinite() {
            frames
                .iter()
                .map(|frame| frame.item.size.y)
                .max()
                .unwrap_or(Length::zero())
        } else {
            regions.current.y
        };

        let mut regions = regions.clone();

        let to = |cursor: Length, width: Length, regions: &Regions| {
            if dir.is_positive() {
                cursor
            } else {
                regions.current.x - cursor - width
            }
        };
        let mut cursor = Length::zero();

        let mut res = vec![];
        let mut frame = Frame::new(Spec::new(regions.current.x, height));

        for (i, child_frame) in frames.into_iter().enumerate() {
            let region = i / columns;
            let size = std::iter::once(&first)
                .chain(sizes.iter())
                .nth(i)
                .copied()
                .unwrap_or_else(|| last_column_gutter.unwrap().0);

            frame.push_frame(
                Point::new(to(cursor, size.x, &regions), Length::zero()),
                child_frame.item,
            );

            cursor += size.x;

            if i % columns == columns - 1 {
                // Refresh column height for non-infinite regions here.
                let height = if regions.current.y.is_infinite() {
                    height
                } else {
                    regions.current.y
                };

                regions.next();
                let old_frame = std::mem::replace(
                    &mut frame,
                    Frame::new(Spec::new(regions.current.x, height)),
                );
                res.push(old_frame.constrain(Constraints::tight(&regions)));
                cursor = Length::zero();
            } else {
                cursor += gutters
                    .get(region)
                    .copied()
                    .unwrap_or_else(|| last_column_gutter.unwrap().1);
            }
        }

        if !frame.elements.is_empty() {
            res.push(frame.constrain(Constraints::tight(&regions)));
        }

        res
    }
}
