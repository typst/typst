use crate::size::ScaleSize;
use super::*;


/// An argument or return value.
#[derive(Clone, PartialEq)]
pub enum Expr {
    Ident(Ident),
    Str(String),
    Number(f64),
    Size(Size),
    Bool(bool),
    Tuple(Tuple),
    Object(Object),
}

impl Expr {
    pub fn name(&self) -> &'static str {
        use Expr::*;
        match self {
            Ident(_) => "identifier",
            Str(_) => "string",
            Number(_) => "number",
            Size(_) => "size",
            Bool(_) => "boolean",
            Tuple(_) => "tuple",
            Object(_) => "object",
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

debug_display!(Expr);

/// An identifier.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Ident(pub String);

impl Ident {
    pub fn new<S>(ident: S) -> Option<Ident> where S: AsRef<str> + Into<String> {
        if is_identifier(ident.as_ref()) {
            Some(Ident(ident.into()))
        } else {
            None
        }
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Display for Ident {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

debug_display!(Ident);

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StringLike(pub String);

/// A sequence of expressions.
#[derive(Clone, PartialEq)]
pub struct Tuple {
    pub items: Vec<Spanned<Expr>>,
}

impl Tuple {
    pub fn new() -> Tuple {
        Tuple { items: vec![] }
    }

    pub fn add(&mut self, item: Spanned<Expr>) {
        self.items.push(item);
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

debug_display!(Tuple);

/// A key-value collection of identifiers and associated expressions.
#[derive(Clone, PartialEq)]
pub struct Object {
    pub pairs: Vec<Pair>,
}

impl Object {
    pub fn new() -> Object {
        Object { pairs: vec![] }
    }

    pub fn add(&mut self, key: Spanned<Ident>, value: Spanned<Expr>) {
        self.pairs.push(Pair { key, value });
    }

    pub fn add_pair(&mut self, pair: Pair) {
        self.pairs.push(pair);
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

debug_display!(Object);

/// A key-value pair in an object.
#[derive(Clone, PartialEq)]
pub struct Pair {
    pub key: Spanned<Ident>,
    pub value: Spanned<Expr>,
}

impl Display for Pair {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.key.v, self.value.v)
    }
}

debug_display!(Pair);

pub trait ExprKind: Sized {
    /// The name of the expression in an `expected <name>` error.
    const NAME: &'static str;

    /// Create from expression.
    fn from_expr(expr: Spanned<Expr>) -> Result<Self, Error>;
}

impl<T> ExprKind for Spanned<T> where T: ExprKind {
    const NAME: &'static str = T::NAME;

    fn from_expr(expr: Spanned<Expr>) -> Result<Self, Error> {
        let span = expr.span;
        T::from_expr(expr).map(|v| Spanned { v, span })
    }
}
/// Implements the expression kind trait for a type.
macro_rules! kind {
    ($type:ty, $name:expr, $($p:pat => $r:expr),* $(,)?) => {
        impl ExprKind for $type {
            const NAME: &'static str = $name;

            fn from_expr(expr: Spanned<Expr>) -> Result<Self, Error> {
                #[allow(unreachable_patterns)]
                Ok(match expr.v {
                    $($p => $r),*,
                    _ => return Err(
                        err!("expected {}, found {}", Self::NAME, expr.v.name())
                    ),
                })
            }
        }
    };
}

kind!(Expr, "expression", e => e);
kind!(Ident, "identifier", Expr::Ident(i) => i);
kind!(String, "string", Expr::Str(s) => s);
kind!(f64, "number", Expr::Number(n) => n);
kind!(bool, "boolean", Expr::Bool(b) => b);
kind!(Size, "size", Expr::Size(s) => s);
kind!(Tuple, "tuple", Expr::Tuple(t) => t);
kind!(Object, "object", Expr::Object(o) => o);
kind!(ScaleSize, "number or size",
    Expr::Size(size)    => ScaleSize::Absolute(size),
    Expr::Number(scale) => ScaleSize::Scaled(scale as f32),
);
kind!(StringLike, "identifier or string",
    Expr::Ident(Ident(s)) => StringLike(s),
    Expr::Str(s) => StringLike(s),
);
