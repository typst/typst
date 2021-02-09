//! Execution of syntax trees.

mod context;
mod state;

pub use context::*;
pub use state::*;

use std::rc::Rc;

use crate::diag::Pass;
use crate::env::Env;
use crate::eval::{ExprMap, TemplateAny, TemplateNode, Value, ValueTemplate};
use crate::geom::Spec;
use crate::layout::{self, Expansion, NodeSpacing, NodeStack};
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
    ctx.start_page_group(Softness::Hard);
    tree.exec_with(&mut ctx, &map);
    ctx.end_page_group(|s| s == Softness::Hard);
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
pub trait ExecWith {
    /// Execute the node.
    fn exec_with(&self, ctx: &mut ExecContext, map: &ExprMap);
}

impl ExecWith for Tree {
    fn exec_with(&self, ctx: &mut ExecContext, map: &ExprMap) {
        for node in self {
            node.exec_with(ctx, map);
        }
    }
}

impl ExecWith for Node {
    fn exec_with(&self, ctx: &mut ExecContext, map: &ExprMap) {
        match self {
            Node::Text(text) => ctx.push_text(text),
            Node::Space => ctx.push_space(),
            Node::Linebreak => ctx.apply_linebreak(),
            Node::Parbreak => ctx.apply_parbreak(),
            Node::Strong => ctx.state.font.strong ^= true,
            Node::Emph => ctx.state.font.emph ^= true,
            Node::Heading(heading) => heading.exec_with(ctx, map),
            Node::Raw(raw) => raw.exec(ctx),
            Node::Expr(expr) => map[&(expr as *const _)].exec(ctx),
        }
    }
}

impl ExecWith for NodeHeading {
    fn exec_with(&self, ctx: &mut ExecContext, map: &ExprMap) {
        let prev = ctx.state.clone();
        let upscale = 1.5 - 0.1 * self.level as f64;
        ctx.state.font.scale *= upscale;
        ctx.state.font.strong = true;

        self.contents.exec_with(ctx, map);
        ctx.apply_parbreak();

        ctx.state = prev;
    }
}

impl Exec for NodeRaw {
    fn exec(&self, ctx: &mut ExecContext) {
        let prev = Rc::clone(&ctx.state.font.families);
        let families = ctx.state.font.families_mut();
        families.list.insert(0, "monospace".to_string());
        families.flatten();

        let em = ctx.state.font.font_size();
        let line_spacing = ctx.state.par.line_spacing.resolve(em);

        let mut children = vec![];
        for line in &self.lines {
            children.push(layout::Node::Text(ctx.make_text_node(line.clone())));
            children.push(layout::Node::Spacing(NodeSpacing {
                amount: line_spacing,
                softness: Softness::Hard,
            }));
        }

        if self.block {
            ctx.apply_parbreak();
        }

        ctx.push(NodeStack {
            dirs: ctx.state.dirs,
            align: ctx.state.align,
            expand: Spec::uniform(Expansion::Fit),
            children,
        });

        if self.block {
            ctx.apply_parbreak();
        }

        ctx.state.font.families = prev;
    }
}

impl Exec for Value {
    fn exec(&self, ctx: &mut ExecContext) {
        match self {
            Value::None => {}
            Value::Str(s) => ctx.push_text(s),
            Value::Template(template) => template.exec(ctx),
            other => ctx.push_text(pretty(other)),
        }
    }
}

impl Exec for ValueTemplate {
    fn exec(&self, ctx: &mut ExecContext) {
        for part in self {
            part.exec(ctx);
        }
    }
}

impl Exec for TemplateNode {
    fn exec(&self, ctx: &mut ExecContext) {
        match self {
            Self::Tree { tree, map } => tree.exec_with(ctx, &map),
            Self::Any(any) => any.exec(ctx),
        }
    }
}

impl Exec for TemplateAny {
    fn exec(&self, ctx: &mut ExecContext) {
        self(ctx);
    }
}
