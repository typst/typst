pub use encode::{FilteredProperties, Properties, Property, ToCss};
pub use resolve::{
    SelectorCandidates, StylesheetData, find_selector_candidates,
    insert_embedded_stylesheet, insert_external_stylesheet_link, resolve_stylesheet,
    write_inline_styles,
};

mod encode;
mod resolve;
