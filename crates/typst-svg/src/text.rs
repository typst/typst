use ecow::EcoString;
use ttf_parser::GlyphId;
use typst_library::layout::{Abs, Ratio, Size, Transform};
use typst_library::text::TextItem;
use typst_library::text::color::{
    GlyphFrame, GlyphFrameItem, glyph_frame, should_outline,
};
use typst_library::visualize::{FillRule, Paint, RelativeTo};

use crate::{DedupId, SVGRenderer, State, SvgMatrix, SvgPathBuilder};

/// Represents a glyph to be rendered.
#[derive(Clone)]
pub enum RenderedGlyph {
    /// A frame that contains an image glpyh.
    Frame(GlyphFrame),
    /// A path is a sequence of drawing commands.
    ///
    /// It is in the format of `M x y L x y C x1 y1 x2 y2 x y Z`.
    Path(EcoString),
}

impl SVGRenderer<'_> {
    /// Render a text item. The text is rendered as a group of glyphs. We will
    /// try to render the text as SVG first, then bitmap, then outline. If none
    /// of them works, we will skip the text.
    pub(super) fn render_text(&mut self, state: &State, text: &TextItem) {
        self.xml.start_element("g");
        self.xml.write_attribute("class", "typst-text");

        // Flip the transform since fonts use a Y-Up coordinate system.
        let state = state.pre_concat(Transform::scale(Ratio::one(), -Ratio::one()));
        self.xml.write_attribute("transform", &SvgMatrix(state.transform));

        let mut x = Abs::pt(0.0);
        let mut y = Abs::pt(0.0);
        for glyph in &text.glyphs {
            let id = GlyphId(glyph.id);
            let x_offset = x + glyph.x_offset.at(text.size);
            let y_offset = y + glyph.y_offset.at(text.size);

            self.render_glyph(&state, text, id, x_offset, y_offset);

            x += glyph.x_advance.at(text.size);
            y += glyph.y_advance.at(text.size);
        }

        self.xml.end_element();
    }

    fn render_glyph(
        &mut self,
        state: &State,
        text: &TextItem,
        glyph_id: GlyphId,
        x_offset: Abs,
        y_offset: Abs,
    ) {
        if should_outline(&text.font, glyph_id) {
            // Pre-scale outlined glyphs, so strokes and fill patterns don't
            // need to consider text size glyph scaling.
            let scale = Ratio::new(text.size.to_pt() / text.font.units_per_em());
            let key = (&text.font, glyph_id, scale);
            let (id, path) = self.glyphs.insert_with_val(key, || {
                let mut builder = SvgPathBuilder::with_scale(scale);
                text.font.ttf().outline_glyph(glyph_id, &mut builder)?;
                Some(RenderedGlyph::Path(builder.path))
            });

            if path.is_some() {
                self.render_path_glyph(state, text, glyph_id, x_offset, y_offset, id)
            }
        } else {
            // Image glyphs apply a `scale` at use site, since colr, svg-, and
            // bitmap glyph images are usually quite large, and having one glyph
            // per text size is a bit of a waste.
            let key = (&text.font, glyph_id);
            let (id, frame) = self.glyphs.insert_with_val(key, || {
                let frame = glyph_frame(&text.font, glyph_id.0)?;
                Some(RenderedGlyph::Frame(frame))
            });

            if frame.is_some() {
                self.render_image_glyph(x_offset, y_offset, text, id);
            }
        }
    }

    /// Write a reference to an image glyph that is stored in font units.
    fn render_image_glyph(
        &mut self,
        x_offset: Abs,
        y_offset: Abs,
        text: &TextItem,
        id: DedupId,
    ) {
        let scale = Ratio::new(text.size.to_pt() / text.font.units_per_em());
        // Flip the transform again, since images are drawn Y-Down.
        let ts = Transform::translate(x_offset, y_offset + text.size)
            .pre_concat(Transform::scale(scale, -scale));

        self.xml.start_element("use");
        self.xml.write_attribute_fmt("xlink:href", format_args!("#{id}"));
        self.xml.write_attribute("transform", &SvgMatrix(ts));
        self.xml.end_element();
    }

    /// Render a pre-scaled path glyph defined by an outline.
    fn render_path_glyph(
        &mut self,
        state: &State,
        text: &TextItem,
        glyph_id: GlyphId,
        x_offset: Abs,
        y_offset: Abs,
        id: DedupId,
    ) {
        // Apply the transform here, because the state transform is used to draw
        // strokes and fills with gradients and tilings.
        let state = state.pre_concat(Transform::translate(x_offset, y_offset));

        let Some(glyph_size) = text.font.ttf().glyph_bounding_box(glyph_id) else {
            // This shouldn't happen, because the glyph has been successfully
            // outlined to create the path.
            return;
        };

        let aspect_ratio = Size::new(
            Abs::pt(glyph_size.width() as f64),
            Abs::pt(glyph_size.height() as f64),
        )
        .aspect_ratio();

        self.xml.start_element("use");
        self.xml.write_attribute_fmt("xlink:href", format_args!("#{id}"));
        self.xml.write_attribute("x", &x_offset.to_pt());
        self.xml.write_attribute("y", &y_offset.to_pt());
        self.write_fill(
            &text.fill,
            FillRule::default(),
            aspect_ratio,
            self.text_paint_transform(&state, &text.fill),
        );
        if let Some(stroke) = &text.stroke {
            self.write_stroke(
                stroke,
                aspect_ratio,
                self.text_paint_transform(&state, &stroke.paint),
            );
        }
        self.xml.end_element();
    }

    fn text_paint_transform(&self, state: &State, paint: &Paint) -> Transform {
        match paint {
            Paint::Solid(_) => Transform::identity(),
            Paint::Gradient(gradient) => match gradient.unwrap_relative(true) {
                RelativeTo::Self_ => Transform::identity(),
                RelativeTo::Parent => Transform::scale(
                    Ratio::new(state.size.x.to_pt()),
                    Ratio::new(state.size.y.to_pt()),
                )
                .post_concat(state.transform.invert().unwrap()),
            },
            Paint::Tiling(tiling) => match tiling.unwrap_relative(true) {
                RelativeTo::Self_ => Transform::identity(),
                RelativeTo::Parent => state.transform.invert().unwrap(),
            },
        }
    }

    /// Build the glyph definitions.
    pub(super) fn write_glyph_defs(&mut self) {
        if self.glyphs.iter().all(|(_, g)| g.is_none()) {
            return;
        }

        self.xml.start_element("defs");
        self.xml.write_attribute("id", "glyph");

        let glyphs = std::mem::take(&mut self.glyphs);
        for (id, glyph) in glyphs.iter() {
            let Some(glyph) = glyph else { continue };

            self.xml.start_element("symbol");
            self.xml.write_attribute("id", &id);
            self.xml.write_attribute("overflow", "visible");

            match glyph {
                RenderedGlyph::Frame(frame) => {
                    let state = State::new(frame.size()).pre_translate(frame.item.pos());
                    match &frame.item {
                        GlyphFrameItem::Tofu(_, shape) => {
                            self.render_shape(&state, shape);
                        }
                        GlyphFrameItem::Image(_, image, size) => {
                            self.render_image(&state, image, size);
                        }
                    }
                }
                RenderedGlyph::Path(path) => {
                    self.xml.start_element("path");
                    self.xml.write_attribute("d", path);
                    self.xml.end_element();
                }
            }

            self.xml.end_element();
        }

        // The glyphs have been taken above, there shouldn't be any new glyphs
        // produced from writing the glyph definitions.
        assert!(self.glyphs.is_empty());

        self.xml.end_element();
    }
}
