//! Typst-kit contains various default implementations of functionality used in
//! typst-cli. It is intended as a single source of truth for things like font
//! searching, package downloads and more. Each component of typst-kit is
//! optional, but enabled by default.
//!
//! # Components
//! - [fonts] contains a default implementation for searching local and system
//!   installed fonts. It is enabled by the `fonts` feature flag, additionally
//!   the `embed-fonts` feature can be used to embed the Typst default fonts.
//!   - For text: Linux Libertine, New Computer Modern
//!   - For math: New Computer Modern Math
//!   - For code: Deja Vu Sans Mono
//! - [download] contains functionality for making simple web requests with
//!   status reporting, useful for downloading packages from package registries.
//!   It is enabled by the `downloads` feature flag, additionally the
//!   `vendor-openssl` can be used on operating systems other than macOS and
//!   Windows to vendor OpenSSL when building.
//! - [package] contains package storage and downloading functionality based on
//!   [download]. It is enabled by the `packages` feature flag and implies the
//!   `downloads` feature flag.

#[cfg(feature = "downloads")]
pub mod download;
#[cfg(feature = "fonts")]
pub mod fonts;
#[cfg(feature = "packages")]
pub mod package;
