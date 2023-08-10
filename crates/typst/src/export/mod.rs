//! Exporting into external formats.

mod pdf;
mod render;
mod svg;

pub use self::pdf::pdf;
pub use self::render::{render, render_merged};
pub use self::svg::{svg, svg_merged};
use std::fmt::{Debug, Formatter};

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum Target {
    Pdf,
    Raster,
    Vector,
    Query,
    // Html, // doesn't exist yet
}

impl Debug for Target {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Pdf => "pdf",
            Self::Raster => "raster",
            Self::Vector => "svg",
            Self::Query => "query",
        })
    }
}
