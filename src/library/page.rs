use super::prelude::*;
use crate::style::{Paper, PaperClass};

/// `page`: Configure pages.
pub fn page(ctx: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    castable! {
        Paper,
        Expected: "string",
        Value::Str(string) => Paper::from_name(&string).ok_or("unknown paper")?,
    }

    let paper = args.named::<Paper>("paper")?.or_else(|| args.find());
    let width = args.named("width")?;
    let height = args.named("height")?;
    let margins = args.named("margins")?;
    let left = args.named("left")?;
    let top = args.named("top")?;
    let right = args.named("right")?;
    let bottom = args.named("bottom")?;
    let flip = args.named("flip")?;
    let fill = args.named("fill")?;

    ctx.template.modify(move |style| {
        let page = style.page_mut();

        if let Some(paper) = paper {
            page.class = paper.class();
            page.size = paper.size();
        }

        if let Some(width) = width {
            page.class = PaperClass::Custom;
            page.size.w = width;
        }

        if let Some(height) = height {
            page.class = PaperClass::Custom;
            page.size.h = height;
        }

        if let Some(margins) = margins {
            page.margins = Sides::splat(Some(margins));
        }

        if let Some(left) = left {
            page.margins.left = Some(left);
        }

        if let Some(top) = top {
            page.margins.top = Some(top);
        }

        if let Some(right) = right {
            page.margins.right = Some(right);
        }

        if let Some(bottom) = bottom {
            page.margins.bottom = Some(bottom);
        }

        if flip.unwrap_or(false) {
            std::mem::swap(&mut page.size.w, &mut page.size.h);
        }

        if let Some(fill) = fill {
            page.fill = Some(Paint::Color(fill));
        }
    });

    ctx.template.pagebreak(false);

    Ok(Value::None)
}

/// `pagebreak`: Start a new page.
pub fn pagebreak(_: &mut EvalContext, _: &mut Args) -> TypResult<Value> {
    let mut template = Template::new();
    template.pagebreak(true);
    Ok(Value::Template(template))
}

/// Layouts its children onto one or multiple pages.
#[derive(Debug, Hash)]
pub struct PageNode {
    /// The node that produces the actual pages.
    pub child: PackedNode,
    /// The size of the page.
    pub size: Size,
    /// The background fill.
    pub fill: Option<Paint>,
}

impl PageNode {
    /// Layout the page run into a sequence of frames, one per page.
    pub fn layout(&self, ctx: &mut LayoutContext) -> Vec<Rc<Frame>> {
        // When one of the lengths is infinite the page fits its content along
        // that axis.
        let expand = self.size.to_spec().map(Length::is_finite);
        let regions = Regions::repeat(self.size, self.size, expand);

        // Layout the child.
        let mut frames: Vec<_> =
            self.child.layout(ctx, &regions).into_iter().map(|c| c.item).collect();

        // Add background fill if requested.
        if let Some(fill) = self.fill {
            for frame in &mut frames {
                let element = Element::Geometry(Geometry::Rect(frame.size), fill);
                Rc::make_mut(frame).prepend(Point::zero(), element)
            }
        }

        frames
    }
}
