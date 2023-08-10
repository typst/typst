use crate::prelude::*;
use std::fmt::Write;
use typst::export;

/// Provides access to the target format, and associated information.
///
/// This allows show rules to only be applied in certain formats.
/// ```example
/// #set text(red) if target().is-pdf()
/// ```
///
/// ### is-pdf()
/// True when the target is a pdf, which is the default when the target is not specified and the extension isn't something else.
///
/// - returns: bool
///
/// ### is-vector()
/// True when the target is vectorized (SVG).
///
/// - returns: bool
///
/// ### is-raster()
/// True when the target is rasterized (PNG, JPG, etc).
///
/// - returns: bool
///
/// ### is-query()
/// True when the target is a query and the target isn't overridden.
///
/// - returns: bool
///
/// ### is-pageless()
/// True when the target is a pageless format. HTML will be pageless, and queries are pageless.
///
/// - returns: bool
///
/// ### is-layouted()
/// True when the target is a layouted format. PDF, SVG, and PNG are layouted.
///
/// - returns: bool
///
/// Display: Target
/// Category: meta
#[func]
pub fn target(
    /// The virtual machine.
    vm: &mut Vm,
) -> Target {
    Target::new(vm.vt.world.target())
}

#[derive(Clone, PartialEq, Hash)]
pub struct Target(export::Target);

cast! {
    type Target: "target",
}

impl Debug for Target {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("target(")?;
        self.0.fmt(f)?;
        f.write_char(')')
    }
}

impl Target {
    /// Create a new target
    pub fn new(target: export::Target) -> Self {
        Self(target)
    }

    /// Call a method on a target.
    pub fn call_method(
        self,
        method: &str,
        args: Args,
        span: Span,
    ) -> SourceResult<Value> {
        args.finish()?;
        let output = match method {
            "is-pdf" => self.0 == export::Target::Pdf,
            "is-vector" => self.0 == export::Target::Vector,
            "is-raster" => self.0 == export::Target::Raster,
            "is-query" => self.0 == export::Target::Query,
            "is-pageless" => false, // When HTML gets added, this will be true for it.
            "is-layouted" => matches!(
                self.0,
                export::Target::Pdf | export::Target::Vector | export::Target::Raster
            ),
            _ => bail!(span, "type target has no method `{}`", method),
        };

        Ok(Value::Bool(output))
    }
}
