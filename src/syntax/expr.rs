use super::*;


/// An argument or return value.
#[derive(Clone, PartialEq)]
pub enum Expression {
    Ident(Ident),
    Str(String),
    Number(f64),
    Size(Size),
    Bool(bool),
    Tuple(Tuple),
    Object(Object),
}

impl Display for Expression {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use Expression::*;
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

/// A sequence of expressions.
#[derive(Clone, PartialEq)]
pub struct Tuple {
    pub items: Vec<Spanned<Expression>>,
}

impl Tuple {
    pub fn new() -> Tuple {
        Tuple { items: vec![] }
    }

    pub fn add(&mut self, item: Spanned<Expression>) {
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

/// A key-value collection of identifiers and associated expressions.
#[derive(Clone, PartialEq)]
pub struct Object {
    pub pairs: Vec<Pair>,
}

impl Object {
    pub fn new() -> Object {
        Object { pairs: vec![] }
    }

    pub fn add(&mut self, key: Spanned<Ident>, value: Spanned<Expression>) {
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

#[derive(Clone, PartialEq)]
pub struct Pair {
    pub key: Spanned<Ident>,
    pub value: Spanned<Expression>,
}

impl Display for Pair {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.key.v, self.value.v)
    }
}

debug_display!(Ident);
debug_display!(Expression);
debug_display!(Tuple);
debug_display!(Object);
debug_display!(Pair);


/// Kinds of expressions.
pub trait ExpressionKind: Sized {
    /// The name of the expression in an `expected <name>` error.
    const NAME: &'static str;

    /// Create from expression.
    fn from_expr(expr: Spanned<Expression>) -> ParseResult<Self>;
}

/// Implements the expression kind trait for a type.
macro_rules! kind {
    ($type:ty, $name:expr, $($patterns:tt)*) => {
        impl ExpressionKind for $type {
            const NAME: &'static str = $name;

            fn from_expr(expr: Spanned<Expression>) -> ParseResult<Self> {
                #[allow(unreachable_patterns)]
                Ok(match expr.v {
                    $($patterns)*,
                    _ => error!("expected {}", Self::NAME),
                })
            }
        }
    };
}

kind!(Expression, "expression", e                          => e);
kind!(Ident,      "identifier", Expression::Ident(ident)   => ident);
kind!(String,     "string",     Expression::Str(string)    => string);
kind!(f64,        "number",     Expression::Number(num)    => num);
kind!(bool,       "boolean",    Expression::Bool(boolean)  => boolean);
kind!(Size,       "size",       Expression::Size(size)     => size);
kind!(Tuple,      "tuple",      Expression::Tuple(tuple)   => tuple);
kind!(Object,     "object",     Expression::Object(object) => object);

kind!(ScaleSize,  "number or size",
    Expression::Size(size)    => ScaleSize::Absolute(size),
    Expression::Number(scale) => ScaleSize::Scaled(scale as f32)
);

impl<T> ExpressionKind for Spanned<T> where T: ExpressionKind {
    const NAME: &'static str = T::NAME;

    fn from_expr(expr: Spanned<Expression>) -> ParseResult<Spanned<T>> {
        let span = expr.span;
        T::from_expr(expr).map(|v| Spanned { v, span })
    }
}
