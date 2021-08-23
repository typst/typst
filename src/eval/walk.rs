use std::fmt::Write;
use std::rc::Rc;

use super::{Eval, EvalContext, Template, Value};
use crate::diag::TypResult;
use crate::geom::Gen;
use crate::layout::{ParChild, ParNode, StackChild, StackNode};
use crate::syntax::*;
use crate::util::EcoString;

/// Walk a syntax node and fill the currently built template.
pub trait Walk {
    /// Walk the node.
    fn walk(&self, ctx: &mut EvalContext) -> TypResult<()>;
}

impl Walk for SyntaxTree {
    fn walk(&self, ctx: &mut EvalContext) -> TypResult<()> {
        for node in self.iter() {
            node.walk(ctx)?;
        }
        Ok(())
    }
}

impl Walk for SyntaxNode {
    fn walk(&self, ctx: &mut EvalContext) -> TypResult<()> {
        match self {
            Self::Space => ctx.template.space(),
            Self::Linebreak(_) => ctx.template.linebreak(),
            Self::Parbreak(_) => ctx.template.parbreak(),
            Self::Strong(_) => ctx.template.modify(|s| s.font_mut().strong ^= true),
            Self::Emph(_) => ctx.template.modify(|s| s.font_mut().emph ^= true),
            Self::Text(text) => ctx.template.text(text),
            Self::Raw(raw) => raw.walk(ctx)?,
            Self::Heading(heading) => heading.walk(ctx)?,
            Self::List(list) => list.walk(ctx)?,
            Self::Enum(enum_) => enum_.walk(ctx)?,
            Self::Expr(expr) => match expr.eval(ctx)? {
                Value::None => {}
                Value::Int(v) => ctx.template.text(v.to_string()),
                Value::Float(v) => ctx.template.text(v.to_string()),
                Value::Str(v) => ctx.template.text(v),
                Value::Template(v) => ctx.template += v,
                // For values which can't be shown "naturally", we print the
                // representation in monospace.
                other => ctx.template.monospace(other.to_string()),
            },
        }
        Ok(())
    }
}

impl Walk for RawNode {
    fn walk(&self, ctx: &mut EvalContext) -> TypResult<()> {
        if self.block {
            ctx.template.parbreak();
        }

        ctx.template.monospace(&self.text);

        if self.block {
            ctx.template.parbreak();
        }

        Ok(())
    }
}

impl Walk for HeadingNode {
    fn walk(&self, ctx: &mut EvalContext) -> TypResult<()> {
        let level = self.level;
        let body = self.body.eval(ctx)?;

        ctx.template.parbreak();
        ctx.template.save();
        ctx.template.modify(move |state| {
            let font = state.font_mut();
            let upscale = 1.6 - 0.1 * level as f64;
            font.size *= upscale;
            font.strong = true;
        });
        ctx.template += body;
        ctx.template.restore();
        ctx.template.parbreak();

        Ok(())
    }
}

impl Walk for ListNode {
    fn walk(&self, ctx: &mut EvalContext) -> TypResult<()> {
        let body = self.body.eval(ctx)?;
        walk_item(ctx, '•'.into(), body);
        Ok(())
    }
}

impl Walk for EnumNode {
    fn walk(&self, ctx: &mut EvalContext) -> TypResult<()> {
        let body = self.body.eval(ctx)?;
        let mut label = EcoString::new();
        write!(&mut label, "{}.", self.number.unwrap_or(1)).unwrap();
        walk_item(ctx, label, body);
        Ok(())
    }
}

fn walk_item(ctx: &mut EvalContext, label: EcoString, body: Template) {
    ctx.template += Template::from_block(move |state| {
        let label = ParNode {
            dir: state.dirs.inline,
            line_spacing: state.line_spacing(),
            children: vec![ParChild::Text(
                label.clone(),
                state.aligns.inline,
                Rc::clone(&state.font),
            )],
            decorations: vec![],
        };
        StackNode {
            dirs: Gen::new(state.dirs.block, state.dirs.inline),
            children: vec![
                StackChild::Any(label.into(), Gen::default()),
                StackChild::Spacing((state.font.size / 2.0).into()),
                StackChild::Any(body.to_stack(&state).into(), Gen::default()),
            ],
        }
    });
}
