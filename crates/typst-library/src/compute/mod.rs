//! Computational functions.

pub mod calc;
mod construct;
mod data;
mod foundations;

pub use self::construct::*;
pub use self::data::*;
pub use self::foundations::*;

use crate::prelude::*;

/// Hook up all compute definitions.
pub(super) fn define(global: &mut Scope) {
    global.define("type", type_func());
    global.define("repr", repr_func());
    global.define("panic", panic_func());
    global.define("assert", assert_func());
    global.define("eval", eval_func());
    global.define("int", int_func());
    global.define("float", float_func());
    global.define("luma", luma_func());
    global.define("rgb", rgb_func());
    global.define("cmyk", cmyk_func());
    global.define("color", color_module());
    global.define("datetime", datetime_func());
    global.define("symbol", symbol_func());
    global.define("str", str_func());
    global.define("bytes", bytes_func());
    global.define("label", label_func());
    global.define("regex", regex_func());
    global.define("array", array_func());
    global.define("range", range_func());
    global.define("read", read_func());
    global.define("csv", csv_func());
    global.define("json", json_func());
    global.define("toml", toml_func());
    global.define("yaml", yaml_func());
    global.define("xml", xml_func());
    global.define("calc", calc::module());
}
