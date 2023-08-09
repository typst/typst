use crate::prelude::*;
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
/// ### is-svg()
/// True when the target is a svg.
///
/// - returns: bool
///
/// ### is-png()
/// True when the target is a png.
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

#[derive(Clone, PartialEq, Hash, Debug)]
pub struct Target(export::Target);

cast! {
    type Target: "target",
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
            "is-svg" => self.0 == export::Target::Svg,
            "is-png" => self.0 == export::Target::Png,
            "is-query" => self.0 == export::Target::Query,
            "is-pageless" => false, // When HTML gets added, this will be true for it.
            "is-layouted" => matches!(
                self.0,
                export::Target::Pdf | export::Target::Svg | export::Target::Png
            ),
            _ => bail!(span, "type target has no method `{}`", method),
        };

        Ok(Value::Bool(output))
    }
}
