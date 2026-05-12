//! Automatic font downloading.

use std::fmt;
use std::io;
use std::path::PathBuf;

use crate::downloader::Downloader;

pub struct FontDownloader {
    downloader: Box<dyn Downloader>,
}

#[cfg(feature = "font-downloader")]
impl FontDownloader {
    pub fn new(downloader: impl Downloader + 'static) -> Self {
        Self { downloader: Box::new(downloader) }
    }

    /// Returns the platform-specific default cache directory for downloaded fonts:
    /// - Linux:   `~/.cache/typst/fonts/google/`
    /// - macOS:   `~/Library/Caches/typst/fonts/google/`
    /// - Windows: `%LOCALAPPDATA%\typst\fonts\google\`
    pub fn default_cache_dir() -> Option<PathBuf> {
        dirs::cache_dir().map(|d| d.join("typst/fonts/google"))
    }

    /// Downloads all available variants of `family` into the platform cache directory.
    ///
    /// Returns the paths of all font files.
    pub fn download_family(
        &self,
        family: &str,
    ) -> Result<Vec<PathBuf>, FontDownloadError> {
        let cache_dir = Self::default_cache_dir().ok_or(FontDownloadError::NoCacheDir)?;
        let family_dir = cache_dir.join(family_slug(family));

        let css = self.fetch_css(family)?;
        let urls = font_urls_from_css(&css);

        if urls.is_empty() {
            return Err(FontDownloadError::NoVariantsFound(family.to_owned()));
        }

        let mut paths = Vec::new();
        let mut first = true;
        for url in urls {
            let filename =
                url_filename(&url).ok_or(FontDownloadError::CssParseFailure)?;
            let dest = family_dir.join(filename);

            if dest.exists() {
                paths.push(dest);
                continue;
            }

            // ProgressDownloader doesn't really work for our case since we make many
            // separate downloads per font, so just show first download instead.
            let data = if first {
                first = false;
                std::fs::create_dir_all(&family_dir)?;
                self.downloader
                    .download(&family.to_owned(), &url)
                    .map_err(FontDownloadError::Http)?
            } else {
                self.downloader.download(&(), &url).map_err(FontDownloadError::Http)?
            };

            // Atomic write
            let tmp = dest.with_file_name(format!("{filename}.tmp"));
            std::fs::write(&tmp, &data)?;
            std::fs::rename(&tmp, &dest)?;

            paths.push(dest);
        }

        Ok(paths)
    }

    fn fetch_css(&self, family: &str) -> Result<String, FontDownloadError> {
        let encoded = family.replace(' ', "+");
        let url = format!(
            "https://fonts.googleapis.com/css2?family={encoded}:\
             ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;\
             1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900\
             &display=swap"
        );

        let bytes = self.downloader.download(&(), &url).map_err(|e| {
            // Google Fonts returns 400 Bad Request for invalid family names
            if e.kind() == io::ErrorKind::InvalidInput {
                FontDownloadError::NoVariantsFound(family.to_owned())
            } else {
                FontDownloadError::Http(e)
            }
        })?;

        String::from_utf8(bytes).map_err(|_| FontDownloadError::CssParseFailure)
    }
}

/// Errors that can occur while downloading a font family.
#[derive(Debug)]
pub enum FontDownloadError {
    /// A network error fetching the CSS index or a font file.
    Http(io::Error),
    /// The CSS response from Google Fonts could not be parsed.
    CssParseFailure,
    /// Google Fonts returned no font variants for the given family name.
    NoVariantsFound(String),
    /// A filesystem I/O error.
    Io(io::Error),
    /// The platform cache directory could not be determined.
    NoCacheDir,
}

impl fmt::Display for FontDownloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http(e) => write!(f, "network error: {e}"),
            Self::CssParseFailure => {
                write!(f, "could not parse Google Fonts CSS response")
            }
            Self::NoVariantsFound(name) => {
                write!(f, "no variants found for font family '{name}' on Google Fonts")
            }
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::NoCacheDir => write!(f, "could not determine font cache directory"),
        }
    }
}

impl From<io::Error> for FontDownloadError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

/// Extracts font file URLs from Google Fonts CSS response.
fn font_urls_from_css(css: &str) -> Vec<String> {
    let mut urls = Vec::new();
    let mut rest = css;
    while let Some(start) = rest.find("url(") {
        rest = &rest[start + 4..];
        let Some(end) = rest.find(')') else { break };
        let inner = rest[..end].trim().trim_matches(|c| c == '"' || c == '\'');
        let after = rest[end + 1..].trim_start();
        if after.starts_with("format('truetype')") && !inner.is_empty() {
            urls.push(inner.to_owned());
        }
        rest = &rest[end + 1..];
    }
    urls
}

fn url_filename(url: &str) -> Option<&str> {
    url.rsplit('/').next().filter(|s| !s.is_empty())
}

/// Converts family name to slug suitable for filesystem, e.g. "Open Sans" -> "open-sans".
fn family_slug(family: &str) -> String {
    family
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_owned()
}
