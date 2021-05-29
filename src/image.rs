//! Image handling.

use std::collections::{hash_map::Entry, HashMap};
use std::fmt::{self, Debug, Formatter};
use std::io::Cursor;
use std::path::Path;

use image::io::Reader as ImageReader;
use image::{DynamicImage, GenericImageView, ImageFormat};
use serde::{Deserialize, Serialize};

use crate::loading::{FileHash, Loader};

/// A loaded image.
pub struct Image {
    /// The original format the image was encoded in.
    pub format: ImageFormat,
    /// The decoded image.
    pub buf: DynamicImage,
}

impl Image {
    /// Parse an image from raw data in a supported format (PNG or JPEG).
    ///
    /// The image format is determined automatically.
    pub fn parse(data: &[u8]) -> Option<Self> {
        let cursor = Cursor::new(data);
        let reader = ImageReader::new(cursor).with_guessed_format().ok()?;
        let format = reader.format()?;
        let buf = reader.decode().ok()?;
        Some(Self { format, buf })
    }

    /// The width of the image.
    pub fn width(&self) -> u32 {
        self.buf.width()
    }

    /// The height of the image.
    pub fn height(&self) -> u32 {
        self.buf.height()
    }
}

impl Debug for Image {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Image")
            .field("format", &self.format)
            .field("color", &self.buf.color())
            .field("width", &self.width())
            .field("height", &self.height())
            .finish()
    }
}

/// Caches decoded images.
pub struct ImageCache {
    /// Loaded images indexed by [`ImageId`].
    images: Vec<Image>,
    /// Maps from file hashes to ids of decoded images.
    map: HashMap<FileHash, ImageId>,
    /// Callback for loaded images.
    on_load: Option<Box<dyn Fn(ImageId, &Image)>>,
}

impl ImageCache {
    /// Create a new, empty image cache.
    pub fn new() -> Self {
        Self {
            images: vec![],
            map: HashMap::new(),
            on_load: None,
        }
    }

    /// Load and decode an image file from a path.
    pub fn load(&mut self, loader: &mut dyn Loader, path: &Path) -> Option<ImageId> {
        Some(match self.map.entry(loader.resolve(path)?) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let buffer = loader.load_file(path)?;
                let image = Image::parse(&buffer)?;
                let id = ImageId(self.images.len() as u32);
                if let Some(callback) = &self.on_load {
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
    /// only be called with ids returned by [`load()`](Self::load).
    #[track_caller]
    pub fn get(&self, id: ImageId) -> &Image {
        &self.images[id.0 as usize]
    }

    /// Register a callback which is invoked each time an image is loaded.
    pub fn on_load<F>(&mut self, f: F)
    where
        F: Fn(ImageId, &Image) + 'static,
    {
        self.on_load = Some(Box::new(f));
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
