use super::TextNode;
use crate::prelude::*;

/// A text space.
///
/// Tags: text.
#[func]
#[capable(Unlabellable, Behave)]
#[derive(Debug, Hash)]
pub struct SpaceNode;

#[node]
impl SpaceNode {
    fn construct(_: &Vm, _: &mut Args) -> SourceResult<Content> {
        Ok(Self.pack())
    }
}

impl Unlabellable for SpaceNode {}

impl Behave for SpaceNode {
    fn behaviour(&self) -> Behaviour {
        Behaviour::Weak(2)
    }
}

/// A line break.
///
/// Tags: text.
#[func]
#[capable(Behave)]
#[derive(Debug, Hash)]
pub struct LinebreakNode {
    pub justify: bool,
}

#[node]
impl LinebreakNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let justify = args.named("justify")?.unwrap_or(false);
        Ok(Self { justify }.pack())
    }
}

impl Behave for LinebreakNode {
    fn behaviour(&self) -> Behaviour {
        Behaviour::Destructive
    }
}

/// Strongly emphasizes content by increasing the font weight.
///
/// Tags: text.
#[func]
#[capable(Show)]
#[derive(Debug, Hash)]
pub struct StrongNode(pub Content);

#[node]
impl StrongNode {
    /// The delta to apply on the font weight.
    pub const DELTA: i64 = 300;

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "body" => Some(Value::Content(self.0.clone())),
            _ => None,
        }
    }
}

impl Show for StrongNode {
    fn show(&self, _: &mut Vt, _: &Content, styles: StyleChain) -> SourceResult<Content> {
        Ok(self.0.clone().styled(TextNode::DELTA, Delta(styles.get(Self::DELTA))))
    }
}

/// A delta that is summed up when folded.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Delta(pub i64);

castable! {
    Delta,
    v: i64 => Self(v),
}

impl Fold for Delta {
    type Output = i64;

    fn fold(self, outer: Self::Output) -> Self::Output {
        outer + self.0
    }
}

/// Emphasizes content by flipping the italicness.
///
/// Tags: text.
#[func]
#[capable(Show)]
#[derive(Debug, Hash)]
pub struct EmphNode(pub Content);

#[node]
impl EmphNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "body" => Some(Value::Content(self.0.clone())),
            _ => None,
        }
    }
}

impl Show for EmphNode {
    fn show(&self, _: &mut Vt, _: &Content, _: StyleChain) -> SourceResult<Content> {
        Ok(self.0.clone().styled(TextNode::EMPH, Toggle))
    }
}

/// A toggle that turns on and off alternatingly if folded.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Toggle;

impl Fold for Toggle {
    type Output = bool;

    fn fold(self, outer: Self::Output) -> Self::Output {
        !outer
    }
}

/// Convert a string or content to lowercase.
///
/// Tags: text.
#[func]
pub fn lower(args: &mut Args) -> SourceResult<Value> {
    case(Case::Lower, args)
}

/// Convert a string or content to uppercase.
///
/// Tags: text.
#[func]
pub fn upper(args: &mut Args) -> SourceResult<Value> {
    case(Case::Upper, args)
}

/// Change the case of text.
fn case(case: Case, args: &mut Args) -> SourceResult<Value> {
    let Spanned { v, span } = args.expect("string or content")?;
    Ok(match v {
        Value::Str(v) => Value::Str(case.apply(&v).into()),
        Value::Content(v) => Value::Content(v.styled(TextNode::CASE, Some(case))),
        v => bail!(span, "expected string or content, found {}", v.type_name()),
    })
}

/// A case transformation on text.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Case {
    /// Everything is lowercased.
    Lower,
    /// Everything is uppercased.
    Upper,
}

impl Case {
    /// Apply the case to a string.
    pub fn apply(self, text: &str) -> String {
        match self {
            Self::Lower => text.to_lowercase(),
            Self::Upper => text.to_uppercase(),
        }
    }
}

/// Display text in small capitals.
///
/// Tags: text.
#[func]
pub fn smallcaps(args: &mut Args) -> SourceResult<Value> {
    let body: Content = args.expect("content")?;
    Ok(Value::Content(body.styled(TextNode::SMALLCAPS, true)))
}
