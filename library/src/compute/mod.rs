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
    global.define("type", type_);
    global.define("repr", repr);
    global.define("panic", panic);
    global.define("assert", assert);
    global.define("eval", eval);
    global.define("int", int);
    global.define("float", float);
    global.define("luma", luma);
    global.define("rgb", rgb);
    global.define("cmyk", cmyk);
    global.define("datetime", datetime);
    global.define("symbol", symbol);
    global.define("str", str);
    global.define("label", label);
    global.define("regex", regex);
    global.define("range", range);
    global.define("read", read);
    global.define("csv", csv);
    global.define("json", json);
    global.define("toml", toml);
    global.define("yaml", yaml);
    global.define("xml", xml);
    global.define("calc", calc::module());
}
