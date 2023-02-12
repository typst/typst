use crate::prelude::*;

use super::AlignNode;

/// # Repeat
/// Repeats content to the available space.
///
/// This can be useful when implementing a custom index, reference, or outline.
///
/// Space may be inserted between the instances of the body parameter, so be
/// sure to include negative space if you need the instances to overlap.
///
/// ## Example
/// ```example
/// Sign on the dotted line:
/// #box(width: 1fr, repeat[.])
///
/// #set text(10pt)
/// #v(8pt, weak: true)
/// #align(right)[
///   Berlin, the 22nd of December, 2022
/// ]
/// ```
///
/// ## Parameters
/// - body: `Content` (positional, required)
///   The content to repeat.
///
/// ## Category
/// layout
#[func]
#[capable(Layout)]
#[derive(Debug, Hash)]
pub struct RepeatNode(pub Content);

#[node]
impl RepeatNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "body" => Some(Value::Content(self.0.clone())),
            _ => None,
        }
    }
}

impl Layout for RepeatNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let pod = Regions::one(regions.size, Axes::new(false, false));
        let piece = self.0.layout(vt, styles, pod)?.into_frame();
        let align = styles.get(AlignNode::ALIGNS).x.resolve(styles);

        let fill = regions.size.x;
        let width = piece.width();
        let count = (fill / width).floor();
        let remaining = fill % width;
        let apart = remaining / (count - 1.0);

        let size = Size::new(regions.size.x, piece.height());
        let mut frame = Frame::new(size);
        if piece.has_baseline() {
            frame.set_baseline(piece.baseline());
        }

        let mut offset = Abs::zero();
        if count == 1.0 {
            offset += align.position(remaining);
        }

        if width > Abs::zero() {
            for _ in 0..(count as usize).min(1000) {
                frame.push_frame(Point::with_x(offset), piece.clone());
                offset += piece.width() + apart;
            }
        }

        Ok(Fragment::frame(frame))
    }
}
