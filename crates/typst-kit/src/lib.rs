//! Typst Kit contains useful building blocks for Typst integrations. It is
//! intended as a single source of truth for things like font searching, package
//! loading and more. In particular, it contains various implementations of
//! functionality used in `typst-cli`.
//!
//! # Features flags
//! Crate functionality that incurs additional dependencies is heavily
//! feature-flagged, so that you can pick exactly what you need. By default, all
//! features are disabled. The available feature flags are:
//!
//! - `embedded-fonts`: Enables loading of embedded fonts via
//!   [`fonts::embedded`].
//! - `scan-fonts`: Enables font discovery at paths and from the system via
//!   [`fonts::scan`] and [`fonts::system`].
//! - `system-files`: Enables loading of files from standard locations via
//!   [`files::SystemFiles`].
//! - `system-packages`: Enables loading of packages from standard locations via
//!   [`packages::SystemPackages`].
//! - `universe-packages`: Enables loading of packages from Typst Universe via
//!   [`packages::UniversePackages`].
//! - `emit-diagnostics`: Enables emitting terminal-style diagnostics via
//!   [`diagnostics::emit`].
//! - `system-downloader`: Enables network requests via
//!   [`downloader::SystemDownloader`].
//! - `watcher`: Enables file system watching via [`watcher::Watcher`].
//! - `server`: Enables a live-reloading HTTP serving via [`server::HttpServer`]
//! - `vendor-openssl`: Whether to vendor OpenSSL for the `system-downloader`.
//!   Not applicable to Windows and macOS build.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(
    not(all(
        feature = "embedded-fonts",
        feature = "scan-fonts",
        feature = "system-files",
        feature = "system-packages",
        feature = "universe-packages",
        feature = "emit-diagnostics",
        feature = "system-downloader",
        feature = "watcher",
        feature = "server",
    )),
    allow(rustdoc::broken_intra_doc_links)
)]

pub mod diagnostics;
pub mod downloader;
pub mod files;
pub mod fonts;
pub mod packages;
pub mod server;
pub mod watcher;
