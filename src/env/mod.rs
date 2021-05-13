//! Font and resource loading.

#[cfg(feature = "fs")]
mod fs;
mod image;

pub use self::image::*;
#[cfg(feature = "fs")]
pub use fs::*;

use std::any::Any;
use std::collections::{hash_map::Entry, HashMap};
use std::rc::Rc;

use serde::{Deserialize, Serialize};

use crate::font::{Face, FaceInfo, FontVariant};

/// Handles font and resource loading.
pub struct Env {
    /// The loader that serves the font face and file buffers.
    loader: Box<dyn Loader>,
    /// Loaded resources indexed by [`ResourceId`].
    resources: Vec<Box<dyn Any>>,
    /// Maps from URL to loaded resource.
    urls: HashMap<String, ResourceId>,
    /// Faces indexed by [`FaceId`]. `None` if not yet loaded.
    faces: Vec<Option<Face>>,
    /// Maps a family name to the ids of all faces that are part of the family.
    families: HashMap<String, Vec<FaceId>>,
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
            resources: vec![],
            urls: HashMap::new(),
            faces,
            families,
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
        let idx = best?.0 as usize;
        let slot = &mut self.faces[idx];
        if slot.is_none() {
            let index = infos[idx].index;
            let buffer = self.loader.load_face(idx)?;
            let face = Face::new(buffer, index)?;
            *slot = Some(face);
        }

        best
    }

    /// Load a file from a local or remote URL, parse it into a cached resource
    /// and return a unique identifier that allows to retrieve the parsed
    /// resource through [`resource()`](Self::resource).
    pub fn load_resource<F, R>(&mut self, url: &str, parse: F) -> Option<ResourceId>
    where
        F: FnOnce(Buffer) -> Option<R>,
        R: 'static,
    {
        Some(match self.urls.entry(url.to_string()) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let buffer = self.loader.load_file(url)?;
                let resource = parse(buffer)?;
                let len = self.resources.len();
                self.resources.push(Box::new(resource));
                *entry.insert(ResourceId(len as u32))
            }
        })
    }

    /// Get a reference to a queried face.
    ///
    /// # Panics
    /// This panics if no face with this id was loaded. This function should
    /// only be called with ids returned by [`query_face()`](Self::query_face).
    #[track_caller]
    pub fn face(&self, id: FaceId) -> &Face {
        self.faces[id.0 as usize].as_ref().expect("font face was not loaded")
    }

    /// Get a reference to a loaded resource.
    ///
    /// This panics if no resource with this id was loaded. This function should
    /// only be called with ids returned by
    /// [`load_resource()`](Self::load_resource).
    #[track_caller]
    pub fn resource<R: 'static>(&self, id: ResourceId) -> &R {
        self.resources[id.0 as usize]
            .downcast_ref()
            .expect("bad resource type")
    }
}

/// Loads fonts and resources from a remote or local source.
pub trait Loader {
    /// Descriptions of all font faces this loader serves.
    fn faces(&self) -> &[FaceInfo];

    /// Load the font face with the given index in [`faces()`](Self::faces).
    fn load_face(&mut self, idx: usize) -> Option<Buffer>;

    /// Load a file from a URL.
    fn load_file(&mut self, url: &str) -> Option<Buffer>;
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

/// A unique identifier for a loaded resource.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ResourceId(u32);

impl ResourceId {
    /// A blank initialization value.
    pub const MAX: Self = Self(u32::MAX);
}
