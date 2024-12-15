//! Typst-kit contains various default implementations of functionality used in
//! typst-cli. It is intended as a single source of truth for things like font
//! searching, package downloads and more. Each component of typst-kit is
//! optional, but enabled by default.
//!
//! # Components
//! - [fonts] contains a default implementation for searching local and system
//!   installed fonts. It is enabled by the `fonts` feature flag, additionally
//!   the `embed-fonts` feature can be used to embed the Typst default fonts.
//!   - For text: Libertinus Serif, New Computer Modern
//!   - For math: New Computer Modern Math
//!   - For code: Deja Vu Sans Mono
//! - [package_downloads] contains functionality for handling package downloading
//!   It is enabled by the `downloads` feature flag.
//! - [package] contains package storage and downloading functionality based on
//!   [package_downloads]. It is enabled by the `packages` feature flag and implies the
//!   `downloads` feature flag.

#[cfg(feature = "fonts")]
pub mod fonts;
#[cfg(feature = "packages")]
pub mod package;
#[cfg(feature = "downloads")]
pub mod package_downloads;
