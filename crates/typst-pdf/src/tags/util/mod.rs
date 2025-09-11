use krilla::tagging as kt;
use krilla::tagging::{ArtifactType, NaiveRgbColor};
use typst_library::pdf::{ArtifactKind, TableHeaderScope};
use typst_library::visualize::Paint;

mod idvec;
mod prop;

pub use idvec::*;
pub use prop::*;

// Best effort fallible conversion.
pub fn paint_to_color(paint: &Paint) -> Option<NaiveRgbColor> {
    match paint {
        Paint::Solid(color) => {
            let c = color.to_rgb();
            Some(NaiveRgbColor::new_f32(c.red, c.green, c.blue))
        }
        Paint::Gradient(_) => None,
        Paint::Tiling(_) => None,
    }
}

pub trait ArtifactKindExt {
    fn to_krilla(self) -> ArtifactType;
}

impl ArtifactKindExt for ArtifactKind {
    fn to_krilla(self) -> ArtifactType {
        match self {
            Self::Header => ArtifactType::Header,
            Self::Footer => ArtifactType::Footer,
            Self::Page => ArtifactType::Page,
            Self::Other => ArtifactType::Other,
        }
    }
}

pub trait TableHeaderScopeExt {
    fn to_krilla(self) -> kt::TableHeaderScope;
}

impl TableHeaderScopeExt for TableHeaderScope {
    fn to_krilla(self) -> kt::TableHeaderScope {
        match self {
            Self::Both => kt::TableHeaderScope::Both,
            Self::Column => kt::TableHeaderScope::Column,
            Self::Row => kt::TableHeaderScope::Row,
        }
    }
}
