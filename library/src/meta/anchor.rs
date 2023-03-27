use typst::{
    diag::SourceResult,
    model::{
        element, Content, Label, Locatable, MetaElem, Show, StyleChain, Synthesize, Vt,
    },
};

/// An anchor represents an element that can be [referenced]($func/ref).
///
/// ```example
/// #let myfigure(caption, images) = anchor(
///     caption,
///     block(
///         stack(
///             align(horizon, grid(..images, columns: 2)),
///             align(center, caption),
///         ),
///         breakable: false,
///     )
/// )
///
/// #myfigure("Figure 1", (
///     image("cylinder.svg"),
///     image("tetrahedron.svg"),
/// )) <fig1>
///
/// #myfigure("Figure 2", (
///     image("tetrahedron.svg"),
///     image("cylinder.svg"),
/// )) <fig2>
///
/// See @fig1 and @fig2
/// ```
///
/// Display: Anchor
/// Category: meta
#[element(Locatable, Synthesize, Show)]
pub struct AnchorElem {
    /// The name of the anchor as seen in references to it.
    #[required]
    pub ref_name: Content,

    /// The body of the anchor.
    #[required]
    pub body: Content,

    /// The label matched to this anchor.
    #[internal]
    #[synthesized]
    matched_label: Option<Label>,
}

impl Synthesize for AnchorElem {
    fn synthesize(&mut self, styles: StyleChain) {
        let label = MetaElem::active_label_in(styles);

        // Reference errors may need to refer to an anchor's span, so ensure it is not detached.
        debug_assert!(
            !self.span().is_detached(),
            "Anchor elements must not be in detached sources (at label: {:?})",
            label
        );

        self.push_matched_label(label);
    }
}

impl Show for AnchorElem {
    fn show(&self, _vt: &mut Vt, _styles: StyleChain) -> SourceResult<Content> {
        Ok(self.body().styled(MetaElem::set_active_label(None)))
    }
}
