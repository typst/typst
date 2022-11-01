use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;

use comemo::Tracked;

use super::{
    Args, Capability, Content, Func, Interruption, Node, NodeId, Regex, StyleChain,
    StyleEntry, Value,
};
use crate::diag::SourceResult;
use crate::library::structure::{DescNode, EnumNode, ListNode};
use crate::library::text::TextNode;
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
        world: Tracked<dyn World>,
        sel: Selector,
        target: Target,
    ) -> SourceResult<Option<Content>> {
        let content = match (target, &self.pattern) {
            (Target::Node(node), &Pattern::Node(id)) if node.id() == id => {
                let node = node.to::<dyn Show>().unwrap().unguard_parts(sel);
                self.call(world, || Value::Content(node))?
            }

            (Target::Text(text), Pattern::Regex(regex)) => {
                let mut result = vec![];
                let mut cursor = 0;

                for mat in regex.find_iter(text) {
                    let start = mat.start();
                    if cursor < start {
                        result.push(TextNode(text[cursor .. start].into()).pack());
                    }

                    result.push(self.call(world, || Value::Str(mat.as_str().into()))?);
                    cursor = mat.end();
                }

                if result.is_empty() {
                    return Ok(None);
                }

                if cursor < text.len() {
                    result.push(TextNode(text[cursor ..].into()).pack());
                }

                Content::sequence(result)
            }

            _ => return Ok(None),
        };

        Ok(Some(content.styled_with_entry(StyleEntry::Guard(sel))))
    }

    /// Call the recipe function, with the argument if desired.
    fn call<F>(&self, world: Tracked<dyn World>, arg: F) -> SourceResult<Content>
    where
        F: FnOnce() -> Value,
    {
        let args = if self.func.v.argc() == Some(0) {
            Args::new(self.func.span, [])
        } else {
            Args::new(self.func.span, [arg()])
        };

        Ok(self.func.v.call_detached(world, args)?.display(world))
    }

    /// What kind of structure the property interrupts.
    pub fn interruption(&self) -> Option<Interruption> {
        if let Pattern::Node(id) = self.pattern {
            if id == NodeId::of::<ListNode>()
                || id == NodeId::of::<EnumNode>()
                || id == NodeId::of::<DescNode>()
            {
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
    Node(&'a Content),
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

/// A node that can be realized given some styles.
pub trait Show: 'static + Sync + Send {
    /// Unguard nested content against recursive show rules.
    fn unguard_parts(&self, sel: Selector) -> Content;

    /// Access a field on this node.
    fn field(&self, name: &str) -> Option<Value>;

    /// The base recipe for this node that is executed if there is no
    /// user-defined show rule.
    fn realize(
        &self,
        world: Tracked<dyn World>,
        styles: StyleChain,
    ) -> SourceResult<Content>;

    /// Finalize this node given the realization of a base or user recipe. Use
    /// this for effects that should work even in the face of a user-defined
    /// show rule, for example:
    /// - Application of general settable properties
    ///
    /// Defaults to just the realized content.
    #[allow(unused_variables)]
    fn finalize(
        &self,
        world: Tracked<dyn World>,
        styles: StyleChain,
        realized: Content,
    ) -> SourceResult<Content> {
        Ok(realized)
    }
}

impl Capability for dyn Show {}
