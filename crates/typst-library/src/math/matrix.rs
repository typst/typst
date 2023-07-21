use typst::model::Resolve;

use super::*;

const ROW_GAP: Em = Em::new(0.5);
const COL_GAP: Em = Em::new(0.5);
const VERTICAL_PADDING: Ratio = Ratio::new(0.1);

const DEFAULT_STROKE_THICKNESS: Em = Em::new(0.05);

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

    /// Draws augmentation lines in a matrix.
    ///
    /// - `{none}`: No lines are drawn.
    /// - A single number: A vertical augmentation line is drawn
    ///   after the specified column number.
    /// - A dictionary: With a dictionary, multiple augmentation lines can be drawn
    ///   both horizontally and vertically. Additionally, the style of the lines can be set.
    ///   The dictionary can contain the following keys:
    ///   - `hline`: The offsets at which horizontal lines should be drawn. For example, an
    ///     offset of `2` would result in a horizontal line being drawn after the second
    ///     row of the matrix. Accepts either an integer for a single line, or an array
    ///     of integers for multiple lines.
    ///   - `vline`: The offsets at which vertical lines should be drawn. For example, an
    ///     offset of `2` would result in a vertical line being drawn after the second
    ///     column of the matrix. Accepts either an integer for a single line, or an array
    ///     of integers for multiple lines.
    ///   - `stroke`: How to stroke the line. See the [line's documentation]($func/line.stroke)
    ///     for more details. If set to `{auto}`, takes on a thickness of 0.05em and square line caps.
    ///
    /// ```example
    /// $ mat(1, 0, 1; 0, 1, 2; augment: #2) $
    /// ```
    ///
    /// ```example
    /// $ mat(0, 0, 0; 1, 1, 1; augment: #(hline: 1, stroke: 2pt + green)) $
    /// ```
    #[default(None)]
    #[resolve]
    pub augment: Option<Augment>,

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

        let augment = self.augment(ctx.styles());

        if let Some(aug) = &augment {
            if let Smart::Custom(hline) = &aug.hline {
                for offset in &hline.0 {
                    if *offset == 0 || *offset >= self.rows().len() {
                        bail!(
                            self.span(),
                            "cannot draw a horizontal line after row {} of a matrix with {} rows",
                            offset,
                            self.rows().len()
                        );
                    }
                }
            }

            if let Smart::Custom(vline) = &aug.vline {
                let ncols = self.rows().first().map_or(0, |row| row.len());

                for offset in &vline.0 {
                    if *offset == 0 || *offset >= ncols {
                        bail!(
                            self.span(),
                            "cannot draw a vertical line after column {} of a matrix with {} columns",
                            offset,
                            ncols
                        );
                    }
                }
            }
        }

        let delim = self.delim(ctx.styles());

        let frame = layout_mat_body(ctx, &self.rows(), augment, self.span())?;

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
fn layout_mat_body(
    ctx: &mut MathContext,
    rows: &[Vec<Content>],
    augment: Option<Augment<Abs>>,
    span: Span,
) -> SourceResult<Frame> {
    let row_gap = ROW_GAP.scaled(ctx);
    let col_gap = COL_GAP.scaled(ctx);

    let half_row_gap = row_gap * 0.5;
    let half_col_gap = col_gap * 0.5;

    // We provide a default stroke thickness that scales
    // with font size to ensure that augmentation lines
    // look correct by default at all matrix sizes.
    // The line cap is also set to square because it looks more "correct".
    let default_stroke_thickness = DEFAULT_STROKE_THICKNESS.scaled(ctx);
    let default_stroke = Stroke {
        thickness: default_stroke_thickness,
        line_cap: LineCap::Square,
        ..Default::default()
    };

    let (hline, vline, stroke) = match &augment {
        Some(v) => (
            v.hline_or_default(),
            v.vline_or_default(),
            v.stroke_or_default(default_stroke),
        ),
        _ => (vec![], vec![], default_stroke),
    };

    let ncols = rows.first().map_or(0, |row| row.len());
    let nrows = rows.len();
    if ncols == 0 || nrows == 0 {
        return Ok(Frame::new(Size::zero()));
    }

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

    ctx.style(ctx.style.for_denominator());
    for (row, (ascent, descent)) in rows.iter().zip(&mut heights) {
        for (cell, col) in row.iter().zip(&mut cols) {
            let cell = ctx.layout_row(cell)?;

            ascent.set_max(cell.ascent());
            descent.set_max(cell.descent());

            col.push(cell);
        }
    }
    ctx.unstyle();

    // For each row, combine maximum ascent and descent into a row height.
    // Sum the row heights and then add the total height of the gaps between rows.
    let total_height =
        heights.iter().map(|&(a, b)| a + b).sum::<Abs>() + row_gap * (nrows - 1) as f64;

    // Width starts at zero because it can't be calculated until later
    let mut frame = Frame::new(Size::new(Abs::zero(), total_height));

    let mut x = Abs::zero();

    for (index, col) in cols.into_iter().enumerate() {
        let AlignmentResult { points, width: rcol } = alignments(&col);

        let mut y = Abs::zero();

        for (cell, &(ascent, descent)) in col.into_iter().zip(&heights) {
            let cell = cell.into_aligned_frame(ctx, &points, Align::Center);
            let pos = Point::new(
                if points.is_empty() { x + (rcol - cell.width()) / 2.0 } else { x },
                y + ascent - cell.ascent(),
            );

            frame.push_frame(pos, cell);

            y += ascent + descent + row_gap;
        }

        // Advance to the end of the column
        x += rcol;

        // If a vertical line should be inserted after this column
        if vline.contains(&(index + 1)) {
            frame.push(
                Point::with_x(x + half_col_gap),
                vline_item(total_height, stroke.clone(), span),
            );
        }

        // Advance to the start of the next column
        x += col_gap;
    }

    // Once all the columns are laid out, the total width can be calculated
    let total_width = x - col_gap;

    // This allows the horizontal lines to be laid out
    for line in hline {
        let offset = (heights[0..line].iter().map(|&(a, b)| a + b).sum::<Abs>()
            + row_gap * (line - 1) as f64)
            + half_row_gap;

        frame.push(Point::with_y(offset), hline_item(total_width, stroke.clone(), span));
    }

    frame.size_mut().x = total_width;

    Ok(frame)
}

fn hline_item(length: Abs, stroke: Stroke, span: Span) -> FrameItem {
    let hline_geom = Geometry::Line(Point::with_x(length));

    FrameItem::Shape(
        Shape {
            geometry: hline_geom,
            fill: None,
            stroke: Some(stroke),
        },
        span,
    )
}

fn vline_item(length: Abs, stroke: Stroke, span: Span) -> FrameItem {
    let vline_geom = Geometry::Line(Point::with_y(length));

    FrameItem::Shape(
        Shape {
            geometry: vline_geom,
            fill: None,
            stroke: Some(stroke),
        },
        span,
    )
}

/// Layout the outer wrapper around the body of a vector or matrix.
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

/// Parameters specifying how augmentation lines
/// should be drawn on a matrix.
#[derive(Default, Clone, Hash)]
pub struct Augment<T = Length> {
    pub hline: Smart<Offsets>,
    pub vline: Smart<Offsets>,
    pub stroke: Smart<PartialStroke<T>>,
}

impl Augment<Abs> {
    fn hline_or_default(&self) -> Vec<usize> {
        match &self.hline {
            Smart::Custom(v) => v.0.to_vec(),
            _ => vec![],
        }
    }

    fn vline_or_default(&self) -> Vec<usize> {
        match &self.vline {
            Smart::Custom(v) => v.0.to_vec(),
            _ => vec![],
        }
    }

    fn stroke_or_default(&self, default: Stroke) -> Stroke {
        match &self.stroke {
            Smart::Custom(v) => v.clone().unwrap_or(default),
            _ => default,
        }
    }
}

impl Resolve for Augment {
    type Output = Augment<Abs>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        Augment {
            hline: self.hline,
            vline: self.vline,
            stroke: self.stroke.resolve(styles),
        }
    }
}

cast! {
    Augment,
    self => {
        let mut v = Dict::new();

        let hline = match self.hline {
            Smart::Custom(v) => v,
            _ => Offsets(vec![]),
        };

        let vline = match self.vline {
            Smart::Custom(v) => v,
            _ => Offsets(vec![]),
        };

        let stroke = match self.stroke {
            Smart::Custom(v) => v,
            _ => PartialStroke::default(),
        };

        v.insert("hline".into(), hline.into_value());
        v.insert("vline".into(), vline.into_value());
        v.insert("stroke".into(), stroke.into_value());

        v.into_value()
    },
    v: usize => Augment {
        hline: Smart::Auto,
        vline: Smart::Custom(Offsets(vec![v])),
        stroke: Smart::Auto,
    },
    mut dict: Dict => {
        let hline = dict.take("hline").ok().map(Offsets::from_value)
            .transpose()?.map(Smart::Custom).unwrap_or(Smart::Auto);

        let vline = dict.take("vline").ok().map(Offsets::from_value)
            .transpose()?.map(Smart::Custom).unwrap_or(Smart::Auto);

        let stroke = dict.take("stroke").ok().map(PartialStroke::from_value)
            .transpose()?.map(Smart::Custom).unwrap_or(Smart::Auto);

        Augment { hline, vline, stroke }
    },
}

/// The offsets at which augmentation lines
/// should be drawn on a matrix.
#[derive(Debug, Default, Clone, Hash)]
pub struct Offsets(Vec<usize>);

cast! {
    Offsets,
    self => self.0.into_value(),
    v: usize => Self(vec![v]),
    v: Array => Self(v.into_iter().map(Value::cast).collect::<StrResult<_>>()?),
}
