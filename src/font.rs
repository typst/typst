//! Font handling.

use fontdock::{ContainsChar, FaceFromVec, FontSource};
use ttf_parser::Face;

#[cfg(feature = "fs")]
use fontdock::fs::{FsIndex, FsSource};

/// A font loader that is backed by a dynamic source.
pub type FontLoader = fontdock::FontLoader<Box<dyn FontSource<Face = FaceBuf>>>;

/// An owned font face.
pub struct FaceBuf {
    data: Box<[u8]>,
    face: Face<'static>,
}

impl FaceBuf {
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

impl FaceFromVec for FaceBuf {
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

impl ContainsChar for FaceBuf {
    fn contains_char(&self, c: char) -> bool {
        self.get().glyph_index(c).is_some()
    }
}

/// Simplify font loader construction from an [`FsIndex`].
#[cfg(feature = "fs")]
pub trait FsIndexExt {
    /// Create a font loader backed by a boxed [`FsSource`] which serves all
    /// indexed font faces.
    fn into_dynamic_loader(self) -> FontLoader;
}

#[cfg(feature = "fs")]
impl FsIndexExt for FsIndex {
    fn into_dynamic_loader(self) -> FontLoader {
        let (files, descriptors) = self.into_vecs();
        FontLoader::new(Box::new(FsSource::new(files)), descriptors)
    }
}
