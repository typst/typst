//! Font and image loading.

#[cfg(feature = "fs")]
mod fs;
mod image;

pub use self::image::*;
#[cfg(feature = "fs")]
pub use fs::*;

use std::collections::{hash_map::Entry, HashMap};
use std::rc::Rc;

use serde::{Deserialize, Serialize};

use crate::font::{Face, FaceInfo, FontVariant};

/// Handles font and image loading.
pub struct Env {
    /// The loader that serves the font face and file buffers.
    loader: Box<dyn Loader>,
    /// Faces indexed by [`FaceId`]. `None` if not yet loaded.
    faces: Vec<Option<Face>>,
    /// Maps a family name to the ids of all faces that are part of the family.
    families: HashMap<String, Vec<FaceId>>,
    /// Loaded images indexed by [`ImageId`].
    images: Vec<Image>,
    /// Maps from paths to loaded images.
    paths: HashMap<String, ImageId>,
    /// Callback for loaded font faces.
    on_face_load: Option<Box<dyn Fn(FaceId, &Face)>>,
    /// Callback for loaded images.
    on_image_load: Option<Box<dyn Fn(ImageId, &Image)>>,
}

impl Env {
    /// Create an environment from a `loader`.
    pub fn new(loader: impl Loader + 'static) -> Self {
        let infos = loader.faces();

        let mut faces = vec![];
        let mut families = HashMap::<String, Vec<FaceId>>::new();

        for (i, info) in infos.iter().enumerate() {
            let id = FaceId(i as u32);
            faces.push(None);
            families
                .entry(info.family.to_lowercase())
                .and_modify(|vec| vec.push(id))
                .or_insert_with(|| vec![id]);
        }

        Self {
            loader: Box::new(loader),
            faces,
            families,
            images: vec![],
            paths: HashMap::new(),
            on_face_load: None,
            on_image_load: None,
        }
    }

    /// Create an empty environment for testing purposes.
    pub fn blank() -> Self {
        struct BlankLoader;

        impl Loader for BlankLoader {
            fn faces(&self) -> &[FaceInfo] {
                &[]
            }

            fn load_face(&mut self, _: usize) -> Option<Buffer> {
                None
            }

            fn load_file(&mut self, _: &str) -> Option<Buffer> {
                None
            }
        }

        Self::new(BlankLoader)
    }

    /// Query for and load the font face from the given `family` that most
    /// closely matches the given `variant`.
    pub fn query_face(&mut self, family: &str, variant: FontVariant) -> Option<FaceId> {
        // Check whether a family with this name exists.
        let ids = self.families.get(family)?;
        let infos = self.loader.faces();

        let mut best = None;
        let mut best_key = None;

        // Find the best matching variant of this font.
        for &id in ids {
            let current = infos[id.0 as usize].variant;

            // This is a perfect match, no need to search further.
            if current == variant {
                best = Some(id);
                break;
            }

            // If this is not a perfect match, we compute a key that we want to
            // minimize among all variants. This key prioritizes style, then
            // stretch distance and then weight distance.
            let key = (
                current.style != variant.style,
                current.stretch.distance(variant.stretch),
                current.weight.distance(variant.weight),
            );

            if best_key.map_or(true, |b| key < b) {
                best = Some(id);
                best_key = Some(key);
            }
        }

        // Load the face if it's not already loaded.
        let id = best?;
        let idx = id.0 as usize;
        let slot = &mut self.faces[idx];
        if slot.is_none() {
            let index = infos[idx].index;
            let buffer = self.loader.load_face(idx)?;
            let face = Face::new(buffer, index)?;
            if let Some(callback) = &self.on_face_load {
                callback(id, &face);
            }
            *slot = Some(face);
        }

        best
    }

    /// Get a reference to a queried face.
    ///
    /// This panics if no face with this id was loaded. This function should
    /// only be called with ids returned by [`query_face()`](Self::query_face).
    #[track_caller]
    pub fn face(&self, id: FaceId) -> &Face {
        self.faces[id.0 as usize].as_ref().expect("font face was not loaded")
    }

    /// Register a callback which is invoked when a font face was loaded.
    pub fn on_face_load<F>(&mut self, f: F)
    where
        F: Fn(FaceId, &Face) + 'static,
    {
        self.on_face_load = Some(Box::new(f));
    }

    /// Load and decode an image file from a path.
    pub fn load_image(&mut self, path: &str) -> Option<ImageId> {
        Some(match self.paths.entry(path.to_string()) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let buffer = self.loader.load_file(path)?;
                let image = Image::parse(&buffer)?;
                let id = ImageId(self.images.len() as u32);
                if let Some(callback) = &self.on_image_load {
                    callback(id, &image);
                }
                self.images.push(image);
                *entry.insert(id)
            }
        })
    }

    /// Get a reference to a loaded image.
    ///
    /// This panics if no image with this id was loaded. This function should
    /// only be called with ids returned by [`load_image()`](Self::load_image).
    #[track_caller]
    pub fn image(&self, id: ImageId) -> &Image {
        &self.images[id.0 as usize]
    }

    /// Register a callback which is invoked when an image was loaded.
    pub fn on_image_load<F>(&mut self, f: F)
    where
        F: Fn(ImageId, &Image) + 'static,
    {
        self.on_image_load = Some(Box::new(f));
    }
}

/// Loads fonts and images from a remote or local source.
pub trait Loader {
    /// Descriptions of all font faces this loader serves.
    fn faces(&self) -> &[FaceInfo];

    /// Load the font face with the given index in [`faces()`](Self::faces).
    fn load_face(&mut self, idx: usize) -> Option<Buffer>;

    /// Load a file from a path.
    fn load_file(&mut self, path: &str) -> Option<Buffer>;
}

/// A shared byte buffer.
pub type Buffer = Rc<Vec<u8>>;

/// A unique identifier for a loaded font face.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct FaceId(u32);

impl FaceId {
    /// A blank initialization value.
    pub const MAX: Self = Self(u32::MAX);

    /// Create a face id from the raw underlying value.
    ///
    /// This should only be called with values returned by
    /// [`into_raw`](Self::into_raw).
    pub fn from_raw(v: u32) -> Self {
        Self(v)
    }

    /// Convert into the raw underlying value.
    pub fn into_raw(self) -> u32 {
        self.0
    }
}

/// A unique identifier for a loaded image.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ImageId(u32);

impl ImageId {
    /// Create an image id from the raw underlying value.
    ///
    /// This should only be called with values returned by
    /// [`into_raw`](Self::into_raw).
    pub fn from_raw(v: u32) -> Self {
        Self(v)
    }

    /// Convert into the raw underlying value.
    pub fn into_raw(self) -> u32 {
        self.0
    }
}
