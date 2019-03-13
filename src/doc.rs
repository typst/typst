//! Abstract representation of a typesetted document.

use crate::font::Font;


/// A representation of a typesetted document.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    /// The pages of the document.
    pub pages: Vec<Page>,
    /// The fonts used in the document.
    pub fonts: Vec<Font>,
}

/// Default styles for a document.
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    /// The width of the paper.
    pub width: Size,
    /// The height of the paper.
    pub height: Size,

    /// The left margin of the paper.
    pub margin_left: Size,
    /// The top margin of the paper.
    pub margin_top: Size,
    /// The right margin of the paper.
    pub margin_right: Size,
    /// The bottom margin of the paper.
    pub margin_bottom: Size,

    /// A fallback list of font families to use.
    pub font_families: Vec<String>,
    /// The font size.
    pub font_size: f32,
    /// The line spacing (as a multiple of the font size).
    pub line_spacing: f32,
}

impl Default for Style {
    fn default() -> Style {
        Style {
            // A4 paper
            width: Size::from_mm(210.0),
            height: Size::from_mm(297.0),

            // A bit more on top and bottom
            margin_left: Size::from_cm(2.5),
            margin_top: Size::from_cm(3.0),
            margin_right: Size::from_cm(2.5),
            margin_bottom: Size::from_cm(3.0),

            // Default font family
            font_families: (&[
                "NotoSans", "NotoSansMath"
            ]).iter().map(ToString::to_string).collect(),
            font_size: 12.0,
            line_spacing: 1.25,
        }
    }
}

/// A page with text contents in a document.
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    /// The width of the page.
    pub width: Size,
    /// The height of the page.
    pub height: Size,
    /// Text content on the page.
    pub text: Vec<Text>,
}

/// A series of text command, that can be written on to a page.
#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    /// The text commands.
    pub commands: Vec<TextCommand>,
}

/// Different commands for rendering text.
#[derive(Debug, Clone, PartialEq)]
pub enum TextCommand {
    /// Writing of the text.
    Text(String),
    /// Moves from the *start* of the current line by an (x,y) offset.
    Move(Size, Size),
    /// Use the indexed font in the documents font list with a given font size.
    SetFont(usize, f32),
}

/// A general distance type that can convert between units.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Size {
    /// The size in typographic points (1/72 inches).
    points: f32,
}

impl Size {
    /// Create an zeroed size.
    #[inline]
    pub fn zero() -> Size { Size { points: 0.0 } }

    /// Create a size from a number of points.
    #[inline]
    pub fn from_points(points: f32) -> Size { Size { points } }

    /// Create a size from a number of inches.
    #[inline]
    pub fn from_inches(inches: f32) -> Size { Size { points: 72.0 * inches } }

    /// Create a size from a number of millimeters.
    #[inline]
    pub fn from_mm(mm: f32) -> Size { Size { points: 2.83465 * mm  } }

    /// Create a size from a number of centimeters.
    #[inline]
    pub fn from_cm(cm: f32) -> Size { Size { points: 28.3465 * cm } }

    /// Create a size from a number of points.
    #[inline]
    pub fn to_points(&self) -> f32 { self.points }

    /// Create a size from a number of inches.
    #[inline]
    pub fn to_inches(&self) -> f32 { self.points * 0.0138889 }

    /// Create a size from a number of millimeters.
    #[inline]
    pub fn to_mm(&self) -> f32 { self.points * 0.352778 }

    /// Create a size from a number of centimeters.
    #[inline]
    pub fn to_cm(&self) -> f32 { self.points * 0.0352778 }
}

mod size {
    use super::Size;
    use std::cmp::Ordering;
    use std::fmt;
    use std::iter::Sum;
    use std::ops::*;

    impl fmt::Display for Size {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}pt", self.points)
        }
    }

    macro_rules! impl_reflexive {
        ($trait:ident, $func:ident, $assign_trait:ident, $assign_func:ident) => {
            impl $trait for Size {
                type Output = Size;

                #[inline]
                fn $func(self, other: Size) -> Size {
                    Size { points: $trait::$func(self.points, other.points) }
                }
            }

            impl $assign_trait for Size {
                #[inline]
                fn $assign_func(&mut self, other: Size) {
                    $assign_trait::$assign_func(&mut self.points, other.points);
                }
            }
        };
    }

    macro_rules! impl_num_back {
        ($trait:ident, $func:ident, $assign_trait:ident, $assign_func:ident, $ty:ty) => {
            impl $trait<$ty> for Size {
                type Output = Size;

                #[inline]
                fn $func(self, other: $ty) -> Size {
                    Size { points: $trait::$func(self.points, other as f32) }
                }
            }

            impl $assign_trait<$ty> for Size {
                #[inline]
                fn $assign_func(&mut self, other: $ty) {
                    $assign_trait::$assign_func(&mut self.points, other as f32);
                }
            }
        };
    }

    macro_rules! impl_num_both {
        ($trait:ident, $func:ident, $assign_trait:ident, $assign_func:ident, $ty:ty) => {
            impl_num_back!($trait, $func, $assign_trait, $assign_func, $ty);

            impl $trait<Size> for $ty {
                type Output = Size;

                #[inline]
                fn $func(self, other: Size) -> Size {
                    Size { points: $trait::$func(self as f32, other.points) }
                }
            }
        };
    }

    impl Neg for Size {
        type Output = Size;

        fn neg(self) -> Size {
            Size { points: -self.points }
        }
    }

    impl_reflexive!(Add, add, AddAssign, add_assign);
    impl_reflexive!(Sub, sub, SubAssign, sub_assign);
    impl_num_both!(Mul, mul, MulAssign, mul_assign, f32);
    impl_num_both!(Mul, mul, MulAssign, mul_assign, i32);
    impl_num_back!(Div, div, DivAssign, div_assign, f32);
    impl_num_back!(Div, div, DivAssign, div_assign, i32);

    impl PartialOrd for Size {
        fn partial_cmp(&self, other: &Size) -> Option<Ordering> {
            self.points.partial_cmp(&other.points)
        }
    }

    impl Sum for Size {
        fn sum<I>(iter: I) -> Size where I: Iterator<Item=Size> {
            iter.fold(Size::zero(), Add::add)
        }
    }
}
