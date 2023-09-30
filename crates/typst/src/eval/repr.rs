use ecow::{eco_format, EcoString};

/// A trait that defines the `repr` of a Typst value.
pub trait Repr {
    /// Return the debug representation of the value.
    fn repr(&self) -> EcoString;
}

impl Repr for bool {
    fn repr(&self) -> EcoString {
        match self {
            true => "true".into(),
            false => "false".into(),
        }
    }
}

impl Repr for i64 {
    fn repr(&self) -> EcoString {
        eco_format!("{}", self)
    }
}

impl Repr for f64 {
    fn repr(&self) -> EcoString {
        eco_format!("{}", self)
    }
}

impl Repr for &str {
    fn repr(&self) -> EcoString {
        let mut representation = EcoString::with_capacity(self.len() + 2);
        representation.push('"');
        for c in self.chars() {
            match c {
                '\0' => representation.push_str(r"\u{0}"),
                '\'' => representation.push('\''),
                '"' => representation.push_str(r#"\""#),
                _ => c.escape_debug().for_each(|c| representation.push(c)),
            }
        }
        representation.push('"');
        representation
    }
}

impl Repr for EcoString {
    fn repr(&self) -> EcoString {
        self.as_ref().repr()
    }
}
