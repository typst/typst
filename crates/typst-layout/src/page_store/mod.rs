//! Disk-backed page storage for memory-efficient document processing.
//!
//! After the convergence loop produces a final document (with all page frames
//! in memory for introspection), this module serializes page frames to a
//! temporary file and drops them from memory. During PDF export, pages are
//! read back one at a time, keeping only a single page's worth of frame data
//! in memory.
//!
//! # Architecture
//!
//! Page frames contain types that can't be directly serialized (Arc<Font>,
//! Content in Tags, etc.). We handle this by:
//!
//! - **Fonts**: Referenced by a hash of their data bytes + collection index.
//!   A `FontRegistry` resolves these back to `Font` objects during deserialization.
//! - **Images**: Referenced by a hash of their data. An `ImageRegistry` resolves
//!   these back to `Image` objects.
//! - **Tags (Content)**: Stored in a separate in-memory `TagStore` keyed by
//!   sequential ID. Serialized frames reference tags by ID.
//! - **Tiling patterns**: Contain nested Frames. Stored in a separate in-memory
//!   store (rare in practice).
//! - **Everything else**: Serialized directly via serde/bincode.

mod types;
mod store;
mod registry;
mod converter;
mod frame_store;

pub use store::DiskPageStore;
pub use registry::{FontRegistry, ImageRegistry};
pub use converter::FrameConverter;
pub use frame_store::{DiskFrameStore, SyncDiskFrameStore};
