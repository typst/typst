//! Execution of syntax trees.

mod context;
mod state;

pub use context::*;
pub use state::*;

use std::rc::Rc;

use crate::diag::Pass;
use crate::eval::{NodeMap, TemplateFunc, TemplateNode, TemplateValue, Value};
use crate::layout;
use crate::pretty::pretty;
use crate::syntax::*;

/// Execute a template to produce a layout tree.
///
/// The `state` is the base state that may be updated over the course of
/// execution.
pub fn exec(template: &TemplateValue, state: State) -> Pass<layout::Tree> {
    let mut ctx = ExecContext::new(state);
    template.exec(&mut ctx);
    ctx.finish()
}

/// Execute a node.
///
/// This manipulates active styling and document state and produces layout
/// nodes. Because syntax nodes and layout nodes do not correspond one-to-one,
/// constructed layout nodes are pushed into the context instead of returned.
/// The context takes care of reshaping the nodes into the correct tree
/// structure.
pub trait Exec {
    /// Execute the node.
    fn exec(&self, ctx: &mut ExecContext);
}

/// Execute a node with a node map that applies to it.
pub trait ExecWithMap {
    /// Execute the node.
    fn exec_with_map(&self, ctx: &mut ExecContext, map: &NodeMap);
}

impl ExecWithMap for Tree {
    fn exec_with_map(&self, ctx: &mut ExecContext, map: &NodeMap) {
        for node in self {
            node.exec_with_map(ctx, map);
        }
    }
}

impl ExecWithMap for Node {
    fn exec_with_map(&self, ctx: &mut ExecContext, map: &NodeMap) {
        match self {
            Node::Text(text) => ctx.push_text(text.clone()),
            Node::Space => ctx.push_word_space(),
            _ => map[&(self as *const _)].exec(ctx),
        }
    }
}

impl Exec for Value {
    fn exec(&self, ctx: &mut ExecContext) {
        match self {
            Value::None => {}
            Value::Int(v) => ctx.push_text(pretty(v)),
            Value::Float(v) => ctx.push_text(pretty(v)),
            Value::Str(v) => ctx.push_text(v.clone()),
            Value::Template(v) => v.exec(ctx),
            Value::Error => {}
            other => {
                // For values which can't be shown "naturally", we print
                // the representation in monospace.
                let prev = Rc::clone(&ctx.state.font.families);
                ctx.set_monospace();
                ctx.push_text(pretty(other));
                ctx.state.font.families = prev;
            }
        }
    }
}

impl Exec for TemplateValue {
    fn exec(&self, ctx: &mut ExecContext) {
        for node in self {
            node.exec(ctx);
        }
    }
}

impl Exec for TemplateNode {
    fn exec(&self, ctx: &mut ExecContext) {
        match self {
            Self::Tree { tree, map } => tree.exec_with_map(ctx, &map),
            Self::Str(v) => ctx.push_text(v.clone()),
            Self::Func(v) => v.exec(ctx),
        }
    }
}

impl Exec for TemplateFunc {
    fn exec(&self, ctx: &mut ExecContext) {
        self(ctx);
    }
}
