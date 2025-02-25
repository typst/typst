//! Data loading.

#[path = "cbor.rs"]
mod cbor_;
#[path = "csv.rs"]
mod csv_;
#[path = "json.rs"]
mod json_;
#[path = "read.rs"]
mod read_;
#[path = "toml.rs"]
mod toml_;
#[path = "xml.rs"]
mod xml_;
#[path = "yaml.rs"]
mod yaml_;

use comemo::Tracked;
use ecow::EcoString;
use typst_syntax::Spanned;

pub use self::cbor_::*;
pub use self::csv_::*;
pub use self::json_::*;
pub use self::read_::*;
pub use self::toml_::*;
pub use self::xml_::*;
pub use self::yaml_::*;

use crate::diag::{At, SourceResult};
use crate::foundations::OneOrMultiple;
use crate::foundations::{cast, Bytes, Scope, Str};
use crate::World;

/// Hook up all `data-loading` definitions.
pub(super) fn define(global: &mut Scope) {
    global.start_category(crate::Category::DataLoading);
    global.define_func::<read>();
    global.define_func::<csv>();
    global.define_func::<json>();
    global.define_func::<toml>();
    global.define_func::<yaml>();
    global.define_func::<cbor>();
    global.define_func::<xml>();
    global.reset_category();
}

/// Something we can retrieve byte data from.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum DataSource {
    /// A path to a file.
    Path(EcoString),
    /// Raw bytes.
    Bytes(Bytes),
}

cast! {
    DataSource,
    self => match self {
        Self::Path(v) => v.into_value(),
        Self::Bytes(v) => v.into_value(),
    },
    v: EcoString => Self::Path(v),
    v: Bytes => Self::Bytes(v),
}

/// Loads data from a path or provided bytes.
pub trait Load {
    /// Bytes or a list of bytes (if there are multiple sources).
    type Output;

    /// Load the bytes.
    fn load(&self, world: Tracked<dyn World + '_>) -> SourceResult<Self::Output>;
}

impl Load for Spanned<DataSource> {
    type Output = Bytes;

    fn load(&self, world: Tracked<dyn World + '_>) -> SourceResult<Bytes> {
        self.as_ref().load(world)
    }
}

impl Load for Spanned<&DataSource> {
    type Output = Bytes;

    fn load(&self, world: Tracked<dyn World + '_>) -> SourceResult<Bytes> {
        match &self.v {
            DataSource::Path(path) => {
                let file_id = self.span.resolve_path(path).at(self.span)?;
                world.file(file_id).at(self.span)
            }
            DataSource::Bytes(bytes) => Ok(bytes.clone()),
        }
    }
}

impl Load for Spanned<OneOrMultiple<DataSource>> {
    type Output = Vec<Bytes>;

    fn load(&self, world: Tracked<dyn World + '_>) -> SourceResult<Vec<Bytes>> {
        self.as_ref().load(world)
    }
}

impl Load for Spanned<&OneOrMultiple<DataSource>> {
    type Output = Vec<Bytes>;

    fn load(&self, world: Tracked<dyn World + '_>) -> SourceResult<Vec<Bytes>> {
        self.v
            .0
            .iter()
            .map(|source| Spanned::new(source, self.span).load(world))
            .collect()
    }
}

/// A value that can be read from a file.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Readable {
    /// A decoded string.
    Str(Str),
    /// Raw bytes.
    Bytes(Bytes),
}

impl Readable {
    pub fn into_bytes(self) -> Bytes {
        match self {
            Self::Bytes(v) => v,
            Self::Str(v) => Bytes::from_string(v),
        }
    }

    pub fn into_source(self) -> DataSource {
        DataSource::Bytes(self.into_bytes())
    }
}

cast! {
    Readable,
    self => match self {
        Self::Str(v) => v.into_value(),
        Self::Bytes(v) => v.into_value(),
    },
    v: Str => Self::Str(v),
    v: Bytes => Self::Bytes(v),
}
