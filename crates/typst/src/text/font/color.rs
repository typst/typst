//! Utilities for color font handling

use std::io::Read;

use ttf_parser::GlyphId;
use usvg::{Tree, TreeParsing};

use crate::text::TextItem;

/// A SVG document with information about its dimensions
pub struct SizedSvg {
    /// The declared width of the SVG
    pub width: f32,
    /// The declared height of the SVG
    pub height: f32,
    /// The computed bounding box of the root element
    pub bbox: usvg::Rect,
    /// The SVG document
    pub tree: Tree,
}

/// Retrieve and measure the SVG document for a given glyph, if it exists.
///
/// This function decodes compressed SVG if needed, and computes dimensions
/// of the glyph.
pub fn get_svg_glyph<'a>(text: &'a TextItem, glyph: GlyphId) -> Option<SizedSvg> {
    let mut data = text.font.ttf().glyph_svg_image(glyph)?.data;

    // Decompress SVGZ.
    let mut decoded = vec![];
    if data.starts_with(&[0x1f, 0x8b]) {
        let mut decoder = flate2::read::GzDecoder::new(data);
        decoder.read_to_end(&mut decoded).ok()?;
        data = &decoded;
    }

    // Parse XML.
    let xml = std::str::from_utf8(data).ok()?;
    let document = roxmltree::Document::parse(xml).ok()?;
    let root = document.root_element();

    // Parse SVG.
    let opts = usvg::Options::default();
    let mut tree = usvg::Tree::from_xmltree(&document, &opts).ok()?;
    tree.calculate_bounding_boxes();
    let view_box = tree.view_box.rect;

    // If there's no viewbox defined, use the em square for our scale
    // transformation ...
    let upem = text.font.units_per_em() as f32;
    let (mut width, mut height) = (upem, upem);

    // ... but if there's a viewbox or width, use that.
    if root.has_attribute("viewBox") || root.has_attribute("width") {
        width = view_box.width();
    }

    // Same as for width.
    if root.has_attribute("viewBox") || root.has_attribute("height") {
        height = view_box.height();
    }

    // Compute the space we need to draw our glyph.
    // See https://github.com/RazrFalcon/resvg/issues/602 for why
    // using the svg size is problematic here.
    let mut bbox = usvg::BBox::default();
    if let Some(tree_bbox) = tree.root.bounding_box {
        bbox = bbox.expand(tree_bbox);
    }

    Some(SizedSvg { width, height, bbox: bbox.to_rect()?, tree })
}
