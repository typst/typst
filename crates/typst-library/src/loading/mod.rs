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
use typst_syntax::{FileId, Spanned};

pub use self::cbor_::*;
pub use self::csv_::*;
pub use self::json_::*;
pub use self::read_::*;
pub use self::toml_::*;
pub use self::xml_::*;
pub use self::yaml_::*;

use crate::diag::{At, LoadError, LoadResult, LoadedAt, SourceResult};
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
    type Output = Loaded;

    fn load(&self, world: Tracked<dyn World + '_>) -> SourceResult<Self::Output> {
        self.as_ref().load(world)
    }
}

impl Load for Spanned<&DataSource> {
    type Output = Loaded;

    fn load(&self, world: Tracked<dyn World + '_>) -> SourceResult<Self::Output> {
        match &self.v {
            DataSource::Path(path) => {
                let file_id = self.span.resolve_path(path).at(self.span)?;
                let bytes = world.file(file_id).at(self.span)?;
                let source = Spanned::new(LoadSource::Path(file_id), self.span);
                Ok(Loaded::new(source, bytes))
            }
            DataSource::Bytes(bytes) => {
                let source = Spanned::new(LoadSource::Bytes, self.span);
                Ok(Loaded::new(source, bytes.clone()))
            }
        }
    }
}

impl Load for Spanned<OneOrMultiple<DataSource>> {
    type Output = Vec<Loaded>;

    fn load(&self, world: Tracked<dyn World + '_>) -> SourceResult<Self::Output> {
        self.as_ref().load(world)
    }
}

impl Load for Spanned<&OneOrMultiple<DataSource>> {
    type Output = Vec<Loaded>;

    fn load(&self, world: Tracked<dyn World + '_>) -> SourceResult<Self::Output> {
        self.v
            .0
            .iter()
            .map(|source| Spanned::new(source, self.span).load(world))
            .collect()
    }
}

/// Data loaded from a [`DataSource`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Loaded {
    pub source: Spanned<LoadSource>,
    pub bytes: Bytes,
}

impl Loaded {
    pub fn new(source: Spanned<LoadSource>, bytes: Bytes) -> Self {
        Self { source, bytes }
    }

    pub fn load_str(&self) -> SourceResult<&str> {
        self.bytes.load_str().in_invalid_text(self)
    }
}

/// A loaded [`DataSource`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LoadSource {
    Path(FileId),
    Bytes,
}

pub trait LoadStr {
    fn load_str(&self) -> LoadResult<&str>;
}

impl<T: AsRef<[u8]>> LoadStr for T {
    fn load_str(&self) -> LoadResult<&str> {
        std::str::from_utf8(self.as_ref()).map_err(|err| {
            let start = err.valid_up_to();
            let end = start + err.error_len().unwrap_or(0);
            LoadError::new(
                start..end,
                "failed to convert to string",
                "file is not valid utf-8",
            )
        })
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
