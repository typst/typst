use typst_library::diag::SourceResult;
use typst_library::foundations::{Resolve, StyleChain};
use typst_library::layout::{
    Abs, AlignElem, Em, FixedAlignment, Frame, InlineItem, Point, Size,
};
use typst_library::math::ir::{AlignedRow, MathItem, MultilineItem};
use typst_library::math::{EquationElem, LeftRightAlternator, MathSize};
use typst_library::model::ParElem;
use unicode_math_class::MathClass;

use super::MathContext;
use super::fragment::MathFragment;

/// Leading between rows in script and scriptscript size.
const TIGHT_LEADING: Em = Em::new(0.25);

/// A list of math fragments between alignment points and/or linebreaks.
///
/// For multiline equations this represents a distinct "cell", the list of
/// fragments at a specific row and column. For tables, this represents a
/// "sub-column", one of the parts that make up a cell in a table since
/// alignment points can be used to align fragments within a cell for an
/// individual column.
pub type MathRun = Vec<MathFragment>;

/// Layout a [`MultilineItem`] into a [`MathRunFrameBuilder`].
pub fn layout_multiline(
    item: &MultilineItem,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<MathRunFrameBuilder> {
    let nrows = item.rows.len();
    let ncols = item.rows.first().map_or(0, |r| r.len());

    if nrows == 0 || ncols == 0 {
        return Ok(MathRunFrameBuilder::default());
    }

    let mut col_widths = vec![Abs::zero(); ncols];
    let mut rows: Vec<Vec<MathRun>> = Vec::with_capacity(nrows);
    for row in item.rows.iter() {
        let cells = layout_aligned_row(row, ctx, styles)?;
        for (c, cell) in cells.iter().enumerate() {
            col_widths[c].set_max(cell.iter().map(|f| f.width()).sum());
        }
        rows.push(cells);
    }

    let leading = if styles.get(EquationElem::size) >= MathSize::Text {
        styles.resolve(ParElem::leading)
    } else {
        TIGHT_LEADING.resolve(styles)
    };

    let align = styles.resolve(AlignElem::alignment).x;
    let rows = rows.into_iter().map(|cells| {
        let height = measure_row(&cells);
        RowLayout { cells, frame_height: height, row_height: None }
    });

    Ok(stack_rows(
        rows,
        &col_widths,
        LeftRightAlternator::Right,
        align,
        leading,
        Abs::zero(),
    ))
}

/// Layout an [`AlignedRow`].
pub fn layout_aligned_row(
    row: &AlignedRow,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<Vec<MathRun>> {
    let mut cells = Vec::with_capacity(row.len());
    for (c, item) in row.iter().enumerate() {
        let mut frags = ctx.layout_into_fragments(item, styles)?;

        // For a (right-aligned, left-aligned) pair, move the lspace of the item
        // in the left-aligned column to the right-aligned column.
        if c.is_multiple_of(2)
            && let Some(next_item) = row.get(c + 1)
            && let Some(spacing) = alignment_lspace(next_item)
        {
            frags.push(MathFragment::Space(spacing));
        }

        cells.push(frags);
    }

    Ok(cells)
}

/// A single row to be laid out in multiline math.
pub struct RowLayout {
    /// The row's content, split by alignment points.
    pub cells: Vec<MathRun>,
    /// The ascent and descent that the row's final frame should have.
    pub frame_height: (Abs, Abs),
    /// The ascent and descent that should be used for this row when stacking
    /// rows together. If not specified,
    /// [`frame_height`](RowLayout::frame_height) is used instead.
    ///
    /// This is used in table layout, where within each column there are rows
    /// which need to be alignment. But when positioning rows together we need
    /// to align them based on the sizes of all columns of the table.
    pub row_height: Option<(Abs, Abs)>,
}

/// Build and stack line frames for rows with aligned columns.
pub fn stack_rows(
    rows: impl IntoIterator<Item = RowLayout>,
    widths: &[Abs],
    alternator: LeftRightAlternator,
    align: FixedAlignment,
    leading: Abs,
    start_y: Abs,
) -> MathRunFrameBuilder {
    let (points, total_width) = cumulative_alignment_points(widths);
    let has_alignment = !points.is_empty();

    let rows = rows.into_iter().map(|row| {
        let frame =
            row_into_line_frame(row.cells, &points, alternator, Some(row.frame_height));
        (frame, row.row_height.unwrap_or(row.frame_height))
    });

    let mut frames = Vec::new();
    let mut size = Size::new(Abs::zero(), start_y);

    for (i, (sub, (row_ascent, row_descent))) in rows.into_iter().enumerate() {
        if i > 0 {
            size.y += leading;
        }
        let mut pos = Point::with_y(size.y + row_ascent - sub.ascent());
        if !has_alignment {
            pos.x = align.position(total_width - sub.width());
        }
        size.x.set_max(sub.width());
        size.y += row_ascent + row_descent;
        frames.push((sub, pos));
    }

    MathRunFrameBuilder { size, frames }
}

/// Measure the ascent and descent of a row.
pub fn measure_row(cells: &[MathRun]) -> (Abs, Abs) {
    cells
        .iter()
        .flat_map(|sc| sc.iter())
        .filter(|f| !matches!(f, MathFragment::Tag(_)))
        .map(|f| (f.ascent(), f.descent()))
        .reduce(|(a1, d1), (a2, d2)| (a1.max(a2), d1.max(d2)))
        .unwrap_or_default()
}

pub trait MathFragmentsExt {
    fn into_frame(self) -> Frame;
    fn into_par_items(self) -> Vec<InlineItem>;
}

impl MathFragmentsExt for MathRun {
    fn into_frame(self) -> Frame {
        row_into_line_frame(vec![self], &[], LeftRightAlternator::Right, None)
    }

    /// Convert this run of math fragments into a vector of inline items for
    /// paragraph layout. Creates multiple fragments when relation or binary
    /// operators are present to allow for line-breaking opportunities later.
    fn into_par_items(self) -> Vec<InlineItem> {
        let mut items = vec![];

        let mut x = Abs::zero();
        let mut ascent = Abs::zero();
        let mut descent = Abs::zero();
        let mut frame = Frame::soft(Size::zero());
        let mut empty = true;

        let finalize_frame = |frame: &mut Frame, x, ascent, descent| {
            frame.set_size(Size::new(x, ascent + descent));
            frame.set_baseline(Abs::zero());
            frame.translate(Point::with_y(ascent));
        };

        let mut space_is_visible = false;

        let is_space = |f: &MathFragment| matches!(f, MathFragment::Space(_));
        let is_line_break_opportunity = |class, next_fragment| match class {
            // Don't split when two relations are in a row or when preceding a
            // closing parenthesis.
            MathClass::Binary => next_fragment != Some(MathClass::Closing),
            MathClass::Relation => {
                !matches!(next_fragment, Some(MathClass::Relation | MathClass::Closing))
            }
            _ => false,
        };

        let mut iter = self.into_iter().peekable();
        while let Some(fragment) = iter.next() {
            if space_is_visible && is_space(&fragment) {
                items.push(InlineItem::Space(fragment.width(), true));
                continue;
            }

            let class = fragment.class();
            let y = fragment.ascent();

            ascent.set_max(y);
            descent.set_max(fragment.descent());

            let pos = Point::new(x, -y);
            x += fragment.width();
            frame.push_frame(pos, fragment.into_frame());
            empty = false;

            // Split our current frame when we encounter a binary operator or
            // relation so that there is a line-breaking opportunity.
            if is_line_break_opportunity(class, iter.peek().map(|f| f.class())) {
                let mut frame_prev =
                    std::mem::replace(&mut frame, Frame::soft(Size::zero()));

                finalize_frame(&mut frame_prev, x, ascent, descent);
                items.push(InlineItem::Frame(frame_prev));
                empty = true;

                x = Abs::zero();
                ascent = Abs::zero();
                descent = Abs::zero();

                space_is_visible = true;
                if let Some(f_next) = iter.peek()
                    && !is_space(f_next)
                {
                    items.push(InlineItem::Space(Abs::zero(), true));
                }
            } else {
                space_is_visible = false;
            }
        }

        // Don't use `frame.is_empty()` because even an empty frame can
        // contribute width (if it had hidden content).
        if !empty {
            finalize_frame(&mut frame, x, ascent, descent);
            items.push(InlineItem::Frame(frame));
        }

        items
    }
}

/// How the rows from the [`MathRun`] should be aligned and merged into a [`Frame`].
#[derive(Default)]
pub struct MathRunFrameBuilder {
    /// The size of the resulting frame.
    pub size: Size,
    /// Each row's frame, and the position where the frame should
    /// be pushed into the resulting frame.
    pub frames: Vec<(Frame, Point)>,
}

impl MathRunFrameBuilder {
    /// Consumes the builder and returns a [`Frame`].
    fn build(self, mut set_baseline: bool) -> Frame {
        let mut frame = Frame::soft(self.size);
        for (sub, pos) in self.frames.into_iter() {
            if set_baseline && sub.has_baseline() {
                frame.set_baseline(sub.baseline());
            }
            frame.push_frame(pos, sub);
            set_baseline = false;
        }
        frame
    }

    /// Consumes the builder and returns a [`Frame`] with the baseline of the
    /// first item.
    pub fn build_aligned(self) -> Frame {
        self.build(true)
    }

    /// Consumes the builder and returns a [`Frame`] without a default
    /// baseline, which must be manually calculated later.
    pub fn build_unaligned(self) -> Frame {
        self.build(false)
    }
}

impl From<Frame> for MathRunFrameBuilder {
    fn from(frame: Frame) -> Self {
        Self {
            size: frame.size(),
            frames: vec![(frame, Point::zero())],
        }
    }
}

/// Build a frame from a row's cell fragments positioned at alignment points.
fn row_into_line_frame(
    cells: Vec<MathRun>,
    points: &[Abs],
    mut alternator: LeftRightAlternator,
    height: Option<(Abs, Abs)>,
) -> Frame {
    let (ascent, descent) = height.unwrap_or_else(|| measure_row(&cells));

    let mut frame = Frame::soft(Size::new(Abs::zero(), ascent + descent));
    frame.set_baseline(ascent);

    let mut prev_point = Abs::zero();
    let mut point_iter = points.iter().copied();
    let mut x_end = Abs::zero();

    for cell in cells {
        let width = cell.iter().map(|f| f.width()).sum();

        let cell_x = if let Some(point) = point_iter.next()
            && let Some(alt) = alternator.next()
        {
            let x = match alt {
                LeftRightAlternator::Right => point - width,
                _ => prev_point,
            };
            prev_point = point;
            x
        } else {
            prev_point
        };

        let mut x = cell_x;
        for frag in cell {
            let y = ascent - frag.ascent();
            let w = frag.width();
            frame.push_frame(Point::new(x, y), frag.into_frame());
            x += w;
        }
        x_end = x;
    }

    frame.size_mut().x = x_end;
    frame
}

/// Compute cumulative alignment points from column widths.
fn cumulative_alignment_points(widths: &[Abs]) -> (Vec<Abs>, Abs) {
    if widths.len() <= 1 {
        return (Vec::new(), widths.first().copied().unwrap_or_default());
    }
    let mut points = Vec::with_capacity(widths.len());
    let mut cumulative = Abs::zero();
    for &w in widths {
        cumulative += w;
        points.push(cumulative);
    }
    (points, cumulative)
}

/// Returns the resolved alignment lspace of the first non-tag item in a cell,
/// if it has `align_form_infix` set.
fn alignment_lspace(cell: &MathItem) -> Option<Abs> {
    cell.as_slice()
        .iter()
        .find(|item| !matches!(item, MathItem::Tag(_)))
        .and_then(|item| match item {
            MathItem::Component(comp) if comp.props.align_form_infix => {
                comp.props.lspace.map(|lspace| lspace.resolve(comp.styles))
            }
            _ => None,
        })
}
