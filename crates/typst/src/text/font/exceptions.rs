use std::collections::BTreeMap;

use once_cell::sync::Lazy;
use serde::Deserialize;

use super::{FontStretch, FontStyle, FontWeight};

#[derive(Debug, Default, Deserialize)]
pub struct OverrideEntry {
    pub family: Option<String>,
    pub style: Option<FontStyle>,
    pub weight: Option<FontWeight>,
    pub stretch: Option<FontStretch>,
}

pub fn override_entry(postscript_name: &str) -> Option<&'static OverrideEntry> {
    OVERRIDE_MAP.get(postscript_name)
}

/// A map which keys are PostScript name and values are override entries.
static OVERRIDE_MAP: Lazy<BTreeMap<String, OverrideEntry>> =
    Lazy::new(|| serde_yaml::from_str(include_str!("override_list.yml")).unwrap());
