//! Parsing, storing and deduplication of function arguments.

use super::prelude::*;
use Expression::*;

/// Provides a convenient interface to parse the arguments to a function.
pub struct ArgParser<'a> {
    args: &'a FuncArgs,
    positional_index: usize,
}

impl<'a> ArgParser<'a> {
    pub fn new(args: &'a FuncArgs) -> ArgParser<'a> {
        ArgParser {
            args,
            positional_index: 0,
        }
    }

    /// Get the next positional argument of the given type.
    ///
    /// If there are no more arguments or the type is wrong,
    /// this will return an error.
    pub fn get_pos<T>(&mut self) -> ParseResult<Spanned<T::Output>>
    where T: Argument<'a> {
        Self::expected(self.get_pos_opt::<T>()?)
    }

    /// Get the next positional argument if there is any.
    ///
    /// If the argument is of the wrong type, this will return an error.
    pub fn get_pos_opt<T>(&mut self) -> ParseResult<Option<Spanned<T::Output>>>
    where T: Argument<'a> {
        let arg = self.args.positional
            .get(self.positional_index)
            .map(T::from_expr)
            .transpose();

        if let Ok(Some(_)) = arg {
            self.positional_index += 1;
        }

        arg
    }

    /// Get a keyword argument with the given key and type.
    pub fn get_key<T>(&mut self, key: &str) -> ParseResult<Spanned<T::Output>>
    where T: Argument<'a> {
        Self::expected(self.get_key_opt::<T>(key)?)
    }

    /// Get a keyword argument with the given key and type if it is present.
    pub fn get_key_opt<T>(&mut self, key: &str) -> ParseResult<Option<Spanned<T::Output>>>
    where T: Argument<'a> {
        self.args.keyword.iter()
            .find(|entry| entry.val.0.val == key)
            .map(|entry| T::from_expr(&entry.val.1))
            .transpose()
    }

    /// Assert that there are no positional arguments left. Returns an error
    /// otherwise.
    pub fn done(&self) -> ParseResult<()> {
        if self.positional_index == self.args.positional.len() {
            Ok(())
        } else {
            pr!("unexpected argument");
        }
    }

    /// Covert an option to a result with an error on `None`.
    fn expected<T>(val: Option<Spanned<T::Output>>) -> ParseResult<Spanned<T::Output>>
    where T: Argument<'a> {
        val.ok_or_else(|| pr!(@"expected {}", T::ERROR_MESSAGE))
    }
}

/// A kind of argument.
pub trait Argument<'a> {
    type Output;
    const ERROR_MESSAGE: &'static str;

    fn from_expr(expr: &'a Spanned<Expression>) -> ParseResult<Spanned<Self::Output>>;
}

macro_rules! arg {
    ($type:ident, $err:expr, $doc:expr, $output:ty, $wanted:pat => $converted:expr) => (
        #[doc = $doc]
        #[doc = " argument for use with the [`ArgParser`]."]
        pub struct $type;
        impl<'a> Argument<'a> for $type {
            type Output = $output;
            const ERROR_MESSAGE: &'static str = $err;

            fn from_expr(expr: &'a Spanned<Expression>) -> ParseResult<Spanned<Self::Output>> {
                #[allow(unreachable_patterns)]
                match &expr.val {
                    $wanted => Ok(Spanned::new($converted, expr.span)),
                    _ => pr!("expected {}", $err),
                }
            }
        }
    );
}

arg!(ArgExpr,  "expression", "A generic expression", &'a Expression, expr => &expr);
arg!(ArgIdent, "identifier", "An identifier (e.g. `horizontal`)", &'a str, Ident(s) => s.as_str());
arg!(ArgStr,   "string", "A string (e.g. `\"Hello\"`)", &'a str, Str(s) => s.as_str());
arg!(ArgNum,   "number", "A number (e.g. `5.4`)", f64, Num(n) => *n);
arg!(ArgSize,  "size", "A size (e.g. `12pt`)", crate::size::Size, Size(s) => *s);
arg!(ArgBool,  "bool", "A boolean (`true` or `false`)", bool, Bool(b) => *b);

/// An argument key which identifies a layouting axis.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum AxisKey {
    Primary,
    Secondary,
    Vertical,
    Horizontal,
}

impl AxisKey {
    /// The generic version of this axis key in the given system of axes.
    pub fn generic(&self, axes: LayoutAxes) -> GenericAxisKind {
        match self {
            Primary => GenericAxisKind::Primary,
            Secondary => GenericAxisKind::Secondary,
            Vertical => axes.vertical(),
            Horizontal => axes.horizontal(),
        }
    }

    /// The specific version of this axis key in the given system of axes.
    pub fn specific(&self, axes: LayoutAxes) -> SpecificAxisKind {
        match self {
            Primary => axes.primary(),
            Secondary => axes.secondary(),
            Vertical => SpecificAxisKind::Vertical,
            Horizontal => SpecificAxisKind::Horizontal,
        }
    }
}

/// An argument key which identifies a target alignment.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum AlignmentKey {
    Left,
    Top,
    Right,
    Bottom,
    Origin,
    Center,
    End,
}

impl AlignmentKey {
    /// The generic axis this alignment key corresopnds to in the given system
    /// of layouting axes. Falls back to `default` if the alignment is generic.
    pub fn axis(&self, axes: LayoutAxes, default: GenericAxisKind) -> GenericAxisKind {
        use AlignmentKey::*;
        match self {
            Origin | Center | End => default,
            Left | Right => axes.horizontal(),
            Top | Bottom => axes.vertical(),
        }
    }

    /// The generic version of this alignment in the given system of layouting
    /// axes. Returns an error if the alignment is invalid for the given axis.
    pub fn generic(&self, axes: LayoutAxes, axis: GenericAxisKind) -> LayoutResult<Alignment> {
        use AlignmentKey::*;

        let horizontal = axis == axes.horizontal();
        Ok(match self {
            Origin => Alignment::Origin,
            Center => Alignment::Center,
            End => Alignment::End,
            Left if horizontal => axes.left(),
            Right if horizontal => axes.right(),
            Top if !horizontal => axes.top(),
            Bottom if !horizontal => axes.bottom(),
            _ => lr!(
                "invalid alignment `{}` for {} axis",
                format!("{:?}", self).to_lowercase(),
                format!("{:?}", axis).to_lowercase()
            )
        })
    }

    /// The specific version of this alignment in the given system of layouting
    /// axes.
    pub fn specific(&self, axes: LayoutAxes, axis: SpecificAxisKind) -> AlignmentKey {
        use AlignmentKey::*;
        match (self, axis) {
            (Origin, SpecificAxisKind::Horizontal) => Left,
            (End, SpecificAxisKind::Horizontal) => Right,
            (Origin, SpecificAxisKind::Vertical) => Top,
            (End, SpecificAxisKind::Vertical) => Bottom,
            _ => *self,
        }
    }
}

/// An argument key which identifies a margin or padding target.
///
/// A is the axis type used.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum PaddingKey<A> {
    /// All four sides should have the specified padding.
    All,
    /// Both sides of the given axis should have the specified padding.
    Axis(A),
    /// Only the given side of the given axis should have the specified padding.
    AxisAligned(A, AlignmentKey),
}
