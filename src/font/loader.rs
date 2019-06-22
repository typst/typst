//! Loads fonts matching queries.

use std::cell::{RefCell, Ref};
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};

use super::{Font, FontInfo, FontClass, FontProvider};


/// Serves fonts matching queries.
pub struct FontLoader<'p> {
    /// The font providers.
    providers: Vec<&'p (dyn FontProvider + 'p)>,
    /// The fonts available from each provider (indexed like `providers`).
    provider_fonts: Vec<&'p [FontInfo]>,
    /// The internal state. Uses interior mutability because the loader works behind
    /// an immutable reference to ease usage.
    state: RefCell<FontLoaderState<'p>>,
}

/// Internal state of the font loader (seperated to wrap it in a `RefCell`).
struct FontLoaderState<'p> {
    /// The loaded fonts alongside their external indices. Some fonts may not have external indices
    /// because they were loaded but did not contain the required character. However, these are
    /// still stored because they may be needed later. The index is just set to `None` then.
    fonts: Vec<(Option<usize>, Font)>,
    /// Allows to retrieve a font (index) quickly if a query was submitted before.
    query_cache: HashMap<FontQuery, usize>,
    /// Allows to re-retrieve loaded fonts by their info instead of loading them again.
    info_cache: HashMap<&'p FontInfo, usize>,
    /// Indexed by external indices (the ones inside the tuples in the `fonts` vector) and maps to
    /// internal indices (the actual indices into the vector).
    inner_index: Vec<usize>,
}

impl<'p> FontLoader<'p> {
    /// Create a new font loader using a set of providers.
    #[inline]
    pub fn new<P: 'p>(providers: &'p [P]) -> FontLoader<'p> where P: AsRef<dyn FontProvider + 'p> {
        let providers: Vec<_> = providers.iter().map(|p| p.as_ref()).collect();
        let provider_fonts = providers.iter().map(|prov| prov.available()).collect();

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

    /// Returns the font (and its index) best matching the query, if there is any.
    pub fn get(&self, query: FontQuery) -> Option<(usize, Ref<Font>)> {
        // Load results from the cache, if we had the exact same query before.
        let state = self.state.borrow();
        if let Some(&index) = state.query_cache.get(&query) {
            // The font must have an external index already because it is in the query cache.
            // It has been served before.
            let extern_index = state.fonts[index].0.unwrap();
            let font = Ref::map(state, |s| &s.fonts[index].1);

            return Some((extern_index, font));
        }
        drop(state);

        // The outermost loop goes over the fallbacks because we want to serve the font that matches
        // the first possible class.
        for class in &query.fallback {
            // For each class now go over all font infos from all font providers.
            for (provider, infos) in self.providers.iter().zip(&self.provider_fonts) {
                for info in infos.iter() {
                    let matches = info.classes.contains(class)
                        && query.classes.iter().all(|class| info.classes.contains(class));

                    // Proceed only if this font matches the query up to now.
                    if matches {
                        let mut state = self.state.borrow_mut();

                        // Check if we have already loaded this font before, otherwise, we will load
                        // it from the provider. Anyway, have it stored and find out its internal
                        // index.
                        let index = if let Some(&index) = state.info_cache.get(info) {
                            index
                        } else if let Some(mut source) = provider.get(info) {
                            // Read the font program into a vector and parse it.
                            let mut program = Vec::new();
                            source.read_to_end(&mut program).ok()?;
                            let font = Font::new(program).ok()?;

                            // Insert it into the storage and cache it by its info.
                            let index = state.fonts.len();
                            state.info_cache.insert(info, index);
                            state.fonts.push((None, font));

                            index
                        } else {
                            // Strangely, this provider lied and cannot give us the promised font.
                            continue;
                        };

                        // Proceed if this font has the character we need.
                        let has_char = state.fonts[index].1.mapping.contains_key(&query.character);
                        if has_char {
                            // This font is suitable, thus we cache the query result.
                            state.query_cache.insert(query, index);

                            // Now we have to find out the external index of it or assign a new one
                            // if it has none.
                            let external_index = state.fonts[index].0.unwrap_or_else(|| {
                                // We have to assign an external index before serving.
                                let new_index = state.inner_index.len();
                                state.inner_index.push(index);
                                state.fonts[index].0 =  Some(new_index);
                                new_index
                            });

                            // Release the mutable borrow to be allowed to borrow immutably.
                            drop(state);

                            // Finally, get a reference to the actual font.
                            let font = Ref::map(self.state.borrow(), |s| &s.fonts[index].1);
                            return Some((external_index, font));
                        }
                    }
                }
            }
        }

        // Not a single match!
        None
    }

    /// Return the font previously loaded at this index. Panics if the index is not assigned.
    #[inline]
    pub fn get_with_index(&self, index: usize) -> Ref<Font> {
        let state = self.state.borrow();
        let internal = state.inner_index[index];
        Ref::map(state, |s| &s.fonts[internal].1)
    }

    /// Move the whole list of fonts out.
    pub fn into_fonts(self) -> Vec<Font> {
        // Sort the fonts by external index so that they are in the correct order. All fonts that
        // were cached but not used by the outside are sorted to the back and are removed in the
        // next step.
        let mut fonts = self.state.into_inner().fonts;
        fonts.sort_by_key(|&(maybe_index, _)| match maybe_index {
            Some(index) => index,
            None => std::usize::MAX,
        });

        // Remove the fonts that are not used from the outside.
        fonts.into_iter().filter_map(|(maybe_index, font)| {
            if maybe_index.is_some() { Some(font) } else { None }
        }).collect()
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
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct FontQuery {
    /// Which character is needed.
    pub character: char,
    /// Which classes the font has to be part of.
    pub classes: Vec<FontClass>,
    /// A sequence of classes. The font matching the leftmost class in this sequence
    /// should be returned.
    pub fallback: Vec<FontClass>,
}
