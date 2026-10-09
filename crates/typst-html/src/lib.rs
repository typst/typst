//! Typst's HTML exporter.

pub mod attr;
pub mod property;
pub mod tag;

mod charsets;
mod convert;
mod css;
mod document;
mod dom;
mod encode;
mod format;
mod fragment;
mod introspect;
mod link;
mod mathml;
mod rules;
mod typed;

pub use self::document::{html_document, html_document_for_bundle};
pub use self::dom::*;
pub use self::encode::{HtmlOptions, html, html_in_bundle};
pub use self::format::{FORMAT, FrameElem, HtmlElem, HtmlFormat, HtmlFormatOptions};
pub use self::introspect::HtmlIntrospector;
pub use self::link::create_link_anchors;
pub use self::rules::{html_mathml_body, html_span_filled, register};
