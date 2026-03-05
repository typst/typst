use typst_library::diag::SourceResult;
use typst_library::foundations::{Resolve, StyleChain};
use typst_library::layout::{Abs, Em, Frame, FrameItem, Point, Rel, Size};
use typst_library::math::ir::{MathItem, MathProperties, TableItem};
use typst_library::math::{AugmentOffsets, style_for_denominator};
use typst_library::text::TextElem;
use typst_library::visualize::{FillRule, FixedStroke, Geometry, LineCap, Shape};
use typst_syntax::Span;

use super::MathContext;
use super::fragment::{FrameFragment, GlyphFragment, MathFragment};
use super::run::{measure_sub_columns, stack_sub_column_rows};

const DEFAULT_STROKE_THICKNESS: Em = Em::new(0.05);

/// Layout a [`TableItem`].
#[typst_macros::time(name = "math table layout", span = props.span)]
pub fn layout_table(
    item: &TableItem,
    ctx: &mut MathContext,
    styles: StyleChain,
    props: &MathProperties,
) -> SourceResult<()> {
    let rows = &item.cells;
    let nrows = rows.len();
    let ncols = rows.first().map_or(0, |row| row.len());

    if ncols == 0 || nrows == 0 {
        ctx.push(FrameFragment::new(props, styles, Frame::soft(Size::zero())));
        return Ok(());
    }

    let gap = item.gap.zip_map(ctx.region.size, Rel::relative_to);
    let half_gap = gap * 0.5;

    // We provide a default stroke thickness that scales
    // with font size to ensure that augmentation lines
    // look correct by default at all matrix sizes.
    // The line cap is also set to square because it looks more "correct".
    let default_stroke_thickness = DEFAULT_STROKE_THICKNESS.resolve(styles);
    let default_stroke = FixedStroke {
        thickness: default_stroke_thickness,
        paint: styles.get_ref(TextElem::fill).as_decoration(),
        cap: LineCap::Square,
        ..Default::default()
    };

    let (mut hline, mut vline, stroke) = match &item.augment {
        Some(augment) => {
            // We need to get stroke here for ownership.
            let stroke =
                augment.stroke.clone().unwrap_or_default().unwrap_or(default_stroke);
            (augment.hline.clone(), augment.vline.clone(), stroke)
        }
        _ => (AugmentOffsets::default(), AugmentOffsets::default(), default_stroke),
    };

    // Before the full matrix body can be laid out, the
    // individual cells must first be independently laid out
    // so we can ensure alignment across rows and columns.
    let mut cols: Vec<Vec<CellLayout>> =
        (0..ncols).map(|_| Vec::with_capacity(nrows)).collect();

    // This variable stores the maximum ascent and descent for each row.
    let mut heights = vec![(Abs::zero(), Abs::zero()); nrows];

    let denom_style = style_for_denominator(styles);
    // We pad ascent and descent with the ascent and descent of the paren
    // to ensure that normal matrices are aligned with others unless they are
    // way too big.
    // This will never panic as a paren will never shape into nothing.
    let paren =
        GlyphFragment::new_char(ctx, styles.chain(&denom_style), '(', Span::detached())
            .unwrap();

    for (r, row) in rows.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            let cell = layout_cell(cell, ctx, styles)?;

            heights[r].0.set_max(cell.dims.0.max(paren.ascent()));
            heights[r].1.set_max(cell.dims.1.max(paren.descent()));

            cols[c].push(cell);
        }
    }

    for line in hline.0.iter_mut() {
        if *line < 0 {
            *line += nrows as isize;
        }
    }

    for line in vline.0.iter_mut() {
        if *line < 0 {
            *line += ncols as isize;
        }
    }

    // For each row, combine maximum ascent and descent into a row height.
    // Sum the row heights, then add the total height of the gaps between rows.
    let mut total_height =
        heights.iter().map(|&(a, b)| a + b).sum::<Abs>() + gap.y * (nrows - 1) as f64;

    if hline.0.contains(&0) {
        total_height += gap.y;
    }

    if hline.0.contains(&(nrows as isize)) {
        total_height += gap.y;
    }

    // Width starts at zero because it can't be calculated until later
    let mut frame = Frame::soft(Size::new(Abs::zero(), total_height));

    let mut x = Abs::zero();

    if vline.0.contains(&0) {
        frame.push(
            Point::with_x(x + half_gap.x),
            line_item(total_height, true, stroke.clone(), props.span),
        );
        x += gap.x;
    }

    for (index, col) in cols.into_iter().enumerate() {
        let sub_widths = compute_sub_column_widths(&col);
        let rows = col.into_iter().enumerate().map(|(row, cell_layout)| {
            (cell_layout.sub_columns, cell_layout.dims, heights[row])
        });

        let builder = stack_sub_column_rows(
            rows,
            &sub_widths,
            item.alternator,
            item.align,
            gap.y,
            if hline.0.contains(&0) { gap.y } else { Abs::zero() },
        );

        for (cell, mut pos) in builder.frames {
            pos.x += x;
            frame.push_frame(pos, cell);
        }

        // Advance to the end of the column
        x += builder.size.x;

        // If a vertical line should be inserted after this column
        if vline.0.contains(&(index as isize + 1)) {
            frame.push(
                Point::with_x(x + half_gap.x),
                line_item(total_height, true, stroke.clone(), props.span),
            );
        }

        // Advance to the start of the next column
        x += gap.x;
    }

    let total_width = if !(vline.0.contains(&(ncols as isize))) { x - gap.x } else { x };

    // This allows the horizontal lines to be laid out
    for line in hline.0 {
        let offset = if line == 0 {
            gap.y
        } else {
            (heights[0..line as usize].iter().map(|&(a, b)| a + b).sum::<Abs>()
                + gap.y * (line - 1) as f64)
                + half_gap.y
        };

        frame.push(
            Point::with_y(offset),
            line_item(total_width, false, stroke.clone(), props.span),
        );
    }

    frame.size_mut().x = total_width;

    let axis = ctx.font().math().axis_height.resolve(styles);
    let height = frame.height();
    frame.set_baseline(height / 2.0 + axis);

    ctx.push(FrameFragment::new(props, styles, frame));
    Ok(())
}

struct CellLayout {
    sub_columns: Vec<Vec<MathFragment>>,
    dims: (Abs, Abs),
}

/// Layout one cell, split at alignment points into sub-columns.
fn layout_cell(
    cell: &[MathItem],
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<CellLayout> {
    let mut sub_columns = Vec::with_capacity(cell.len());
    for item in cell {
        sub_columns.push(ctx.layout_into_fragments(item, styles)?);
    }
    let dims = measure_sub_columns(&sub_columns);
    Ok(CellLayout { sub_columns, dims })
}

/// Compute max sub-column widths across a table column.
fn compute_sub_column_widths(col: &[CellLayout]) -> Vec<Abs> {
    let len = col.iter().map(|cell| cell.sub_columns.len()).max().unwrap_or(1);
    let mut sub_widths = vec![Abs::zero(); len];
    for cell in col {
        for (i, sub_column) in cell.sub_columns.iter().enumerate() {
            let width = sub_column.iter().map(|f| f.width()).sum();
            sub_widths[i].set_max(width);
        }
    }
    sub_widths
}

fn line_item(length: Abs, vertical: bool, stroke: FixedStroke, span: Span) -> FrameItem {
    let line_geom = if vertical {
        Geometry::Line(Point::with_y(length))
    } else {
        Geometry::Line(Point::with_x(length))
    };

    FrameItem::Shape(
        Shape {
            geometry: line_geom,
            fill: None,
            fill_rule: FillRule::default(),
            stroke: Some(stroke),
        },
        span,
    )
}
