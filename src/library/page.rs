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
    let width = args.named::<Smart<_>>("width")?;
    let height = args.named::<Smart<_>>("height")?;
    let flip = args.named("flip")?;
    let margins = args.named("margins")?;
    let left = args.named("left")?;
    let top = args.named("top")?;
    let right = args.named("right")?;
    let bottom = args.named("bottom")?;
    let fill = args.named("fill")?;

    ctx.template.modify(move |style| {
        let page = style.page_mut();

        if let Some(paper) = paper {
            page.class = paper.class();
            page.size = paper.size();
        }

        if let Some(width) = width {
            page.class = PaperClass::Custom;
            page.size.x = width.unwrap_or(Length::inf());
        }

        if let Some(height) = height {
            page.class = PaperClass::Custom;
            page.size.y = height.unwrap_or(Length::inf());
        }

        if flip.unwrap_or(false) {
            std::mem::swap(&mut page.size.x, &mut page.size.y);
        }

        if let Some(margins) = margins {
            page.margins = Sides::splat(margins);
        }

        if let Some(left) = left {
            page.margins.left = left;
        }

        if let Some(top) = top {
            page.margins.top = top;
        }

        if let Some(right) = right {
            page.margins.right = right;
        }

        if let Some(bottom) = bottom {
            page.margins.bottom = bottom;
        }

        if let Some(fill) = fill {
            page.fill = fill;
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
    /// The size of the page.
    pub size: Size,
    /// The background fill.
    pub fill: Option<Paint>,
    /// The node that produces the actual pages.
    pub child: PackedNode,
}

impl PageNode {
    /// Layout the page run into a sequence of frames, one per page.
    pub fn layout(&self, ctx: &mut LayoutContext) -> Vec<Rc<Frame>> {
        // When one of the lengths is infinite the page fits its content along
        // that axis.
        let expand = self.size.map(Length::is_finite);
        let regions = Regions::repeat(self.size, self.size, expand);

        // Layout the child.
        let mut frames: Vec<_> =
            self.child.layout(ctx, &regions).into_iter().map(|c| c.item).collect();

        // Add background fill if requested.
        if let Some(fill) = self.fill {
            for frame in &mut frames {
                let shape = Shape::filled(Geometry::Rect(frame.size), fill);
                Rc::make_mut(frame).prepend(Point::zero(), Element::Shape(shape));
            }
        }

        frames
    }
}
