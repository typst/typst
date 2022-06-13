//! Image handling.

use std::collections::{hash_map::Entry, HashMap};
use std::ffi::OsStr;
use std::fmt::{self, Debug, Formatter};
use std::io;
use std::path::Path;
use std::sync::Arc;

use image::io::Reader as ImageReader;
use image::{DynamicImage, ImageFormat};

use crate::loading::{FileHash, Loader};

/// A unique identifier for a loaded image.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ImageId(u32);

impl ImageId {
    /// Create an image id from the raw underlying value.
    ///
    /// This should only be called with values returned by
    /// [`into_raw`](Self::into_raw).
    pub const fn from_raw(v: u32) -> Self {
        Self(v)
    }

    /// Convert into the raw underlying value.
    pub const fn into_raw(self) -> u32 {
        self.0
    }
}

/// Storage for loaded and decoded images.
pub struct ImageStore {
    loader: Arc<dyn Loader>,
    files: HashMap<FileHash, ImageId>,
    images: Vec<Image>,
}

impl ImageStore {
    /// Create a new, empty image store.
    pub fn new(loader: Arc<dyn Loader>) -> Self {
        Self {
            loader,
            files: HashMap::new(),
            images: vec![],
        }
    }

    /// Get a reference to a loaded image.
    ///
    /// This panics if no image with this `id` was loaded. This function should
    /// only be called with ids returned by this store's [`load()`](Self::load)
    /// method.
    #[track_caller]
    pub fn get(&self, id: ImageId) -> &Image {
        &self.images[id.0 as usize]
    }

    /// Load and decode an image file from a path relative to the compilation
    /// environment's root.
    pub fn load(&mut self, path: &Path) -> io::Result<ImageId> {
        let hash = self.loader.resolve(path)?;
        Ok(*match self.files.entry(hash) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let buffer = self.loader.load(path)?;
                let ext = path.extension().and_then(OsStr::to_str).unwrap_or_default();
                let image = Image::parse(&buffer, ext)?;
                let id = ImageId(self.images.len() as u32);
                self.images.push(image);
                entry.insert(id)
            }
        })
    }
}

/// A loaded image.
#[derive(Debug)]
pub enum Image {
    /// A pixel raster format, like PNG or JPEG.
    Raster(RasterImage),
    /// An SVG vector graphic.
    Svg(Svg),
}

impl Image {
    /// Parse an image from raw data. The file extension is used as a hint for
    /// which error message describes the problem best.
    pub fn parse(data: &[u8], ext: &str) -> io::Result<Self> {
        match Svg::parse(data) {
            Ok(svg) => return Ok(Self::Svg(svg)),
            Err(err) if matches!(ext, "svg" | "svgz") => return Err(err),
            Err(_) => {}
        }

        match RasterImage::parse(data) {
            Ok(raster) => return Ok(Self::Raster(raster)),
            Err(err) if matches!(ext, "png" | "jpg" | "jpeg" | "gif") => return Err(err),
            Err(_) => {}
        }

        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unknown image format",
        ))
    }

    /// The width of the image in pixels.
    pub fn width(&self) -> u32 {
        match self {
            Self::Raster(image) => image.width(),
            Self::Svg(image) => image.width(),
        }
    }

    /// The height of the image in pixels.
    pub fn height(&self) -> u32 {
        match self {
            Self::Raster(image) => image.height(),
            Self::Svg(image) => image.height(),
        }
    }
}

/// A raster image, supported through the image crate.
pub struct RasterImage {
    /// The original format the image was encoded in.
    pub format: ImageFormat,
    /// The decoded image.
    pub buf: DynamicImage,
}

impl RasterImage {
    /// Parse an image from raw data in a supported format (PNG, JPEG or GIF).
    ///
    /// The image format is determined automatically.
    pub fn parse(data: &[u8]) -> io::Result<Self> {
        let cursor = io::Cursor::new(data);
        let reader = ImageReader::new(cursor).with_guessed_format()?;
        let format = reader
            .format()
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidData))?;

        let buf = reader
            .decode()
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        Ok(Self { format, buf })
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

impl Debug for RasterImage {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Image")
            .field("format", &self.format)
            .field("color", &self.buf.color())
            .field("width", &self.width())
            .field("height", &self.height())
            .finish()
    }
}

/// An SVG image, supported through the usvg crate.
pub struct Svg(pub usvg::Tree);

impl Svg {
    /// Parse an SVG file from a data buffer. This also handles `.svgz`
    /// compressed files.
    pub fn parse(data: &[u8]) -> io::Result<Self> {
        let usvg_opts = usvg::Options::default();
        usvg::Tree::from_data(data, &usvg_opts.to_ref())
            .map(Self)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }

    /// The width of the image in rounded-up nominal SVG pixels.
    pub fn width(&self) -> u32 {
        self.0.svg_node().size.width().ceil() as u32
    }

    /// The height of the image in rounded-up nominal SVG pixels.
    pub fn height(&self) -> u32 {
        self.0.svg_node().size.height().ceil() as u32
    }
}

impl Debug for Svg {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Svg")
            .field("width", &self.0.svg_node().size.width())
            .field("height", &self.0.svg_node().size.height())
            .field("viewBox", &self.0.svg_node().view_box)
            .finish()
    }
}
