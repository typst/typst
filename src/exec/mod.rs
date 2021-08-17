//! Execution of syntax trees.

mod context;
mod state;

pub use context::*;
pub use state::*;

use std::fmt::Write;

use crate::eval::{ExprMap, Template, TemplateFunc, TemplateNode, TemplateTree, Value};
use crate::geom::Gen;
use crate::layout::{LayoutTree, StackChild, StackNode};
use crate::syntax::*;
use crate::util::EcoString;
use crate::Context;

/// Execute a template to produce a layout tree.
pub fn exec(ctx: &mut Context, template: &Template) -> LayoutTree {
    let mut ctx = ExecContext::new(ctx);
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

impl ExecWithMap for SyntaxNode {
    fn exec_with_map(&self, ctx: &mut ExecContext, map: &ExprMap) {
        match self {
            Self::Space => ctx.space(),
            Self::Text(text) => ctx.text(text),
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

        ctx.text_mono(&self.text);

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
        exec_item(ctx, '•'.into(), &self.body, map);
    }
}

impl ExecWithMap for EnumItem {
    fn exec_with_map(&self, ctx: &mut ExecContext, map: &ExprMap) {
        let mut label = EcoString::new();
        write!(&mut label, "{}.", self.number.unwrap_or(1)).unwrap();
        exec_item(ctx, label, &self.body, map);
    }
}

fn exec_item(ctx: &mut ExecContext, label: EcoString, body: &SyntaxTree, map: &ExprMap) {
    let label = ctx.exec_to_stack(|ctx| ctx.text(label));
    let body = ctx.exec_tree(body, map);
    ctx.block(StackNode {
        dirs: Gen::new(ctx.state.dirs.main, ctx.state.dirs.cross),
        aspect: None,
        children: vec![
            StackChild::Any(label.into(), Gen::default()),
            StackChild::Spacing((ctx.state.font.size / 2.0).into()),
            StackChild::Any(body.into(), Gen::default()),
        ],
    });
}

impl Exec for Value {
    fn exec(&self, ctx: &mut ExecContext) {
        match self {
            Value::None => {}
            Value::Int(v) => ctx.text(v.to_string()),
            Value::Float(v) => ctx.text(v.to_string()),
            Value::Str(v) => ctx.text(v),
            Value::Template(v) => v.exec(ctx),
            // For values which can't be shown "naturally", we print the
            // representation in monospace.
            other => ctx.text_mono(other.to_string()),
        }
    }
}

impl Exec for Template {
    fn exec(&self, ctx: &mut ExecContext) {
        for node in self.iter() {
            node.exec(ctx);
        }
    }
}

impl Exec for TemplateNode {
    fn exec(&self, ctx: &mut ExecContext) {
        match self {
            Self::Tree(v) => v.exec(ctx),
            Self::Func(v) => v.exec(ctx),
            Self::Str(v) => ctx.text(v),
        }
    }
}

impl Exec for TemplateTree {
    fn exec(&self, ctx: &mut ExecContext) {
        self.tree.exec_with_map(ctx, &self.map)
    }
}

impl Exec for TemplateFunc {
    fn exec(&self, ctx: &mut ExecContext) {
        let snapshot = ctx.state.clone();
        self(ctx);
        ctx.state = snapshot;
    }
}
