//! Loading of fonts by queries.

use std::fmt::{self, Debug, Formatter};
use std::cell::{RefCell, Ref};
use std::collections::HashMap;
use crate::font::{Font, FontProvider, FontFamily, FontInfo};


/// Serves matching fonts given a query.
pub struct FontLoader<'p> {
    /// The font providers.
    providers: &'p [Box<dyn FontProvider + 'p>],
    /// All available fonts indexed by provider.
    provider_fonts: Vec<&'p [FontInfo]>,
    /// The internal state.
    state: RefCell<FontLoaderState<'p>>,
}

/// Internal state of the font loader (wrapped in a RefCell).
struct FontLoaderState<'p> {
    /// The loaded fonts along with their external indices.
    fonts: Vec<(Option<usize>, Font)>,
    /// Allows to retrieve cached results for queries.
    query_cache: HashMap<FontQuery<'p>, usize>,
    /// Allows to lookup fonts by their infos.
    info_cache: HashMap<&'p FontInfo, usize>,
    /// Indexed by outside and indices maps to internal indices.
    inner_index: Vec<usize>,
}

impl<'p> FontLoader<'p> {
    /// Create a new font loader.
    pub fn new(providers: &'p [Box<dyn FontProvider + 'p>]) -> FontLoader {
        let provider_fonts = providers.iter()
            .map(|prov| prov.available()).collect();

        FontLoader {
            providers,
            provider_fonts,
            state: RefCell::new(FontLoaderState {
                query_cache: HashMap::new(),
                info_cache: HashMap::new(),
                inner_index: vec![],
                fonts: vec![],
            }),
        }
    }

    /// Return the best matching font and it's index (if there is any) given the query.
    pub fn get(&self, query: FontQuery<'p>) -> Option<(usize, Ref<Font>)> {
        // Check if we had the exact same query before.
        let state = self.state.borrow();
        if let Some(&index) = state.query_cache.get(&query) {
            // That this is the query cache means it must has an index as we've served it before.
            let extern_index = state.fonts[index].0.unwrap();
            let font = Ref::map(state, |s| &s.fonts[index].1);

            return Some((extern_index, font));
        }
        drop(state);

        // Go over all font infos from all font providers that match the query.
        for family in query.families {
            for (provider, infos) in self.providers.iter().zip(&self.provider_fonts) {
                for info in infos.iter() {
                    // Check whether this info matches the query.
                    if Self::matches(query, family, info) {
                        let mut state = self.state.borrow_mut();

                        // Check if we have already loaded this font before.
                        // Otherwise we'll fetch the font from the provider.
                        let index = if let Some(&index) = state.info_cache.get(info) {
                            index
                        } else if let Some(mut source) = provider.get(info) {
                            // Read the font program into a vec.
                            let mut program = Vec::new();
                            source.read_to_end(&mut program).ok()?;

                            // Create a font from it.
                            let font = Font::new(program).ok()?;

                            // Insert it into the storage.
                            let index = state.fonts.len();
                            state.info_cache.insert(info, index);
                            state.fonts.push((None, font));

                            index
                        } else {
                            continue;
                        };

                        // Check whether this font has the character we need.
                        let has_char = state.fonts[index].1.mapping.contains_key(&query.character);
                        if has_char {
                            // We can take this font, so we store the query.
                            state.query_cache.insert(query, index);

                            // Now we have to find out the external index of it, or assign a new
                            // one if it has not already one.
                            let maybe_extern_index = state.fonts[index].0;
                            let extern_index = maybe_extern_index.unwrap_or_else(|| {
                                // We have to assign an external index before serving.
                                let extern_index = state.inner_index.len();
                                state.inner_index.push(index);
                                state.fonts[index].0 =  Some(extern_index);
                                extern_index
                            });

                            // Release the mutable borrow and borrow immutably.
                            drop(state);
                            let font = Ref::map(self.state.borrow(), |s| &s.fonts[index].1);

                            // Finally we can return it.
                            return Some((extern_index, font));
                        }
                    }
                }
            }
        }

        None
    }

    /// Return a loaded font at an index. Panics if the index is out of bounds.
    pub fn get_with_index(&self, index: usize) -> Ref<Font> {
        let state = self.state.borrow();
        let internal = state.inner_index[index];
        Ref::map(state, |s| &s.fonts[internal].1)
    }

    /// Return the list of fonts.
    pub fn into_fonts(self) -> Vec<Font> {
        // Sort the fonts by external key so that they are in the correct order.
        let mut fonts = self.state.into_inner().fonts;
        fonts.sort_by_key(|&(maybe_index, _)| match maybe_index {
            Some(index) => index as isize,
            None => -1,
        });

        // Remove the fonts that are not used from the outside
        fonts.into_iter().filter_map(|(maybe_index, font)| {
            maybe_index.map(|_| font)
        }).collect()
    }

    /// Check whether the query and the current family match the info.
    fn matches(query: FontQuery, family: &FontFamily, info: &FontInfo) -> bool {
        info.families.contains(family)
          && info.italic == query.italic && info.bold == query.bold
    }
}

impl Debug for FontLoader<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let state = self.state.borrow();
        f.debug_struct("FontLoader")
            .field("providers", &self.providers.len())
            .field("provider_fonts", &self.provider_fonts)
            .field("fonts", &state.fonts)
            .field("query_cache", &state.query_cache)
            .field("info_cache", &state.info_cache)
            .field("inner_index", &state.inner_index)
            .finish()
    }
}

/// A query for a font with specific properties.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct FontQuery<'p> {
    /// A fallback list of font families to accept. The first family in this list, that also
    /// satisfies the other conditions, shall be returned.
    pub families: &'p [FontFamily],
    /// Whether the font shall be in italics.
    pub italic: bool,
    /// Whether the font shall be in boldface.
    pub bold: bool,
    /// Which character we need.
    pub character: char,
}
