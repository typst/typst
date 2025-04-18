use typst_library::diag::{bail, warning, SourceResult};
use typst_library::foundations::{Content, Packed, Resolve, StyleChain};
use typst_library::layout::{
    Abs, Axes, Em, FixedAlignment, Frame, FrameItem, Point, Ratio, Rel, Size,
};
use typst_library::math::{Augment, AugmentOffsets, CasesElem, MatElem, VecElem};
use typst_library::text::TextElem;
use typst_library::visualize::{FillRule, FixedStroke, Geometry, LineCap, Shape};
use typst_syntax::Span;

use super::{
    alignments, delimiter_alignment, style_for_denominator, AlignmentResult,
    FrameFragment, GlyphFragment, LeftRightAlternator, MathContext, DELIM_SHORT_FALL,
};

const VERTICAL_PADDING: Ratio = Ratio::new(0.1);
const DEFAULT_STROKE_THICKNESS: Em = Em::new(0.05);

/// Lays out a [`VecElem`].
#[typst_macros::time(name = "math.vec", span = elem.span())]
pub fn layout_vec(
    elem: &Packed<VecElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    let span = elem.span();

    let column: Vec<&Content> = elem.children.iter().collect();
    let frame = layout_body(
        ctx,
        styles,
        &[column],
        elem.align(styles),
        LeftRightAlternator::Right,
        None,
        Axes::with_y(elem.gap(styles)),
        span,
        "elements",
    )?;

    let delim = elem.delim(styles);
    layout_delimiters(ctx, styles, frame, delim.open(), delim.close(), span)
}

/// Lays out a [`CasesElem`].
#[typst_macros::time(name = "math.cases", span = elem.span())]
pub fn layout_cases(
    elem: &Packed<CasesElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    let span = elem.span();

    let column: Vec<&Content> = elem.children.iter().collect();
    let frame = layout_body(
        ctx,
        styles,
        &[column],
        FixedAlignment::Start,
        LeftRightAlternator::None,
        None,
        Axes::with_y(elem.gap(styles)),
        span,
        "branches",
    )?;

    let delim = elem.delim(styles);
    let (open, close) =
        if elem.reverse(styles) { (None, delim.close()) } else { (delim.open(), None) };
    layout_delimiters(ctx, styles, frame, open, close, span)
}

/// Lays out a [`MatElem`].
#[typst_macros::time(name = "math.mat", span = elem.span())]
pub fn layout_mat(
    elem: &Packed<MatElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    let span = elem.span();
    let rows = &elem.rows;
    let ncols = rows.first().map_or(0, |row| row.len());

    let augment = elem.augment(styles);
    if let Some(aug) = &augment {
        for &offset in &aug.hline.0 {
            if offset > rows.len() as isize || offset.unsigned_abs() > rows.len() + 1 {
                bail!(
                    span,
                    "cannot draw a horizontal line after row {} of a matrix with {} rows",
                    if offset < 0 { rows.len() as isize + offset } else { offset },
                    rows.len()
                );
            }
        }

        for &offset in &aug.vline.0 {
            if offset > ncols as isize || offset.unsigned_abs() > ncols + 1 {
                bail!(
                    span,
                    "cannot draw a vertical line after column {} of a matrix with {} columns",
                    if offset < 0 { ncols as isize + offset } else { offset },
                    ncols
                );
            }
        }
    }

    // Transpose rows of the matrix into columns.
    let mut row_iters: Vec<_> = rows.iter().map(|i| i.iter()).collect();
    let columns: Vec<Vec<_>> = (0..ncols)
        .map(|_| row_iters.iter_mut().map(|i| i.next().unwrap()).collect())
        .collect();

    let frame = layout_body(
        ctx,
        styles,
        &columns,
        elem.align(styles),
        LeftRightAlternator::Right,
        augment,
        Axes::new(elem.column_gap(styles), elem.row_gap(styles)),
        span,
        "cells",
    )?;

    let delim = elem.delim(styles);
    layout_delimiters(ctx, styles, frame, delim.open(), delim.close(), span)
}

/// Layout the inner contents of a matrix, vector, or cases.
#[allow(clippy::too_many_arguments)]
fn layout_body(
    ctx: &mut MathContext,
    styles: StyleChain,
    columns: &[Vec<&Content>],
    align: FixedAlignment,
    alternator: LeftRightAlternator,
    augment: Option<Augment<Abs>>,
    gap: Axes<Rel<Abs>>,
    span: Span,
    children: &str,
) -> SourceResult<Frame> {
    let nrows = columns.first().map_or(0, |col| col.len());
    let ncols = columns.len();
    if ncols == 0 || nrows == 0 {
        return Ok(Frame::soft(Size::zero()));
    }

    let gap = gap.zip_map(ctx.region.size, Rel::relative_to);
    let half_gap = gap * 0.5;

    // We provide a default stroke thickness that scales
    // with font size to ensure that augmentation lines
    // look correct by default at all matrix sizes.
    // The line cap is also set to square because it looks more "correct".
    let default_stroke_thickness = DEFAULT_STROKE_THICKNESS.resolve(styles);
    let default_stroke = FixedStroke {
        thickness: default_stroke_thickness,
        paint: TextElem::fill_in(styles).as_decoration(),
        cap: LineCap::Square,
        ..Default::default()
    };

    let (hline, vline, stroke) = match augment {
        Some(augment) => {
            // We need to get stroke here for ownership.
            let stroke = augment.stroke.unwrap_or_default().unwrap_or(default_stroke);
            (augment.hline, augment.vline, stroke)
        }
        _ => (AugmentOffsets::default(), AugmentOffsets::default(), default_stroke),
    };

    // Before the full matrix body can be laid out, the
    // individual cells must first be independently laid out
    // so we can ensure alignment across rows and columns.
    let mut cols = vec![vec![]; ncols];

    // This variable stores the maximum ascent and descent for each row.
    let mut heights = vec![(Abs::zero(), Abs::zero()); nrows];

    let denom_style = style_for_denominator(styles);
    // We pad ascent and descent with the ascent and descent of the paren
    // to ensure that normal matrices are aligned with others unless they are
    // way too big.
    let paren =
        GlyphFragment::new(ctx, styles.chain(&denom_style), '(', Span::detached());

    for (column, col) in columns.iter().zip(&mut cols) {
        for (cell, (ascent, descent)) in column.iter().zip(&mut heights) {
            let cell_span = cell.span();
            let cell = ctx.layout_into_run(cell, styles.chain(&denom_style))?;

            // We ignore linebreaks in the cells as we can't differentiate
            // alignment points for the whole body from ones for a specific
            // cell, and multiline cells don't quite make sense at the moment.
            if cell.is_multiline() {
                ctx.engine.sink.warn(warning!(
                   cell_span,
                   "linebreaks are ignored in {}", children;
                   hint: "use commas instead to separate each line"
                ));
            }

            ascent.set_max(cell.ascent().max(paren.ascent));
            descent.set_max(cell.descent().max(paren.descent));

            col.push(cell);
        }
    }

    // For each row, combine maximum ascent and descent into a row height.
    // Sum the row heights, then add the total height of the gaps between rows.
    let mut total_height =
        heights.iter().map(|&(a, b)| a + b).sum::<Abs>() + gap.y * (nrows - 1) as f64;

    if hline.0.contains(&0_isize) || hline.0.contains(&-(nrows as isize + 1)) {
        total_height += gap.y;
    }

    if hline.0.contains(&(nrows as isize)) || hline.0.contains(&-1) {
        total_height += gap.y;
    }

    // Width starts at zero because it can't be calculated until later
    let mut frame = Frame::soft(Size::new(Abs::zero(), total_height));

    let mut x = Abs::zero();

    if vline.0.contains(&(0_isize)) || vline.0.contains(&-(ncols as isize + 1)) {
        frame.push(
            Point::with_x(x + half_gap.x),
            line_item(total_height, true, stroke.clone(), span),
        );
        x += gap.x;
    }

    for (index, col) in cols.into_iter().enumerate() {
        let AlignmentResult { points, width: rcol } = alignments(&col);

        let mut y =
            if hline.0.contains(&0_isize) || hline.0.contains(&-(nrows as isize + 1)) {
                gap.y
            } else {
                Abs::zero()
            };

        for (cell, &(ascent, descent)) in col.into_iter().zip(&heights) {
            let cell = cell.into_line_frame(&points, alternator);
            let pos = Point::new(
                if points.is_empty() {
                    x + align.position(rcol - cell.width())
                } else {
                    x
                },
                y + ascent - cell.ascent(),
            );

            frame.push_frame(pos, cell);

            y += ascent + descent + gap.y;
        }

        // Advance to the end of the column
        x += rcol;

        // If a vertical line should be inserted after this column
        if vline.0.contains(&(index as isize + 1))
            || (vline.0.contains(&(index as isize - ncols as isize)))
        {
            frame.push(
                Point::with_x(x + half_gap.x),
                line_item(total_height, true, stroke.clone(), span),
            );
        }

        // Advance to the start of the next column
        x += gap.x;
    }

    let total_width =
        if !(vline.0.contains(&(ncols as isize)) || vline.0.contains(&-1_isize)) {
            x - gap.x
        } else {
            x
        };

    // This allows the horizontal lines to be laid out
    for line in hline.0 {
        let real_line =
            if line < 0 { nrows + 1 - line.unsigned_abs() } else { line as usize };
        let offset = if line == 0 || real_line == 0 {
            gap.y
        } else {
            (heights[0..real_line].iter().map(|&(a, b)| a + b).sum::<Abs>()
                + gap.y * (real_line - 1) as f64)
                + half_gap.y
        };

        frame.push(
            Point::with_y(offset),
            line_item(total_width, false, stroke.clone(), span),
        );
    }

    frame.size_mut().x = total_width;

    Ok(frame)
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

/// Layout the outer wrapper around the body of a vector or matrix.
fn layout_delimiters(
    ctx: &mut MathContext,
    styles: StyleChain,
    mut frame: Frame,
    left: Option<char>,
    right: Option<char>,
    span: Span,
) -> SourceResult<()> {
    let short_fall = DELIM_SHORT_FALL.resolve(styles);
    let axis = scaled!(ctx, styles, axis_height);
    let height = frame.height();
    let target = height + VERTICAL_PADDING.of(height);
    frame.set_baseline(height / 2.0 + axis);

    if let Some(left) = left {
        let mut left = GlyphFragment::new(ctx, styles, left, span)
            .stretch_vertical(ctx, target, short_fall);
        left.align_on_axis(ctx, delimiter_alignment(left.c));
        ctx.push(left);
    }

    ctx.push(FrameFragment::new(styles, frame));

    if let Some(right) = right {
        let mut right = GlyphFragment::new(ctx, styles, right, span)
            .stretch_vertical(ctx, target, short_fall);
        right.align_on_axis(ctx, delimiter_alignment(right.c));
        ctx.push(right);
    }

    Ok(())
}
