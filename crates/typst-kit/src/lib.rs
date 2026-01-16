//! Typst Kit contains useful building blocks for Typst integrations. It is
//! intended as a single source of truth for things like font searching, package
//! loading and more. In particular, it contains various implementations of
//! functionality used in `typst-cli`.
//!
//! Crate functionality that incurs additional dependencies is heavily
//! feature-flagged, so that you can pick exactly what you need. By default, all
//! features are disabled. The available feature flags are:
//!
//! - `embedded-fonts`: Enables [`fonts::embedded`]
//! - `scan-fonts`: Enables [`fonts::scan`] and [`fonts::system`]
//! - `system-files`: Enables [`files::SystemFiles`]
//! - `system-packages`: Enables [`packages::SystemPackages`]
//! - `universe-packages`: Enables [`packages::UniversePackages`]
//! - `emit-diagnostics`: Enables [`diagnostics::emit`]
//! - `system-downloader`: Enables [`downloader::SystemDownloader`]
//! - `watcher`: Enables [`watcher::Watcher`]
//! - `server`: Enables [`server::HttpServer`]

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
