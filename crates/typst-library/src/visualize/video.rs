//! Video handling.

use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use ecow::EcoString;
use typst_syntax::Spanned;
use typst_utils::LazyHash;

use crate::diag::{At, SourceResult, bail};
use crate::engine::Engine;
use crate::foundations::{
    Bytes, Derived, Packed, Smart, StyleChain, Synthesize, elem, scope,
};
use crate::introspection::{Locatable, Tagged};
use crate::layout::{Length, Rel, Sizing};
use crate::loading::{DataSource, Load, Loaded};
use crate::model::Figurable;
use crate::text::{LocalName, Locale};
use crate::visualize::image::{
    ExchangeFormat, Image, ImageFit, ImageKind, RasterFormat, RasterImage,
};

/// A video element that embeds a video into the document.
///
/// The video is embedded as a Screen annotation in PDF output, with the
/// poster image displayed as a fallback. Non-PDF outputs show the poster.
///
/// You can wrap the video in a [`figure`] to give it a number and caption.
///
/// # Example
/// ```example
/// #video("demo.mp4", "poster.png", width: 80%)
/// ```
#[elem(scope, Locatable, Tagged, Synthesize, LocalName, Figurable)]
pub struct VideoElem {
    /// Path to a video file (MP4).
    #[required]
    #[parse(
        let source = args.expect::<Spanned<DataSource>>("source")?;
        let loaded = source.load(engine.world)?;
        Derived::new(source.v, loaded)
    )]
    pub source: Derived<DataSource, Loaded>,

    /// Poster/fallback image shown in non-supporting viewers.
    #[required]
    #[parse(
        let poster_source = args.expect::<Spanned<DataSource>>("poster")?;
        let poster_loaded = poster_source.load(engine.world)?;
        Derived::new(poster_source.v, poster_loaded)
    )]
    pub poster: Derived<DataSource, Loaded>,

    /// The video's width.
    pub width: Smart<Rel<Length>>,

    /// The video's height.
    pub height: Sizing,

    /// Alternative text for accessibility.
    pub alt: Option<EcoString>,

    /// How the poster should be scaled within the area.
    #[default(ImageFit::Cover)]
    pub fit: ImageFit,

    /// The locale of this element (used for the alternative description).
    #[internal]
    #[synthesized]
    pub locale: Locale,
}

impl Synthesize for Packed<VideoElem> {
    fn synthesize(&mut self, _: &mut Engine, styles: StyleChain) -> SourceResult<()> {
        self.locale = Some(Locale::get_in(styles));
        Ok(())
    }
}

#[scope]
impl VideoElem {}

impl Packed<VideoElem> {
    /// Decodes the video element into a `Video` struct.
    pub fn decode(&self, _engine: &mut Engine, styles: StyleChain) -> SourceResult<Video> {
        let span = self.span();
        let video_loaded = &self.source.derived;
        let poster_loaded = &self.poster.derived;

        // Validate video data is MP4.
        let video_data = &video_loaded.data;
        if !is_mp4(video_data) {
            bail!(span, "video must be in MP4 format");
        }

        // Decode the poster image as a raster image (PNG/JPG/GIF/WebP).
        let poster_format = ExchangeFormat::detect(
            &poster_loaded.data,
        )
        .ok_or("unknown poster image format")
        .at(span)?;

        let raster_format = RasterFormat::Exchange(poster_format);
        let raster = RasterImage::new(poster_loaded.data.clone(), raster_format, Smart::Auto)
            .at(span)?;
        let poster = Image::new(ImageKind::Raster(raster), None, Smart::Auto);

        // Derive a filename from the source path.
        let filename = match &self.source.source {
            DataSource::Path(path) => {
                let resolved = path.resolve_if_some(span.id()).ok();
                resolved
                    .map(|p| p.vpath().get_without_slash().to_string())
                    .unwrap_or_else(|| "video.mp4".to_string())
            }
            DataSource::Bytes(_) => "video.mp4".to_string(),
        };

        Ok(Video::new(
            video_data.clone(),
            "video/mp4".into(),
            filename.into(),
            poster,
            self.alt.get_cloned(styles),
        ))
    }
}

impl LocalName for Packed<VideoElem> {
    const KEY: &'static str = "figure";
}

impl Figurable for Packed<VideoElem> {}

/// Check if data is an MP4 file (ftyp box).
fn is_mp4(data: &[u8]) -> bool {
    data.len() >= 12 && &data[4..8] == b"ftyp"
}

/// A video with its data, poster image, and metadata.
///
/// Values of this type are cheap to clone and hash.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Video(Arc<LazyHash<VideoInner>>);

#[derive(Hash)]
struct VideoInner {
    data: Bytes,
    mime_type: EcoString,
    filename: EcoString,
    poster: Image,
    alt: Option<EcoString>,
}

impl Video {
    /// Create a new video.
    #[comemo::memoize]
    fn new(
        data: Bytes,
        mime_type: EcoString,
        filename: EcoString,
        poster: Image,
        alt: Option<EcoString>,
    ) -> Video {
        Video(Arc::new(LazyHash::new(VideoInner {
            data,
            mime_type,
            filename,
            poster,
            alt,
        })))
    }

    /// The raw video data.
    pub fn data(&self) -> &Bytes {
        &self.0.data
    }

    /// The MIME type.
    pub fn mime_type(&self) -> &str {
        &self.0.mime_type
    }

    /// The filename.
    pub fn filename(&self) -> &str {
        &self.0.filename
    }

    /// The poster image (used for layout sizing and fallback display).
    pub fn poster(&self) -> &Image {
        &self.0.poster
    }

    /// The alt text.
    pub fn alt(&self) -> Option<&str> {
        self.0.alt.as_deref()
    }
}

impl Debug for Video {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Video")
            .field("mime_type", &self.mime_type())
            .field("filename", &self.filename())
            .field("poster_width", &self.poster().width())
            .field("poster_height", &self.poster().height())
            .field("alt", &self.alt())
            .finish()
    }
}
