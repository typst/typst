//! Exporting into external formats.

mod pdf;
mod render;
mod svg;

pub use self::pdf::pdf;
pub use self::render::{render, render_merged};
pub use self::svg::{svg, svg_merged};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Target {
    Pdf,
    Png,
    Svg,
    Query,
    // Html, // doesn't exist yet
}
