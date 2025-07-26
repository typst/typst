use smallvec::{SmallVec, smallvec};
use typst_syntax::Spanned;
use typst_utils::{Numeric, default_math_class};
use unicode_math_class::MathClass;

use crate::diag::{At, HintedStrResult, StrResult, bail};
use crate::foundations::{
    Array, Content, Dict, Fold, NoneValue, Resolve, Smart, StyleChain, Symbol, Value,
    array, cast, dict, elem,
};
use crate::layout::{Abs, Em, HAlignment, Length, Rel};
use crate::math::Mathy;
use crate::visualize::Stroke;

const DEFAULT_ROW_GAP: Em = Em::new(0.2);
const DEFAULT_COL_GAP: Em = Em::new(0.5);

/// A column vector.
///
/// Content in the vector's elements can be aligned with the
/// [`align`]($math.vec.align) parameter, or the `&` symbol.
///
/// # Example
/// ```example
/// $ vec(a, b, c) dot vec(1, 2, 3)
///     = a + 2b + 3c $
/// ```
#[elem(title = "Vector", Mathy)]
pub struct VecElem {
    /// The delimiter to use.
    ///
    /// Can be a single character specifying the left delimiter, in which case
    /// the right delimiter is inferred. Otherwise, can be an array containing a
    /// left and a right delimiter.
    ///
    /// ```example
    /// #set math.vec(delim: "[")
    /// $ vec(1, 2) $
    /// ```
    #[default(DelimiterPair::PAREN)]
    pub delim: DelimiterPair,

    /// The horizontal alignment that each element should have.
    ///
    /// ```example
    /// #set math.vec(align: right)
    /// $ vec(-1, 1, -1) $
    /// ```
    #[default(HAlignment::Center)]
    pub align: HAlignment,

    /// The gap between elements.
    ///
    /// ```example
    /// #set math.vec(gap: 1em)
    /// $ vec(1, 2) $
    /// ```
    #[default(DEFAULT_ROW_GAP.into())]
    pub gap: Rel<Length>,

    /// The elements of the vector.
    #[variadic]
    pub children: Vec<Content>,
}

/// A matrix.
///
/// The elements of a row should be separated by commas, while the rows
/// themselves should be separated by semicolons. The semicolon syntax merges
/// preceding arguments separated by commas into an array. You can also use this
/// special syntax of math function calls to define custom functions that take
/// 2D data.
///
/// Content in cells can be aligned with the [`align`]($math.mat.align)
/// parameter, or content in cells that are in the same row can be aligned with
/// the `&` symbol.
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
#[elem(title = "Matrix", Mathy)]
pub struct MatElem {
    /// The delimiter to use.
    ///
    /// Can be a single character specifying the left delimiter, in which case
    /// the right delimiter is inferred. Otherwise, can be an array containing a
    /// left and a right delimiter.
    ///
    /// ```example
    /// #set math.mat(delim: "[")
    /// $ mat(1, 2; 3, 4) $
    /// ```
    #[default(DelimiterPair::PAREN)]
    pub delim: DelimiterPair,

    /// The horizontal alignment that each cell should have.
    ///
    /// ```example
    /// #set math.mat(align: right)
    /// $ mat(-1, 1, 1; 1, -1, 1; 1, 1, -1) $
    /// ```
    #[default(HAlignment::Center)]
    pub align: HAlignment,

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
    #[fold]
    pub augment: Option<Augment>,

    /// The gap between rows and columns.
    ///
    /// This is a shorthand to set `row-gap` and `column-gap` to the same value.
    ///
    /// ```example
    /// #set math.mat(gap: 1em)
    /// $ mat(1, 2; 3, 4) $
    /// ```
    #[external]
    pub gap: Rel<Length>,

    /// The gap between rows.
    ///
    /// ```example
    /// #set math.mat(row-gap: 1em)
    /// $ mat(1, 2; 3, 4) $
    /// ```
    #[parse(
        let gap = args.named("gap")?;
        args.named("row-gap")?.or(gap)
    )]
    #[default(DEFAULT_ROW_GAP.into())]
    pub row_gap: Rel<Length>,

    /// The gap between columns.
    ///
    /// ```example
    /// #set math.mat(column-gap: 1em)
    /// $ mat(1, 2; 3, 4) $
    /// ```
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
#[elem(Mathy)]
pub struct CasesElem {
    /// The delimiter to use.
    ///
    /// Can be a single character specifying the left delimiter, in which case
    /// the right delimiter is inferred. Otherwise, can be an array containing a
    /// left and a right delimiter.
    ///
    /// ```example
    /// #set math.cases(delim: "[")
    /// $ x = cases(1, 2) $
    /// ```
    #[default(DelimiterPair::BRACE)]
    pub delim: DelimiterPair,

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
    #[default(DEFAULT_ROW_GAP.into())]
    pub gap: Rel<Length>,

    /// The branches of the case distinction.
    #[variadic]
    pub children: Vec<Content>,
}

/// A delimiter is a single character that is used to delimit a matrix, vector
/// or cases. The character has to be a Unicode codepoint tagged as a math
/// "opening", "closing" or "fence".
///
/// Typically, the delimiter is stretched to fit the height of whatever it
/// delimits.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Delimiter(Option<char>);

cast! {
    Delimiter,
    self => self.0.into_value(),
    _: NoneValue => Self::none(),
    v: Symbol => Self::char(v.get().parse::<char>().map_err(|_| "expected a single-codepoint symbol")?)?,
    v: char => Self::char(v)?,
}

impl Delimiter {
    pub fn none() -> Self {
        Self(None)
    }

    pub fn char(c: char) -> StrResult<Self> {
        if !matches!(
            default_math_class(c),
            Some(MathClass::Opening | MathClass::Closing | MathClass::Fence),
        ) {
            bail!("invalid delimiter: \"{}\"", c)
        }
        Ok(Self(Some(c)))
    }

    pub fn get(self) -> Option<char> {
        self.0
    }

    pub fn find_matching(self) -> Self {
        match self.0 {
            None => Self::none(),
            Some('[') => Self(Some(']')),
            Some(']') => Self(Some('[')),
            Some('{') => Self(Some('}')),
            Some('}') => Self(Some('{')),
            Some(c) => match default_math_class(c) {
                Some(MathClass::Opening) => Self(char::from_u32(c as u32 + 1)),
                Some(MathClass::Closing) => Self(char::from_u32(c as u32 - 1)),
                _ => Self(Some(c)),
            },
        }
    }
}

/// A pair of delimiters (one closing, one opening) used for matrices, vectors
/// and cases.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct DelimiterPair {
    open: Delimiter,
    close: Delimiter,
}

cast! {
    DelimiterPair,

    self => array![self.open, self.close].into_value(),

    v: Array => match v.as_slice() {
        [open, close] => Self {
            open: open.clone().cast()?,
            close: close.clone().cast()?,
        },
        _ => bail!("expected 2 delimiters, found {}", v.len())
    },
    v: Delimiter => Self { open: v, close: v.find_matching() }
}

impl DelimiterPair {
    const PAREN: Self = Self {
        open: Delimiter(Some('(')),
        close: Delimiter(Some(')')),
    };
    const BRACE: Self = Self {
        open: Delimiter(Some('{')),
        close: Delimiter(Some('}')),
    };

    /// The delimiter's opening character.
    pub fn open(self) -> Option<char> {
        self.open.get()
    }

    /// The delimiter's closing character.
    pub fn close(self) -> Option<char> {
        self.close.get()
    }
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
                // Usually, folding an inner `auto` with an `outer` prefers
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
pub struct AugmentOffsets(pub SmallVec<[isize; 1]>);

cast! {
    AugmentOffsets,
    self => self.0.into_value(),
    v: isize => Self(smallvec![v]),
    v: Array => Self(v.into_iter().map(Value::cast).collect::<HintedStrResult<_>>()?),
}
