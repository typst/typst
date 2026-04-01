use std::cmp::{max, min};
use base64::Engine;
use ecow::EcoString;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Range;
use ttf_parser::GlyphId;
use typst_library::layout::{Abs, Ratio, Size, Transform};
use typst_library::text::color::{
    GlyphFrame, GlyphFrameItem, glyph_frame, should_outline,
};
use typst_library::text::{Font, TextItem};
use typst_library::visualize::{FillRule, Paint, RelativeTo};
use write_fonts::from_obj::ToOwnedTable;
use write_fonts::read::tables::glyf::CurvePoint;
use write_fonts::read::{FontRef, TableProvider};
use write_fonts::tables::cmap::Cmap;
use write_fonts::tables::glyf::{Bbox, GlyfLocaBuilder, Glyph, SimpleGlyph};
use write_fonts::tables::head::Head;
use write_fonts::tables::hhea::Hhea;
use write_fonts::tables::hmtx::{Hmtx, LongMetric};
use write_fonts::tables::maxp::Maxp;
use write_fonts::tables::name::Name;
use write_fonts::tables::os2::Os2;
use write_fonts::tables::post::Post;
use write_fonts::FontBuilder;

use crate::path::SvgPathBuilder;
use crate::write::{SvgElem, SvgIdRef, SvgTransform, SvgWrite};
use crate::{DedupId, SVGRenderer, State};

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
    ///
    /// The text is made selectable in browsers by including a `<text>` element with
    /// `fill=transparent`.
    pub(super) fn render_text(
        &mut self,
        svg: &mut SvgElem,
        state: &State,
        text: &TextItem,
    ) {
        let svg = &mut svg.elem("g");

        // Flip the transform since fonts use a Y-Up coordinate system.
        let state = state.pre_concat(Transform::scale(Ratio::one(), -Ratio::one()));
        svg.attr("transform", SvgTransform(state.transform));
        svg.attr("font-family", text.font.svg_font_family());
        svg.attr("font-size", text.size.to_pt());

        let mut x = Abs::pt(0.0);
        let mut y = Abs::pt(0.0);

        struct SpanItem {
            x_offset: Abs,
            y_offset: Abs,
            x_advance: Abs,
            range: Range<usize>,
        }

        let mut span_items = Vec::<SpanItem>::new();

        for glyph in &text.glyphs {
            let id = GlyphId(glyph.id);
            let x_offset = x + glyph.x_offset.at(text.size);
            let y_offset = y + glyph.y_offset.at(text.size);

            // merge with previous span if they are contiguous and have the same y_offset
            if let Some(last) = span_items.last_mut() && last.x_advance == (x_offset - last.x_offset) && y_offset == last.y_offset {
                last.x_advance += glyph.x_advance.at(text.size);

                // merge the range. can't just do "last.range.start..glyph_range.end" because RTL => glyphs in different order than source characters => pain
                let glyph_range = glyph.range();
                last.range = min(last.range.start, glyph_range.start)..max(last.range.end, glyph_range.end);
            } else {
                span_items.push(SpanItem {
                    x_offset,
                    y_offset,
                    x_advance: glyph.x_advance.at(text.size),
                    range: glyph.range(),
                });
            }

            self.render_glyph(svg, &state, text, id, x_offset, y_offset);
            self.save_glyph_for_subset(text.font.clone(), glyph.id as u32);

            x += glyph.x_advance.at(text.size);
            y += glyph.y_advance.at(text.size);
        }

        svg.with_preserving_whitespace(|svg| {
            let text_el = &mut svg.elem("text");
            text_el
                .attr("fill", "transparent")
                .attr("style", "font-variant-ligatures: none")
                .attr("transform", "scale(1,-1)");

            for item in span_items {
                let mut text_el = text_el.elem("tspan");

                text_el
                    .attr("x", item.x_offset.to_pt())
                    .attr("y", item.y_offset.to_pt());

                let text = &text.text[item.range];

                // check if all whitespace
                if let None = text.split_whitespace().next() {
                    text_el.attr("style", "white-space: pre");
                }

                text_el.text(text);
            }
        });
    }

    fn render_glyph(
        &mut self,
        svg: &mut SvgElem,
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
                Some(RenderedGlyph::Path(builder.finsish()))
            });

            if path.is_some() {
                self.render_path_glyph(svg, state, text, glyph_id, x_offset, y_offset, id)
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
                self.render_image_glyph(svg, x_offset, y_offset, text, id);
            }
        }
    }

    /// Write a reference to an image glyph that is stored in font units.
    fn render_image_glyph(
        &mut self,
        svg: &mut SvgElem,
        x_offset: Abs,
        y_offset: Abs,
        text: &TextItem,
        id: DedupId,
    ) {
        let scale = Ratio::new(text.size.to_pt() / text.font.units_per_em());
        // Flip the transform again, since images are drawn Y-Down.
        let ts = Transform::translate(x_offset, y_offset + text.size)
            .pre_concat(Transform::scale(scale, -scale));

        svg.elem("use")
            .attr("xlink:href", SvgIdRef(id))
            .attr("transform", SvgTransform(ts));
    }

    /// Render a pre-scaled path glyph defined by an outline.
    #[allow(clippy::too_many_arguments)]
    fn render_path_glyph(
        &mut self,
        svg: &mut SvgElem,
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

        let mut use_ = svg.elem("use");
        use_.attr("xlink:href", SvgIdRef(id))
            .attr("x", x_offset.to_pt())
            .attr("y", y_offset.to_pt());

        self.write_fill(
            &mut use_,
            &text.fill,
            FillRule::default(),
            aspect_ratio,
            self.text_paint_transform(&state, &text.fill),
        );
        if let Some(stroke) = &text.stroke {
            self.write_stroke(
                &mut use_,
                stroke,
                aspect_ratio,
                self.text_paint_transform(&state, &stroke.paint),
            );
        }
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
    pub(super) fn write_glyph_defs(&mut self, svg: &mut SvgElem) {
        if self.glyphs.iter().all(|(_, g)| g.is_none()) {
            return;
        }

        let mut defs = svg.elem("defs");
        let glyphs = std::mem::take(&mut self.glyphs);
        for (id, glyph) in glyphs.iter() {
            let Some(glyph) = glyph else { continue };

            let mut symbol = defs.elem("symbol");
            symbol.attr("id", id);
            symbol.attr("overflow", "visible");

            match glyph {
                RenderedGlyph::Frame(frame) => {
                    let state = State::new(frame.size()).pre_translate(frame.item.pos());
                    match &frame.item {
                        GlyphFrameItem::Tofu(_, shape) => {
                            self.render_shape(&mut symbol, &state, shape);
                        }
                        GlyphFrameItem::Image(_, image, size) => {
                            self.render_image(&mut symbol, &state, image, size);
                        }
                    }
                }
                RenderedGlyph::Path(path) => {
                    symbol.elem("path").attr("d", path);
                }
            }
        }

        // The glyphs have been taken above, there shouldn't be any new glyphs
        // produced from writing the glyph definitions.
        assert!(self.glyphs.is_empty());
    }

    /// Build the stub fonts for text metrics / correct text selection.
    pub(super) fn write_text_metrics(&self, svg: &mut SvgElem) {
        use base64::engine::general_purpose::STANDARD as B64_STANDARD;

        if self.fonts_for_subset.is_empty() {
            return;
        }

        let mut style = String::new();
        for (font, glyphs) in &self.fonts_for_subset {
            let b64 = B64_STANDARD.encode(&subset_font(font, glyphs));
            write!(
                &mut style,
                "@font-face {{ font-family: '{}'; src: url('data:font/ttf;base64,{b64}') format('truetype'); }}",
                font.svg_font_family(),
            ).unwrap();
        }

        svg.elem("style").text(&style);
    }

    /// Save the glyph ID & font for later subsetting in [`SVGRenderer::write_text_metrics`].
    fn save_glyph_for_subset(&mut self, font: Font, glyph_id: u32) {
        self.fonts_for_subset.entry(font).or_default().insert(glyph_id);
    }
}

pub(crate) trait FontExt {
    fn svg_font_family(&self) -> EcoString;
}

impl FontExt for Font {
    #[comemo::memoize]
    fn svg_font_family(&self) -> EcoString {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        let hash = hasher.finish();

        format!("typst-embedded-font-{hash}").into()
    }
}

/// Subset the font to only include the used glyphs.
fn subset_font(font: &Font, glyphs: &HashSet<u32>) -> Vec<u8> {
    let fr = FontRef::from_index(font.data().as_slice(), font.index()).unwrap();
    let ttf = font.ttf();

    let mut head: Head = fr.head().unwrap().to_owned_table();
    let mut hhea: Hhea = fr.hhea().unwrap().to_owned_table();
    let mut hmtx: Hmtx = fr.hmtx().unwrap().to_owned_table();
    let os2: Os2 = fr.os2().unwrap().to_owned_table();
    let name: Name = fr.name().unwrap().to_owned_table();
    let post: Post = fr.post().unwrap().to_owned_table();

    let mut needed_pairs = HashMap::with_capacity(glyphs.len());
    for subtable in ttf.tables().cmap.unwrap().subtables {
        subtable.codepoints(|cp| {
            let Some(gid) = subtable.glyph_index(cp) else { return };
            if glyphs.contains(&(gid.0 as u32)) {
                let Some(c) = char::from_u32(cp) else { return };
                needed_pairs.insert(c, write_fonts::types::GlyphId::new(gid.0 as u32));
            }
        });
    }

    let n_glyphs = needed_pairs.len() + 1;

    let mut glyf = GlyfLocaBuilder::new();

    glyf.add_glyph(&SimpleGlyph {
        bbox: Bbox { x_min: 0, y_min: 0, x_max: 1, y_max: 1 },
        contours: vec![
            vec![CurvePoint::on_curve(0, 0), CurvePoint::on_curve(0, 0)].into(),
        ],
        instructions: Vec::new(),
    })
    .unwrap();

    let old_metrics = hmtx.h_metrics.clone();
    hmtx.h_metrics.resize(needed_pairs.len() + 1, old_metrics[0].clone());
    hhea.number_of_h_metrics = n_glyphs as u16;

    for (i, (_, gid)) in needed_pairs.iter_mut().enumerate() {
        glyf.add_glyph(&Glyph::Empty).unwrap();

        let ttf_gid = GlyphId(gid.to_u32() as u16);

        let advance = ttf.glyph_hor_advance(ttf_gid).unwrap_or(0);
        let side_bearing = ttf.glyph_hor_side_bearing(ttf_gid).unwrap_or(0);

        hmtx.h_metrics[i + 1] = LongMetric { advance, side_bearing };
        *gid = (i as u16 + 1).into();
    }

    hmtx.left_side_bearings.clear(); // only clear() after the loop above

    let (glyf, loca, loca_fmt) = glyf.build();
    head.index_to_loc_format = loca_fmt as i16;

    let cmap = Cmap::from_mappings(needed_pairs).unwrap();

    FontBuilder::new()
        .add_table(&head)
        .unwrap()
        .add_table(&hhea)
        .unwrap()
        .add_table(&cmap)
        .unwrap()
        .add_table(&hmtx)
        .unwrap()
        .add_table(&Maxp {
            num_glyphs: n_glyphs as u16,
            max_points: Some(2),
            max_contours: Some(1),
            max_composite_points: Some(0),
            max_composite_contours: Some(0),
            max_zones: Some(1),
            max_twilight_points: Some(0),
            max_storage: Some(0),
            max_function_defs: Some(0),
            max_instruction_defs: Some(0),
            max_stack_elements: Some(0),
            max_size_of_instructions: Some(0),
            max_component_elements: Some(0),
            max_component_depth: Some(1),
        })
        .unwrap()
        .add_table(&os2)
        .unwrap()
        .add_table(&name)
        .unwrap()
        .add_table(&post)
        .unwrap()
        .add_table(&glyf)
        .unwrap()
        .add_table(&loca)
        .unwrap()
        .build()
}
