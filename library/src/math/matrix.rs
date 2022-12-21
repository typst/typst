use super::*;

/// # Vector
/// A column vector.
///
/// _Note:_ Matrices are not yet supported.
///
/// ## Example
/// ```
/// $ vec(a, b, c) dot vec(1, 2, 3)
///     = a + 2b + 3c $
/// ```
///
/// ## Parameters
/// - elements: Content (positional, variadic)
///   The elements of the vector.
///
/// ## Category
/// math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct VecNode(Vec<Content>);

#[node]
impl VecNode {
    /// The delimiter to use.
    ///
    /// # Example
    /// ```
    /// #set vec(delim: "[")
    /// $ vec(1, 2) $
    /// ```
    pub const DELIM: Delimiter = Delimiter::Paren;

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.all()?).pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "elements" => {
                Some(Value::Array(self.0.iter().cloned().map(Value::Content).collect()))
            }
            _ => None,
        }
    }
}

impl Texify for VecNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        let kind = match t.styles.get(Self::DELIM) {
            Delimiter::Paren => "pmatrix",
            Delimiter::Bracket => "bmatrix",
            Delimiter::Brace => "Bmatrix",
            Delimiter::Bar => "vmatrix",
        };

        t.push_str("\\begin{");
        t.push_str(kind);
        t.push_str("}");

        for component in &self.0 {
            component.texify(t)?;
            t.push_str("\\\\");
        }
        t.push_str("\\end{");
        t.push_str(kind);
        t.push_str("}");

        Ok(())
    }
}

/// A vector / matrix delimiter.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Delimiter {
    Paren,
    Bracket,
    Brace,
    Bar,
}

castable! {
    Delimiter,
    /// Delimit the vector with parentheses.
    "(" => Self::Paren,
    /// Delimit the vector with brackets.
    "[" => Self::Bracket,
    /// Delimit the vector with curly braces.
    "{" => Self::Brace,
    /// Delimit the vector with vertical bars.
    "|" => Self::Bar,
}

/// # Cases
/// A case distinction.
///
/// ## Example
/// ```
/// $ f(x, y) := cases(
///   1 "if" (x dot y)/2 <= 0,
///   2 "if" x in NN,
///   3 "if" x "is even",
///   4 "else",
/// ) $
/// ```
///
/// ## Parameters
/// - branches: Content (positional, variadic)
///   The branches of the case distinction.
///
/// ## Category
/// math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct CasesNode(Vec<Content>);

#[node]
impl CasesNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.all()?).pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "branches" => {
                Some(Value::Array(self.0.iter().cloned().map(Value::Content).collect()))
            }
            _ => None,
        }
    }
}

impl Texify for CasesNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\begin{cases}");
        for component in &self.0 {
            component.texify(t)?;
            t.push_str("\\\\");
        }
        t.push_str("\\end{cases}");
        Ok(())
    }
}
