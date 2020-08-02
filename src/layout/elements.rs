//! The elements layouts are composed of.

use std::fmt::{self, Debug, Formatter};

use ttf_parser::GlyphId;
use fontdock::FaceId;
use crate::geom::Size;

/// A sequence of positioned layout elements.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutElements(pub Vec<(Size, LayoutElement)>);

impl LayoutElements {
    /// Create an empty sequence.
    pub fn new() -> Self {
        LayoutElements(vec![])
    }

    /// Add an element at a position.
    pub fn push(&mut self, pos: Size, element: LayoutElement) {
        self.0.push((pos, element));
    }

    /// Add a sequence of elements offset by an `offset`.
    pub fn extend_offset(&mut self, offset: Size, more: Self) {
        for (subpos, element) in more.0 {
            self.0.push((subpos + offset, element));
        }
    }
}

impl Default for LayoutElements {
    fn default() -> Self {
        Self::new()
    }
}

/// A layouting action, which is the basic building block layouts are composed
/// of.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutElement {
    /// Shaped text.
    Text(Shaped),
}

/// A shaped run of text.
#[derive(Clone, PartialEq)]
pub struct Shaped {
    pub text: String,
    pub face: FaceId,
    pub glyphs: Vec<GlyphId>,
    pub offsets: Vec<f64>,
    pub size: f64,
}

impl Shaped {
    /// Create an empty shape run.
    pub fn new(face: FaceId, size: f64) -> Shaped {
        Shaped {
            text: String::new(),
            face,
            glyphs: vec![],
            offsets: vec![],
            size,
        }
    }

    /// Encode the glyph ids into a big-endian byte buffer.
    pub fn encode_glyphs(&self) -> Vec<u8> {
        const BYTES_PER_GLYPH: usize = 2;
        let mut bytes = Vec::with_capacity(BYTES_PER_GLYPH * self.glyphs.len());
        for g in &self.glyphs {
            bytes.push((g.0 >> 8) as u8);
            bytes.push((g.0 & 0xff) as u8);
        }
        bytes
    }
}

impl Debug for Shaped {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Shaped({})", self.text)
    }
}
