use super::prelude::*;
use super::PadNode;
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

    let page = ctx.style.page_mut();

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

    Ok(Value::None)
}

/// `pagebreak`: Start a new page.
pub fn pagebreak(_: &mut EvalContext, _: &mut Args) -> TypResult<Value> {
    Ok(Value::Node(Node::Pagebreak))
}

/// Layouts its children onto one or multiple pages.
#[derive(Debug, Hash)]
pub struct PageNode(pub PackedNode);

impl PageNode {
    /// Layout the page run into a sequence of frames, one per page.
    pub fn layout(&self, ctx: &mut LayoutContext) -> Vec<Rc<Frame>> {
        // TODO(set): Get style from styles.
        let style = crate::style::PageStyle::default();

        // When one of the lengths is infinite the page fits its content along
        // that axis.
        let expand = style.size.map(Length::is_finite);
        let regions = Regions::repeat(style.size, style.size, expand);

        // Layout the child.
        let padding = style.margins();
        let padded = PadNode { child: self.0.clone(), padding }.pack();
        let mut frames: Vec<_> =
            padded.layout(ctx, &regions).into_iter().map(|c| c.item).collect();

        // Add background fill if requested.
        if let Some(fill) = style.fill {
            for frame in &mut frames {
                let shape = Shape::filled(Geometry::Rect(frame.size), fill);
                Rc::make_mut(frame).prepend(Point::zero(), Element::Shape(shape));
            }
        }

        frames
    }
}
