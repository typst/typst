use super::*;

/// Serif (roman) font style.
///
/// Tags: math.
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct SerifNode(pub Content);

#[node]
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
///
/// Tags: math.
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct SansNode(pub Content);

#[node]
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
///
/// Tags: math.
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct BoldNode(pub Content);

#[node]
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
///
/// Tags: math.
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct ItalNode(pub Content);

#[node]
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
///
/// Tags: math.
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct CalNode(pub Content);

#[node]
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
///
/// Tags: math.
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct FrakNode(pub Content);

#[node]
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
///
/// Tags: math.
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct MonoNode(pub Content);

#[node]
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
///
/// Tags: math.
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct BbNode(pub Content);

#[node]
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
