use typst_library::diag::{bail, SourceResult};
use typst_library::foundations::{Content, Packed, StyleChain};
use typst_library::layout::{
    Abs, Axes, Em, FixedAlignment, Frame, FrameItem, Point, Ratio, Rel, Size,
};
use typst_library::math::{Augment, AugmentOffsets, CasesElem, MatElem, VecElem};
use typst_library::text::TextElem;
use typst_library::visualize::{FillRule, FixedStroke, Geometry, LineCap, Shape};
use typst_syntax::Span;

use super::{
    alignments, delimiter_alignment, scaled_font_size, stack, style_for_denominator,
    AlignmentResult, FrameFragment, GlyphFragment, LeftRightAlternator, MathContext,
    Scaled, DELIM_SHORT_FALL,
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
    let delim = elem.delim(styles);
    let frame = layout_vec_body(
        ctx,
        styles,
        elem.children(),
        elem.align(styles),
        elem.gap(styles).at(scaled_font_size(ctx, styles)),
        LeftRightAlternator::Right,
    )?;

    layout_delimiters(ctx, styles, frame, delim.open(), delim.close(), elem.span())
}

/// Lays out a [`MatElem`].
#[typst_macros::time(name = "math.mat", span = elem.span())]
pub fn layout_mat(
    elem: &Packed<MatElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    let augment = elem.augment(styles);
    let rows = elem.rows();

    if let Some(aug) = &augment {
        for &offset in &aug.hline.0 {
            if offset == 0 || offset.unsigned_abs() >= rows.len() {
                bail!(
                    elem.span(),
                    "cannot draw a horizontal line after row {} of a matrix with {} rows",
                    if offset < 0 { rows.len() as isize + offset } else { offset },
                    rows.len()
                );
            }
        }

        let ncols = elem.rows().first().map_or(0, |row| row.len());

        for &offset in &aug.vline.0 {
            if offset == 0 || offset.unsigned_abs() >= ncols {
                bail!(
                        elem.span(),
                        "cannot draw a vertical line after column {} of a matrix with {} columns",
                        if offset < 0 { ncols as isize + offset } else { offset },
                        ncols
                    );
            }
        }
    }

    let font_size = scaled_font_size(ctx, styles);
    let column_gap = elem.column_gap(styles).at(font_size);
    let row_gap = elem.row_gap(styles).at(font_size);
    let delim = elem.delim(styles);
    let frame = layout_mat_body(
        ctx,
        styles,
        rows,
        elem.align(styles),
        augment,
        Axes::new(column_gap, row_gap),
        elem.span(),
    )?;

    layout_delimiters(ctx, styles, frame, delim.open(), delim.close(), elem.span())
}

/// Lays out a [`CasesElem`].
#[typst_macros::time(name = "math.cases", span = elem.span())]
pub fn layout_cases(
    elem: &Packed<CasesElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    let delim = elem.delim(styles);
    let frame = layout_vec_body(
        ctx,
        styles,
        elem.children(),
        FixedAlignment::Start,
        elem.gap(styles).at(scaled_font_size(ctx, styles)),
        LeftRightAlternator::None,
    )?;

    let (open, close) =
        if elem.reverse(styles) { (None, delim.close()) } else { (delim.open(), None) };

    layout_delimiters(ctx, styles, frame, open, close, elem.span())
}

/// Layout the inner contents of a vector.
fn layout_vec_body(
    ctx: &mut MathContext,
    styles: StyleChain,
    column: &[Content],
    align: FixedAlignment,
    row_gap: Rel<Abs>,
    alternator: LeftRightAlternator,
) -> SourceResult<Frame> {
    let gap = row_gap.relative_to(ctx.region.size.y);

    let denom_style = style_for_denominator(styles);
    let mut flat = vec![];
    for child in column {
        flat.push(ctx.layout_into_run(child, styles.chain(&denom_style))?);
    }
    // We pad ascent and descent with the ascent and descent of the paren
    // to ensure that normal vectors are aligned with others unless they are
    // way too big.
    let paren =
        GlyphFragment::new(ctx, styles.chain(&denom_style), '(', Span::detached());
    Ok(stack(flat, align, gap, 0, alternator, Some((paren.ascent, paren.descent))))
}

/// Layout the inner contents of a matrix.
fn layout_mat_body(
    ctx: &mut MathContext,
    styles: StyleChain,
    rows: &[Vec<Content>],
    align: FixedAlignment,
    augment: Option<Augment<Abs>>,
    gap: Axes<Rel<Abs>>,
    span: Span,
) -> SourceResult<Frame> {
    let ncols = rows.first().map_or(0, |row| row.len());
    let nrows = rows.len();
    if ncols == 0 || nrows == 0 {
        return Ok(Frame::soft(Size::zero()));
    }

    let gap = gap.zip_map(ctx.region.size, Rel::relative_to);
    let half_gap = gap * 0.5;

    // We provide a default stroke thickness that scales
    // with font size to ensure that augmentation lines
    // look correct by default at all matrix sizes.
    // The line cap is also set to square because it looks more "correct".
    let font_size = scaled_font_size(ctx, styles);
    let default_stroke_thickness = DEFAULT_STROKE_THICKNESS.at(font_size);
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

    // This variable stores the maximum ascent and descent for each row.
    let mut heights = vec![(Abs::zero(), Abs::zero()); nrows];

    // We want to transpose our data layout to columns
    // before final layout. For efficiency, the columns
    // variable is set up here and newly generated
    // individual cells are then added to it.
    let mut cols = vec![vec![]; ncols];

    let denom_style = style_for_denominator(styles);
    // We pad ascent and descent with the ascent and descent of the paren
    // to ensure that normal matrices are aligned with others unless they are
    // way too big.
    let paren =
        GlyphFragment::new(ctx, styles.chain(&denom_style), '(', Span::detached());

    for (row, (ascent, descent)) in rows.iter().zip(&mut heights) {
        for (cell, col) in row.iter().zip(&mut cols) {
            let cell = ctx.layout_into_run(cell, styles.chain(&denom_style))?;

            ascent.set_max(cell.ascent().max(paren.ascent));
            descent.set_max(cell.descent().max(paren.descent));

            col.push(cell);
        }
    }

    // For each row, combine maximum ascent and descent into a row height.
    // Sum the row heights, then add the total height of the gaps between rows.
    let total_height =
        heights.iter().map(|&(a, b)| a + b).sum::<Abs>() + gap.y * (nrows - 1) as f64;

    // Width starts at zero because it can't be calculated until later
    let mut frame = Frame::soft(Size::new(Abs::zero(), total_height));

    let mut x = Abs::zero();

    for (index, col) in cols.into_iter().enumerate() {
        let AlignmentResult { points, width: rcol } = alignments(&col);

        let mut y = Abs::zero();

        for (cell, &(ascent, descent)) in col.into_iter().zip(&heights) {
            let cell = cell.into_line_frame(&points, LeftRightAlternator::Right);
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
            || vline.0.contains(&(1 - ((ncols - index) as isize)))
        {
            frame.push(
                Point::with_x(x + half_gap.x),
                line_item(total_height, true, stroke.clone(), span),
            );
        }

        // Advance to the start of the next column
        x += gap.x;
    }

    // Once all the columns are laid out, the total width can be calculated
    let total_width = x - gap.x;

    // This allows the horizontal lines to be laid out
    for line in hline.0 {
        let real_line =
            if line < 0 { nrows - line.unsigned_abs() } else { line as usize };
        let offset = (heights[0..real_line].iter().map(|&(a, b)| a + b).sum::<Abs>()
            + gap.y * (real_line - 1) as f64)
            + half_gap.y;

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
    let font_size = scaled_font_size(ctx, styles);
    let short_fall = DELIM_SHORT_FALL.at(font_size);
    let axis = ctx.constants.axis_height().scaled(ctx, font_size);
    let height = frame.height();
    let target = height + VERTICAL_PADDING.of(height);
    frame.set_baseline(height / 2.0 + axis);

    if let Some(left) = left {
        let mut left = GlyphFragment::new(ctx, styles, left, span)
            .stretch_vertical(ctx, target, short_fall);
        left.align_on_axis(ctx, delimiter_alignment(left.c));
        ctx.push(left);
    }

    ctx.push(FrameFragment::new(ctx, styles, frame));

    if let Some(right) = right {
        let mut right = GlyphFragment::new(ctx, styles, right, span)
            .stretch_vertical(ctx, target, short_fall);
        right.align_on_axis(ctx, delimiter_alignment(right.c));
        ctx.push(right);
    }

    Ok(())
}
