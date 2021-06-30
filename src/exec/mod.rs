//! Execution of syntax trees.

mod context;
mod state;

pub use context::*;
pub use state::*;

use std::rc::Rc;

use crate::diag::Pass;
use crate::eval::{ExprMap, TemplateFunc, TemplateNode, TemplateValue, Value};
use crate::geom::{Dir, Gen};
use crate::layout::{LayoutTree, StackChild, StackNode};
use crate::pretty::pretty;
use crate::syntax::*;

/// Execute a template to produce a layout tree.
pub fn exec(template: &TemplateValue, state: State) -> Pass<LayoutTree> {
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

/// Execute a node with an expression map that applies to it.
pub trait ExecWithMap {
    /// Execute the node.
    fn exec_with_map(&self, ctx: &mut ExecContext, map: &ExprMap);
}

impl ExecWithMap for SyntaxTree {
    fn exec_with_map(&self, ctx: &mut ExecContext, map: &ExprMap) {
        for node in self {
            node.exec_with_map(ctx, map);
        }
    }
}

impl ExecWithMap for Node {
    fn exec_with_map(&self, ctx: &mut ExecContext, map: &ExprMap) {
        match self {
            Self::Text(text) => ctx.push_text(text),
            Self::Space => ctx.push_word_space(),
            Self::Linebreak(_) => ctx.linebreak(),
            Self::Parbreak(_) => ctx.parbreak(),
            Self::Strong(_) => ctx.state.font_mut().strong ^= true,
            Self::Emph(_) => ctx.state.font_mut().emph ^= true,
            Self::Raw(n) => n.exec(ctx),
            Self::Heading(n) => n.exec_with_map(ctx, map),
            Self::List(n) => n.exec_with_map(ctx, map),
            Self::Enum(n) => n.exec_with_map(ctx, map),
            Self::Expr(n) => map[&(n as *const _)].exec(ctx),
        }
    }
}

impl Exec for RawNode {
    fn exec(&self, ctx: &mut ExecContext) {
        if self.block {
            ctx.parbreak();
        }

        let snapshot = Rc::clone(&ctx.state.font);
        ctx.set_monospace();
        ctx.push_text(&self.text);
        ctx.state.font = snapshot;

        if self.block {
            ctx.parbreak();
        }
    }
}

impl ExecWithMap for HeadingNode {
    fn exec_with_map(&self, ctx: &mut ExecContext, map: &ExprMap) {
        ctx.parbreak();

        let snapshot = ctx.state.clone();
        let font = ctx.state.font_mut();
        let upscale = 1.6 - 0.1 * self.level as f64;
        font.size *= upscale;
        font.strong = true;

        self.body.exec_with_map(ctx, map);
        ctx.state = snapshot;

        ctx.parbreak();
    }
}

impl ExecWithMap for ListItem {
    fn exec_with_map(&self, ctx: &mut ExecContext, map: &ExprMap) {
        exec_item(ctx, "•".to_string(), &self.body, map);
    }
}

impl ExecWithMap for EnumItem {
    fn exec_with_map(&self, ctx: &mut ExecContext, map: &ExprMap) {
        let label = self.number.unwrap_or(1).to_string() + ".";
        exec_item(ctx, label, &self.body, map);
    }
}

fn exec_item(ctx: &mut ExecContext, label: String, body: &SyntaxTree, map: &ExprMap) {
    let label = ctx.exec_stack(|ctx| ctx.push_text(label));
    let body = ctx.exec_tree_stack(body, map);
    let stack = StackNode {
        dirs: Gen::new(Dir::TTB, ctx.state.lang.dir),
        aspect: None,
        children: vec![
            StackChild::Any(label.into(), Gen::default()),
            StackChild::Spacing(ctx.state.font.size / 2.0),
            StackChild::Any(body.into(), Gen::default()),
        ],
    };

    ctx.push_into_stack(stack);
}

impl Exec for Value {
    fn exec(&self, ctx: &mut ExecContext) {
        match self {
            Value::None => {}
            Value::Int(v) => ctx.push_text(pretty(v)),
            Value::Float(v) => ctx.push_text(pretty(v)),
            Value::Str(v) => ctx.push_text(v),
            Value::Template(v) => v.exec(ctx),
            Value::Error => {}
            other => {
                // For values which can't be shown "naturally", we print
                // the representation in monospace.
                let prev = Rc::clone(&ctx.state.font.families);
                ctx.set_monospace();
                ctx.push_text(pretty(other));
                ctx.state.font_mut().families = prev;
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
            Self::Str(v) => ctx.push_text(v),
            Self::Func(v) => v.exec(ctx),
        }
    }
}

impl Exec for TemplateFunc {
    fn exec(&self, ctx: &mut ExecContext) {
        let snapshot = ctx.state.clone();
        self(ctx);
        ctx.state = snapshot;
    }
}
