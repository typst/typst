//! Rendering into svg text or module.

pub(crate) use tiny_skia as sk;

use std::collections::HashMap;
use std::sync::Arc;

use crate::doc::Document;

pub(crate) mod escape;
#[cfg(feature = "flat-vector")]
pub(crate) mod flat_vector;
pub(crate) mod font;
pub(crate) mod hash;
pub(crate) mod path2d;
pub(crate) mod render;
pub(crate) mod utils;
pub(crate) mod vector;

use ir::{GlyphPackBuilder, ImmutStr, StyleNs, SvgItem};
use render::GlyphRenderTask;
use utils::AbsExt;
use vector::codegen::generate_text;
use vector::*;

pub use font::{FontGlyphProvider, GlyphProvider, IGlyphProvider};
pub use vector::{geom, ir};

#[cfg(feature = "flat-vector")]
pub use render::{
    dynamic_layout::DynamicLayoutSvgExporter, flat::serialize_multi_doc_standalone,
    incremental::IncrementalSvgExporter,
};

#[cfg(feature = "flat-vector")]
pub use flat_ir::{
    LayoutElem, Module, ModuleBuilder, MultiSvgDocument, Pages, SerializedModule,
    SvgDocument,
};
#[cfg(feature = "flat-vector")]
pub use flat_vector::ir as flat_ir;

/// All the features that can be enabled or disabled.
pub trait ExportFeature {
    /// Whether to enable tracing.
    const ENABLE_TRACING: bool;

    /// Whether to render text element.
    /// The text elements is selectable and searchable.
    const SHOULD_RENDER_TEXT_ELEMENT: bool;

    /// Whether to use stable glyph id.
    /// If enabled, the glyph id will be stable across different svg files.
    const USE_STABLE_GLYPH_ID: bool;

    /// Whether to include js for interactive and responsive actions.
    /// If enabled, users can interact with the svg file.
    const WITH_RESPONSIVE_JS: bool;
}

/// The default feature set which is used for exporting full-fledged svg.
pub struct DefaultExportFeature;
pub type DefaultSvgTask = SvgTask<DefaultExportFeature>;

impl ExportFeature for DefaultExportFeature {
    const ENABLE_TRACING: bool = false;
    const SHOULD_RENDER_TEXT_ELEMENT: bool = true;
    const USE_STABLE_GLYPH_ID: bool = true;
    const WITH_RESPONSIVE_JS: bool = true;
}

/// The feature set which is used for exporting plain svg.
pub struct SvgExportFeature;
pub type PlainSvgTask = SvgTask<SvgExportFeature>;

impl ExportFeature for SvgExportFeature {
    const ENABLE_TRACING: bool = false;
    const SHOULD_RENDER_TEXT_ELEMENT: bool = false;
    const USE_STABLE_GLYPH_ID: bool = true;
    const WITH_RESPONSIVE_JS: bool = false;
}

/// Maps the style name to the style definition.
/// See [`StyleNs`].
type StyleDefMap = HashMap<(StyleNs, ImmutStr), String>;
/// Maps the clip path's data to the clip path id.
type ClipPathMap = HashMap<ImmutStr, u32>;

/// The task context for exporting svg.
/// It is also as a namespace for all the functions used in the task.
pub struct SvgTask<Feat: ExportFeature> {
    /// Provides glyphs.
    /// See [`GlyphProvider`].
    glyph_provider: GlyphProvider,

    /// Stores the glyphs used in the document.
    glyph_pack: GlyphPackBuilder,
    /// Stores the style definitions used in the document.
    style_defs: StyleDefMap,
    /// Stores the clip paths used in the document.
    clip_paths: ClipPathMap,

    _feat_phantom: std::marker::PhantomData<Feat>,
}

/// Unfortunately, `Default` derive does not work for generic structs.
impl<Feat: ExportFeature> Default for SvgTask<Feat> {
    fn default() -> Self {
        Self {
            glyph_provider: GlyphProvider::default(),

            glyph_pack: GlyphPackBuilder::default(),
            style_defs: StyleDefMap::default(),
            clip_paths: ClipPathMap::default(),

            _feat_phantom: std::marker::PhantomData,
        }
    }
}

impl<Feat: ExportFeature> SvgTask<Feat> {
    /// Sets the glyph provider for task.
    pub fn set_glyph_provider(&mut self, glyph_provider: GlyphProvider) {
        self.glyph_provider = glyph_provider;
    }

    /// Return integral page size for showing document.
    pub(self) fn page_size(sz: Size) -> Axes<u32> {
        let (width_px, height_px) = {
            let width_px = (sz.x.0.ceil()).round().max(1.0) as u32;
            let height_px = (sz.y.0.ceil()).round().max(1.0) as u32;

            (width_px, height_px)
        };

        Axes::new(width_px, height_px)
    }

    /// fork a render task with module.
    #[cfg(feature = "flat-vector")]
    pub fn fork_page_render_task<'m, 't>(
        &'t mut self,
        module: &'m flat_ir::Module,
    ) -> SvgRenderTask<'m, 't, Feat> {
        SvgRenderTask::<Feat> {
            glyph_provider: self.glyph_provider.clone(),

            module,

            glyph_pack: &mut self.glyph_pack,
            style_defs: &mut self.style_defs,
            clip_paths: &mut self.clip_paths,

            should_render_text_element: true,
            use_stable_glyph_id: true,

            _feat_phantom: Default::default(),
        }
    }

    /// fork a render task.
    #[cfg(not(feature = "flat-vector"))]
    pub fn fork_page_render_task<'m>(&mut self) -> SvgRenderTask<'m, '_, Feat> {
        SvgRenderTask::<Feat> {
            glyph_provider: self.glyph_provider.clone(),

            glyph_pack: &mut self.glyph_pack,
            style_defs: &mut self.style_defs,
            clip_paths: &mut self.clip_paths,

            should_render_text_element: true,
            use_stable_glyph_id: true,

            _feat_phantom: Default::default(),
            _m_phantom: Default::default(),
        }
    }

    pub fn fork_glyph_render_task(&self) -> GlyphRenderTask {
        GlyphRenderTask { glyph_provider: self.glyph_provider.clone() }
    }

    /// Render glyphs into the svg_body.
    fn render_glyphs(
        &mut self,
        glyphs: &GlyphPack,
        use_stable_glyph_id: bool,
    ) -> Vec<SvgText> {
        let mut render_task = self.fork_glyph_render_task();

        let mut svg_body = Vec::new();

        for (abs_ref, item) in glyphs.iter() {
            let glyph_id = if Feat::USE_STABLE_GLYPH_ID && use_stable_glyph_id {
                abs_ref.as_svg_id("g")
            } else {
                abs_ref.as_unstable_svg_id("g")
            };
            svg_body.push(SvgText::Plain(
                render_task.render_glyph(&glyph_id, item).unwrap_or_default(),
            ))
        }

        svg_body
    }

    /// Render pages into the svg_body.
    pub fn render_pages_transient(
        &mut self,
        output: &Document,
        pages: Vec<SvgItem>,
        svg_body: &mut Vec<SvgText>,
    ) {
        #[cfg(feature = "flat-vector")]
        let module = Module::default();
        let mut render_task = {
            #[cfg(feature = "flat-vector")]
            let render_task = self.fork_page_render_task(&module);

            #[cfg(not(feature = "flat-vector"))]
            let render_task = self.fork_page_render_task();

            render_task
        };

        render_task.use_stable_glyph_id = false;

        // accumulate the height of pages
        let mut acc_height = 0u32;
        for (idx, page) in pages.iter().enumerate() {
            let size = Self::page_size(output.pages[idx].size().into());

            let attributes = vec![
                ("transform", format!("translate(0, {})", acc_height)),
                ("data-page-width", size.x.to_string()),
                ("data-page-height", size.y.to_string()),
            ];

            let page_svg = render_task.render_item(page);

            svg_body.push(SvgText::Content(Arc::new(SvgTextNode {
                attributes,
                content: vec![SvgText::Content(page_svg)],
            })));
            acc_height += size.y;
        }
    }
}

/// The task context for rendering svg items
/// The 'm lifetime is the lifetime of the module which stores the frame data.
/// The 't lifetime is the lifetime of SVG task.
pub struct SvgRenderTask<'m, 't, Feat: ExportFeature> {
    /// Provides glyphs.
    /// See [`GlyphProvider`].
    pub glyph_provider: GlyphProvider,

    #[cfg(feature = "flat-vector")]
    pub module: &'m Module,

    /// Stores the glyphs used in the document.
    glyph_pack: &'t mut GlyphPackBuilder,
    /// Stores the style definitions used in the document.
    style_defs: &'t mut StyleDefMap,
    /// Stores the clip paths used in the document.
    clip_paths: &'t mut ClipPathMap,

    /// See [`ExportFeature`].
    pub should_render_text_element: bool,
    /// See [`ExportFeature`].
    pub use_stable_glyph_id: bool,

    pub _feat_phantom: std::marker::PhantomData<Feat>,
    #[cfg(not(feature = "flat-vector"))]
    pub _m_phantom: std::marker::PhantomData<&'m ()>,
}

#[derive(Default)]
pub struct SvgExporter {}

impl SvgExporter {
    /// Render the header of SVG.
    /// <svg> .. </svg>
    /// ^^^^^
    fn header_inner(w: f32, h: f32) -> String {
        format!(
            r#"<svg class="typst-doc" viewBox="0 0 {:.3} {:.3}" width="{:.3}" height="{:.3}" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:h5="http://www.w3.org/1999/xhtml">"#,
            w, h, w, h,
        )
    }

    /// Render the header of SVG for [`Document`].
    /// <svg> .. </svg>
    /// ^^^^^
    fn header_doc(output: &Document) -> String {
        // calculate the width and height of SVG
        let w = output
            .pages
            .iter()
            .map(|p| p.size().x.to_f32().ceil())
            .max_by(|a, b| a.total_cmp(b))
            .unwrap();
        let h = output.pages.iter().map(|p| p.size().y.to_f32().ceil()).sum::<f32>();

        Self::header_inner(w, h)
    }

    /// Render the style for SVG
    /// <svg> <style/> .. </svg>
    ///       ^^^^^^^^
    /// See [`StyleDefMap`].
    fn style_defs(style_defs: StyleDefMap, svg: &mut Vec<SvgText>) {
        // style defs
        svg.push(r#"<style type="text/css">"#.into());

        // sort and push the style defs
        let mut style_defs = style_defs.into_iter().collect::<Vec<_>>();
        style_defs.sort_by(|a, b| a.0.cmp(&b.0));
        svg.extend(style_defs.into_iter().map(|v| SvgText::Plain(v.1)));

        svg.push("</style>".into());
    }

    /// Render the clip paths for SVG
    /// <svg> <defs> <clipPath/> </defs> .. </svg>
    ///              ^^^^^^^^^^^
    /// See [`ClipPathMap`].
    fn clip_paths(clip_paths: ClipPathMap, svg: &mut Vec<SvgText>) {
        let mut clip_paths = clip_paths.into_iter().collect::<Vec<_>>();
        clip_paths.sort_by(|a, b| a.1.cmp(&b.1));
        for (clip_path, id) in clip_paths {
            svg.push(SvgText::Plain(format!(
                r##"<clipPath id="c{:x}"><path d="{}"/></clipPath>"##,
                id, clip_path
            )));
        }
    }

    /// Template SVG.
    fn render_svg_template<Feat: ExportFeature>(
        t: SvgTask<Feat>,
        header: String,
        mut body: Vec<SvgText>,
        glyphs: impl IntoIterator<Item = SvgText>,
    ) -> Vec<SvgText> {
        let mut svg = vec![
            SvgText::Plain(header),
            // base style
            r#"<style type="text/css">"#.into(),
            include_str!("./typst.svg.css").into(),
            "</style>".into(),
            // attach the glyph defs, clip paths, and style defs
            "<defs>".into(),
            "<g>".into(),
        ];
        svg.extend(glyphs);
        svg.push("</g>".into());
        Self::clip_paths(t.clip_paths, &mut svg);
        svg.push("</defs>".into());
        Self::style_defs(t.style_defs, &mut svg);

        // body
        svg.append(&mut body);

        if Feat::WITH_RESPONSIVE_JS {
            // attach the javascript for animations
            svg.push(r#"<script type="text/javascript">"#.into());
            svg.push(include_str!("./typst.svg.js").into());
            svg.push("</script>".into());
        }

        // close SVG
        svg.push("</svg>".into());

        svg
    }

    /// Render SVG for [`Document`].
    /// It does not flatten the vector items before rendering so called "transient".
    fn render_transient_svg<Feat: ExportFeature>(output: &Document) -> Vec<SvgText> {
        let mut t = SvgTask::<Feat>::default();

        // render SVG header
        let header = Self::header_doc(output);

        // lowering the document into svg items
        let mut lower_builder = LowerBuilder::new(output);
        let pages = output
            .pages
            .iter()
            .map(|p| lower_builder.lower(p))
            .collect::<Vec<_>>();

        // render SVG body
        let mut svg_body = vec![];
        t.render_pages_transient(output, pages, &mut svg_body);

        // render the glyphs collected from the pages
        let (glyphs, ..) = std::mem::take(&mut t.glyph_pack).finalize();
        let glyphs = t.render_glyphs(&glyphs, false);

        // template SVG
        Self::render_svg_template(t, header, svg_body, glyphs.into_iter())
    }

    /// Render SVG wrapped with HTML for [`Document`].
    /// It does not flatten the vector items before rendering so called "transient".
    fn render_transient_html(output: &Document) -> Vec<SvgText> {
        // render SVG
        let mut svg = Self::render_transient_svg::<DefaultExportFeature>(output);

        // wrap SVG with html
        let mut html: Vec<SvgText> = Vec::with_capacity(svg.len() + 3);
        html.push(r#"<html><head><meta charset="utf-8" /><title>"#.into());
        html.push(SvgText::Plain(
            output
                .title
                .clone()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "Typst Document".into()),
        ));
        html.push(r#"</title></head><body>"#.into());
        html.append(&mut svg);
        html.push(r#"</body></html>"#.into());

        html
    }
}

/// Render SVG wrapped with html for [`Document`].
pub fn render_svg_html(output: &Document) -> String {
    generate_text(SvgExporter::render_transient_html(output))
}

/// Render SVG for [`Document`].
pub fn render_svg(output: &Document) -> String {
    generate_text(SvgExporter::render_transient_svg::<SvgExportFeature>(output))
}

#[cfg(feature = "flat-vector")]
pub use render::flat::export_module;
