use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use comemo::Tracked;
use ecow::EcoString;
use once_cell::sync::Lazy;
use siphasher::sip128::Hasher128;
use usvg::{ImageHrefResolver, Node, PostProcessingSteps, TreeParsing, TreePostProc};

use crate::diag::{format_xml_like_error, StrResult};
use crate::foundations::Bytes;
use crate::layout::Axes;
use crate::text::{FontVariant, FontWeight};
use crate::visualize::Image;
use crate::World;

/// A decoded SVG.
#[derive(Clone, Hash)]
pub struct SvgImage(Arc<Repr>);

/// The internal representation.
struct Repr {
    data: Bytes,
    size: Axes<f64>,
    font_hash: u128,
    tree: sync::SyncTree,
}

impl SvgImage {
    /// Decode an SVG image without fonts.
    #[comemo::memoize]
    pub fn new(data: Bytes) -> StrResult<SvgImage> {
        let tree = usvg::Tree::from_data(&data, &OPTIONS).map_err(format_usvg_error)?;
        Ok(Self(Arc::new(Repr {
            data,
            size: tree_size(&tree),
            font_hash: 0,
            // Safety: We just created the tree and hold the only reference.
            tree: unsafe { sync::SyncTree::new(tree) },
        })))
    }

    /// Decode an SVG image with access to fonts.
    #[comemo::memoize]
    pub fn with_fonts(
        data: Bytes,
        world: Tracked<dyn World + '_>,
        families: &[String],
    ) -> StrResult<SvgImage> {
        let mut tree =
            usvg::Tree::from_data(&data, &OPTIONS).map_err(format_usvg_error)?;
        let mut font_hash = 0;
        if tree.has_text_nodes() {
            let (fontdb, hash) = load_svg_fonts(world, &mut tree, families);
            tree.postprocess(PostProcessingSteps::default(), &fontdb);
            font_hash = hash;
        }
        tree.calculate_bounding_boxes();
        Ok(Self(Arc::new(Repr {
            data,
            size: tree_size(&tree),
            font_hash,
            // Safety: We just created the tree and hold the only reference.
            tree: unsafe { sync::SyncTree::new(tree) },
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

    /// Performs an operation with the usvg tree.
    ///
    /// This makes the tree uniquely available to the current thread and blocks
    /// other accesses to it.
    ///
    /// # Safety
    /// The caller may not hold any references to `Rc`s contained in the usvg
    /// Tree after `f` returns.
    ///
    /// # Why is it unsafe?
    /// Sadly, usvg's Tree is neither `Sync` nor `Send` because it uses `Rc`
    /// internally and sending a tree to another thread could result in data
    /// races when an `Rc`'s ref-count is modified from two threads at the same
    /// time.
    ///
    /// However, access to the tree is actually safe if we don't clone `Rc`s /
    /// only clone them while holding a mutex and drop all clones before the
    /// mutex is released. Sadly, we can't enforce this variant at the type
    /// system level. Therefore, access is guarded by this function (which makes
    /// it reasonable hard to keep references around) and its usage still
    /// remains `unsafe` (because it's still possible to have `Rc`s escape).
    ///
    /// See also: <https://github.com/RazrFalcon/resvg/issues/544>
    pub unsafe fn with<F>(&self, f: F)
    where
        F: FnOnce(&usvg::Tree),
    {
        self.0.tree.with(f)
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

/// Discover and load the fonts referenced by an SVG.
fn load_svg_fonts(
    world: Tracked<dyn World + '_>,
    tree: &mut usvg::Tree,
    families: &[String],
) -> (fontdb::Database, u128) {
    let book = world.book();
    let mut fontdb = fontdb::Database::new();
    let mut hasher = siphasher::sip128::SipHasher13::new();
    let mut loaded = HashMap::<usize, Option<String>>::new();

    // Loads a font into the database and return it's usvg-compatible name.
    let mut load_into_db = |id: usize| -> Option<String> {
        loaded
            .entry(id)
            .or_insert_with(|| {
                let font = world.font(id)?;
                fontdb.load_font_source(fontdb::Source::Binary(Arc::new(
                    font.data().clone(),
                )));
                font.data().hash(&mut hasher);
                font.find_name(ttf_parser::name_id::TYPOGRAPHIC_FAMILY)
                    .or_else(|| font.find_name(ttf_parser::name_id::FAMILY))
            })
            .clone()
    };

    // Determine the best font for each text node.
    for child in &mut tree.root.children {
        traverse_svg(child, &mut |node| {
            let usvg::Node::Text(ref mut text) = node else { return };
            for chunk in &mut text.chunks {
                'spans: for span in &mut chunk.spans {
                    let Some(text) = chunk.text.get(span.start..span.end) else {
                        continue;
                    };
                    let variant = FontVariant {
                        style: span.font.style.into(),
                        weight: FontWeight::from_number(span.font.weight),
                        stretch: span.font.stretch.into(),
                    };

                    // Find a font that covers the whole text among the span's fonts
                    // and the current document font families.
                    let mut like = None;
                    for family in span.font.families.iter().chain(families) {
                        let Some(id) = book.select(&family.to_lowercase(), variant)
                        else {
                            continue;
                        };
                        let Some(info) = book.info(id) else { continue };
                        like.get_or_insert(info);

                        if text.chars().all(|c| info.coverage.contains(c as u32)) {
                            if let Some(usvg_family) = load_into_db(id) {
                                span.font.families = vec![usvg_family];
                                continue 'spans;
                            }
                        }
                    }

                    // If we didn't find a match, select a fallback font.
                    if let Some(id) = book.select_fallback(like, variant, text) {
                        if let Some(usvg_family) = load_into_db(id) {
                            span.font.families = vec![usvg_family];
                        }
                    }
                }
            }
        });
    }

    (fontdb, hasher.finish128().as_u128())
}

/// Search for all font families referenced by an SVG.
fn traverse_svg<F>(node: &mut usvg::Node, f: &mut F)
where
    F: FnMut(&mut usvg::Node),
{
    f(node);

    node.subroots_mut(|subroot| {
        for child in &mut subroot.children {
            traverse_svg(child, f);
        }
    });

    if let Node::Group(ref mut group) = node {
        for child in &mut group.children {
            traverse_svg(child, f);
        }
    }
}

/// The ceiled pixel size of an SVG.
fn tree_size(tree: &usvg::Tree) -> Axes<f64> {
    Axes::new(tree.size.width() as f64, tree.size.height() as f64)
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

mod sync {
    use std::sync::Mutex;

    /// A synchronized wrapper around a `usvg::Tree`.
    pub struct SyncTree(Mutex<usvg::Tree>);

    impl SyncTree {
        /// Create a new synchronized tree.
        ///
        /// # Safety
        /// The tree must be completely owned by `tree`, there may not be any
        /// other references to `Rc`s contained in it.
        pub unsafe fn new(tree: usvg::Tree) -> Self {
            Self(Mutex::new(tree))
        }

        /// Perform an operation with the usvg tree.
        ///
        /// # Safety
        /// The caller may not hold any references to `Rc`s contained in
        /// the usvg Tree after returning.
        pub unsafe fn with<F>(&self, f: F)
        where
            F: FnOnce(&usvg::Tree),
        {
            let tree = self.0.lock().unwrap();
            f(&tree)
        }
    }

    // Safety: usvg's Tree is only non-Sync and non-Send because it uses `Rc`
    // internally. By wrapping it in a mutex and forbidding outstanding
    // references to the tree to remain after a `with` call, we guarantee that
    // no two threads try to change a ref-count at the same time.
    unsafe impl Sync for SyncTree {}
    unsafe impl Send for SyncTree {}
}
