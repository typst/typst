use std::{ops::Deref, sync::Arc};

use super::{
    ir::{self, Abs, Axes, Point, Ratio, Scalar, SvgItem},
    SvgTextBuilder, SvgTextNode,
};
use crate::export::svg::{ExportFeature, SvgRenderTask};

/// A build pattern for applying transforms to the group of items.
/// See [`ir::Transform`].
pub trait TransformContext: Sized {
    fn transform_matrix(self, matrix: &ir::Transform) -> Self;
    fn transform_translate(self, matrix: Axes<Abs>) -> Self;
    fn transform_scale(self, x: Ratio, y: Ratio) -> Self;
    fn transform_rotate(self, matrix: Scalar) -> Self;
    fn transform_skew(self, matrix: (Ratio, Ratio)) -> Self;
    fn transform_clip(self, matrix: &ir::PathItem) -> Self;

    /// See [`ir::TransformItem`].
    fn transform(self, transform: &ir::TransformItem) -> Self {
        match transform {
            ir::TransformItem::Matrix(transform) => {
                self.transform_matrix(transform.as_ref())
            }
            ir::TransformItem::Translate(transform) => {
                self.transform_translate(*transform.clone())
            }
            ir::TransformItem::Scale(transform) => {
                self.transform_scale(transform.0, transform.1)
            }
            ir::TransformItem::Rotate(transform) => {
                self.transform_rotate(*transform.clone())
            }
            ir::TransformItem::Skew(transform) => self.transform_skew(*transform.clone()),
            ir::TransformItem::Clip(transform) => self.transform_clip(transform.as_ref()),
        }
    }
}

/// A RAII trait for rendering SVG items into underlying context.
pub trait GroupContext: Sized {
    /// Render an item at point into underlying context.
    fn render_item_at(&mut self, pos: Point, item: &SvgItem);
    /// Render an item into underlying context.
    fn render_item(&mut self, item: &SvgItem) {
        self.render_item_at(Point::default(), item);
    }

    /// Render a semantic text into underlying context.
    fn render_semantic_text(&mut self, _text: &ir::TextItem, _width: Scalar) {}

    /// Render a glyph into underlying context.
    fn render_glyph(&mut self, pos: Scalar, item: &ir::GlyphItem);

    /// Render a geometrical shape into underlying context.
    fn render_path(&mut self, path: &ir::PathItem);

    /// Render a semantic link into underlying context.
    fn render_link(&mut self, link: &ir::LinkItem);

    /// Render an image into underlying context.
    fn render_image(&mut self, image_item: &ir::ImageItem);
}

/// A trait for rendering SVG items into underlying context.
/// The trait self has a lifetime `'s` which is the lifetime of the underlying context.
pub trait RenderVm<'s> {
    type Resultant;
    type Group: GroupContext + TransformContext + Into<Self::Resultant>;

    /// Start a new `<g/>` like object.
    fn start_group(&'s mut self) -> Self::Group;

    /// Start a new `<g/>` like object for frame group.
    fn start_frame(&'s mut self, _group: &ir::GroupItem) -> Self::Group {
        self.start_group()
    }

    /// Start a new `<g/>` like object for text.
    fn start_text(&'s mut self, _text: &ir::TextItem) -> Self::Group {
        self.start_group()
    }

    /// Render an item into underlying context.
    fn render_item(&'s mut self, item: &SvgItem) -> Self::Resultant {
        match item.deref() {
            ir::SvgItem::Group(group) => self.render_group(group),
            ir::SvgItem::Transformed(transformed) => self.render_transformed(transformed),
            ir::SvgItem::Text(text) => self.render_text(text),
            ir::SvgItem::Path(path) => {
                let mut g = self.start_group();
                g.render_path(path);
                g.into()
            }
            ir::SvgItem::Link(link) => {
                let mut g = self.start_group();
                g.render_link(link);
                g.into()
            }
            ir::SvgItem::Image(image) => {
                let mut g = self.start_group();
                g.render_image(image);
                g.into()
            }
        }
    }

    /// Render a frame group into underlying context.
    fn render_group(&'s mut self, group: &ir::GroupItem) -> Self::Resultant {
        let mut group_ctx = self.start_frame(group);

        for (pos, item_ref) in group.0.iter() {
            group_ctx.render_item_at(*pos, item_ref);
        }

        group_ctx.into()
    }

    /// Render a transformed frame into underlying context.
    fn render_transformed(
        &'s mut self,
        transformed: &ir::TransformedItem,
    ) -> Self::Resultant {
        let mut ts = self.start_group().transform(&transformed.0);
        ts.render_item(&transformed.1);
        ts.into()
    }

    /// Render a text into the underlying context.
    // todo: combine with flat item one
    fn render_text(&'s mut self, text: &ir::TextItem) -> Self::Resultant {
        let group_ctx = self.start_text(text);

        let ppem = Scalar(text.shape.ppem.0);

        let mut group_ctx = group_ctx.transform_scale(ppem, -ppem);

        let mut x = 0f32;
        for (offset, advance, glyph) in text.content.glyphs.iter() {
            let offset = x + offset.0;
            let ts = offset / ppem.0;

            group_ctx.render_glyph(Scalar(ts), glyph);

            x += advance.0;
        }

        group_ctx.render_semantic_text(text, Scalar(x));
        group_ctx.into()
    }
}

/// Example of how to implement a RenderVm.
impl<'s, 'm: 's, 't: 's, Feat: ExportFeature + 's> RenderVm<'s>
    for SvgRenderTask<'m, 't, Feat>
{
    // type Resultant = String;
    type Resultant = Arc<SvgTextNode>;
    type Group = SvgTextBuilder<'s, 'm, 't, Feat>;

    fn start_group(&'s mut self) -> Self::Group {
        Self::Group {
            t: self,
            attributes: vec![],
            content: Vec::with_capacity(1),
        }
    }

    fn start_frame(&'s mut self, _group: &ir::GroupItem) -> Self::Group {
        let mut g = self.start_group();
        g.attributes.push(("class", "group".to_owned()));
        g
    }

    fn start_text(&'s mut self, text: &ir::TextItem) -> Self::Group {
        let mut g = self.start_group();
        g.with_text_shape(&text.shape);
        g
    }
}
