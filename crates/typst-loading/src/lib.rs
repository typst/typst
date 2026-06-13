//! Data loading.

#[path = "cbor.rs"]
mod cbor_;
#[path = "csv.rs"]
mod csv_;
#[path = "json.rs"]
mod json_;
#[path = "toml.rs"]
mod toml_;
#[path = "xml.rs"]
mod xml_;
#[path = "yaml.rs"]
mod yaml_;

pub use self::cbor_::*;
pub use self::csv_::*;
pub use self::json_::*;
pub use self::toml_::*;
pub use self::xml_::*;
pub use self::yaml_::*;

use typst_library::foundations::Scope;

/// Hook up the `data-loading` definitions for more complicated file formats.
pub fn register(global: &mut Scope) {
    global.start_category(typst_library::Category::DataLoading);
    global.define_func::<csv>();
    global.define_func::<json>();
    global.define_func::<toml>();
    global.define_func::<yaml>();
    global.define_func::<cbor>();
    global.define_func::<xml>();
    global.reset_category();
}
