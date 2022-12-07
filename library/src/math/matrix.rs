use super::*;

/// A column vector in a mathematical formula.
#[derive(Debug, Hash)]
pub struct VecNode(Vec<Content>);

#[node(Texify)]
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
    Expected: "type of bracket or bar",
    Value::Str(s) => match s.as_str() {
        "(" => Self::Paren,
        "[" => Self::Bracket,
        "{" => Self::Brace,
        "|" => Self::Bar,
        _ => Err("expected \"(\", \"[\", \"{\", or \"|\"")?,
    },
}

/// A case distinction in a mathematical formula.
#[derive(Debug, Hash)]
pub struct CasesNode(Vec<Content>);

#[node(Texify)]
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
