//! Caching for incremental compilation.

use std::collections::HashMap;

use crate::layout::{Frame, Regions};

/// A cache for incremental compilation.
#[derive(Default, Debug, Clone)]
pub struct Cache {
    /// A map that holds the layouted nodes from past compilations.
    pub frames: HashMap<u64, FramesEntry>,
}

impl Cache {
    /// Create a new, empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear the cache.
    pub fn clear(&mut self) {
        self.frames.clear();
    }
}

/// Frames from past compilations and checks for their validity in future
/// compilations.
#[derive(Debug, Clone)]
pub struct FramesEntry {
    /// The regions in which these frames are valid.
    pub regions: Regions,
    /// Cached frames for a node.
    pub frames: Vec<Frame>,
}
