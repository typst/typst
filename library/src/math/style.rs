use super::*;

/// Serif (roman) font style.
#[derive(Debug, Hash)]
pub struct SerifNode(Content);

#[node(Texify)]
impl SerifNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Texify for SerifNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\mathrm{");
        self.0.texify_unparen(t)?;
        t.push_str("}");
        Ok(())
    }
}

/// Sans-serif font style.
#[derive(Debug, Hash)]
pub struct SansNode(Content);

#[node(Texify)]
impl SansNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Texify for SansNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\mathsf{");
        self.0.texify_unparen(t)?;
        t.push_str("}");
        Ok(())
    }
}

/// Bold font style.
#[derive(Debug, Hash)]
pub struct BoldNode(Content);

#[node(Texify)]
impl BoldNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Texify for BoldNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\mathbf{");
        self.0.texify_unparen(t)?;
        t.push_str("}");
        Ok(())
    }
}

/// Italic font style.
#[derive(Debug, Hash)]
pub struct ItalNode(Content);

#[node(Texify)]
impl ItalNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Texify for ItalNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\mathit{");
        self.0.texify_unparen(t)?;
        t.push_str("}");
        Ok(())
    }
}

/// Calligraphic font style.
#[derive(Debug, Hash)]
pub struct CalNode(Content);

#[node(Texify)]
impl CalNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Texify for CalNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\mathcal{");
        self.0.texify_unparen(t)?;
        t.push_str("}");
        Ok(())
    }
}

/// Fraktur font style.
#[derive(Debug, Hash)]
pub struct FrakNode(Content);

#[node(Texify)]
impl FrakNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Texify for FrakNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\mathfrak{");
        self.0.texify_unparen(t)?;
        t.push_str("}");
        Ok(())
    }
}

/// Monospace font style.
#[derive(Debug, Hash)]
pub struct MonoNode(Content);

#[node(Texify)]
impl MonoNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Texify for MonoNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\mathtt{");
        self.0.texify_unparen(t)?;
        t.push_str("}");
        Ok(())
    }
}

/// Blackboard bold (double-struck) font style.
#[derive(Debug, Hash)]
pub struct BbNode(Content);

#[node(Texify)]
impl BbNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Texify for BbNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\mathbb{");
        self.0.texify_unparen(t)?;
        t.push_str("}");
        Ok(())
    }
}
