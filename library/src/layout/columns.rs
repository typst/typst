use crate::prelude::*;
use crate::text::TextNode;

/// # Columns
/// Separate a region into multiple equally sized columns.
///
/// The `column` function allows to separate the interior of any container into
/// multiple columns. It will not equalize the height of the columns, instead,
/// the columns will take up the height of their container or the remaining
/// height on the page. The columns function can break across pages if
/// necessary.
///
/// ## Example
/// ```example
/// = Towards Advanced Deep Learning
///
/// #box(height: 68pt,
///  columns(2, gutter: 11pt)[
///    #set par(justify: true)
///    This research was funded by the
///    National Academy of Sciences.
///    NAoS provided support for field
///    tests and interviews with a
///    grant of up to USD 40.000 for a
///    period of 6 months.
///  ]
/// )
///
/// In recent years, deep learning has
/// increasingly been used to solve a
/// variety of problems.
/// ```
///
/// ## Parameters
/// - count: `usize` (positional, required)
///   The number of columns.
///
/// - body: `Content` (positional, required)
///   The content that should be layouted into the columns.
///
/// ## Category
/// layout
#[func]
#[capable(Layout)]
#[derive(Debug, Hash)]
pub struct ColumnsNode {
    /// How many columns there should be.
    pub count: NonZeroUsize,
    /// The child to be layouted into the columns. Most likely, this should be a
    /// flow or stack node.
    pub body: Content,
}

#[node]
impl ColumnsNode {
    /// The size of the gutter space between each column.
    #[property(resolve)]
    pub const GUTTER: Rel<Length> = Ratio::new(0.04).into();

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self {
            count: args.expect("column count")?,
            body: args.expect("body")?,
        }
        .pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "count" => Some(Value::Int(self.count.get() as i64)),
            "body" => Some(Value::Content(self.body.clone())),
            _ => None,
        }
    }
}

impl Layout for ColumnsNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        // Separating the infinite space into infinite columns does not make
        // much sense.
        if !regions.first.x.is_finite() {
            return self.body.layout(vt, styles, regions);
        }

        // Determine the width of the gutter and each column.
        let columns = self.count.get();
        let gutter = styles.get(Self::GUTTER).relative_to(regions.base.x);
        let width = (regions.first.x - gutter * (columns - 1) as f64) / columns as f64;

        let backlog: Vec<_> = std::iter::once(&regions.first.y)
            .chain(regions.backlog)
            .flat_map(|&height| std::iter::repeat(height).take(columns))
            .skip(1)
            .collect();

        // Create the pod regions.
        let pod = Regions {
            first: Size::new(width, regions.first.y),
            base: Size::new(width, regions.base.y),
            backlog: &backlog,
            last: regions.last,
            expand: Axes::new(true, regions.expand.y),
        };

        // Layout the children.
        let mut frames = self.body.layout(vt, styles, pod)?.into_iter();
        let mut finished = vec![];

        let dir = styles.get(TextNode::DIR);
        let total_regions = (frames.len() as f32 / columns as f32).ceil() as usize;

        // Stitch together the columns for each region.
        for region in regions.iter().take(total_regions) {
            // The height should be the parent height if we should expand.
            // Otherwise its the maximum column height for the frame. In that
            // case, the frame is first created with zero height and then
            // resized.
            let height = if regions.expand.y { region.y } else { Abs::zero() };
            let mut output = Frame::new(Size::new(regions.first.x, height));
            let mut cursor = Abs::zero();

            for _ in 0..columns {
                let Some(frame) = frames.next() else { break };
                if !regions.expand.y {
                    output.size_mut().y.set_max(frame.height());
                }

                let width = frame.width();
                let x = if dir.is_positive() {
                    cursor
                } else {
                    regions.first.x - cursor - width
                };

                output.push_frame(Point::with_x(x), frame);
                cursor += width + gutter;
            }

            finished.push(output);
        }

        Ok(Fragment::frames(finished))
    }
}

/// # Column Break
/// A forced column break.
///
/// The function will behave like a [page break]($func/pagebreak) when used in a
/// single column layout or the last column on a page. Otherwise, content after
/// the column break will be placed in the next column.
///
/// ## Example
/// ```example
/// #set page(columns: 2)
/// Preliminary findings from our
/// ongoing research project have
/// revealed a hitherto unknown
/// phenomenon of extraordinary
/// significance.
///
/// #colbreak()
/// Through rigorous experimentation
/// and analysis, we have discovered
/// a hitherto uncharacterized process
/// that defies our current
/// understanding of the fundamental
/// laws of nature.
/// ```
///
/// ## Parameters
/// - weak: `bool` (named)
///   If `{true}`, the column break is skipped if the current column is already
///   empty.
///
/// ## Category
/// layout
#[func]
#[capable(Behave)]
#[derive(Debug, Hash)]
pub struct ColbreakNode {
    pub weak: bool,
}

#[node]
impl ColbreakNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let weak = args.named("weak")?.unwrap_or(false);
        Ok(Self { weak }.pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "weak" => Some(Value::Bool(self.weak)),
            _ => None,
        }
    }
}

impl Behave for ColbreakNode {
    fn behaviour(&self) -> Behaviour {
        if self.weak {
            Behaviour::Weak(1)
        } else {
            Behaviour::Destructive
        }
    }
}
