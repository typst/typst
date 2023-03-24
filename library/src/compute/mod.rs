//! Computational functions.

pub mod calc;
mod construct;
mod data;
mod foundations;

use typst::eval::Scope;

pub use self::construct::{
    cmyk, float, int, label, luma, range, regex, rgb, str, symbol,
};
pub use self::data::{csv, json, read, xml};
pub use self::foundations::{assert, eval, panic, repr, type_};

pub(super) fn define(scope: &mut Scope) {
    scope.define("type", type_);
    scope.define("repr", repr);
    scope.define("panic", panic);
    scope.define("assert", assert);
    scope.define("eval", eval);
    scope.define("int", int);
    scope.define("float", float);
    scope.define("luma", luma);
    scope.define("rgb", rgb);
    scope.define("cmyk", cmyk);
    scope.define("symbol", symbol);
    scope.define("str", str);
    scope.define("label", label);
    scope.define("regex", regex);
    scope.define("range", range);
    scope.define("read", read);
    scope.define("csv", csv);
    scope.define("json", json);
    scope.define("xml", xml);
}
