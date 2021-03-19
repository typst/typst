//! Execution of syntax trees.

mod context;
mod state;

pub use context::*;
pub use state::*;

use std::rc::Rc;

use crate::diag::Pass;
use crate::env::Env;
use crate::eval::{ExprMap, TemplateFunc, TemplateNode, Value, ValueTemplate};
use crate::layout::{self, NodeFixed, NodeSpacing, NodeStack};
use crate::pretty::pretty;
use crate::syntax::*;

/// Execute a syntax tree to produce a layout tree.
///
/// The `map` shall be an expression map computed for this tree with
/// [`eval`](crate::eval::eval). Note that `tree` must be the _exact_ same tree
/// as used for evaluation (no cloned version), because the expression map
/// depends on the pointers being stable.
///
/// The `state` is the base state that may be updated over the course of
/// execution.
pub fn exec(
    env: &mut Env,
    tree: &Tree,
    map: &ExprMap,
    state: State,
) -> Pass<layout::Tree> {
    let mut ctx = ExecContext::new(env, state);
    tree.exec_with_map(&mut ctx, &map);
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

impl ExecWithMap for Tree {
    fn exec_with_map(&self, ctx: &mut ExecContext, map: &ExprMap) {
        for node in self {
            node.exec_with_map(ctx, map);
        }
    }
}

impl ExecWithMap for Node {
    fn exec_with_map(&self, ctx: &mut ExecContext, map: &ExprMap) {
        match self {
            Node::Text(text) => ctx.push_text(text),
            Node::Space => ctx.push_space(),
            Node::Linebreak => ctx.push_linebreak(),
            Node::Parbreak => ctx.push_parbreak(),
            Node::Strong => ctx.state.font.strong ^= true,
            Node::Emph => ctx.state.font.emph ^= true,
            Node::Heading(heading) => heading.exec_with_map(ctx, map),
            Node::Raw(raw) => raw.exec(ctx),
            Node::Expr(expr) => map[&(expr as *const _)].exec(ctx),
        }
    }
}

impl ExecWithMap for NodeHeading {
    fn exec_with_map(&self, ctx: &mut ExecContext, map: &ExprMap) {
        let prev = ctx.state.clone();
        let upscale = 1.5 - 0.1 * self.level as f64;
        ctx.state.font.scale *= upscale;
        ctx.state.font.strong = true;

        self.contents.exec_with_map(ctx, map);
        ctx.push_parbreak();

        ctx.state = prev;
    }
}

impl Exec for NodeRaw {
    fn exec(&self, ctx: &mut ExecContext) {
        let prev = Rc::clone(&ctx.state.font.families);
        ctx.set_monospace();

        let em = ctx.state.font.font_size();
        let leading = ctx.state.par.leading.resolve(em);

        let mut children = vec![];
        let mut newline = false;
        for line in &self.lines {
            if newline {
                children.push(layout::Node::Spacing(NodeSpacing {
                    amount: leading,
                    softness: 2,
                }));
            }

            children.push(layout::Node::Text(ctx.make_text_node(line.clone())));
            newline = true;
        }

        if self.block {
            ctx.push_parbreak();
        }

        // This is wrapped in a fixed node to make sure the stack fits to its
        // content instead of filling the available area.
        ctx.push(NodeFixed {
            width: None,
            height: None,
            child: NodeStack {
                dirs: ctx.state.dirs,
                aligns: ctx.state.aligns,
                children,
            }
            .into(),
        });

        if self.block {
            ctx.push_parbreak();
        }

        ctx.state.font.families = prev;
    }
}

impl Exec for Value {
    fn exec(&self, ctx: &mut ExecContext) {
        match self {
            Value::None => {}
            Value::Int(v) => ctx.push_text(&pretty(v)),
            Value::Float(v) => ctx.push_text(&pretty(v)),
            Value::Str(v) => ctx.push_text(v),
            Value::Template(v) => v.exec(ctx),
            Value::Error => {}
            other => {
                // For values which can't be shown "naturally", we print
                // the representation in monospace.
                let prev = Rc::clone(&ctx.state.font.families);
                ctx.set_monospace();
                ctx.push_text(&pretty(other));
                ctx.state.font.families = prev;
            }
        }
    }
}

impl Exec for ValueTemplate {
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
