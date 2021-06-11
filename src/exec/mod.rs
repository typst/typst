//! Execution of syntax trees.

mod context;
mod state;

pub use context::*;
pub use state::*;

use std::rc::Rc;

use crate::diag::Pass;
use crate::eval::{ExprMap, TemplateFunc, TemplateNode, TemplateValue, Value};
use crate::geom::{Dir, Gen};
use crate::layout::{self, FixedNode, StackChild, StackNode};
use crate::pretty::pretty;
use crate::syntax;

/// Execute a template to produce a layout tree.
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

/// Execute a node with an expression map that applies to it.
pub trait ExecWithMap {
    /// Execute the node.
    fn exec_with_map(&self, ctx: &mut ExecContext, map: &ExprMap);
}

impl ExecWithMap for syntax::Tree {
    fn exec_with_map(&self, ctx: &mut ExecContext, map: &ExprMap) {
        for node in self {
            node.exec_with_map(ctx, map);
        }
    }
}

impl ExecWithMap for syntax::Node {
    fn exec_with_map(&self, ctx: &mut ExecContext, map: &ExprMap) {
        match self {
            Self::Text(text) => ctx.push_text(text),
            Self::Space => ctx.push_word_space(),
            Self::Linebreak(_) => ctx.linebreak(),
            Self::Parbreak(_) => ctx.parbreak(),
            Self::Strong(_) => ctx.state.font_mut().strong ^= true,
            Self::Emph(_) => ctx.state.font_mut().emph ^= true,
            Self::Raw(raw) => raw.exec(ctx),
            Self::Heading(heading) => heading.exec_with_map(ctx, map),
            Self::List(list) => list.exec_with_map(ctx, map),
            Self::Expr(expr) => map[&(expr as *const _)].exec(ctx),
        }
    }
}

impl Exec for syntax::RawNode {
    fn exec(&self, ctx: &mut ExecContext) {
        if self.block {
            ctx.parbreak();
        }

        let snapshot = ctx.state.clone();
        ctx.set_monospace();
        ctx.push_text(&self.text);
        ctx.state = snapshot;

        if self.block {
            ctx.parbreak();
        }
    }
}

impl ExecWithMap for syntax::HeadingNode {
    fn exec_with_map(&self, ctx: &mut ExecContext, map: &ExprMap) {
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

impl ExecWithMap for syntax::ListNode {
    fn exec_with_map(&self, ctx: &mut ExecContext, map: &ExprMap) {
        ctx.parbreak();

        let bullet = ctx.exec_stack(|ctx| ctx.push_text("•"));
        let body = ctx.exec_tree_stack(&self.body, map);

        let stack = StackNode {
            dirs: Gen::new(Dir::TTB, ctx.state.lang.dir),
            aspect: None,
            children: vec![
                StackChild::Any(bullet.into(), Gen::default()),
                StackChild::Spacing(ctx.state.font.size / 2.0),
                StackChild::Any(body.into(), Gen::default()),
            ],
        };

        ctx.push(FixedNode {
            width: None,
            height: None,
            child: stack.into(),
        });

        ctx.parbreak();
    }
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
        self(ctx);
    }
}
