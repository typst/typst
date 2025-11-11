use typst_library::diag::{SourceResult, bail, warning};
use typst_library::foundations::{Content, Packed, Resolve, StyleChain, SymbolElem};
use typst_library::layout::{
    Abs, Axes, Em, FixedAlignment, Frame, FrameItem, Point, Ratio, Rel, Size,
};
use typst_library::math::{Augment, AugmentOffsets, CasesElem, MatElem, VecElem};
use typst_library::text::TextElem;
use typst_library::visualize::{FillRule, FixedStroke, Geometry, LineCap, Shape};
use typst_syntax::Span;
use unicode_math_class::MathClass;

use super::{
    AlignmentResult, DELIM_SHORT_FALL, FrameFragment, GlyphFragment, LeftRightAlternator,
    MathContext, alignments, style_for_denominator,
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
        elem.align.resolve(styles),
        LeftRightAlternator::Right,
        None,
        Axes::with_y(elem.gap.resolve(styles)),
        span,
        "elements",
    )?;

    let delim = elem.delim.get(styles);
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
        Axes::with_y(elem.gap.resolve(styles)),
        span,
        "branches",
    )?;

    let delim = elem.delim.get(styles);
    let (open, close) = if elem.reverse.get(styles) {
        (None, delim.close())
    } else {
        (delim.open(), None)
    };
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
    let nrows = rows.len();
    let ncols = rows.first().map_or(0, |row| row.len());

    let augment = elem.augment.resolve(styles);
    if let Some(aug) = &augment {
        for &offset in &aug.hline.0 {
            if offset > nrows as isize || offset.unsigned_abs() > nrows {
                bail!(
                    span,
                    "cannot draw a horizontal line at offset {offset} \
                     in a matrix with {nrows} rows",
                );
            }
        }

        for &offset in &aug.vline.0 {
            if offset > ncols as isize || offset.unsigned_abs() > ncols {
                bail!(
                    span,
                    "cannot draw a vertical line at offset {offset} \
                     in a matrix with {ncols} columns",
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
        elem.align.resolve(styles),
        LeftRightAlternator::Right,
        augment,
        Axes::new(elem.column_gap.resolve(styles), elem.row_gap.resolve(styles)),
        span,
        "cells",
    )?;

    let delim = elem.delim.get(styles);
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
        paint: styles.get_ref(TextElem::fill).as_decoration(),
        cap: LineCap::Square,
        ..Default::default()
    };

    let (mut hline, mut vline, stroke) = match augment {
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
    // This will never panic as a paren will never shape into nothing.
    let paren =
        GlyphFragment::new_char(ctx, styles.chain(&denom_style), '(', Span::detached())
            .unwrap();

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

            ascent.set_max(cell.ascent().max(paren.ascent()));
            descent.set_max(cell.descent().max(paren.descent()));

            col.push(cell);
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
            line_item(total_height, true, stroke.clone(), span),
        );
        x += gap.x;
    }

    for (index, col) in cols.into_iter().enumerate() {
        let AlignmentResult { points, width: rcol } = alignments(&col);

        let mut y = if hline.0.contains(&0) { gap.y } else { Abs::zero() };

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
        if vline.0.contains(&(index as isize + 1)) {
            frame.push(
                Point::with_x(x + half_gap.x),
                line_item(total_height, true, stroke.clone(), span),
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
    let axis = ctx.font().math().axis_height.resolve(styles);
    let height = frame.height();
    let target = height + VERTICAL_PADDING.of(height);
    frame.set_baseline(height / 2.0 + axis);

    if let Some(left_c) = left {
        let mut left =
            ctx.layout_into_fragment(&SymbolElem::packed(left_c).spanned(span), styles)?;
        left.stretch_vertical(ctx, target, short_fall);
        left.center_on_axis();
        left.set_class(MathClass::Opening);
        ctx.push(left);
    }

    ctx.push(FrameFragment::new(styles, frame));

    if let Some(right_c) = right {
        let mut right =
            ctx.layout_into_fragment(&SymbolElem::packed(right_c).spanned(span), styles)?;
        right.stretch_vertical(ctx, target, short_fall);
        right.center_on_axis();
        right.set_class(MathClass::Closing);
        ctx.push(right);
    }

    Ok(())
}
