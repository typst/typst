use std::num::NonZeroUsize;

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{elem, Content, Packed, StyleChain};
use crate::layout::{
    Abs, Axes, Dir, Fragment, Frame, LayoutMultiple, Length, Point, Ratio, Regions, Rel,
    Size,
};
use crate::realize::{Behave, Behaviour};
use crate::text::TextElem;
use crate::util::Numeric;

/// Separates a region into multiple equally sized columns.
///
/// The `column` function allows to separate the interior of any container into
/// multiple columns. It will not equalize the height of the columns, instead,
/// the columns will take up the height of their container or the remaining
/// height on the page. The columns function can break across pages if
/// necessary.
///
/// If you need to insert columns across your whole document, you can use the
/// [`{page}` function's `columns` parameter]($page.columns) instead.
///
/// # Example
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
#[elem(LayoutMultiple)]
pub struct ColumnsElem {
    /// The number of columns.
    #[positional]
    #[default(NonZeroUsize::new(2).unwrap())]
    pub count: NonZeroUsize,

    /// The size of the gutter space between each column.
    #[resolve]
    #[default(Ratio::new(0.04).into())]
    pub gutter: Rel<Length>,

    /// The content that should be layouted into the columns.
    #[required]
    pub body: Content,
}

impl LayoutMultiple for Packed<ColumnsElem> {
    #[typst_macros::time(name = "columns", span = self.span())]
    fn layout(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let body = self.body();

        // Separating the infinite space into infinite columns does not make
        // much sense.
        if !regions.size.x.is_finite() {
            return body.layout(engine, styles, regions);
        }

        // Determine the width of the gutter and each column.
        let columns = self.count(styles).get();
        let gutter = self.gutter(styles).relative_to(regions.base().x);
        let width = (regions.size.x - gutter * (columns - 1) as f64) / columns as f64;

        let backlog: Vec<_> = std::iter::once(&regions.size.y)
            .chain(regions.backlog)
            .flat_map(|&height| std::iter::repeat(height).take(columns))
            .skip(1)
            .collect();

        // Create the pod regions.
        let pod = Regions {
            size: Size::new(width, regions.size.y),
            full: regions.full,
            backlog: &backlog,
            last: regions.last,
            expand: Axes::new(true, regions.expand.y),
            root: regions.root,
        };

        // Layout the children.
        let mut frames = body.layout(engine, styles, pod)?.into_iter();
        let mut finished = vec![];

        let dir = TextElem::dir_in(styles);
        let total_regions = (frames.len() as f32 / columns as f32).ceil() as usize;

        // Stitch together the columns for each region.
        for region in regions.iter().take(total_regions) {
            // The height should be the parent height if we should expand.
            // Otherwise its the maximum column height for the frame. In that
            // case, the frame is first created with zero height and then
            // resized.
            let height = if regions.expand.y { region.y } else { Abs::zero() };
            let mut output = Frame::hard(Size::new(regions.size.x, height));
            let mut cursor = Abs::zero();

            for _ in 0..columns {
                let Some(frame) = frames.next() else { break };
                if !regions.expand.y {
                    output.size_mut().y.set_max(frame.height());
                }

                let width = frame.width();
                let x = if dir == Dir::LTR {
                    cursor
                } else {
                    regions.size.x - cursor - width
                };

                output.push_frame(Point::with_x(x), frame);
                cursor += width + gutter;
            }

            finished.push(output);
        }

        Ok(Fragment::frames(finished))
    }
}

/// Forces a column break.
///
/// The function will behave like a [page break]($pagebreak) when used in a
/// single column layout or the last column on a page. Otherwise, content after
/// the column break will be placed in the next column.
///
/// # Example
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
#[elem(title = "Column Break", Behave)]
pub struct ColbreakElem {
    /// If `{true}`, the column break is skipped if the current column is
    /// already empty.
    #[default(false)]
    pub weak: bool,
}

impl Behave for Packed<ColbreakElem> {
    fn behaviour(&self) -> Behaviour {
        if self.weak(StyleChain::default()) {
            Behaviour::Weak(1)
        } else {
            Behaviour::Destructive
        }
    }
}
