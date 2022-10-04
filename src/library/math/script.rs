use std::fmt::Write;

use super::*;
use crate::library::prelude::*;

/// A sub- and/or superscript in a mathematical formula.
#[derive(Debug, Hash)]
pub struct ScriptNode {
    /// The base.
    pub base: MathNode,
    /// The subscript.
    pub sub: Option<MathNode>,
    /// The superscript.
    pub sup: Option<MathNode>,
}

impl Texify for ScriptNode {
    fn texify(&self) -> EcoString {
        let mut tex = self.base.texify();

        if let Some(sub) = &self.sub {
            write!(tex, "_{{{}}}", sub.texify()).unwrap();
        }

        if let Some(sup) = &self.sup {
            write!(tex, "^{{{}}}", sup.texify()).unwrap();
        }

        tex
    }
}
