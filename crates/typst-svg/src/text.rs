use std::fmt::Write;
use std::hash::{DefaultHasher, Hash, Hasher};
use base64::Engine;
use ecow::EcoString;
use ttf_parser::GlyphId;
use write_fonts::{dump_table, FontBuilder};
use write_fonts::from_obj::ToOwnedTable;
use write_fonts::read::{FontRef, TableProvider};
use write_fonts::read::tables::cmap::CmapSubtable;
use write_fonts::read::tables::glyf::CurvePoint;
use write_fonts::tables::cmap::Cmap;
use write_fonts::tables::glyf::{Bbox, GlyfLocaBuilder, Glyph, SimpleGlyph};
use write_fonts::tables::head::Head;
use write_fonts::tables::hhea::Hhea;
use write_fonts::tables::hmtx::Hmtx;
use write_fonts::tables::maxp::Maxp;
use write_fonts::tables::name::Name;
use write_fonts::tables::os2::Os2;
use write_fonts::tables::post::Post;
use typst_library::layout::{Abs, Ratio, Size, Transform};
use typst_library::text::{Font, TextItem};
use typst_library::text::color::{
    GlyphFrame, GlyphFrameItem, glyph_frame, should_outline,
};
use typst_library::visualize::{FillRule, Paint, RelativeTo};

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

        let mut x = Abs::pt(0.0);
        let mut y = Abs::pt(0.0);

        struct SpanItem<'text> {
            x_offset: Abs,
            y_offset: Abs,
            x_advance: Abs,
            text: &'text str,
        }

        let mut span_items = Vec::<SpanItem>::new();

        for glyph in &text.glyphs {
            let id = GlyphId(glyph.id);
            let x_offset = x + glyph.x_offset.at(text.size);
            let y_offset = y + glyph.y_offset.at(text.size);

            span_items.push(SpanItem {
                x_offset,
                y_offset,
                x_advance: glyph.x_advance.at(text.size),
                text: &text.text.as_str()[glyph.range()],
            });

            self.render_glyph(svg, &state, text, id, x_offset, y_offset);
            self.save_glyph_for_subset(text.font.clone(), glyph.id as u32);

            x += glyph.x_advance.at(text.size);
            y += glyph.y_advance.at(text.size);
        }

        let text_el = &mut svg.elem("text");
        text_el
            .attr("fill", "transparent")
            .attr("style", "font-variant-ligatures: none")
            .attr("transform", "scale(1,-1)");

        text_el.with_preserving_whitespace(|text_el| {
            for item in span_items {
                text_el
                    .elem("tspan")
                    .attr("x", item.x_offset.to_pt())
                    .attr("y", item.y_offset.to_pt())
                    .attr("font-size", text.size.to_pt())
                    .attr("style", "user-select: all")
                    .text(item.text);
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
            let fr = FontRef::new(font.data().as_slice()).unwrap();

            let mut head: Head = fr.head().unwrap().to_owned_table();
            let mut hhea: Hhea = fr.hhea().unwrap().to_owned_table();
            let mut hmtx: Hmtx = fr.hmtx().unwrap().to_owned_table();
            let os2: Os2 = fr.os2().unwrap().to_owned_table();
            let name: Name = fr.name().unwrap().to_owned_table();
            let post: Post = fr.post().unwrap().to_owned_table();

            let cmap = fr.cmap().unwrap();

            let mut glyf = GlyfLocaBuilder::new();

            glyf.add_glyph(&SimpleGlyph {
                bbox: Bbox { x_min: 0, y_min: 0, x_max: 1, y_max: 1 },
                contours: vec![
                    vec![CurvePoint::on_curve(0, 0), CurvePoint::on_curve(0, 0)]
                        .into(),
                ],
                instructions: Vec::new(),
            })
                .unwrap();

            let max_gid = glyphs.iter().copied().max().unwrap_or(0);
            for _ in 1..=max_gid {
                glyf.add_glyph(&Glyph::Empty).unwrap();
            }

            let (glyf, loca, loca_fmt) = glyf.build();
            head.index_to_loc_format = loca_fmt as i16;

            hhea.number_of_h_metrics = (max_gid + 1) as u16;
            hmtx.h_metrics.truncate((max_gid + 1) as usize);
            hmtx.left_side_bearings.clear();

            println!(
                "max_gid: {max_gid}, glyf: {:?}, loca: {:?}, font: {:?}, glyphs: {glyphs:?}",
                dump_table(&glyf),
                dump_table(&loca),
                font.index()
            );

            let Some(cmap12) = cmap
                .encoding_records()
                .iter()
                .map(|r| r.subtable(cmap.offset_data()))
                .flat_map(|s| match s {
                    Ok(CmapSubtable::Format12(cmap12)) => Some(cmap12),
                    _ => None,
                })
                .nth(0)
            else {
                panic!("font does not contain a format 12 cmap subtable")
            };

            let needed_pairs = cmap12
                .iter()
                .filter(|(_, gid)| glyphs.contains(&gid.to_u32()))
                .map(|(cp, gid)| (unsafe { char::from_u32_unchecked(cp) }, gid))
                .collect::<Vec<_>>();
            let cmap = Cmap::from_mappings(needed_pairs).unwrap();

            let new_font = FontBuilder::new()
                .add_table(&head)
                .unwrap()
                .add_table(&hhea)
                .unwrap()
                .add_table(&cmap)
                .unwrap()
                .add_table(&hmtx)
                .unwrap()
                .add_table(&Maxp {
                    num_glyphs: (max_gid + 1) as u16,
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
                .build();

            let b64 = B64_STANDARD.encode(&new_font);
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
        self.fonts_for_subset.entry(font).or_default().push(glyph_id);
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
