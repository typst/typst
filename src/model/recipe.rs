use std::fmt::{self, Debug, Formatter};

use super::{Content, Interruption, NodeId, Show, ShowNode, StyleChain, StyleEntry};
use crate::diag::TypResult;
use crate::eval::{Args, Func, Regex, Value};
use crate::library::structure::{EnumNode, ListNode};
use crate::syntax::Spanned;
use crate::World;

/// A show rule recipe.
#[derive(Clone, PartialEq, Hash)]
pub struct Recipe {
    /// The patterns to customize.
    pub pattern: Pattern,
    /// The function that defines the recipe.
    pub func: Spanned<Func>,
}

impl Recipe {
    /// Whether the recipe is applicable to the target.
    pub fn applicable(&self, target: Target) -> bool {
        match (&self.pattern, target) {
            (Pattern::Node(id), Target::Node(node)) => *id == node.id(),
            (Pattern::Regex(_), Target::Text(_)) => true,
            _ => false,
        }
    }

    /// Try to apply the recipe to the target.
    pub fn apply(
        &self,
        world: &dyn World,
        styles: StyleChain,
        sel: Selector,
        target: Target,
    ) -> TypResult<Option<Content>> {
        let content = match (target, &self.pattern) {
            (Target::Node(node), &Pattern::Node(id)) if node.id() == id => {
                let node = node.unguard(sel);
                self.call(world, || {
                    let dict = node.encode(styles);
                    Value::Content(Content::Show(node, Some(dict)))
                })?
            }

            (Target::Text(text), Pattern::Regex(regex)) => {
                let mut result = vec![];
                let mut cursor = 0;

                for mat in regex.find_iter(text) {
                    let start = mat.start();
                    if cursor < start {
                        result.push(Content::Text(text[cursor .. start].into()));
                    }

                    result.push(self.call(world, || Value::Str(mat.as_str().into()))?);
                    cursor = mat.end();
                }

                if result.is_empty() {
                    return Ok(None);
                }

                if cursor < text.len() {
                    result.push(Content::Text(text[cursor ..].into()));
                }

                Content::sequence(result)
            }

            _ => return Ok(None),
        };

        Ok(Some(content.styled_with_entry(StyleEntry::Guard(sel))))
    }

    /// Call the recipe function, with the argument if desired.
    fn call<F>(&self, world: &dyn World, arg: F) -> TypResult<Content>
    where
        F: FnOnce() -> Value,
    {
        let args = if self.func.v.argc() == Some(0) {
            Args::new(self.func.span, [])
        } else {
            Args::new(self.func.span, [arg()])
        };

        Ok(self.func.v.call_detached(world, args)?.display())
    }

    /// What kind of structure the property interrupts.
    pub fn interruption(&self) -> Option<Interruption> {
        if let Pattern::Node(id) = self.pattern {
            if id == NodeId::of::<ListNode>() || id == NodeId::of::<EnumNode>() {
                return Some(Interruption::List);
            }
        }

        None
    }
}

impl Debug for Recipe {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "Recipe matching {:?} from {:?}",
            self.pattern, self.func.span
        )
    }
}

/// A show rule pattern that may match a target.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Pattern {
    /// Defines the appearence of some node.
    Node(NodeId),
    /// Defines text to be replaced.
    Regex(Regex),
}

impl Pattern {
    /// Define a simple text replacement pattern.
    pub fn text(text: &str) -> Self {
        Self::Regex(Regex::new(&regex::escape(text)).unwrap())
    }
}

/// A target for a show rule recipe.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Target<'a> {
    /// A showable node.
    Node(&'a ShowNode),
    /// A slice of text.
    Text(&'a str),
}

/// Identifies a show rule recipe.
#[derive(Debug, Copy, Clone, PartialEq, Hash)]
pub enum Selector {
    /// The nth recipe from the top of the chain.
    Nth(usize),
    /// The base recipe for a kind of node.
    Base(NodeId),
}
