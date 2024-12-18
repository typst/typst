use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use comemo::Tracked;
use ecow::EcoString;
use siphasher::sip128::{Hasher128, SipHasher13};

use crate::diag::{format_xml_like_error, StrResult};
use crate::foundations::Bytes;
use crate::layout::Axes;
use crate::text::{
    Font, FontBook, FontFlags, FontStretch, FontStyle, FontVariant, FontWeight,
};
use crate::World;

/// A decoded SVG.
#[derive(Clone, Hash)]
pub struct SvgImage(Arc<Repr>);

/// The internal representation.
struct Repr {
    data: Bytes,
    size: Axes<f64>,
    flatten_text: bool,
    font_hash: u128,
    tree: usvg::Tree,
}

impl SvgImage {
    /// Decode an SVG image without fonts.
    #[comemo::memoize]
    pub fn new(data: Bytes) -> StrResult<SvgImage> {
        let tree =
            usvg::Tree::from_data(&data, &base_options()).map_err(format_usvg_error)?;
        Ok(Self(Arc::new(Repr {
            data,
            size: tree_size(&tree),
            font_hash: 0,
            flatten_text: false,
            tree,
        })))
    }

    /// Decode an SVG image with access to fonts.
    #[comemo::memoize]
    pub fn with_fonts(
        data: Bytes,
        world: Tracked<dyn World + '_>,
        flatten_text: bool,
        families: &[&str],
    ) -> StrResult<SvgImage> {
        let book = world.book();
        let resolver = Mutex::new(FontResolver::new(world, book, families));
        let tree = usvg::Tree::from_data(
            &data,
            &usvg::Options {
                font_resolver: usvg::FontResolver {
                    select_font: Box::new(|font, db| {
                        resolver.lock().unwrap().select_font(font, db)
                    }),
                    select_fallback: Box::new(|c, exclude_fonts, db| {
                        resolver.lock().unwrap().select_fallback(c, exclude_fonts, db)
                    }),
                },
                ..base_options()
            },
        )
        .map_err(format_usvg_error)?;
        let font_hash = resolver.into_inner().unwrap().finish();
        Ok(Self(Arc::new(Repr {
            data,
            size: tree_size(&tree),
            font_hash,
            flatten_text,
            tree,
        })))
    }

    /// The raw image data.
    pub fn data(&self) -> &Bytes {
        &self.0.data
    }

    /// The SVG's width in pixels.
    pub fn width(&self) -> f64 {
        self.0.size.x
    }

    /// Whether the SVG's text should be flattened.
    pub fn flatten_text(&self) -> bool {
        self.0.flatten_text
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
fn format_usvg_error(error: usvg::Error) -> EcoString {
    match error {
        usvg::Error::NotAnUtf8Str => "file is not valid utf-8".into(),
        usvg::Error::MalformedGZip => "file is not compressed correctly".into(),
        usvg::Error::ElementsLimitReached => "file is too large".into(),
        usvg::Error::InvalidSize => {
            "failed to parse SVG (width, height, or viewbox is invalid)".into()
        }
        usvg::Error::ParsingFailed(error) => format_xml_like_error("SVG", error),
    }
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
    to_id: HashMap<usize, Option<fontdb::ID>>,
    /// The reverse mapping.
    from_id: HashMap<fontdb::ID, Font>,
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
            to_id: HashMap::new(),
            from_id: HashMap::new(),
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
