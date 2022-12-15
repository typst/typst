use super::*;

/// A column vector.
///
/// Tags: math.
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct VecNode(Vec<Content>);

#[node]
impl VecNode {
    /// The kind of delimiter.
    pub const DELIM: Delimiter = Delimiter::Paren;

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.all()?).pack())
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
    /// Delimit matrices with parentheses.
    "(" => Self::Paren,
    /// Delimit matrices with brackets.
    "[" => Self::Bracket,
    /// Delimit matrices with curly braces.
    "{" => Self::Brace,
    /// Delimit matrices with vertical bars.
    "|" => Self::Bar,
}

/// A case distinction.
///
/// Tags: math.
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct CasesNode(Vec<Content>);

#[node]
impl CasesNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.all()?).pack())
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
