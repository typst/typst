use smallvec::{smallvec, SmallVec};

use crate::diag::{bail, At, SourceResult, StrResult};
use crate::foundations::{
    cast, dict, elem, Array, Cast, Content, Dict, Fold, Packed, Resolve, Smart,
    StyleChain, Value,
};
use crate::layout::{
    Abs, Axes, Em, FixedAlignment, Frame, FrameItem, Length, Point, Ratio, Rel, Size,
};
use crate::math::{
    alignments, scaled_font_size, stack, style_for_denominator, AlignmentResult,
    FrameFragment, GlyphFragment, LayoutMath, MathContext, Scaled, DELIM_SHORT_FALL,
};
use crate::syntax::{Span, Spanned};
use crate::text::TextElem;
use crate::util::Numeric;
use crate::visualize::{FixedStroke, Geometry, LineCap, Shape, Stroke};

const DEFAULT_ROW_GAP: Em = Em::new(0.5);
const DEFAULT_COL_GAP: Em = Em::new(0.5);
const VERTICAL_PADDING: Ratio = Ratio::new(0.1);
const DEFAULT_STROKE_THICKNESS: Em = Em::new(0.05);

/// A column vector.
///
/// Content in the vector's elements can be aligned with the `&` symbol.
///
/// # Example
/// ```example
/// $ vec(a, b, c) dot vec(1, 2, 3)
///     = a + 2b + 3c $
/// ```
#[elem(title = "Vector", LayoutMath)]
pub struct VecElem {
    /// The delimiter to use.
    ///
    /// ```example
    /// #set math.vec(delim: "[")
    /// $ vec(1, 2) $
    /// ```
    #[default(Some(Delimiter::Paren))]
    pub delim: Option<Delimiter>,

    /// The gap between elements.
    ///
    /// ```example
    /// #set math.vec(gap: 1em)
    /// $ vec(1, 2) $
    /// ```
    #[resolve]
    #[default(DEFAULT_ROW_GAP.into())]
    pub gap: Rel<Length>,

    /// The elements of the vector.
    #[variadic]
    pub children: Vec<Content>,
}

impl LayoutMath for Packed<VecElem> {
    #[typst_macros::time(name = "math.vec", span = self.span())]
    fn layout_math(&self, ctx: &mut MathContext, styles: StyleChain) -> SourceResult<()> {
        let delim = self.delim(styles);
        let frame = layout_vec_body(
            ctx,
            styles,
            self.children(),
            FixedAlignment::Center,
            self.gap(styles),
        )?;

        layout_delimiters(
            ctx,
            styles,
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
/// # Example
/// ```example
/// $ mat(
///   1, 2, ..., 10;
///   2, 2, ..., 10;
///   dots.v, dots.v, dots.down, dots.v;
///   10, 10, ..., 10;
/// ) $
/// ```
#[elem(title = "Matrix", LayoutMath)]
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
    ///   after the specified column number. Negative numbers start from the end.
    /// - A dictionary: With a dictionary, multiple augmentation lines can be
    ///   drawn both horizontally and vertically. Additionally, the style of the
    ///   lines can be set. The dictionary can contain the following keys:
    ///   - `hline`: The offsets at which horizontal lines should be drawn.
    ///     For example, an offset of `2` would result in a horizontal line
    ///     being drawn after the second row of the matrix. Accepts either an
    ///     integer for a single line, or an array of integers
    ///     for multiple lines. Like for a single number, negative numbers start from the end.
    ///   - `vline`: The offsets at which vertical lines should be drawn.
    ///     For example, an offset of `2` would result in a vertical line being
    ///     drawn after the second column of the matrix. Accepts either an
    ///     integer for a single line, or an array of integers
    ///     for multiple lines. Like for a single number, negative numbers start from the end.
    ///   - `stroke`: How to [stroke]($stroke) the line. If set to `{auto}`,
    ///     takes on a thickness of 0.05em and square line caps.
    ///
    /// ```example
    /// $ mat(1, 0, 1; 0, 1, 2; augment: #2) $
    /// // Equivalent to:
    /// $ mat(1, 0, 1; 0, 1, 2; augment: #(-1)) $
    /// ```
    ///
    /// ```example
    /// $ mat(0, 0, 0; 1, 1, 1; augment: #(hline: 1, stroke: 2pt + green)) $
    /// ```
    #[resolve]
    #[fold]
    pub augment: Option<Augment>,

    /// The gap between rows and columns.
    ///
    /// ```example
    /// #set math.mat(gap: 1em)
    /// $ mat(1, 2; 3, 4) $
    /// ```
    #[external]
    pub gap: Rel<Length>,

    /// The gap between rows. Takes precedence over `gap`.
    ///
    /// ```example
    /// #set math.mat(row-gap: 1em)
    /// $ mat(1, 2; 3, 4) $
    /// ```
    #[resolve]
    #[parse(
        let gap = args.named("gap")?;
        args.named("row-gap")?.or(gap)
    )]
    #[default(DEFAULT_ROW_GAP.into())]
    pub row_gap: Rel<Length>,

    /// The gap between columns. Takes precedence over `gap`.
    ///
    /// ```example
    /// #set math.mat(column-gap: 1em)
    /// $ mat(1, 2; 3, 4) $
    /// ```
    #[resolve]
    #[parse(args.named("column-gap")?.or(gap))]
    #[default(DEFAULT_COL_GAP.into())]
    pub column_gap: Rel<Length>,

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

impl LayoutMath for Packed<MatElem> {
    #[typst_macros::time(name = "math.mat", span = self.span())]
    fn layout_math(&self, ctx: &mut MathContext, styles: StyleChain) -> SourceResult<()> {
        let augment = self.augment(styles);
        let rows = self.rows();

        if let Some(aug) = &augment {
            for &offset in &aug.hline.0 {
                if offset == 0 || offset.unsigned_abs() >= rows.len() {
                    bail!(
                        self.span(),
                        "cannot draw a horizontal line after row {} of a matrix with {} rows",
                        if offset < 0 { rows.len() as isize + offset } else { offset },
                        rows.len()
                    );
                }
            }

            let ncols = self.rows().first().map_or(0, |row| row.len());

            for &offset in &aug.vline.0 {
                if offset == 0 || offset.unsigned_abs() >= ncols {
                    bail!(
                        self.span(),
                        "cannot draw a vertical line after column {} of a matrix with {} columns",
                        if offset < 0 { ncols as isize + offset } else { offset },
                        ncols
                    );
                }
            }
        }

        let delim = self.delim(styles);
        let frame = layout_mat_body(
            ctx,
            styles,
            rows,
            augment,
            Axes::new(self.column_gap(styles), self.row_gap(styles)),
            self.span(),
        )?;

        layout_delimiters(
            ctx,
            styles,
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
/// # Example
/// ```example
/// $ f(x, y) := cases(
///   1 "if" (x dot y)/2 <= 0,
///   2 "if" x "is even",
///   3 "if" x in NN,
///   4 "else",
/// ) $
/// ```
#[elem(LayoutMath)]
pub struct CasesElem {
    /// The delimiter to use.
    ///
    /// ```example
    /// #set math.cases(delim: "[")
    /// $ x = cases(1, 2) $
    /// ```
    #[default(Delimiter::Brace)]
    pub delim: Delimiter,

    /// Whether the direction of cases should be reversed.
    ///
    /// ```example
    /// #set math.cases(reverse: true)
    /// $ cases(1, 2) = x $
    /// ```
    #[default(false)]
    pub reverse: bool,

    /// The gap between branches.
    ///
    /// ```example
    /// #set math.cases(gap: 1em)
    /// $ x = cases(1, 2) $
    /// ```
    #[resolve]
    #[default(DEFAULT_ROW_GAP.into())]
    pub gap: Rel<Length>,

    /// The branches of the case distinction.
    #[variadic]
    pub children: Vec<Content>,
}

impl LayoutMath for Packed<CasesElem> {
    #[typst_macros::time(name = "math.cases", span = self.span())]
    fn layout_math(&self, ctx: &mut MathContext, styles: StyleChain) -> SourceResult<()> {
        let delim = self.delim(styles);
        let frame = layout_vec_body(
            ctx,
            styles,
            self.children(),
            FixedAlignment::Start,
            self.gap(styles),
        )?;

        let (open, close) = if self.reverse(styles) {
            (None, Some(delim.close()))
        } else {
            (Some(delim.open()), None)
        };

        layout_delimiters(ctx, styles, frame, open, close, self.span())
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
    styles: StyleChain,
    column: &[Content],
    align: FixedAlignment,
    row_gap: Rel<Abs>,
) -> SourceResult<Frame> {
    let gap = row_gap.relative_to(ctx.regions.base().y);

    let denom_style = style_for_denominator(styles);
    let mut flat = vec![];
    for child in column {
        flat.push(ctx.layout_into_run(child, styles.chain(&denom_style))?);
    }

    Ok(stack(flat, align, gap, 0))
}

/// Layout the inner contents of a matrix.
fn layout_mat_body(
    ctx: &mut MathContext,
    styles: StyleChain,
    rows: &[Vec<Content>],
    augment: Option<Augment<Abs>>,
    gap: Axes<Rel<Abs>>,
    span: Span,
) -> SourceResult<Frame> {
    let gap = gap.zip_map(ctx.regions.base(), Rel::relative_to);
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

    let ncols = rows.first().map_or(0, |row| row.len());
    let nrows = rows.len();
    if ncols == 0 || nrows == 0 {
        return Ok(Frame::soft(Size::zero()));
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

    let denom_style = style_for_denominator(styles);
    for (row, (ascent, descent)) in rows.iter().zip(&mut heights) {
        for (cell, col) in row.iter().zip(&mut cols) {
            let cell = ctx.layout_into_run(cell, styles.chain(&denom_style))?;

            ascent.set_max(cell.ascent());
            descent.set_max(cell.descent());

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
            let cell = cell.into_line_frame(&points, FixedAlignment::Center);
            let pos = Point::new(
                if points.is_empty() { x + (rcol - cell.width()) / 2.0 } else { x },
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
        left.center_on_axis(ctx);
        ctx.push(left);
    }

    ctx.push(FrameFragment::new(ctx, styles, frame));

    if let Some(right) = right {
        let mut right = GlyphFragment::new(ctx, styles, right, span)
            .stretch_vertical(ctx, target, short_fall);
        right.center_on_axis(ctx);
        ctx.push(right);
    }

    Ok(())
}

/// Parameters specifying how augmentation lines
/// should be drawn on a matrix.
#[derive(Debug, Default, Clone, PartialEq, Hash)]
pub struct Augment<T: Numeric = Length> {
    pub hline: AugmentOffsets,
    pub vline: AugmentOffsets,
    pub stroke: Smart<Stroke<T>>,
}

impl<T: Numeric + Fold> Fold for Augment<T> {
    fn fold(self, outer: Self) -> Self {
        Self {
            stroke: match (self.stroke, outer.stroke) {
                (Smart::Custom(inner), Smart::Custom(outer)) => {
                    Smart::Custom(inner.fold(outer))
                }
                // Usually, folding an inner `auto` with an `outer` preferres
                // the explicit `auto`. However, here `auto` means unspecified
                // and thus we want `outer`.
                (inner, outer) => inner.or(outer),
            },
            ..self
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
        // if the stroke is auto and there is only one vertical line,
        if self.stroke.is_auto() && self.hline.0.is_empty() && self.vline.0.len() == 1 {
            return self.vline.0[0].into_value();
        }

        dict! {
            "hline" => self.hline,
            "vline" => self.vline,
            "stroke" => self.stroke,
        }.into_value()
    },
    v: isize => Augment {
        hline: AugmentOffsets::default(),
        vline: AugmentOffsets(smallvec![v]),
        stroke: Smart::Auto,
    },
    mut dict: Dict => {
        let mut take = |key| dict.take(key).ok().map(AugmentOffsets::from_value).transpose();
        let hline = take("hline")?.unwrap_or_default();
        let vline = take("vline")?.unwrap_or_default();
        let stroke = dict.take("stroke")
            .ok()
            .map(Stroke::from_value)
            .transpose()?
            .map(Smart::Custom)
            .unwrap_or(Smart::Auto);
        Augment { hline, vline, stroke }
    },
}

cast! {
    Augment<Abs>,
    self => self.into_value(),
}

/// The offsets at which augmentation lines should be drawn on a matrix.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct AugmentOffsets(SmallVec<[isize; 1]>);

cast! {
    AugmentOffsets,
    self => self.0.into_value(),
    v: isize => Self(smallvec![v]),
    v: Array => Self(v.into_iter().map(Value::cast).collect::<StrResult<_>>()?),
}
