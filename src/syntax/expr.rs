//! Expressions in function headers.

use std::fmt::{self, Debug, Formatter};
use std::iter::FromIterator;
use std::ops::Deref;
use std::str::FromStr;
use std::u8;

use crate::diagnostic::Diagnostics;
use crate::length::Length;
use super::func::{Key, Value};
use super::span::{Span, Spanned};
use super::tokens::is_identifier;

/// An argument or return value.
#[derive(Clone, PartialEq)]
pub enum Expr {
    /// An identifier: `ident`.
    Ident(Ident),
    /// A string: `"string"`.
    Str(String),
    /// A number: `1.2, 200%`.
    Number(f64),
    /// A length: `2cm, 5.2in`.
    Length(Length),
    /// A bool: `true, false`.
    Bool(bool),
    /// A color value, including the alpha channel: `#f79143ff`.
    Color(RgbaColor),
    /// A tuple: `(false, 12cm, "hi")`.
    Tuple(Tuple),
    /// A named tuple: `cmyk(37.7, 0, 3.9, 1.1)`.
    NamedTuple(NamedTuple),
    /// An object: `{ fit: false, width: 12pt }`.
    Object(Object),
    /// An operator that negates the contained expression.
    Neg(Box<Spanned<Expr>>),
    /// An operator that adds the contained expressions.
    Add(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    /// An operator that subtracts contained expressions.
    Sub(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    /// An operator that multiplies the contained expressions.
    Mul(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    /// An operator that divides the contained expressions.
    Div(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
}

impl Expr {
    /// A natural-language name of the type of this expression, e.g. "identifier".
    pub fn name(&self) -> &'static str {
        use Expr::*;
        match self {
            Ident(_) => "identifier",
            Str(_) => "string",
            Number(_) => "number",
            Length(_) => "length",
            Bool(_) => "bool",
            Color(_) => "color",
            Tuple(_) => "tuple",
            NamedTuple(_) => "named tuple",
            Object(_) => "object",
            Neg(_) => "negation",
            Add(_, _) => "addition",
            Sub(_, _) => "subtraction",
            Mul(_, _) => "multiplication",
            Div(_, _) => "division",
        }
    }
}

impl Debug for Expr {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use Expr::*;
        match self {
            Ident(i) => i.fmt(f),
            Str(s) => s.fmt(f),
            Number(n) => n.fmt(f),
            Length(s) => s.fmt(f),
            Bool(b) => b.fmt(f),
            Color(c) => c.fmt(f),
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

/// A unicode identifier.
///
/// # Example
/// ```typst
/// [func: "hi", ident]
///  ^^^^        ^^^^^
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Ident(pub String);

impl Ident {
    /// Create a new identifier from a string checking that it is a valid
    /// unicode identifier.
    pub fn new<S>(ident: S) -> Option<Ident> where S: AsRef<str> + Into<String> {
        if is_identifier(ident.as_ref()) {
            Some(Ident(ident.into()))
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
    /// Indicates whether this is a user-provided value or a
    /// default value provided as a fail-over by the parser.
    /// This color may be overwritten if this property is true.
    pub healed: bool,
}

impl RgbaColor {
    /// Constructs a new color.
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> RgbaColor {
        RgbaColor { r, g, b, a, healed: false }
    }

    /// Constructs a new color with the healed property set to true.
    pub fn new_healed(r: u8, g: u8, b: u8, a: u8) -> RgbaColor {
        RgbaColor { r, g, b, a, healed: true }
    }
}

impl FromStr for RgbaColor {
    type Err = ParseColorError;

    /// Constructs a new color from a hex string like `7a03c2`.
    /// Do not specify a leading `#`.
    fn from_str(hex_str: &str) -> Result<RgbaColor, Self::Err> {
        if !hex_str.is_ascii() {
            return Err(ParseColorError);
        }

        let len = hex_str.len();
        let long =  len == 6 || len == 8;
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

        Ok(RgbaColor::new(values[0], values[1], values[2], values[3]))
    }
}

impl Debug for RgbaColor {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if f.alternate() {
            write!(
                f,
                "rgba({:02}, {:02}, {:02}, {:02})",
                self.r, self.g, self.b, self.a,
            )?;
        } else {
            write!(
                f,
                "#{:02x}{:02x}{:02x}{:02x}",
                self.r, self.g, self.b, self.a,
            )?;
        }
        if self.healed {
            f.write_str(" [healed]")?;
        }
        Ok(())
    }
}

/// The error returned when parsing a [`RgbaColor`] from a string fails.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ParseColorError;

impl std::error::Error for ParseColorError {}

impl fmt::Display for ParseColorError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("invalid color")
    }
}

/// An untyped sequence of expressions.
///
/// # Example
/// ```typst
/// (false, 12cm, "hi")
/// ```
#[derive(Default, Clone, PartialEq)]
pub struct Tuple {
    /// The elements of the tuple.
    pub items: Vec<Spanned<Expr>>,
}

impl Tuple {
    /// Create an empty tuple.
    pub fn new() -> Tuple {
        Tuple { items: vec![] }
    }

    /// Add an element.
    pub fn add(&mut self, item: Spanned<Expr>) {
        self.items.push(item);
    }

    /// Extract (and remove) the first matching value and remove and generate
    /// diagnostics for all previous items that did not match.
    pub fn get<V: Value>(&mut self, diagnostics: &mut Diagnostics) -> Option<V> {
        while !self.items.is_empty() {
            let expr = self.items.remove(0);
            let span = expr.span;
            match V::parse(expr) {
                Ok(output) => return Some(output),
                Err(v) => diagnostics.push(Spanned { v, span }),
            }
        }
        None
    }

    /// Extract and return an iterator over all values that match and generate
    /// diagnostics for all items that do not match.
    pub fn get_all<'a, V: Value>(&'a mut self, diagnostics: &'a mut Diagnostics)
    -> impl Iterator<Item=V> + 'a {
        self.items.drain(..).filter_map(move |expr| {
            let span = expr.span;
            match V::parse(expr) {
                Ok(output) => Some(output),
                Err(v) => { diagnostics.push(Spanned { v, span }); None }
            }
        })
    }

    /// Iterate over the items of this tuple.
    pub fn iter<'a>(&'a self) -> std::slice::Iter<'a, Spanned<Expr>> {
        self.items.iter()
    }
}

impl IntoIterator for Tuple {
    type Item = Spanned<Expr>;
    type IntoIter = std::vec::IntoIter<Spanned<Expr>>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a> IntoIterator for &'a Tuple {
    type Item = &'a Spanned<Expr>;
    type IntoIter = std::slice::Iter<'a, Spanned<Expr>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl FromIterator<Spanned<Expr>> for Tuple {
    fn from_iter<I: IntoIterator<Item=Spanned<Expr>>>(iter: I) -> Self {
        Tuple { items: iter.into_iter().collect() }
    }
}

impl Debug for Tuple {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_list()
            .entries(&self.items)
            .finish()
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
    /// The name of the tuple and where it is in the user source.
    pub name: Spanned<Ident>,
    /// The elements of the tuple.
    pub tuple: Spanned<Tuple>,
}

impl NamedTuple {
    /// Create a named tuple from a tuple.
    pub fn new(name: Spanned<Ident>, tuple: Spanned<Tuple>) -> NamedTuple {
        NamedTuple { name, tuple }
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
/// { fit: false, width: 12cm, items: (1, 2, 3) }
/// ```
#[derive(Default, Clone, PartialEq)]
pub struct Object {
    /// The key-value pairs of the object.
    pub pairs: Vec<Spanned<Pair>>,
}

/// A key-value pair in an object.
#[derive(Debug, Clone, PartialEq)]
pub struct Pair {
    /// The key part.
    /// ```typst
    /// key: value
    /// ^^^
    /// ```
    pub key: Spanned<Ident>,
    /// The value part.
    /// ```typst
    /// key: value
    ///      ^^^^^
    /// ```
    pub value: Spanned<Expr>,
}

impl Object {
    /// Create an empty object.
    pub fn new() -> Object {
        Object { pairs: vec![] }
    }

    /// Add a pair to object.
    pub fn add(&mut self, pair: Spanned<Pair>) {
        self.pairs.push(pair);
    }

    /// Extract (and remove) a pair with the given key string and matching
    /// value.
    ///
    /// Inserts an error if the value does not match. If the key is not
    /// contained, no error is inserted.
    pub fn get<V: Value>(&mut self, diagnostics: &mut Diagnostics, key: &str) -> Option<V> {
        let index = self.pairs.iter().position(|pair| pair.v.key.v.as_str() == key)?;
        self.get_index::<V>(diagnostics, index)
    }

    /// Extract (and remove) a pair with a matching key and value.
    ///
    /// Inserts an error if the value does not match. If no matching key is
    /// found, no error is inserted.
    pub fn get_with_key<K: Key, V: Value>(
        &mut self,
        diagnostics: &mut Diagnostics,
    ) -> Option<(K, V)> {
        for (index, pair) in self.pairs.iter().enumerate() {
            let key = Spanned { v: pair.v.key.v.as_str(), span: pair.v.key.span };
            if let Some(key) = K::parse(key) {
                return self.get_index::<V>(diagnostics, index).map(|value| (key, value));
            }
        }
        None
    }

    /// Extract (and remove) all pairs with matching keys and values.
    ///
    /// Inserts errors for values that do not match.
    pub fn get_all<'a, K: Key, V: Value>(
        &'a mut self,
        diagnostics: &'a mut Diagnostics,
    ) -> impl Iterator<Item=(K, V)> + 'a {
        let mut index = 0;
        std::iter::from_fn(move || {
            if index < self.pairs.len() {
                let key = &self.pairs[index].v.key;
                let key = Spanned { v: key.v.as_str(), span: key.span };

                Some(if let Some(key) = K::parse(key) {
                    self.get_index::<V>(diagnostics, index).map(|v| (key, v))
                } else {
                    index += 1;
                    None
                })
            } else {
                None
            }
        }).filter_map(|x| x)
    }

    /// Extract all key value pairs with span information.
    ///
    /// The spans are over both key and value, like so:
    /// ```typst
    /// { key: value }
    ///   ^^^^^^^^^^
    /// ```
    pub fn get_all_spanned<'a, K: Key + 'a, V: Value + 'a>(
        &'a mut self,
        diagnostics: &'a mut Diagnostics,
    ) -> impl Iterator<Item=Spanned<(K, V)>> + 'a {
        self.get_all::<Spanned<K>, Spanned<V>>(diagnostics)
            .map(|(k, v)| Spanned::new((k.v, v.v), Span::merge(k.span, v.span)))
    }

    /// Extract the argument at the given index and insert an error if the value
    /// does not match.
    fn get_index<V: Value>(&mut self, diagnostics: &mut Diagnostics, index: usize) -> Option<V> {
        let expr = self.pairs.remove(index).v.value;
        let span = expr.span;
        match V::parse(expr) {
            Ok(output) => Some(output),
            Err(v) => { diagnostics.push(Spanned { v, span }); None }
        }
    }

    /// Iterate over the pairs of this object.
    pub fn iter<'a>(&'a self) -> std::slice::Iter<'a, Spanned<Pair>> {
        self.pairs.iter()
    }
}

impl IntoIterator for Object {
    type Item = Spanned<Pair>;
    type IntoIter = std::vec::IntoIter<Spanned<Pair>>;

    fn into_iter(self) -> Self::IntoIter {
        self.pairs.into_iter()
    }
}

impl<'a> IntoIterator for &'a Object {
    type Item = &'a Spanned<Pair>;
    type IntoIter = std::slice::Iter<'a, Spanned<Pair>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl FromIterator<Spanned<Pair>> for Object {
    fn from_iter<I: IntoIterator<Item=Spanned<Pair>>>(iter: I) -> Self {
        Object { pairs: iter.into_iter().collect() }
    }
}

impl Debug for Object {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_map()
            .entries(self.pairs.iter().map(|p| (&p.v.key.v, &p.v.value.v)))
            .finish()
    }
}
