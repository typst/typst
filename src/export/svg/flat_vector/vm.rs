use std::{ops::Deref, sync::Arc};

use super::ir;
use crate::export::svg::{
    ir::{AbsoulteRef, Point, Scalar},
    vector::{GroupContext, SvgTextBuilder, SvgTextNode, TransformContext},
    ExportFeature, SvgRenderTask,
};

/// A RAII trait for rendering flatten SVG items into underlying context.
pub trait FlatGroupContext: Sized {
    fn render_item_ref_at(&mut self, pos: Point, item: &AbsoulteRef);
    fn render_item_ref(&mut self, item: &AbsoulteRef) {
        self.render_item_ref_at(Point::default(), item);
    }

    fn render_glyph_ref(&mut self, pos: Scalar, item: &AbsoulteRef);

    fn render_flat_text_semantics(&mut self, _text: &ir::FlatTextItem, _width: Scalar) {}
}

/// A virtual machine for rendering a flatten frame.
/// This is a stateful object that is used to render a frame.
/// The 's lifetime is the lifetime of the virtual machine itself.
/// The 'm lifetime is the lifetime of the module which stores the frame data.
pub trait FlatRenderVm<'s, 'm> {
    type Resultant;
    type Group: GroupContext + FlatGroupContext + TransformContext + Into<Self::Resultant>;

    fn get_item(&self, value: &AbsoulteRef) -> Option<&'m ir::FlatSvgItem>;

    fn start_flat_group(&'s mut self, value: &AbsoulteRef) -> Self::Group;

    fn start_flat_frame(
        &'s mut self,
        value: &AbsoulteRef,
        _group: &ir::GroupRef,
    ) -> Self::Group {
        self.start_flat_group(value)
    }

    fn start_flat_text(
        &'s mut self,
        value: &AbsoulteRef,
        _text: &ir::FlatTextItem,
    ) -> Self::Group {
        self.start_flat_group(value)
    }

    /// Render an item into the a `<g/>` element.
    fn render_flat_item(&'s mut self, abs_ref: &AbsoulteRef) -> Self::Resultant {
        let item: &'m ir::FlatSvgItem = self.get_item(abs_ref).unwrap();
        match item.deref() {
            ir::FlatSvgItem::Group(group) => self.render_group_ref(abs_ref, group),
            ir::FlatSvgItem::Item(transformed) => {
                self.render_transformed_ref(abs_ref, transformed)
            }
            ir::FlatSvgItem::Text(text) => self.render_flat_text(abs_ref, text),
            ir::FlatSvgItem::Path(path) => {
                let mut g = self.start_flat_group(abs_ref);
                g.render_path(path);
                g.into()
            }
            ir::FlatSvgItem::Link(link) => {
                let mut g = self.start_flat_group(abs_ref);
                g.render_link(link);
                g.into()
            }
            ir::FlatSvgItem::Image(image) => {
                let mut g = self.start_flat_group(abs_ref);
                g.render_image(image);
                g.into()
            }
            ir::FlatSvgItem::None => {
                panic!("FlatRenderVm.RenderFrame.UnknownItem {:?}", item)
            }
        }
    }

    /// Render a frame group into underlying context.
    fn render_group_ref(
        &'s mut self,
        abs_ref: &AbsoulteRef,
        group: &ir::GroupRef,
    ) -> Self::Resultant {
        let mut group_ctx = self.start_flat_frame(abs_ref, group);

        for (pos, item_ref) in group.0.iter() {
            // let item = self.get_item(&item_ref).unwrap();
            group_ctx.render_item_ref_at(*pos, item_ref);
        }

        group_ctx.into()
    }

    /// Render a transformed frame into underlying context.
    fn render_transformed_ref(
        &'s mut self,
        abs_ref: &AbsoulteRef,
        transformed: &ir::TransformedRef,
    ) -> Self::Resultant {
        let mut ts = self.start_flat_group(abs_ref).transform(&transformed.0);

        let item_ref = &transformed.1;
        // let item = self.get_item(&item_ref).unwrap();
        ts.render_item_ref(item_ref);
        ts.into()
    }

    /// Render a text into the underlying context.
    fn render_flat_text(
        &'s mut self,
        abs_ref: &AbsoulteRef,
        text: &ir::FlatTextItem,
    ) -> Self::Resultant {
        let group_ctx = self.start_flat_text(abs_ref, text);

        let ppem = Scalar(text.shape.ppem.0);

        let mut group_ctx = group_ctx.transform_scale(ppem, -ppem);

        let mut x = 0f32;
        for (offset, advance, glyph) in text.content.glyphs.iter() {
            let offset = x + offset.0;
            let ts = offset / ppem.0;

            group_ctx.render_glyph_ref(Scalar(ts), glyph);

            x += advance.0;
        }

        group_ctx.render_flat_text_semantics(text, Scalar(x));
        group_ctx.into()
    }
}

impl<'s, 'm: 's, 't: 's, Feat: ExportFeature + 's> FlatRenderVm<'s, 'm>
    for SvgRenderTask<'m, 't, Feat>
{
    // type Resultant = String;
    type Resultant = Arc<SvgTextNode>;
    type Group = SvgTextBuilder<'s, 'm, 't, Feat>;

    fn get_item(&self, value: &AbsoulteRef) -> Option<&'m ir::FlatSvgItem> {
        self.module.get_item(value)
    }

    fn start_flat_group(&'s mut self, v: &AbsoulteRef) -> Self::Group {
        Self::Group {
            t: self,
            attributes: vec![("data-tid", v.as_svg_id("g"))],
            content: Vec::with_capacity(1),
        }
    }

    fn start_flat_frame(
        &'s mut self,
        value: &AbsoulteRef,
        _group: &ir::GroupRef,
    ) -> Self::Group {
        let mut g = self.start_flat_group(value);
        g.attributes.push(("class", "group".to_owned()));
        g
    }

    fn start_flat_text(
        &'s mut self,
        value: &AbsoulteRef,
        text: &ir::FlatTextItem,
    ) -> Self::Group {
        let mut g = self.start_flat_group(value);
        g.with_text_shape(&text.shape);
        g
    }
}
