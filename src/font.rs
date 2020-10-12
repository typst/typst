//! Font handling.

use std::cell::RefCell;
use std::rc::Rc;

use fontdock::{ContainsChar, FaceFromVec, FontSource};
use ttf_parser::Face;

/// A reference-counted shared font loader backed by a dynamic font source.
pub type SharedFontLoader = Rc<RefCell<FontLoader>>;

/// A font loader backed by a dynamic source.
pub type FontLoader = fontdock::FontLoader<Box<DynSource>>;

/// The dynamic font source.
pub type DynSource = dyn FontSource<Face = OwnedFace>;

/// An owned font face.
pub struct OwnedFace {
    data: Box<[u8]>,
    face: Face<'static>,
}

impl OwnedFace {
    /// Get a reference to the underlying face.
    pub fn get(&self) -> &Face<'_> {
        // We can't implement Deref because that would leak the internal 'static
        // lifetime.
        &self.face
    }

    /// The raw face data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

impl FaceFromVec for OwnedFace {
    fn from_vec(vec: Vec<u8>, i: u32) -> Option<Self> {
        let data = vec.into_boxed_slice();

        // SAFETY: The slices's location is stable in memory since we don't
        //         touch it and it can't be touched from outside this type.
        let slice: &'static [u8] =
            unsafe { std::slice::from_raw_parts(data.as_ptr(), data.len()) };

        Some(Self {
            data,
            face: Face::from_slice(slice, i).ok()?,
        })
    }
}

impl ContainsChar for OwnedFace {
    fn contains_char(&self, c: char) -> bool {
        self.get().glyph_index(c).is_some()
    }
}
