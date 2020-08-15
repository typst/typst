//! Expressions in function headers.

use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;
use std::str::FromStr;
use std::u8;

use fontdock::{FontStyle, FontWeight, FontWidth};

use crate::layout::{Dir, SpecAlign};
use crate::length::{Length, ScaleLength};
use crate::paper::Paper;
use crate::table::{BorrowedKey, Table};
use crate::Feedback;
use super::span::{Span, SpanVec, Spanned};
use super::tokens::is_identifier;
use super::tree::SyntaxTree;

/// An expression.
#[derive(Clone, PartialEq)]
pub enum Expr {
    /// An identifier: `ident`.
    Ident(Ident),
    /// A string: `"string"`.
    Str(String),
    /// A boolean: `true, false`.
    Bool(bool),
    /// A number: `1.2, 200%`.
    Number(f64),
    /// A length: `2cm, 5.2in`.
    Length(Length),
    /// A color value with alpha channel: `#f79143ff`.
    Color(RgbaColor),
    /// A syntax tree containing typesetting content.
    Tree(SyntaxTree),
    /// A tuple: `(false, 12cm, "hi")`.
    Tuple(Tuple),
    /// A named tuple: `cmyk(37.7, 0, 3.9, 1.1)`.
    NamedTuple(NamedTuple),
    /// An object: `{ fit=false, width=12pt }`.
    Object(Object),
    /// An operation that negates the contained expression.
    Neg(Box<Spanned<Expr>>),
    /// An operation that adds the contained expressions.
    Add(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    /// An operation that subtracts the contained expressions.
    Sub(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    /// An operation that multiplies the contained expressions.
    Mul(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    /// An operation that divides the contained expressions.
    Div(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
}

impl Expr {
    /// A natural-language name of the type of this expression, e.g.
    /// "identifier".
    pub fn name(&self) -> &'static str {
        use Expr::*;
        match self {
            Ident(_)      => "identifier",
            Str(_)        => "string",
            Bool(_)       => "bool",
            Number(_)     => "number",
            Length(_)     => "length",
            Color(_)      => "color",
            Tree(_)       => "syntax tree",
            Tuple(_)      => "tuple",
            NamedTuple(_) => "named tuple",
            Object(_)     => "object",
            Neg(_)        => "negation",
            Add(_, _)     => "addition",
            Sub(_, _)     => "subtraction",
            Mul(_, _)     => "multiplication",
            Div(_, _)     => "division",
        }
    }
}

impl Debug for Expr {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use Expr::*;
        match self {
            Ident(i) => i.fmt(f),
            Str(s) => s.fmt(f),
            Bool(b) => b.fmt(f),
            Number(n) => n.fmt(f),
            Length(s) => s.fmt(f),
            Color(c) => c.fmt(f),
            Tree(t) => t.fmt(f),
            Tuple(t) => t.fmt(f),
            NamedTuple(t) => t.fmt(f),
            Object(o) => o.fmt(f),
            Neg(e) => write!(f, "-{:?}", e),
            Add(a, b) => write!(f, "({:?} + {:?})", a, b),
            Sub(a, b) => write!(f, "({:?} - {:?})", a, b),
            Mul(a, b) => write!(f, "({:?} * {:?})", a, b),
            Div(a, b) => write!(f, "({:?} / {:?})", a, b),
        }
    }
}

/// An identifier as defined by unicode with a few extra permissible characters.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Ident(pub String);

impl Ident {
    /// Create a new identifier from a string checking that it is a valid.
    pub fn new(ident: impl AsRef<str> + Into<String>) -> Option<Self> {
        if is_identifier(ident.as_ref()) {
            Some(Self(ident.into()))
        } else {
            None
        }
    }

    /// Return a reference to the underlying string.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Debug for Ident {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "`{}`", self.0)
    }
}

/// An 8-bit RGBA color.
///
/// # Example
/// ```typst
/// [page: background=#423abaff]
///                   ^^^^^^^^
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct RgbaColor {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
    /// Alpha channel.
    pub a: u8,
    /// This is true if this value was provided as a fail-over by the parser
    /// because the user-defined value was invalid. This color may be
    /// overwritten if this property is true.
    pub healed: bool,
}

impl RgbaColor {
    /// Constructs a new color.
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a, healed: false }
    }

    /// Constructs a new color with the healed property set to true.
    pub fn new_healed(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a, healed: true }
    }
}

impl FromStr for RgbaColor {
    type Err = ParseColorError;

    /// Constructs a new color from a hex string like `7a03c2`. Do not specify a
    /// leading `#`.
    fn from_str(hex_str: &str) -> Result<Self, Self::Err> {
        if !hex_str.is_ascii() {
            return Err(ParseColorError);
        }

        let len = hex_str.len();
        let long  = len == 6 || len == 8;
        let short = len == 3 || len == 4;
        let alpha = len == 4 || len == 8;

        if !long && !short {
            return Err(ParseColorError);
        }

        let mut values: [u8; 4] = [255; 4];

        for elem in if alpha { 0..4 } else { 0..3 } {
            let item_len = if long { 2 } else { 1 };
            let pos = elem * item_len;

            let item = &hex_str[pos..(pos+item_len)];
            values[elem] = u8::from_str_radix(item, 16)
                .map_err(|_| ParseColorError)?;

            if short {
                // Duplicate number for shorthand notation, i.e. `a` -> `aa`
                values[elem] += values[elem] * 16;
            }
        }

        Ok(Self::new(values[0], values[1], values[2], values[3]))
    }
}

impl Debug for RgbaColor {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if f.alternate() {
            write!(
                f, "rgba({:02}, {:02}, {:02}, {:02})",
                self.r, self.g, self.b, self.a,
            )?;
        } else {
            write!(
                f, "#{:02x}{:02x}{:02x}{:02x}",
                self.r, self.g, self.b, self.a,
            )?;
        }
        if self.healed {
            f.write_str(" [healed]")?;
        }
        Ok(())
    }
}

/// The error when parsing an `RgbaColor` fails.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ParseColorError;

impl std::error::Error for ParseColorError {}

impl fmt::Display for ParseColorError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("invalid color")
    }
}

/// An untyped sequence of expressions.
///
/// # Example
/// ```typst
/// (false, 12cm, "hi")
/// ```
#[derive(Default, Clone, PartialEq)]
pub struct Tuple(pub SpanVec<Expr>);

impl Tuple {
    /// Create an empty tuple.
    pub fn new() -> Self {
        Self(vec![])
    }

    /// Add an element.
    pub fn push(&mut self, item: Spanned<Expr>) {
        self.0.push(item);
    }

    /// Expect a specific value type and generate errors for every argument
    /// until an argument of the value type is found.
    pub fn expect<T: TryFromExpr>(&mut self, f: &mut Feedback) -> Option<T> {
        while !self.0.is_empty() {
            let item = self.0.remove(0);
            if let Some(val) = T::try_from_expr(item.as_ref(), f) {
                return Some(val);
            }
        }
        None
    }

    /// Extract the first argument of the value type if there is any.
    pub fn get<T: TryFromExpr>(&mut self) -> Option<T> {
        for (i, item) in self.0.iter().enumerate() {
            let expr = item.as_ref();
            if let Some(val) = T::try_from_expr(expr, &mut Feedback::new()) {
                self.0.remove(i);
                return Some(val);
            }
        }
        None
    }

    /// Extract all arguments of the value type.
    pub fn all<'a, T: TryFromExpr>(&'a mut self) -> impl Iterator<Item = T> + 'a {
        let mut i = 0;
        std::iter::from_fn(move || {
            while i < self.0.len() {
                let expr = self.0[i].as_ref();
                let val = T::try_from_expr(expr, &mut Feedback::new());
                if val.is_some() {
                    self.0.remove(i);
                    return val;
                } else {
                    i += 1;
                }
            }
            None
        })
    }
}

impl Debug for Tuple {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_list().entries(&self.0).finish()
    }
}

/// A named, untyped sequence of expressions.
///
/// # Example
/// ```typst
/// hsl(93, 10, 19.4)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct NamedTuple {
    /// The name of the tuple.
    pub name: Spanned<Ident>,
    /// The elements of the tuple.
    pub tuple: Spanned<Tuple>,
}

impl NamedTuple {
    /// Create a named tuple from a name and a tuple.
    pub fn new(name: Spanned<Ident>, tuple: Spanned<Tuple>) -> Self {
        Self { name, tuple }
    }
}

impl Deref for NamedTuple {
    type Target = Tuple;

    fn deref(&self) -> &Self::Target {
        &self.tuple.v
    }
}

/// A key-value collection of identifiers and associated expressions.
///
/// # Example
/// ```typst
/// { fit = false, width = 12cm, items = (1, 2, 3) }
/// ```
#[derive(Default, Clone, PartialEq)]
pub struct Object(pub SpanVec<Pair>);

/// A key-value pair in an object.
#[derive(Debug, Clone, PartialEq)]
pub struct Pair {
    pub key: Spanned<Ident>,
    pub value: Spanned<Expr>,
}

impl Object {
    /// Create an empty object.
    pub fn new() -> Self {
        Self(vec![])
    }

    /// Add a pair to object.
    pub fn push(&mut self, pair: Spanned<Pair>) {
        self.0.push(pair);
    }

    /// Extract an argument with the given key if there is any.
    ///
    /// Generates an error if there is a matching key, but the value is of the
    /// wrong type.
    pub fn get<T: TryFromExpr>(&mut self, key: &str, f: &mut Feedback) -> Option<T> {
        for (i, pair) in self.0.iter().enumerate() {
            if pair.v.key.v.as_str() == key {
                let pair = self.0.remove(i);
                return T::try_from_expr(pair.v.value.as_ref(), f);
            }
        }
        None
    }

    /// Extract all key-value pairs where the value is of the given type.
    pub fn all<'a, T: TryFromExpr>(&'a mut self)
        -> impl Iterator<Item = (Spanned<Ident>, T)> + 'a
    {
        let mut i = 0;
        std::iter::from_fn(move || {
            while i < self.0.len() {
                let expr = self.0[i].v.value.as_ref();
                let val = T::try_from_expr(expr, &mut Feedback::new());
                if let Some(val) = val {
                    let pair = self.0.remove(i);
                    return Some((pair.v.key, val));
                } else {
                    i += 1;
                }
            }
            None
        })
    }
}

impl Debug for Object {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_map()
            .entries(self.0.iter().map(|p| (&p.v.key.v, &p.v.value.v)))
            .finish()
    }
}

/// A table expression.
///
/// # Example
/// ```typst
/// (false, 12cm, greeting="hi")
/// ```
pub type TableExpr = Table<TableExprEntry>;

/// An entry in a table expression.
///
/// Contains the key's span and the value.
pub struct TableExprEntry {
    pub key: Span,
    pub val: Spanned<Expr>,
}

impl TableExpr {
    /// Retrieve and remove the matching value with the lowest number key,
    /// skipping and ignoring all non-matching entries with lower keys.
    pub fn take<T: TryFromExpr>(&mut self) -> Option<T> {
        for (&key, entry) in self.nums() {
            let expr = entry.val.as_ref();
            if let Some(val) = T::try_from_expr(expr, &mut Feedback::new()) {
                self.remove(key);
                return Some(val);
            }
        }
        None
    }

    /// Retrieve and remove the matching value with the lowest number key,
    /// removing and generating errors for all non-matching entries with lower
    /// keys.
    pub fn expect<T: TryFromExpr>(&mut self, f: &mut Feedback) -> Option<T> {
        while let Some((num, _)) = self.first() {
            let entry = self.remove(num).unwrap();
            if let Some(val) = T::try_from_expr(entry.val.as_ref(), f) {
                return Some(val);
            }
        }
        None
    }

    /// Retrieve and remove a matching value associated with the given key if
    /// there is any.
    ///
    /// Generates an error if the key exists but the value does not match.
    pub fn take_with_key<'a, T, K>(&mut self, key: K, f: &mut Feedback) -> Option<T>
    where
        T: TryFromExpr,
        K: Into<BorrowedKey<'a>>,
    {
        self.remove(key).and_then(|entry| {
            let expr = entry.val.as_ref();
            T::try_from_expr(expr, f)
        })
    }

    /// Retrieve and remove all matching pairs with number keys, skipping and
    /// ignoring non-matching entries.
    ///
    /// The pairs are returned in order of increasing keys.
    pub fn take_all_num<'a, T>(&'a mut self) -> impl Iterator<Item = (u64, T)> + 'a
    where
        T: TryFromExpr,
    {
        let mut skip = 0;
        std::iter::from_fn(move || {
            for (&key, entry) in self.nums().skip(skip) {
                let expr = entry.val.as_ref();
                if let Some(val) = T::try_from_expr(expr, &mut Feedback::new()) {
                    self.remove(key);
                    return Some((key, val));
                }
                skip += 1;
            }

            None
        })
    }

    /// Retrieve and remove all matching pairs with string keys, skipping and
    /// ignoring non-matching entries.
    ///
    /// The pairs are returned in order of increasing keys.
    pub fn take_all_str<'a, T>(&'a mut self) -> impl Iterator<Item = (String, T)> + 'a
    where
        T: TryFromExpr,
    {
        let mut skip = 0;
        std::iter::from_fn(move || {
            for (key, entry) in self.strs().skip(skip) {
                let expr = entry.val.as_ref();
                if let Some(val) = T::try_from_expr(expr, &mut Feedback::new()) {
                    let key = key.clone();
                    self.remove(&key);
                    return Some((key, val));
                }
                skip += 1;
            }

            None
        })
    }

    /// Generated `"unexpected argument"` errors for all remaining entries.
    pub fn unexpected(&self, f: &mut Feedback) {
        for entry in self.values() {
            let span = Span::merge(entry.key, entry.val.span);
            error!(@f, span, "unexpected argument");
        }
    }
}

/// A trait for converting expressions into specific types.
pub trait TryFromExpr: Sized {
    // This trait takes references because we don't want to move the expression
    // out of its origin in case this returns `None`. This solution is not
    // perfect because we need to do some cloning in the impls for this trait,
    // but I haven't got a better solution, for now.

    /// Try to convert an expression into this type.
    ///
    /// Returns `None` and generates an appropriate error if the expression is
    /// not valid for this type.
    fn try_from_expr(expr: Spanned<&Expr>, f: &mut Feedback) -> Option<Self>;
}

macro_rules! impl_match {
    ($type:ty, $name:expr, $($p:pat => $r:expr),* $(,)?) => {
        impl TryFromExpr for $type {
            fn try_from_expr(expr: Spanned<&Expr>, f: &mut Feedback) -> Option<Self> {
                #[allow(unreachable_patterns)]
                match expr.v {
                    $($p => Some($r)),*,
                    other => {
                        error!(
                            @f, expr.span,
                            "expected {}, found {}", $name, other.name()
                        );
                        None
                    }
                }
            }
        }
    };
}

macro_rules! impl_ident {
    ($type:ty, $name:expr, $parse:expr) => {
        impl TryFromExpr for $type {
            fn try_from_expr(expr: Spanned<&Expr>, f: &mut Feedback) -> Option<Self> {
                if let Expr::Ident(ident) = expr.v {
                    let val = $parse(ident.as_str());
                    if val.is_none() {
                        error!(@f, expr.span, "invalid {}", $name);
                    }
                    val
                } else {
                    error!(
                        @f, expr.span,
                        "expected {}, found {}", $name, expr.v.name()
                    );
                    None
                }
            }
        }
    };
}

impl<T: TryFromExpr> TryFromExpr for Spanned<T> {
    fn try_from_expr(expr: Spanned<&Expr>, f: &mut Feedback) -> Option<Self> {
        let span = expr.span;
        T::try_from_expr(expr, f).map(|v| Spanned { v, span })
    }
}

impl_match!(Expr, "expression", e => e.clone());
impl_match!(Ident, "identifier", Expr::Ident(i) => i.clone());
impl_match!(String, "string", Expr::Str(s) => s.clone());
impl_match!(bool, "bool", Expr::Bool(b) => b.clone());
impl_match!(f64, "number", Expr::Number(n) => n.clone());
impl_match!(Length, "length", Expr::Length(l) => l.clone());
impl_match!(SyntaxTree, "tree", Expr::Tree(t) => t.clone());
impl_match!(Tuple, "tuple", Expr::Tuple(t) => t.clone());
impl_match!(Object, "object", Expr::Object(o) => o.clone());
impl_match!(ScaleLength, "number or length",
    &Expr::Length(length) => ScaleLength::Absolute(length),
    &Expr::Number(scale) => ScaleLength::Scaled(scale),
);

/// A value type that matches identifiers and strings and implements
/// `Into<String>`.
pub struct StringLike(pub String);

impl From<StringLike> for String {
    fn from(like: StringLike) -> String {
        like.0
    }
}

impl_match!(StringLike, "identifier or string",
    Expr::Ident(Ident(s)) => StringLike(s.clone()),
    Expr::Str(s) => StringLike(s.clone()),
);

impl_ident!(Dir, "direction", |s| match s {
    "ltr" => Some(Self::LTR),
    "rtl" => Some(Self::RTL),
    "ttb" => Some(Self::TTB),
    "btt" => Some(Self::BTT),
    _ => None,
});

impl_ident!(SpecAlign, "alignment", |s| match s {
    "left" => Some(Self::Left),
    "right" => Some(Self::Right),
    "top" => Some(Self::Top),
    "bottom" => Some(Self::Bottom),
    "center" => Some(Self::Center),
    _ => None,
});

impl_ident!(FontStyle, "font style", FontStyle::from_name);
impl_ident!(Paper, "paper", Paper::from_name);

impl TryFromExpr for FontWeight {
    fn try_from_expr(expr: Spanned<&Expr>, f: &mut Feedback) -> Option<Self> {
        match expr.v {
            &Expr::Number(weight) => {
                const MIN: u16 = 100;
                const MAX: u16 = 900;

                Some(Self(if weight < MIN as f64 {
                    error!(@f, expr.span, "the minimum font weight is {}", MIN);
                    MIN
                } else if weight > MAX as f64 {
                    error!(@f, expr.span, "the maximum font weight is {}", MAX);
                    MAX
                } else {
                    weight.round() as u16
                }))
            }
            Expr::Ident(ident) => {
                let weight = Self::from_name(ident.as_str());
                if weight.is_none() {
                    error!(@f, expr.span, "invalid font weight");
                }
                weight
            }
            other => {
                error!(
                    @f, expr.span,
                    "expected font weight (name or number), found {}",
                    other.name(),
                );
                None
            }
        }
    }
}

impl TryFromExpr for FontWidth {
    fn try_from_expr(expr: Spanned<&Expr>, f: &mut Feedback) -> Option<Self> {
        match expr.v {
            &Expr::Number(width) => {
                const MIN: u16 = 1;
                const MAX: u16 = 9;

                Self::new(if width < MIN as f64 {
                    error!(@f, expr.span, "the minimum font width is {}", MIN);
                    MIN
                } else if width > MAX as f64 {
                    error!(@f, expr.span, "the maximum font width is {}", MAX);
                    MAX
                } else {
                    width.round() as u16
                })
            }
            Expr::Ident(ident) => {
                let width = Self::from_name(ident.as_str());
                if width.is_none() {
                    error!(@f, expr.span, "invalid font width");
                }
                width
            }
            other => {
                error!(
                    @f, expr.span,
                    "expected font width (name or number), found {}",
                    other.name(),
                );
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(expr: Expr) -> TableExprEntry {
        TableExprEntry {
            key: Span::ZERO,
            val: Spanned::zero(expr),
        }
    }

    #[test]
    fn test_table_take_removes_correct_entry() {
        let mut table = TableExpr::new();
        table.insert(1, entry(Expr::Bool(false)));
        table.insert(2, entry(Expr::Str("hi".to_string())));
        assert_eq!(table.take::<String>(), Some("hi".to_string()));
        assert_eq!(table.len(), 1);
        assert_eq!(table.take::<bool>(), Some(false));
        assert!(table.is_empty());
    }

    #[test]
    fn test_table_expect_errors_about_previous_entries() {
        let mut f = Feedback::new();
        let mut table = TableExpr::new();
        table.insert(1, entry(Expr::Bool(false)));
        table.insert(3, entry(Expr::Str("hi".to_string())));
        table.insert(5, entry(Expr::Bool(true)));
        assert_eq!(table.expect::<String>(&mut f), Some("hi".to_string()));
        assert_eq!(f.diagnostics, [error!(Span::ZERO, "expected string, found bool")]);
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn test_table_take_with_key_removes_the_entry() {
        let mut f = Feedback::new();
        let mut table = TableExpr::new();
        table.insert(1, entry(Expr::Bool(false)));
        table.insert("hi", entry(Expr::Bool(true)));
        assert_eq!(table.take_with_key::<bool, _>(1, &mut f), Some(false));
        assert_eq!(table.take_with_key::<f64, _>("hi", &mut f), None);
        assert_eq!(f.diagnostics, [error!(Span::ZERO, "expected number, found bool")]);
        assert!(table.is_empty());
    }

    #[test]
    fn test_table_take_all_removes_the_correct_entries() {
        let mut table = TableExpr::new();
        table.insert(1, entry(Expr::Bool(false)));
        table.insert(3, entry(Expr::Number(0.0)));
        table.insert(7, entry(Expr::Bool(true)));
        assert_eq!(
            table.take_all_num::<bool>().collect::<Vec<_>>(),
            [(1, false), (7, true)],
        );
        assert_eq!(table.len(), 1);
        assert_eq!(table[3].val.v, Expr::Number(0.0));
    }
}
