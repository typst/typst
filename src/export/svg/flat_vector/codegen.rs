use std::sync::Arc;

use super::{
    vm::{FlatGroupContext, FlatRenderVm},
    FlatTextItem,
};
use crate::export::svg::{
    ir::{AbsoulteRef, Point, Scalar},
    vector::{SvgText, SvgTextBuilder, SvgTextNode},
    ExportFeature,
};

/// See [`FlatGroupContext`].
impl<'s, 'm, 't, Feat: ExportFeature> FlatGroupContext
    for SvgTextBuilder<'s, 'm, 't, Feat>
{
    fn render_item_ref_at(&mut self, pos: Point, item: &AbsoulteRef) {
        self.content.push(SvgText::Content(Arc::new(SvgTextNode {
            attributes: vec![(
                "transform",
                format!("translate({:.3},{:.3})", pos.x.0, pos.y.0),
            )],
            content: vec![SvgText::Content(self.t.render_flat_item(item))],
        })));
    }

    fn render_glyph_ref(&mut self, pos: Scalar, glyph: &AbsoulteRef) {
        self.render_glyph_ref_inner(pos, glyph)
    }

    fn render_flat_text_semantics(&mut self, text: &FlatTextItem, width: Scalar) {
        if !(Feat::SHOULD_RENDER_TEXT_ELEMENT && self.t.should_render_text_element) {
            return;
        }

        self.render_text_semantics_inner(&text.shape, &text.content.content, width)
    }
}
