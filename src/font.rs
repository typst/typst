//! Font handling.

use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use ttf_parser::Face;
use fontdock::{FontLoader, FontProvider, ContainsChar, FaceFromVec};

/// A referenced-count shared font loader backed by a dynamic provider.
pub type SharedFontLoader = Rc<RefCell<FontLoader<Box<DynProvider>>>>;

/// The dynamic font provider type backing the font loader.
pub type DynProvider = dyn FontProvider<Face=OwnedFace>;

/// An owned font face.
pub struct OwnedFace {
    data: Vec<u8>,
    face: Face<'static>,
}

impl FaceFromVec for OwnedFace {
    fn from_vec(vec: Vec<u8>, i: u32) -> Option<Self> {
        // The vec's location is stable in memory since we don't touch it and
        // it can't be touched from outside this type.
        let slice: &'static [u8] = unsafe {
            std::slice::from_raw_parts(vec.as_ptr(), vec.len())
        };

        Some(OwnedFace {
            data: vec,
            face: Face::from_slice(slice, i).ok()?,
        })
    }
}

impl OwnedFace {
    /// The raw face data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

impl ContainsChar for OwnedFace {
    fn contains_char(&self, c: char) -> bool {
        self.glyph_index(c).is_some()
    }
}

impl Deref for OwnedFace {
    type Target = Face<'static>;

    fn deref(&self) -> &Self::Target {
        &self.face
    }
}
