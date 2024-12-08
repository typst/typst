use ecow::eco_format;

use crate::diag::{warning, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    elem, Content, Packed, Show, ShowSet, StyleChain, Styles, Synthesize,
};
use crate::introspection::Locatable;
use crate::layout::Em;
use crate::text::{FontWeight, TextElem, TextSize};

use super::Color;

/// A reminder to implement a feature, add content, or fix a bug.
///
/// By default this element is shown as a big red bold "TODO", and produces
/// a warning with an optional message. You can optionally restyle the shown
/// text by using the `show` rule, and you can disable the warning by using the
/// `warn` argument.
///
/// ```example
/// #show todo: it => box(stroke: 1pt, text(fill: blue, size: 16pt, it.message))
///
/// #todo(message: "Complete this example.")
/// ```
///
/// # Example
/// ```example
/// // Warning: "TODO"
/// #todo()
///
/// // Warning: "TODO: this is a message"
/// #todo(message: "this is a message")
///
/// // No warning.
/// #todo(warn: false)
///
/// // Disable warning for all subsequent `todo` elements.
/// #set todo(warn: false)
///
/// // No warning
/// #todo(message: "this is a message")
/// ```
#[elem(Locatable, Synthesize, Show, ShowSet)]
pub struct TodoElem {
    #[default]
    #[borrowed]
    pub message: Option<String>,

    #[default(true)]
    pub warn: bool,
}

impl Synthesize for Packed<TodoElem> {
    fn synthesize(
        &mut self,
        engine: &mut Engine,
        styles: StyleChain,
    ) -> SourceResult<()> {
        // If the `warn` flag is set to `false`, then don't show a warning.
        if !self.warn(styles) {
            return Ok(());
        }

        // We warn in the synthesize to avoid a show rule disabling the warning.
        engine.sink.warn(if let Some(message) = self.message(styles) {
            warning!(self.span(), "TODO: {message}",)
        } else {
            // Purposefully printing a `TODO` to avoid libraries using it as
            // a warning message.
            warning!(self.span(), "TODO")
        });

        Ok(())
    }
}

impl ShowSet for Packed<TodoElem> {
    fn show_set(&self, _: StyleChain) -> crate::foundations::Styles {
        const SIZE: Em = Em::new(1.2);
        // Make the text: red, bold, and 1.2em tall.
        let mut out = Styles::new();
        out.set(TextElem::set_fill(Color::RED.into()));
        out.set(TextElem::set_weight(FontWeight::BOLD));
        out.set(TextElem::set_size(TextSize(SIZE.into())));

        out
    }
}

impl Show for Packed<TodoElem> {
    fn show(&self, _: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        // Show a bold red todo message.
        if let Some(message) = self.message(styles) {
            Ok(TextElem::packed(eco_format!("TODO: {message}")))
        } else {
            Ok(TextElem::packed("TODO"))
        }
    }
}
