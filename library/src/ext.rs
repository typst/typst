use super::*;
use crate::prelude::*;

/// Additional methods on content.
pub trait ContentExt {
    /// Make this content strong.
    fn strong(self) -> Self;

    /// Make this content emphasized.
    fn emph(self) -> Self;

    /// Underline this content.
    fn underlined(self) -> Self;

    /// Add weak vertical spacing above and below the content.
    fn spaced(self, above: Option<Abs>, below: Option<Abs>) -> Self;

    /// Force a size for this content.
    fn boxed(self, sizing: Axes<Option<Rel<Length>>>) -> Self;

    /// Set alignments for this content.
    fn aligned(self, aligns: Axes<Option<GenAlign>>) -> Self;

    /// Pad this content at the sides.
    fn padded(self, padding: Sides<Rel<Length>>) -> Self;

    /// Transform this content's contents without affecting layout.
    fn moved(self, delta: Axes<Rel<Length>>) -> Self;

    /// Fill the frames resulting from a content.
    fn filled(self, fill: Paint) -> Self;

    /// Stroke the frames resulting from a content.
    fn stroked(self, stroke: Stroke) -> Self;
}

impl ContentExt for Content {
    fn strong(self) -> Self {
        text::StrongNode(self).pack()
    }

    fn emph(self) -> Self {
        text::EmphNode(self).pack()
    }

    fn underlined(self) -> Self {
        text::DecoNode::<{ text::UNDERLINE }>(self).pack()
    }

    fn spaced(self, above: Option<Abs>, below: Option<Abs>) -> Self {
        if above.is_none() && below.is_none() {
            return self;
        }

        let mut seq = vec![];
        if let Some(above) = above {
            seq.push(
                layout::VNode { amount: above.into(), weak: true, generated: true }
                    .pack(),
            );
        }

        seq.push(self);
        if let Some(below) = below {
            seq.push(
                layout::VNode { amount: below.into(), weak: true, generated: true }
                    .pack(),
            );
        }

        Content::sequence(seq)
    }

    fn boxed(self, sizing: Axes<Option<Rel<Length>>>) -> Self {
        layout::BoxNode { sizing, child: self }.pack()
    }

    fn aligned(self, aligns: Axes<Option<GenAlign>>) -> Self {
        layout::AlignNode { aligns, child: self }.pack()
    }

    fn padded(self, padding: Sides<Rel<Length>>) -> Self {
        layout::PadNode { padding, child: self }.pack()
    }

    fn moved(self, delta: Axes<Rel<Length>>) -> Self {
        layout::MoveNode { delta, child: self }.pack()
    }

    fn filled(self, fill: Paint) -> Self {
        FillNode { fill, child: self }.pack()
    }

    fn stroked(self, stroke: Stroke) -> Self {
        StrokeNode { stroke, child: self }.pack()
    }
}

/// Additional methods for the style chain.
pub trait StyleMapExt {
    /// Set a font family composed of a preferred family and existing families
    /// from a style chain.
    fn set_family(&mut self, preferred: text::FontFamily, existing: StyleChain);
}

impl StyleMapExt for StyleMap {
    fn set_family(&mut self, preferred: text::FontFamily, existing: StyleChain) {
        self.set(
            text::TextNode::FAMILY,
            text::FallbackList(
                std::iter::once(preferred)
                    .chain(existing.get(text::TextNode::FAMILY).0.iter().cloned())
                    .collect(),
            ),
        );
    }
}

/// Fill the frames resulting from content.
#[derive(Debug, Hash)]
struct FillNode {
    /// How to fill the frames resulting from the `child`.
    fill: Paint,
    /// The content whose frames should be filled.
    child: Content,
}

#[node(LayoutBlock)]
impl FillNode {}

impl LayoutBlock for FillNode {
    fn layout_block(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        let mut frames = self.child.layout_block(world, regions, styles)?;
        for frame in &mut frames {
            let shape = Geometry::Rect(frame.size()).filled(self.fill);
            frame.prepend(Point::zero(), Element::Shape(shape));
        }
        Ok(frames)
    }
}

/// Stroke the frames resulting from content.
#[derive(Debug, Hash)]
struct StrokeNode {
    /// How to stroke the frames resulting from the `child`.
    stroke: Stroke,
    /// The content whose frames should be stroked.
    child: Content,
}

#[node(LayoutBlock)]
impl StrokeNode {}

impl LayoutBlock for StrokeNode {
    fn layout_block(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        let mut frames = self.child.layout_block(world, regions, styles)?;
        for frame in &mut frames {
            let shape = Geometry::Rect(frame.size()).stroked(self.stroke);
            frame.prepend(Point::zero(), Element::Shape(shape));
        }
        Ok(frames)
    }
}
