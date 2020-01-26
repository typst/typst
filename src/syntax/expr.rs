//! Expressions in function headers.

use std::fmt::{self, Display, Formatter};

use crate::error::Errors;
use crate::size::Size;
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
    /// A size: `2cm, 5.2in`.
    Size(Size),
    /// A bool: `true, false`.
    Bool(bool),
    /// A tuple: `(false, 12cm, "hi")`.
    Tuple(Tuple),
    /// An object: `{ fit: false, size: 12pt }`.
    Object(Object),
}

impl Expr {
    /// A natural-language name of the type of this expression, e.g. "identifier".
    pub fn name(&self) -> &'static str {
        use Expr::*;
        match self {
            Ident(_) => "identifier",
            Str(_) => "string",
            Number(_) => "number",
            Size(_) => "size",
            Bool(_) => "bool",
            Tuple(_) => "tuple",
            Object(_) => "object",
        }
    }
}

/// A unicode identifier.
///
/// The identifier must be valid! This is checked in [`Ident::new`] or
/// [`is_identifier`].
///
/// # Example
/// ```typst
/// [func: "hi", ident]
///  ^^^^        ^^^^^
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Ident(pub String);

impl Ident {
    /// Create a new identifier from a string checking that it is valid.
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

/// An untyped sequence of expressions.
///
/// # Example
/// ```typst
/// (false, 12cm, "hi")
/// ```
#[derive(Clone, PartialEq)]
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
    /// errors for all previous items that did not match.
    pub fn get<V: Value>(&mut self, errors: &mut Errors) -> Option<V::Output> {
        while !self.items.is_empty() {
            let expr = self.items.remove(0);
            let span = expr.span;
            match V::parse(expr) {
                Ok(output) => return Some(output),
                Err(err) => errors.push(Spanned { v: err, span }),
            }
        }
        None
    }

    /// Extract and return an iterator over all values that match and generate
    /// errors for all items that do not match.
    pub fn get_all<'a, V: Value>(&'a mut self, errors: &'a mut Errors)
    -> impl Iterator<Item=V::Output> + 'a {
        self.items.drain(..).filter_map(move |expr| {
            let span = expr.span;
            match V::parse(expr) {
                Ok(output) => Some(output),
                Err(err) => { errors.push(Spanned { v: err, span }); None }
            }
        })
    }
}

/// A key-value collection of identifiers and associated expressions.
///
/// The pairs themselves are not spanned, but the combined spans can easily be
/// retrieved by merging the spans of key and value as happening in
/// [`FuncArg::span`](super::func::FuncArg::span).
///
/// # Example
/// ```typst
/// { fit: false, size: 12cm, items: (1, 2, 3) }
/// ```
#[derive(Clone, PartialEq)]
pub struct Object {
    /// The key-value pairs of the object.
    pub pairs: Vec<Pair>,
}

/// A key-value pair in an object.
#[derive(Clone, PartialEq)]
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
    pub fn add(&mut self, pair: Pair) {
        self.pairs.push(pair);
    }

    /// Extract (and remove) a pair with the given key string and matching
    /// value.
    ///
    /// Inserts an error if the value does not match. If the key is not
    /// contained, no error is inserted.
    pub fn get<V: Value>(&mut self, errors: &mut Errors, key: &str) -> Option<V::Output> {
        let index = self.pairs.iter().position(|pair| pair.key.v.as_str() == key)?;
        self.get_index::<V>(errors, index)
    }

    /// Extract (and remove) a pair with a matching key and value.
    ///
    /// Inserts an error if the value does not match. If no matching key is
    /// found, no error is inserted.
    pub fn get_with_key<K: Key, V: Value>(
        &mut self,
        errors: &mut Errors,
    ) -> Option<(K::Output, V::Output)> {
        for (index, pair) in self.pairs.iter().enumerate() {
            let key = Spanned { v: pair.key.v.as_str(), span: pair.key.span };
            if let Some(key) = K::parse(key) {
                return self.get_index::<V>(errors, index).map(|value| (key, value));
            }
        }
        None
    }

    /// Extract (and remove) all pairs with matching keys and values.
    ///
    /// Inserts errors for values that do not match.
    pub fn get_all<'a, K: Key, V: Value>(
        &'a mut self,
        errors: &'a mut Errors,
    ) -> impl Iterator<Item=(K::Output, V::Output)> + 'a {
        let mut index = 0;
        std::iter::from_fn(move || {
            if index < self.pairs.len() {
                let key = &self.pairs[index].key;
                let key = Spanned { v: key.v.as_str(), span: key.span };

                Some(if let Some(key) = K::parse(key) {
                    self.get_index::<V>(errors, index).map(|v| (key, v))
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
        errors: &'a mut Errors,
    ) -> impl Iterator<Item=Spanned<(K::Output, V::Output)>> + 'a {
        self.get_all::<Spanned<K>, Spanned<V>>(errors)
            .map(|(k, v)| Spanned::new((k.v, v.v), Span::merge(k.span, v.span)))
    }

    /// Extract the argument at the given index and insert an error if the value
    /// does not match.
    fn get_index<V: Value>(&mut self, errors: &mut Errors, index: usize) -> Option<V::Output> {
        let expr = self.pairs.remove(index).value;
        let span = expr.span;
        match V::parse(expr) {
            Ok(output) => Some(output),
            Err(err) => { errors.push(Spanned { v: err, span }); None }
        }
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use Expr::*;
        match self {
            Ident(i) => write!(f, "{}", i),
            Str(s) => write!(f, "{:?}", s),
            Number(n) => write!(f, "{}", n),
            Size(s) => write!(f, "{}", s),
            Bool(b) => write!(f, "{}", b),
            Tuple(t) => write!(f, "{}", t),
            Object(o) => write!(f, "{}", o),
        }
    }
}

impl Display for Ident {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for Tuple {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "(")?;

        let mut first = true;
        for item in &self.items {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", item.v)?;
            first = false;
        }

        write!(f, ")")
    }
}

impl Display for Object {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.pairs.len() == 0 {
            return write!(f, "{{}}");
        }

        write!(f, "{{ ")?;

        let mut first = true;
        for pair in &self.pairs {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", pair)?;
            first = false;
        }

        write!(f, " }}")
    }
}

impl Display for Pair {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.key.v, self.value.v)
    }
}

debug_display!(Expr);
debug_display!(Ident);
debug_display!(Tuple);
debug_display!(Object);
debug_display!(Pair);
