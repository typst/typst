use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use memmap2::Mmap;
use same_file::Handle;
use ttf_parser::{name_id, Face};
use walkdir::WalkDir;

use super::{FileId, Loader};
use crate::font::{FaceInfo, FontStretch, FontStyle, FontVariant, FontWeight};
use crate::util::PathExt;

/// Loads fonts and images from the local file system.
///
/// _This is only available when the `fs` feature is enabled._
#[derive(Debug, Default, Clone)]
pub struct FsLoader {
    faces: Vec<FaceInfo>,
    paths: RefCell<HashMap<FileId, PathBuf>>,
}

impl FsLoader {
    /// Create a new loader without any fonts.
    pub fn new() -> Self {
        Self { faces: vec![], paths: RefCell::default() }
    }

    /// Builder-style variant of `search_system`.
    pub fn with_system(mut self) -> Self {
        self.search_system();
        self
    }

    /// Builder-style variant of `search_path`.
    pub fn with_path(mut self, dir: impl AsRef<Path>) -> Self {
        self.search_path(dir);
        self
    }

    /// Builder-style method to wrap the loader in an [`Rc`] to make it usable
    /// with the [`Context`](crate::Context).
    pub fn wrap(self) -> Rc<Self> {
        Rc::new(self)
    }

    /// Search for fonts in the operating system's font directories.
    pub fn search_system(&mut self) {
        self.search_system_impl();
    }

    /// Search for all fonts at a path.
    ///
    /// If the path is a directory, all contained fonts will be searched for
    /// recursively.
    pub fn search_path(&mut self, dir: impl AsRef<Path>) {
        let walk = WalkDir::new(dir)
            .follow_links(true)
            .sort_by(|a, b| a.file_name().cmp(b.file_name()))
            .into_iter()
            .filter_map(|e| e.ok());

        for entry in walk {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                match ext {
                    #[rustfmt::skip]
                    "ttf" | "otf" | "TTF" | "OTF" |
                    "ttc" | "otc" | "TTC" | "OTC" => {
                        self.search_file(path).ok();
                    }
                    _ => {}
                }
            }
        }
    }

    /// Resolve a file id for a path.
    pub fn resolve(&self, path: &Path) -> io::Result<FileId> {
        let file = File::open(path)?;
        let meta = file.metadata()?;
        if meta.is_file() {
            let handle = Handle::from_file(file)?;
            let id = FileId(fxhash::hash64(&handle));
            self.paths.borrow_mut().insert(id, path.normalize());
            Ok(id)
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "not a file"))
        }
    }

    /// Return the path of a resolved file.
    pub fn path(&self, id: FileId) -> Ref<Path> {
        Ref::map(self.paths.borrow(), |paths| paths[&id].as_path())
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    fn search_system_impl(&mut self) {
        self.search_path("/usr/share/fonts");
        self.search_path("/usr/local/share/fonts");

        if let Some(dir) = dirs::font_dir() {
            self.search_path(dir);
        }
    }

    #[cfg(target_os = "macos")]
    fn search_system_impl(&mut self) {
        self.search_path("/Library/Fonts");
        self.search_path("/Network/Library/Fonts");
        self.search_path("/System/Library/Fonts");

        if let Some(dir) = dirs::font_dir() {
            self.search_path(dir);
        }
    }

    #[cfg(windows)]
    fn search_system_impl(&mut self) {
        let windir =
            std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());

        self.search_path(Path::new(&windir).join("Fonts"));

        if let Some(roaming) = dirs::config_dir() {
            self.search_path(roaming.join("Microsoft\\Windows\\Fonts"));
        }

        if let Some(local) = dirs::cache_dir() {
            self.search_path(local.join("Microsoft\\Windows\\Fonts"));
        }
    }

    /// Index the font faces in the file at the given path.
    ///
    /// The file may form a font collection and contain multiple font faces,
    /// which will then all be indexed.
    fn search_file(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();
        let path = path.strip_prefix(".").unwrap_or(path);

        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        for i in 0 .. ttf_parser::fonts_in_collection(&mmap).unwrap_or(1) {
            let face = Face::from_slice(&mmap, i)
                .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

            self.parse_face(path, &face, i)?;
        }

        Ok(())
    }

    /// Parse a single face and insert it into the `families`. This either
    /// merges with an existing family entry if they have the same trimmed
    /// family name, or creates a new one.
    fn parse_face(&mut self, path: &Path, face: &Face<'_>, index: u32) -> io::Result<()> {
        fn find_name(face: &Face, name_id: u16) -> Option<String> {
            face.names().find_map(|entry| {
                (entry.name_id() == name_id).then(|| entry.to_string()).flatten()
            })
        }

        let family = find_name(face, name_id::TYPOGRAPHIC_FAMILY)
            .or_else(|| find_name(face, name_id::FAMILY))
            .ok_or("unknown font family")
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

        let variant = FontVariant {
            style: match (face.is_italic(), face.is_oblique()) {
                (false, false) => FontStyle::Normal,
                (true, _) => FontStyle::Italic,
                (_, true) => FontStyle::Oblique,
            },
            weight: FontWeight::from_number(face.weight().to_number()),
            stretch: FontStretch::from_number(face.width().to_number()),
        };

        let file = self.resolve(path)?;
        self.faces.push(FaceInfo { file, index, family, variant });

        Ok(())
    }
}

impl Loader for FsLoader {
    fn faces(&self) -> &[FaceInfo] {
        &self.faces
    }

    fn resolve_from(&self, base: FileId, path: &Path) -> io::Result<FileId> {
        let full = self.paths.borrow()[&base]
            .parent()
            .expect("base is a file")
            .join(path);
        self.resolve(&full)
    }

    fn load_file(&self, id: FileId) -> io::Result<Vec<u8>> {
        fs::read(&self.paths.borrow()[&id])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_font_dir() {
        let map = FsLoader::new().with_path("fonts").paths.into_inner();
        let mut paths: Vec<_> = map.into_iter().map(|p| p.1).collect();
        paths.sort();

        assert_eq!(paths, [
            Path::new("fonts/EBGaramond-Bold.ttf"),
            Path::new("fonts/EBGaramond-BoldItalic.ttf"),
            Path::new("fonts/EBGaramond-Italic.ttf"),
            Path::new("fonts/EBGaramond-Regular.ttf"),
            Path::new("fonts/Inconsolata-Bold.ttf"),
            Path::new("fonts/Inconsolata-Regular.ttf"),
            Path::new("fonts/LatinModernMath.otf"),
            Path::new("fonts/NotoSansArabic-Regular.ttf"),
            Path::new("fonts/NotoSerifCJKsc-Regular.otf"),
            Path::new("fonts/NotoSerifHebrew-Bold.ttf"),
            Path::new("fonts/NotoSerifHebrew-Regular.ttf"),
            Path::new("fonts/PTSans-Regular.ttf"),
            Path::new("fonts/TwitterColorEmoji.ttf"),
        ]);
    }
}
