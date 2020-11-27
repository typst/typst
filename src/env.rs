//! Environment interactions.

use std::any::Any;
use std::cell::RefCell;
use std::collections::{hash_map::Entry, HashMap};
use std::fmt::{self, Debug, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::font::FontLoader;

/// A reference-counted shared environment.
pub type SharedEnv = Rc<RefCell<Env>>;

/// Encapsulates all environment dependencies (fonts, resources).
#[derive(Debug)]
pub struct Env {
    /// Loads fonts from a dynamic font source.
    pub fonts: FontLoader,
    /// Loads resource from the file system.
    pub resources: ResourceLoader,
}

/// Loads resource from the file system.
pub struct ResourceLoader {
    paths: HashMap<PathBuf, ResourceId>,
    entries: Vec<Box<dyn Any>>,
}

/// A unique identifier for a resource.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ResourceId(usize);

impl ResourceLoader {
    /// Create a new resource loader.
    pub fn new() -> Self {
        Self { paths: HashMap::new(), entries: vec![] }
    }

    /// Load a resource from a path.
    pub fn load<R: 'static>(
        &mut self,
        path: impl AsRef<Path>,
        parse: impl FnOnce(Vec<u8>) -> Option<R>,
    ) -> Option<(ResourceId, &R)> {
        let path = path.as_ref();
        let id = match self.paths.entry(path.to_owned()) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let id = *entry.insert(ResourceId(self.entries.len()));
                let data = fs::read(path).ok()?;
                let resource = parse(data)?;
                self.entries.push(Box::new(resource));
                id
            }
        };

        Some((id, self.get_loaded(id)))
    }

    /// Retrieve a previously loaded resource by its id.
    ///
    /// # Panics
    /// This panics if no resource with this id was loaded.
    pub fn get_loaded<R: 'static>(&self, id: ResourceId) -> &R {
        self.entries[id.0].downcast_ref().expect("bad resource type")
    }
}

impl Debug for ResourceLoader {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_set().entries(self.paths.keys()).finish()
    }
}
