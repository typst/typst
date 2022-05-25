use std::borrow::Cow;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use super::{FileHash, Loader};
use crate::font::FaceInfo;
use crate::util::PathExt;

/// Loads fonts and files from an in-memory storage.
#[derive(Default)]
pub struct MemLoader {
    faces: Vec<FaceInfo>,
    files: HashMap<PathBuf, Cow<'static, [u8]>>,
}

impl MemLoader {
    /// Create a new from-memory loader.
    pub fn new() -> Self {
        Self { faces: vec![], files: HashMap::new() }
    }

    /// Builder-style variant of [`insert`](Self::insert).
    pub fn with<P, D>(mut self, path: P, data: D) -> Self
    where
        P: AsRef<Path>,
        D: Into<Cow<'static, [u8]>>,
    {
        self.insert(path, data);
        self
    }

    /// Insert a path-file mapping. If the data forms a font, then that font
    /// will be available for layouting.
    ///
    /// The data can either be owned or referenced, but the latter only if its
    /// lifetime is `'static`.
    pub fn insert<P, D>(&mut self, path: P, data: D)
    where
        P: AsRef<Path>,
        D: Into<Cow<'static, [u8]>>,
    {
        let path = path.as_ref().normalize();
        let data = data.into();
        self.faces.extend(FaceInfo::from_data(&path, &data));
        self.files.insert(path, data);
    }
}

impl Loader for MemLoader {
    fn faces(&self) -> &[FaceInfo] {
        &self.faces
    }

    fn resolve(&self, path: &Path) -> io::Result<FileHash> {
        let norm = path.normalize();
        if self.files.contains_key(&norm) {
            Ok(FileHash(fxhash::hash64(&norm)))
        } else {
            Err(io::ErrorKind::NotFound.into())
        }
    }

    fn load(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.files
            .get(&path.normalize())
            .map(|cow| cow.clone().into_owned())
            .ok_or_else(|| io::ErrorKind::NotFound.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::FontVariant;

    #[test]
    fn test_recognize_and_load_font() {
        let data = include_bytes!("../../fonts/PTSans-Regular.ttf");
        let path = Path::new("PTSans.ttf");
        let loader = MemLoader::new().with(path, &data[..]);

        // Test that the found was found.
        let info = &loader.faces[0];
        assert_eq!(info.path, path);
        assert_eq!(info.index, 0);
        assert_eq!(info.family, "PT Sans");
        assert_eq!(info.variant, FontVariant::default());
        assert_eq!(loader.faces.len(), 1);

        // Test that the file can be loaded.
        assert_eq!(
            loader.load(Path::new("directory/../PTSans.ttf")).unwrap(),
            data
        );
    }
}
