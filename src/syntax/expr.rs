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
            Bool(_) => "bool",
            Tuple(_) => "tuple",
            Object(_) => "object",
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

    pub fn get<V: Value>(&mut self, errors: &mut Errors, key: &str) -> Option<V::Output> {
        let index = self.pairs.iter().position(|pair| pair.key.v.as_str() == key)?;
        self.get_index::<V>(errors, index)
    }

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

    pub fn get_all_spanned<'a, K: Key + 'a, V: Value + 'a>(
        &'a mut self,
        errors: &'a mut Errors,
    ) -> impl Iterator<Item=Spanned<(K::Output, V::Output)>> + 'a {
        self.get_all::<Spanned<K>, Spanned<V>>(errors)
            .map(|(k, v)| Spanned::new((k.v, v.v), Span::merge(k.span, v.span)))
    }

    fn get_index<V: Value>(&mut self, errors: &mut Errors, index: usize) -> Option<V::Output> {
        let expr = self.pairs.remove(index).value;
        let span = expr.span;
        match V::parse(expr) {
            Ok(output) => Some(output),
            Err(err) => { errors.push(Spanned { v: err, span }); None }
        }
    }
}

/// A key-value pair in an object.
#[derive(Clone, PartialEq)]
pub struct Pair {
    pub key: Spanned<Ident>,
    pub value: Spanned<Expr>,
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
