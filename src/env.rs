//! Environment interactions.

use std::any::Any;
use std::collections::{hash_map::Entry, HashMap};
use std::fmt::{self, Debug, Formatter};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use fontdock::{FaceFromVec, FaceId, FontSource};
use image::io::Reader as ImageReader;
use image::{DynamicImage, GenericImageView, ImageFormat};
use ttf_parser::Face;

#[cfg(feature = "fs")]
use fontdock::{FsIndex, FsSource};

/// Encapsulates all environment dependencies (fonts, resources).
#[derive(Debug)]
pub struct Env {
    /// Loads fonts from a dynamic font source.
    pub fonts: FontLoader,
    /// Loads resource from the file system.
    pub resources: ResourceLoader,
}

impl Env {
    /// Create an empty environment for testing purposes.
    pub fn blank() -> Self {
        struct BlankSource;

        impl FontSource for BlankSource {
            type Face = FaceBuf;

            fn load(&self, _: FaceId) -> Option<Self::Face> {
                None
            }
        }

        Self {
            fonts: FontLoader::new(Box::new(BlankSource), vec![]),
            resources: ResourceLoader::new(),
        }
    }
}

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

/// Loads resource from the file system.
pub struct ResourceLoader {
    paths: HashMap<PathBuf, ResourceId>,
    entries: Vec<Box<dyn Any>>,
}

/// A unique identifier for a resource.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ResourceId(usize);

impl ResourceLoader {
    /// Create a new resource loader.
    pub fn new() -> Self {
        Self { paths: HashMap::new(), entries: vec![] }
    }

    /// Load a resource from a path and parse it.
    pub fn load<P, F, R>(&mut self, path: P, parse: F) -> Option<(ResourceId, &R)>
    where
        P: AsRef<Path>,
        F: FnOnce(Vec<u8>) -> Option<R>,
        R: 'static,
    {
        let path = path.as_ref();
        let id = match self.paths.entry(path.to_owned()) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let data = fs::read(path).ok()?;
                let resource = parse(data)?;
                let len = self.entries.len();
                self.entries.push(Box::new(resource));
                *entry.insert(ResourceId(len))
            }
        };

        Some((id, self.loaded(id)))
    }

    /// Retrieve a previously loaded resource by its id.
    ///
    /// # Panics
    /// This panics if no resource with this id was loaded.
    #[track_caller]
    pub fn loaded<R: 'static>(&self, id: ResourceId) -> &R {
        self.entries[id.0].downcast_ref().expect("bad resource type")
    }
}

impl Debug for ResourceLoader {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_set().entries(self.paths.keys()).finish()
    }
}

/// A loaded image resource.
pub struct ImageResource {
    /// The original format the image was encoded in.
    pub format: ImageFormat,
    /// The decoded image.
    pub buf: DynamicImage,
}

impl ImageResource {
    /// Parse an image resource from raw data in a supported format.
    ///
    /// The image format is determined automatically.
    pub fn parse(data: Vec<u8>) -> Option<Self> {
        let reader = ImageReader::new(Cursor::new(data)).with_guessed_format().ok()?;
        let format = reader.format()?;
        let buf = reader.decode().ok()?;
        Some(Self { format, buf })
    }
}

impl Debug for ImageResource {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let (width, height) = self.buf.dimensions();
        f.debug_struct("ImageResource")
            .field("format", &self.format)
            .field("color", &self.buf.color())
            .field("width", &width)
            .field("height", &height)
            .finish()
    }
}
