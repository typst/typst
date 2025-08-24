use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use comemo::Tracked;
use ecow::{EcoString, eco_format};
use rustc_hash::FxHashMap;
use siphasher::sip128::{Hasher128, SipHasher13};
use typst_syntax::{Span, VirtualPath};

use crate::World;
use crate::diag::{FileError, LoadError, LoadResult, ReportPos, format_xml_like_error};
use crate::foundations::Bytes;
use crate::layout::Axes;
use crate::visualize::VectorFormat;
use crate::visualize::image::raster::{ExchangeFormat, RasterFormat};
use crate::visualize::image::{ImageFormat, determine_format_from_path};

use crate::text::{
    Font, FontBook, FontFlags, FontStretch, FontStyle, FontVariant, FontWeight,
};

/// A decoded SVG.
#[derive(Clone, Hash)]
pub struct SvgImage(Arc<Repr>);

/// The internal representation.
struct Repr {
    data: Bytes,
    size: Axes<f64>,
    font_hash: u128,
    tree: usvg::Tree,
}

impl SvgImage {
    /// Decode an SVG image without fonts.
    #[comemo::memoize]
    #[typst_macros::time(name = "load svg")]
    pub fn new(data: Bytes) -> LoadResult<SvgImage> {
        let tree =
            usvg::Tree::from_data(&data, &base_options()).map_err(format_usvg_error)?;
        Ok(Self(Arc::new(Repr { data, size: tree_size(&tree), font_hash: 0, tree })))
    }

    /// Decode an SVG image with access to fonts.
    #[comemo::memoize]
    #[typst_macros::time(name = "load svg")]
    pub fn with_fonts(
        span: &Span,
        svg_path: &EcoString,
        data: Bytes,
        world: Tracked<dyn World + '_>,
        families: &[&str],
    ) -> LoadResult<SvgImage> {
        let book = world.book();
        let font_resolver = Mutex::new(FontResolver::new(world, book, families));
        let image_resolver = Mutex::new(ImageResolver::new(world, span, svg_path));
        let tree = usvg::Tree::from_data(
            &data,
            &usvg::Options {
                font_resolver: usvg::FontResolver {
                    select_font: Box::new(|font, db| {
                        font_resolver.lock().unwrap().select_font(font, db)
                    }),
                    select_fallback: Box::new(|c, exclude_fonts, db| {
                        font_resolver.lock().unwrap().select_fallback(
                            c,
                            exclude_fonts,
                            db,
                        )
                    }),
                },
                image_href_resolver: usvg::ImageHrefResolver {
                    resolve_data: usvg::ImageHrefResolver::default_data_resolver(),
                    resolve_string: Box::new(|href, _opts| {
                        image_resolver.lock().unwrap().load(href)
                    }),
                },
                ..base_options()
            },
        )
        .map_err(format_usvg_error)?;
        let font_hash = font_resolver.into_inner().unwrap().finish();
        let image_resolve_error = image_resolver.lock().unwrap().error.clone();
        match image_resolve_error {
            Some(err) => Err(err),
            None => {
                Ok(Self(Arc::new(Repr { data, size: tree_size(&tree), font_hash, tree })))
            }
        }
    }

    /// The raw image data.
    pub fn data(&self) -> &Bytes {
        &self.0.data
    }

    /// The SVG's width in pixels.
    pub fn width(&self) -> f64 {
        self.0.size.x
    }

    /// The SVG's height in pixels.
    pub fn height(&self) -> f64 {
        self.0.size.y
    }

    /// Accesses the usvg tree.
    pub fn tree(&self) -> &usvg::Tree {
        &self.0.tree
    }
}

impl Hash for Repr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // An SVG might contain fonts, which must be incorporated into the hash.
        // We can't hash a usvg tree directly, but the raw SVG data + a hash of
        // all used fonts gives us something similar.
        self.data.hash(state);
        self.font_hash.hash(state);
    }
}

/// The base conversion options, to be extended with font-related options
/// because those can change across the document.
fn base_options() -> usvg::Options<'static> {
    usvg::Options {
        // Disable usvg's default to "Times New Roman".
        font_family: String::new(),

        // We don't override the DPI here, because we already
        // force the image into the corresponding DPI by setting
        // the width and height. Changing the DPI only trips up
        // the logic in `resvg`.

        // Override usvg's resource loading defaults.
        resources_dir: None,
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: usvg::ImageHrefResolver::default_data_resolver(),
            resolve_string: Box::new(|_, _| None),
        },

        ..Default::default()
    }
}

/// The pixel size of an SVG.
fn tree_size(tree: &usvg::Tree) -> Axes<f64> {
    Axes::new(tree.size().width() as f64, tree.size().height() as f64)
}

/// Format the user-facing SVG decoding error message.
fn format_usvg_error(error: usvg::Error) -> LoadError {
    let error = match error {
        usvg::Error::NotAnUtf8Str => "file is not valid utf-8",
        usvg::Error::MalformedGZip => "file is not compressed correctly",
        usvg::Error::ElementsLimitReached => "file is too large",
        usvg::Error::InvalidSize => "width, height, or viewbox is invalid",
        usvg::Error::ParsingFailed(error) => return format_xml_like_error("SVG", error),
    };
    LoadError::new(ReportPos::None, "failed to parse SVG", error)
}

/// Provides Typst's fonts to usvg.
struct FontResolver<'a> {
    /// Typst's font book.
    book: &'a FontBook,
    /// The world we use to load fonts.
    world: Tracked<'a, dyn World + 'a>,
    /// The active list of font families at the location of the SVG.
    families: &'a [&'a str],
    /// A mapping from Typst font indices to fontdb IDs.
    to_id: FxHashMap<usize, Option<fontdb::ID>>,
    /// The reverse mapping.
    from_id: FxHashMap<fontdb::ID, Font>,
    /// Accumulates a hash of all used fonts.
    hasher: SipHasher13,
}

impl<'a> FontResolver<'a> {
    /// Create a new font provider.
    fn new(
        world: Tracked<'a, dyn World + 'a>,
        book: &'a FontBook,
        families: &'a [&'a str],
    ) -> Self {
        Self {
            book,
            world,
            families,
            to_id: FxHashMap::default(),
            from_id: FxHashMap::default(),
            hasher: SipHasher13::new(),
        }
    }

    /// Returns a hash of all used fonts.
    fn finish(self) -> u128 {
        self.hasher.finish128().as_u128()
    }
}

impl FontResolver<'_> {
    /// Select a font.
    fn select_font(
        &mut self,
        font: &usvg::Font,
        db: &mut Arc<fontdb::Database>,
    ) -> Option<fontdb::ID> {
        let variant = FontVariant {
            style: font.style().into(),
            weight: FontWeight::from_number(font.weight()),
            stretch: font.stretch().into(),
        };

        // Find a family that is available.
        font.families()
            .iter()
            .filter_map(|family| match family {
                usvg::FontFamily::Named(named) => Some(named.as_str()),
                // We don't support generic families at the moment.
                _ => None,
            })
            .chain(self.families.iter().copied())
            .filter_map(|named| self.book.select(&named.to_lowercase(), variant))
            .find_map(|index| self.get_or_load(index, db))
    }

    /// Select a fallback font.
    fn select_fallback(
        &mut self,
        c: char,
        exclude_fonts: &[fontdb::ID],
        db: &mut Arc<fontdb::Database>,
    ) -> Option<fontdb::ID> {
        // Get the font info of the originally selected font.
        let like = exclude_fonts
            .first()
            .and_then(|first| self.from_id.get(first))
            .map(|font| font.info());

        // usvg doesn't provide a variant in the fallback handler, but
        // `exclude_fonts` is actually never empty in practice. Still, we
        // prefer to fall back to the default variant rather than panicking
        // in case that changes in the future.
        let variant = like.map(|info| info.variant).unwrap_or_default();

        // Select the font.
        let index =
            self.book.select_fallback(like, variant, c.encode_utf8(&mut [0; 4]))?;

        self.get_or_load(index, db)
    }

    /// Tries to retrieve the ID for the index or loads the font, allocating
    /// a new ID.
    fn get_or_load(
        &mut self,
        index: usize,
        db: &mut Arc<fontdb::Database>,
    ) -> Option<fontdb::ID> {
        self.to_id
            .get(&index)
            .copied()
            .unwrap_or_else(|| self.load(index, db))
    }

    /// Tries to load the font with the given index in the font book into the
    /// database and returns its ID.
    fn load(
        &mut self,
        index: usize,
        db: &mut Arc<fontdb::Database>,
    ) -> Option<fontdb::ID> {
        let font = self.world.font(index)?;
        let info = font.info();
        let variant = info.variant;
        let id = Arc::make_mut(db).push_face_info(fontdb::FaceInfo {
            id: fontdb::ID::dummy(),
            source: fontdb::Source::Binary(Arc::new(font.data().clone())),
            index: font.index(),
            families: vec![(
                info.family.clone(),
                ttf_parser::Language::English_UnitedStates,
            )],
            post_script_name: String::new(),
            style: match variant.style {
                FontStyle::Normal => fontdb::Style::Normal,
                FontStyle::Italic => fontdb::Style::Italic,
                FontStyle::Oblique => fontdb::Style::Oblique,
            },
            weight: fontdb::Weight(variant.weight.to_number()),
            stretch: match variant.stretch.round() {
                FontStretch::ULTRA_CONDENSED => ttf_parser::Width::UltraCondensed,
                FontStretch::EXTRA_CONDENSED => ttf_parser::Width::ExtraCondensed,
                FontStretch::CONDENSED => ttf_parser::Width::Condensed,
                FontStretch::SEMI_CONDENSED => ttf_parser::Width::SemiCondensed,
                FontStretch::NORMAL => ttf_parser::Width::Normal,
                FontStretch::SEMI_EXPANDED => ttf_parser::Width::SemiExpanded,
                FontStretch::EXPANDED => ttf_parser::Width::Expanded,
                FontStretch::EXTRA_EXPANDED => ttf_parser::Width::ExtraExpanded,
                FontStretch::ULTRA_EXPANDED => ttf_parser::Width::UltraExpanded,
                _ => unreachable!(),
            },
            monospaced: info.flags.contains(FontFlags::MONOSPACE),
        });

        font.hash(&mut self.hasher);

        self.to_id.insert(index, Some(id));
        self.from_id.insert(id, font);

        Some(id)
    }
}

/// Resolves linked images in an SVG.
/// (Linked SVG images from an SVG are not supported yet.)
struct ImageResolver<'a> {
    /// The world we use to check if resolved images in the SVG are within the project root.
    world: Tracked<'a, dyn World + 'a>,
    /// The span of the file loading the SVG.
    span: &'a Span,
    /// Path to the SVG file or an empty string if the SVG is given as bytes.
    svg_path: &'a EcoString,
    /// The first error that occurred when loading a linked image, if any.
    error: Option<LoadError>,
}

impl<'a> ImageResolver<'a> {
    fn new(
        world: Tracked<'a, dyn World + 'a>,
        span: &'a Span,
        svg_path: &'a EcoString,
    ) -> Self {
        Self { world, span, svg_path, error: None }
    }

    /// Load a linked image or return None if a previous image caused an error.
    fn load(&mut self, href: &str) -> Option<usvg::ImageKind> {
        if self.error.is_none() {
            match self.load_or_error(href) {
                Ok(image) => Some(image),
                Err(err) => {
                    self.error = Some(LoadError::new(
                        ReportPos::None,
                        eco_format!("failed to load linked image {} in SVG", href),
                        err,
                    ));
                    None
                }
            }
        } else {
            None
        }
    }

    /// Load a linked image or return an error message string.
    fn load_or_error(&mut self, href: &str) -> Result<usvg::ImageKind, EcoString> {
        // Exit early if the href is an URL.
        if let Some(pos) = href.find("://") {
            let scheme = &href[..pos];
            if scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
            {
                return Err(EcoString::from("URLs are not allowed"));
            }
        }

        // Resolve the path to the linked image.
        let href_path = VirtualPath::new(self.svg_path.as_str()).join(href);
        let href_file = self
            .span
            .resolve_path(&href_path.as_rooted_path().to_string_lossy())?;

        // Load image if file can be accessed.
        match self.world.file(href_file) {
            Ok(bytes) => {
                let arc_data = Arc::new(bytes.to_vec());
                let format = match determine_format_from_path(href) {
                    Some(format) => Some(format),
                    None => ImageFormat::detect(&arc_data),
                };
                match format {
                    None => Err(EcoString::from("could not determine image format")),
                    Some(ImageFormat::Vector(vector_format)) => match vector_format {
                        VectorFormat::Svg => {
                            Err(EcoString::from("SVG images are not supported yet"))
                        }
                        VectorFormat::Pdf => {
                            Err(EcoString::from("PDF documents are not supported"))
                        }
                    },
                    Some(ImageFormat::Raster(raster_format)) => match raster_format {
                        RasterFormat::Exchange(exchange_format) => {
                            match exchange_format {
                                ExchangeFormat::Gif => Ok(usvg::ImageKind::GIF(arc_data)),
                                ExchangeFormat::Jpg => {
                                    Ok(usvg::ImageKind::JPEG(arc_data))
                                }
                                ExchangeFormat::Png => Ok(usvg::ImageKind::PNG(arc_data)),
                                ExchangeFormat::Webp => {
                                    Ok(usvg::ImageKind::WEBP(arc_data))
                                }
                            }
                        }
                        RasterFormat::Pixel(_) => {
                            Err(EcoString::from("pixel formats are not supported"))
                        }
                    },
                }
            }
            Err(err) => Err(match err {
                FileError::NotFound(path) => {
                    eco_format!("file not found, searched at {}", path.display())
                }
                FileError::AccessDenied => EcoString::from("access denied"),
                FileError::IsDirectory => EcoString::from("is a directory"),
                FileError::Other(Some(msg)) => msg,
                FileError::Other(None) => EcoString::from("unspecified error"),
                _ => EcoString::from("unexpected error"),
            }),
        }
    }
}
