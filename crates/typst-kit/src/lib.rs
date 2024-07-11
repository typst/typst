//! Typst-kit contains various default implementations of functionality used in
//! typst-cli. It is intended as a single source of truth for things like font
//! searching, package downloads and more. Each component of typst-kit is
//! optional but enabled by default.
//!
//! # Components
//! - [fonts] contains a default implementation for searching local and system
//!   installed fonts.

pub mod fonts;
