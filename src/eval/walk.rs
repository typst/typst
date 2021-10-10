use std::rc::Rc;

use super::{Eval, EvalContext, Str, Template, Value};
use crate::diag::TypResult;
use crate::geom::{Dir, Gen};
use crate::layout::{ParChild, ParNode, StackChild, StackNode};
use crate::syntax::*;
use crate::util::BoolExt;

/// Walk markup, filling the currently built template.
pub trait Walk {
    /// Walk the node.
    fn walk(&self, ctx: &mut EvalContext) -> TypResult<()>;
}

impl Walk for Markup {
    fn walk(&self, ctx: &mut EvalContext) -> TypResult<()> {
        for node in self.iter() {
            node.walk(ctx)?;
        }
        Ok(())
    }
}

impl Walk for MarkupNode {
    fn walk(&self, ctx: &mut EvalContext) -> TypResult<()> {
        match self {
            Self::Space => ctx.template.space(),
            Self::Linebreak(_) => ctx.template.linebreak(),
            Self::Parbreak(_) => ctx.template.parbreak(),
            Self::Strong(_) => ctx.template.modify(|s| s.text_mut().strong.flip()),
            Self::Emph(_) => ctx.template.modify(|s| s.text_mut().emph.flip()),
            Self::Text(text) => ctx.template.text(text),
            Self::Raw(raw) => raw.walk(ctx)?,
            Self::Heading(heading) => heading.walk(ctx)?,
            Self::List(list) => list.walk(ctx)?,
            Self::Enum(enum_) => enum_.walk(ctx)?,
            Self::Expr(expr) => match expr.eval(ctx)? {
                Value::None => {}
                Value::Int(v) => ctx.template.text(format_str!("{}", v)),
                Value::Float(v) => ctx.template.text(format_str!("{}", v)),
                Value::Str(v) => ctx.template.text(v),
                Value::Template(v) => ctx.template += v,
                // For values which can't be shown "naturally", we print the
                // representation in monospace.
                other => ctx.template.monospace(other.repr()),
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
        ctx.template.modify(move |style| {
            let text = style.text_mut();
            let upscale = 1.6 - 0.1 * level as f64;
            text.size *= upscale;
            text.strong = true;
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
        walk_item(ctx, Str::from('•'), body);
        Ok(())
    }
}

impl Walk for EnumNode {
    fn walk(&self, ctx: &mut EvalContext) -> TypResult<()> {
        let body = self.body.eval(ctx)?;
        let label = format_str!("{}.", self.number.unwrap_or(1));
        walk_item(ctx, label, body);
        Ok(())
    }
}

fn walk_item(ctx: &mut EvalContext, label: Str, body: Template) {
    ctx.template += Template::from_block(move |style| {
        let label = ParNode {
            dir: style.dir,
            line_spacing: style.line_spacing(),
            children: vec![ParChild::Text(
                (&label).into(),
                style.aligns.inline,
                Rc::clone(&style.text),
                vec![],
            )],
        };
        StackNode {
            dirs: Gen::new(Dir::TTB, style.dir),
            children: vec![
                StackChild::Any(label.into(), Gen::default()),
                StackChild::Spacing((style.text.size / 2.0).into()),
                StackChild::Any(body.to_stack(&style).into(), Gen::default()),
            ],
        }
    });
}
