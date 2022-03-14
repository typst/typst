use std::fs::{self, File};
use std::io;
use std::path::Path;
use std::sync::Arc;

use memmap2::Mmap;
use same_file::Handle;
use walkdir::WalkDir;

use super::{FileHash, Loader};
use crate::font::FaceInfo;

/// Loads fonts and files from the local file system.
///
/// _This is only available when the `fs` feature is enabled._
pub struct FsLoader {
    faces: Vec<FaceInfo>,
}

impl FsLoader {
    /// Create a new loader without any fonts.
    pub fn new() -> Self {
        Self { faces: vec![] }
    }

    /// Builder-style variant of [`search_system`](Self::search_system).
    pub fn with_system(mut self) -> Self {
        self.search_system();
        self
    }

    /// Builder-style variant of [`search_path`](Self::search_path).
    pub fn with_path(mut self, dir: impl AsRef<Path>) -> Self {
        self.search_path(dir);
        self
    }

    /// Builder-style method to wrap the loader in an [`Arc`] to make it usable
    /// with the [`Context`](crate::Context).
    pub fn wrap(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Search for fonts in the operating system's font directories.
    pub fn search_system(&mut self) {
        self.search_system_impl();
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

    /// Search for all fonts at a path.
    ///
    /// If the path is a directory, all contained fonts will be searched for
    /// recursively.
    pub fn search_path(&mut self, path: impl AsRef<Path>) {
        let walk = WalkDir::new(path)
            .follow_links(true)
            .sort_by(|a, b| a.file_name().cmp(b.file_name()))
            .into_iter()
            .filter_map(|e| e.ok());

        for entry in walk {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if matches!(
                    ext,
                    "ttf" | "otf" | "TTF" | "OTF" | "ttc" | "otc" | "TTC" | "OTC",
                ) {
                    self.search_file(path);
                }
            }
        }
    }

    /// Index the font faces in the file at the given path.
    ///
    /// The file may form a font collection and contain multiple font faces,
    /// which will then all be indexed.
    fn search_file(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref();
        let path = path.strip_prefix(".").unwrap_or(path);
        if let Ok(file) = File::open(path) {
            if let Ok(mmap) = unsafe { Mmap::map(&file) } {
                self.faces.extend(FaceInfo::parse(path, &mmap));
            }
        }
    }
}

impl Loader for FsLoader {
    fn faces(&self) -> &[FaceInfo] {
        &self.faces
    }

    fn resolve(&self, path: &Path) -> io::Result<FileHash> {
        let file = File::open(path)?;
        let meta = file.metadata()?;
        if meta.is_file() {
            let handle = Handle::from_file(file)?;
            Ok(FileHash(fxhash::hash64(&handle)))
        } else {
            Err(io::ErrorKind::NotFound.into())
        }
    }

    fn load(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_font_dir() {
        let faces = FsLoader::new().with_path("fonts").faces;
        let mut paths: Vec<_> = faces.into_iter().map(|info| info.path).collect();
        paths.sort();

        assert_eq!(paths, [
            Path::new("fonts/CMU-Serif-Bold.ttf"),
            Path::new("fonts/CMU-Serif-Regular.ttf"),
            Path::new("fonts/IBMPlexMono-Regular.ttf"),
            Path::new("fonts/IBMPlexSans-Bold.ttf"),
            Path::new("fonts/IBMPlexSans-BoldItalic.ttf"),
            Path::new("fonts/IBMPlexSans-Italic.ttf"),
            Path::new("fonts/IBMPlexSans-Regular.ttf"),
            Path::new("fonts/IBMPlexSerif-Regular.ttf"),
            Path::new("fonts/LatinModernMath.otf"),
            Path::new("fonts/NotoSansArabic-Regular.ttf"),
            Path::new("fonts/NotoSerifCJKsc-Regular.otf"),
            Path::new("fonts/NotoSerifHebrew-Bold.ttf"),
            Path::new("fonts/NotoSerifHebrew-Regular.ttf"),
            Path::new("fonts/PTSans-Regular.ttf"),
            Path::new("fonts/Roboto-Regular.ttf"),
            Path::new("fonts/TwitterColorEmoji.ttf"),
        ]);
    }
}
