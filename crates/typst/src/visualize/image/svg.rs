use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use comemo::Tracked;
use ecow::EcoString;

use crate::diag::{format_xml_like_error, StrResult};
use crate::foundations::Bytes;
use crate::layout::Axes;
use crate::text::{
    Font, FontBook, FontFlags, FontStretch, FontStyle, FontVariant, FontWeight,
};
use crate::visualize::Image;
use crate::World;

/// A decoded SVG.
#[derive(Clone, Hash)]
pub struct SvgImage(Arc<Repr>);

/// The internal representation.
struct Repr {
    data: Bytes,
    size: Axes<f64>,
    fontdb: fontdb::Database,
    font_hash: u128,
    tree: usvg::Tree,
}

impl SvgImage {
    /// Decode an SVG image without fonts.
    #[comemo::memoize]
    pub fn new(data: Bytes) -> StrResult<SvgImage> {
        let fontdb = fontdb::Database::new();
        let tree = usvg::Tree::from_data(&data, &options(), &fontdb)
            .map_err(format_usvg_error)?;
        Ok(Self(Arc::new(Repr {
            data,
            size: tree_size(&tree),
            font_hash: 0,
            tree,
            fontdb,
        })))
    }

    /// Decode an SVG image with access to fonts.
    #[comemo::memoize]
    pub fn with_fonts(
        data: Bytes,
        world: Tracked<dyn World + '_>,
        families: &[String],
    ) -> StrResult<SvgImage> {
        let book = world.book();
        let provider = TypstFontProvider::new(world, book, families);
        let tree = usvg::Tree::from_data(&data, &options(), &provider)
            .map_err(format_usvg_error)?;
        let font_hash = 0;
        Ok(Self(Arc::new(Repr {
            data,
            size: tree_size(&tree),
            font_hash,
            tree,
            fontdb: provider.db.into_inner(),
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

    /// The SVG's height in pixels.
    pub fn height(&self) -> f64 {
        self.0.size.y
    }

    /// Access the usvg tree.
    pub fn tree(&self) -> &usvg::Tree {
        &self.0.tree
    }

    /// Access the font database.
    pub fn fontdb(&self) -> &fontdb::Database {
        &self.0.fontdb
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

/// Provides Typst's fonts to usvg.
struct TypstFontProvider<'a> {
    book: &'a FontBook,
    world: Tracked<'a, dyn World + 'a>,
    families: &'a [String],
    db: RefCell<fontdb::Database>,
    to_id: RefCell<HashMap<usize, Option<fontdb::ID>>>,
    from_id: RefCell<HashMap<fontdb::ID, Font>>,
}

impl<'a> TypstFontProvider<'a> {
    /// Create a new font provider.
    fn new(
        world: Tracked<'a, dyn World + 'a>,
        book: &'a FontBook,
        families: &'a [String],
    ) -> Self {
        Self {
            book,
            world,
            families,
            db: RefCell::new(fontdb::Database::new()),
            to_id: RefCell::new(HashMap::new()),
            from_id: RefCell::new(HashMap::new()),
        }
    }
}

impl usvg::FontProvider for TypstFontProvider<'_> {
    fn find_font(&self, font: &usvg::Font) -> Option<fontdb::ID> {
        let variant = FontVariant {
            style: font.style().into(),
            weight: FontWeight::from_number(font.weight()),
            stretch: font.stretch().into(),
        };

        // Find a family that is available.
        font.families()
            .iter()
            .filter_map(|family| match family {
                usvg::FontFamily::Named(named) => Some(named),
                _ => None,
            })
            .chain(self.families)
            .filter_map(|named| self.book.select(&named.to_lowercase(), variant))
            .find_map(|index| self.get_or_load(index))
    }

    fn find_fallback_font(
        &self,
        c: char,
        base_font_id: fontdb::ID,
        _used_fonts: &[fontdb::ID],
    ) -> Option<fontdb::ID> {
        let index = {
            let from_id = self.from_id.borrow();
            let font = from_id.get(&base_font_id)?;
            let info = font.info();
            self.book.select_fallback(
                Some(info),
                info.variant,
                c.encode_utf8(&mut [0; 4]),
            )?
        };
        self.get_or_load(index)
    }

    fn with_database(&self, f: &mut dyn FnMut(&fontdb::Database)) {
        f(&self.db.borrow());
    }
}

impl TypstFontProvider<'_> {
    /// Tries to retrieve the ID for the index or loads the font, allocating
    /// a new ID.
    fn get_or_load(&self, index: usize) -> Option<fontdb::ID> {
        *self
            .to_id
            .borrow_mut()
            .entry(index)
            .or_insert_with(|| self.load(index))
    }

    /// Tries to load the font with the given index in the font book into the
    /// database and returns its ID.
    fn load(&self, index: usize) -> Option<fontdb::ID> {
        let font = self.world.font(index)?;
        let info = font.info();
        let variant = info.variant;
        let id = self.db.borrow_mut().push_face_info(fontdb::FaceInfo {
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
            // TODO: Round to closest.
            stretch: match variant.stretch {
                FontStretch::ULTRA_CONDENSED => ttf_parser::Width::UltraCondensed,
                FontStretch::EXTRA_CONDENSED => ttf_parser::Width::ExtraCondensed,
                FontStretch::CONDENSED => ttf_parser::Width::Condensed,
                FontStretch::SEMI_CONDENSED => ttf_parser::Width::SemiCondensed,
                FontStretch::SEMI_EXPANDED => ttf_parser::Width::SemiExpanded,
                FontStretch::EXPANDED => ttf_parser::Width::Expanded,
                FontStretch::EXTRA_EXPANDED => ttf_parser::Width::ExtraExpanded,
                FontStretch::ULTRA_EXPANDED => ttf_parser::Width::UltraExpanded,
                _ => ttf_parser::Width::Normal,
            },
            monospaced: info.flags.contains(FontFlags::MONOSPACE),
        });
        self.from_id.borrow_mut().insert(id, font);
        Some(id)
    }
}

/// The conversion options.
static OPTIONS: Lazy<usvg::Options> = Lazy::new(|| usvg::Options {
    // Disable usvg's default to "Times New Roman". Instead, we default to
    // the empty family and later, when we traverse the SVG, we check for
    // empty and non-existing family names and replace them with the true
    // fallback family. This way, we can memoize SVG decoding with and
    // without fonts if the SVG does not contain text.
    font_family: String::new(),

    // We override the DPI here so that we get the correct the size when
    // scaling the image to its natural size.
    dpi: Image::DEFAULT_DPI as f32,

    // Override usvg's resource loading defaults.
    resources_dir: None,
    image_href_resolver: ImageHrefResolver {
        resolve_data: ImageHrefResolver::default_data_resolver(),
        resolve_string: Box::new(|_, _| None),
    },

    ..Default::default()
});

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
