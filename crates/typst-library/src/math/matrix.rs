use std::ops::Range;

use super::*;

const ROW_GAP: Em = Em::new(0.5);
const COL_GAP: Em = Em::new(0.5);
const VERTICAL_PADDING: Ratio = Ratio::new(0.1);

/// A column vector.
///
/// Content in the vector's elements can be aligned with the `&` symbol.
///
/// ## Example { #example }
/// ```example
/// $ vec(a, b, c) dot vec(1, 2, 3)
///     = a + 2b + 3c $
/// ```
///
/// Display: Vector
/// Category: math
#[element(LayoutMath)]
pub struct VecElem {
    /// The delimiter to use.
    ///
    /// ```example
    /// #set math.vec(delim: "[")
    /// $ vec(1, 2) $
    /// ```
    #[default(Some(Delimiter::Paren))]
    pub delim: Option<Delimiter>,

    /// The elements of the vector.
    #[variadic]
    pub children: Vec<Content>,
}

impl LayoutMath for VecElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let delim = self.delim(ctx.styles());
        let frame = layout_vec_body(ctx, &self.children(), Align::Center)?;
        layout_delimiters(
            ctx,
            frame,
            delim.map(Delimiter::open),
            delim.map(Delimiter::close),
            self.span(),
        )
    }
}

/// A matrix.
///
/// The elements of a row should be separated by commas, while the rows
/// themselves should be separated by semicolons. The semicolon syntax merges
/// preceding arguments separated by commas into an array. You can also use this
/// special syntax of math function calls to define custom functions that take
/// 2D data.
///
/// Content in cells that are in the same row can be aligned with the `&` symbol.
///
/// ## Example { #example }
/// ```example
/// $ mat(
///   1, 2, ..., 10;
///   2, 2, ..., 10;
///   dots.v, dots.v, dots.down, dots.v;
///   10, 10, ..., 10;
/// ) $
/// ```
///
/// Display: Matrix
/// Category: math
#[element(LayoutMath)]
pub struct MatElem {
    /// The delimiter to use.
    ///
    /// ```example
    /// #set math.mat(delim: "[")
    /// $ mat(1, 2; 3, 4) $
    /// ```
    #[default(Some(Delimiter::Paren))]
    pub delim: Option<Delimiter>,

    /// Draws a horizontal line in the matrix.
    /// Defaults to `none`, resulting in no line.
    ///
    /// ```example
    /// #set math.mat(hline: 1)
    /// $ mat(1, 0, 1; 0, 1, 2) $
    /// ```
    #[default(None)]
    pub hline: Option<Offset>,

    /// Draws a vertical line in the matrix.
    /// Defaults to `none`, resulting in no line.
    ///
    /// ```example
    /// #set math.mat(vline: 2)
    /// $ mat(1, 0, 1; 0, 1, 2) $
    /// ```
    #[default(None)]
    pub vline: Option<Offset>,

    /// An array of arrays with the rows of the matrix.
    ///
    /// ```example
    /// #let data = ((1, 2, 3), (4, 5, 6))
    /// #let matrix = math.mat(..data)
    /// $ v := matrix $
    /// ```
    #[variadic]
    #[parse(
        let mut rows = vec![];
        let mut width = 0;

        let values = args.all::<Spanned<Value>>()?;
        if values.iter().any(|spanned| matches!(spanned.v, Value::Array(_))) {
            for Spanned { v, span } in values {
                let array = v.cast::<Array>().at(span)?;
                let row: Vec<_> = array.into_iter().map(Value::display).collect();
                width = width.max(row.len());
                rows.push(row);
            }
        } else {
            rows = vec![values.into_iter().map(|spanned| spanned.v.display()).collect()];
        }

        for row in &mut rows {
            if row.len() < width {
                row.resize(width, Content::empty());
            }
        }

        rows
    )]
    pub rows: Vec<Vec<Content>>,
}

impl LayoutMath for MatElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        // validate inputs

        let hline = self.hline(ctx.styles());
        let vline = self.vline(ctx.styles());

        if hline.is_some() && hline.unwrap().0 >= self.rows().len() {
            bail!(
                self.span(),
                "cannot draw a horizontal line after row {} of a matrix with {} rows",
                hline.unwrap().0,
                self.rows().len()
            );
        }

        if vline.is_some() {
            let ncols = self.rows().first().map_or(0, |row| row.len());

            if vline.unwrap().0 >= ncols {
                bail!(
                    self.span(),
                    "cannot draw a vertical line after column {} of a matrix with {} columns",
                    vline.unwrap().0,
                    ncols
                );
            }
        }

        let delim = self.delim(ctx.styles());

        let frame = layout_mat_body(ctx, &self.rows(), hline, vline, self.span())?;

        layout_delimiters(
            ctx,
            frame,
            delim.map(Delimiter::open),
            delim.map(Delimiter::close),
            self.span(),
        )
    }
}

/// A case distinction.
///
/// Content across different branches can be aligned with the `&` symbol.
///
/// ## Example { #example }
/// ```example
/// $ f(x, y) := cases(
///   1 "if" (x dot y)/2 <= 0,
///   2 "if" x "is even",
///   3 "if" x in NN,
///   4 "else",
/// ) $
/// ```
///
/// Display: Cases
/// Category: math
#[element(LayoutMath)]
pub struct CasesElem {
    /// The delimiter to use.
    ///
    /// ```example
    /// #set math.cases(delim: "[")
    /// $ x = cases(1, 2) $
    /// ```
    #[default(Delimiter::Brace)]
    pub delim: Delimiter,

    /// The branches of the case distinction.
    #[variadic]
    pub children: Vec<Content>,
}

impl LayoutMath for CasesElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let delim = self.delim(ctx.styles());
        let frame = layout_vec_body(ctx, &self.children(), Align::Left)?;
        layout_delimiters(ctx, frame, Some(delim.open()), None, self.span())
    }
}

/// A vector / matrix delimiter.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum Delimiter {
    /// Delimit with parentheses.
    #[string("(")]
    Paren,
    /// Delimit with brackets.
    #[string("[")]
    Bracket,
    /// Delimit with curly braces.
    #[string("{")]
    Brace,
    /// Delimit with vertical bars.
    #[string("|")]
    Bar,
    /// Delimit with double vertical bars.
    #[string("||")]
    DoubleBar,
}

impl Delimiter {
    /// The delimiter's opening character.
    fn open(self) -> char {
        match self {
            Self::Paren => '(',
            Self::Bracket => '[',
            Self::Brace => '{',
            Self::Bar => '|',
            Self::DoubleBar => '‖',
        }
    }

    /// The delimiter's closing character.
    fn close(self) -> char {
        match self {
            Self::Paren => ')',
            Self::Bracket => ']',
            Self::Brace => '}',
            Self::Bar => '|',
            Self::DoubleBar => '‖',
        }
    }
}

/// Layout the inner contents of a vector.
fn layout_vec_body(
    ctx: &mut MathContext,
    column: &[Content],
    align: Align,
) -> SourceResult<Frame> {
    let gap = ROW_GAP.scaled(ctx);
    ctx.style(ctx.style.for_denominator());
    let mut flat = vec![];
    for child in column {
        flat.push(ctx.layout_row(child)?);
    }
    ctx.unstyle();
    Ok(stack(ctx, flat, align, gap, 0))
}

/// Layout the inner contents of a matrix.
/// To accommodate line drawing, the matrix is split into
/// submatrices that are each laid out by `layout_submat_body`.
fn layout_mat_body(
    ctx: &mut MathContext,
    rows: &[Vec<Content>],
    hline: Option<Offset>,
    vline: Option<Offset>,
    span: Span,
) -> SourceResult<Frame> {
    // we need to split into four cases based on
    // whether hline and vline are none or some

    // for each case, we generate frames for
    // each of the submatrices split by the lines

    // then, we combine them with appropriate spacing
    // into one large frame, which we draw lines on and then return

    let row_gap = ROW_GAP.scaled(ctx);
    let col_gap = COL_GAP.scaled(ctx);

    let ncols = rows.first().map_or(0, |row| row.len());
    let nrows = rows.len();

    if nrows == 0 || ncols == 0 {
        return Ok(Frame::new(Size::zero()));
    }

    let half_row_gap = row_gap * 0.5;
    let half_col_gap = col_gap * 0.5;

    let mut cell_info = precompute_cell_info(ctx, rows)?;

    if hline.is_some() && vline.is_some() {
        // if we have both a horizontal and a vertical line

        let top_left_frame = layout_submat_body(
            ctx,
            0..hline.unwrap().0,
            0..vline.unwrap().0,
            &mut cell_info,
        )?;
        let top_right_frame = layout_submat_body(
            ctx,
            0..hline.unwrap().0,
            vline.unwrap().0..ncols,
            &mut cell_info,
        )?;
        let bottom_left_frame = layout_submat_body(
            ctx,
            hline.unwrap().0..nrows,
            0..vline.unwrap().0,
            &mut cell_info,
        )?;
        let bottom_right_frame = layout_submat_body(
            ctx,
            hline.unwrap().0..nrows,
            vline.unwrap().0..ncols,
            &mut cell_info,
        )?;

        let left_width = top_left_frame.width();
        let top_height = top_left_frame.height();

        let total_width = left_width + row_gap + top_right_frame.width();
        let total_height = top_height + col_gap + bottom_left_frame.height();

        let mut frame = Frame::new(Axes { x: total_width, y: total_height });

        frame.push_frame(Point::zero(), top_left_frame);
        frame.push(
            Point::with_x(left_width + half_row_gap),
            vline_item(total_height, span),
        );
        frame.push(
            Point::with_y(top_height + half_col_gap),
            hline_item(total_width, span),
        );
        frame.push_frame(Point::with_x(left_width + row_gap), top_right_frame);
        frame.push_frame(Point::with_y(top_height + col_gap), bottom_left_frame);
        frame.push_frame(
            Point { x: left_width + row_gap, y: top_height + col_gap },
            bottom_right_frame,
        );

        Ok(frame)
    } else if hline.is_some() {
        // if we have just a horizontal line

        let top_frame =
            layout_submat_body(ctx, 0..hline.unwrap().0, 0..ncols, &mut cell_info)?;
        let bottom_frame =
            layout_submat_body(ctx, hline.unwrap().0..nrows, 0..ncols, &mut cell_info)?;

        let top_height = top_frame.height();

        let total_width = top_frame.width();
        let total_height = top_height + col_gap + bottom_frame.height();

        let mut frame = Frame::new(Axes { x: total_width, y: total_height });

        frame.push_frame(Point::zero(), top_frame);
        frame.push(
            Point::with_y(top_height + half_col_gap),
            hline_item(total_width, span),
        );
        frame.push_frame(Point::with_y(top_height + col_gap), bottom_frame);

        Ok(frame)
    } else if vline.is_some() {
        // if we have just a vertical line

        let left_frame =
            layout_submat_body(ctx, 0..nrows, 0..vline.unwrap().0, &mut cell_info)?;
        let right_frame =
            layout_submat_body(ctx, 0..nrows, vline.unwrap().0..ncols, &mut cell_info)?;

        let left_width = left_frame.width();

        let total_width = left_width + row_gap + right_frame.width();
        let total_height = left_frame.height();

        let mut frame = Frame::new(Axes { x: total_width, y: left_frame.height() });

        frame.push_frame(Point::zero(), left_frame);
        frame.push(
            Point::with_x(left_width + half_row_gap),
            vline_item(total_height, span),
        );
        frame.push_frame(Point::with_x(left_width + row_gap), right_frame);

        Ok(frame)
    } else {
        // if we have no line

        Ok(layout_submat_body(ctx, 0..nrows, 0..ncols, &mut cell_info)?)
    }
}

fn hline_item(length: Abs, span: Span) -> FrameItem {
    let hline_geom = Geometry::Line(Point::with_x(length));

    FrameItem::Shape(
        Shape {
            geometry: hline_geom,
            fill: None,
            stroke: Some(Stroke::default()),
        },
        span,
    )
}

fn vline_item(length: Abs, span: Span) -> FrameItem {
    let vline_geom = Geometry::Line(Point::with_y(length));

    FrameItem::Shape(
        Shape {
            geometry: vline_geom,
            fill: None,
            stroke: Some(Stroke::default()),
        },
        span,
    )
}

/// Layout the body of a matrix, with no additional lines drawn.
/// `submat_rows` represents the indices of the rows to be included,
/// and `submat_cols` represents the indices of the columns to be included.
/// For example, passing `0..2` to `submat_rows` would result in the
/// first and second rows of the matrix to be included.
fn layout_submat_body(
    ctx: &mut MathContext,
    submat_rows: Range<usize>,
    submat_cols: Range<usize>,
    cell_info: &mut PrecomputedCellInfo,
) -> SourceResult<Frame> {
    let row_gap = ROW_GAP.scaled(ctx);
    let col_gap = COL_GAP.scaled(ctx);

    let nrows = submat_rows.end - submat_rows.start;

    let PrecomputedCellInfo { cols, heights, alignment_points, alignment_widths } =
        cell_info;

    let mut frame = Frame::new(Size::new(
        Abs::zero(),
        heights[submat_rows.clone()].iter().map(|&(a, b)| a + b).sum::<Abs>()
            + row_gap * (nrows - 1) as f64,
    ));

    let mut x = Abs::zero();

    for col_index in submat_cols {
        let col = &mut cols[col_index];

        let points = &alignment_points[col_index];
        let col_width = alignment_widths[col_index];

        let mut y = Abs::zero();

        for row_index in submat_rows.clone() {
            // replace with a dummy mathrow to get ownership without
            // doing any memory shifting
            let math_row = std::mem::replace(&mut col[row_index], MathRow::new(vec![]));

            let (ascent, descent) = heights[row_index];

            let cell = math_row.into_aligned_frame(ctx, points, Align::Center);
            let pos = Point::new(
                if points.is_empty() { x + (col_width - cell.width()) / 2.0 } else { x },
                y + ascent - cell.ascent(),
            );

            frame.push_frame(pos, cell);
            y += ascent + descent + row_gap;
        }
        x += col_width + col_gap;
    }

    frame.size_mut().x = x - col_gap;

    Ok(frame)
}

struct PrecomputedCellInfo {
    cols: Vec<Vec<MathRow>>,
    heights: Vec<(Abs, Abs)>,
    alignment_points: Vec<Vec<Abs>>,
    alignment_widths: Vec<Abs>,
}

/// To ensure alignment across submatrices, all cells of the matrix
/// and corresponding alignment data are pre-computed
/// before the submatrices are laid out.
fn precompute_cell_info(
    ctx: &mut MathContext,
    rows: &[Vec<Content>],
) -> SourceResult<PrecomputedCellInfo> {
    let mut alignment_points = Vec::new();
    let mut alignment_widths = Vec::new();

    let ncols = rows.first().map_or(0, |row| row.len());
    let nrows = rows.len();

    if ncols == 0 || nrows == 0 {
        return Ok(PrecomputedCellInfo {
            cols: vec![],
            heights: vec![],
            alignment_points,
            alignment_widths,
        });
    }

    let mut heights = vec![(Abs::zero(), Abs::zero()); nrows];

    ctx.style(ctx.style.for_denominator());

    let mut cols = vec![vec![]; ncols];

    for (row, (ascent, descent)) in rows.iter().zip(&mut heights) {
        for (cell, col) in row.iter().zip(&mut cols) {
            let cell = ctx.layout_row(cell)?;
            ascent.set_max(cell.ascent());
            descent.set_max(cell.descent());
            col.push(cell);
        }
    }

    ctx.unstyle();

    for col in &cols {
        let AlignmentResult { points, width } = alignments(col);

        alignment_points.push(points);
        alignment_widths.push(width);
    }

    Ok(PrecomputedCellInfo { cols, heights, alignment_points, alignment_widths })
}

/// Layout the outer wrapper around a vector's or matrices' body.
fn layout_delimiters(
    ctx: &mut MathContext,
    mut frame: Frame,
    left: Option<char>,
    right: Option<char>,
    span: Span,
) -> SourceResult<()> {
    let axis = scaled!(ctx, axis_height);
    let short_fall = DELIM_SHORT_FALL.scaled(ctx);
    let height = frame.height();
    let target = height + VERTICAL_PADDING.of(height);
    frame.set_baseline(height / 2.0 + axis);

    if let Some(left) = left {
        let mut left =
            GlyphFragment::new(ctx, left, span).stretch_vertical(ctx, target, short_fall);
        left.center_on_axis(ctx);
        ctx.push(left);
    }

    ctx.push(FrameFragment::new(ctx, frame));

    if let Some(right) = right {
        let mut right = GlyphFragment::new(ctx, right, span)
            .stretch_vertical(ctx, target, short_fall);
        right.center_on_axis(ctx);
        ctx.push(right);
    }

    Ok(())
}

/// Used for matrix line offsets.
/// Required so that an integer can be passed
/// as a parameter both in a `#set` rule and
/// directly into the function.
#[derive(Clone, Copy)]
pub struct Offset(usize);

cast! {
    Offset,
    self => self.0.into_value(),
    v: i32 => match usize::try_from(v) {
        Ok(val) => Offset(val),
        Err(_) => bail!("expected non-negative integer")
    },
    v: Content => match v.plain_text().parse::<usize>() {
        Ok(val) => Offset(val),
        Err(_) => bail!("expected non-negative integer"),
    },
}
