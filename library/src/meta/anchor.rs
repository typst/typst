use typst::{
    diag::SourceResult,
    eval::{cast_from_value, cast_to_value, Func, Value},
    model::{
        element, Content, Label, Locatable, Location, MetaElem, Show, StyleChain,
        Synthesize, Vt,
    },
};

use super::ErrorElem;

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
    /// Defines the body of references targeting this anchor.
    ///
    /// Can be set either to the reference body directly, or a function taking the location of the
    /// anchor and returning the reference body.
    #[required]
    pub ref_body: RefBody,

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

#[derive(Debug)]
pub enum RefBody {
    /// A reference's content.
    Content(Content),

    /// A closure mapping from an anchor's location to a reference's content.
    Func(Func),
}

impl RefBody {
    /// Apply the pattern to the given numbers.
    pub fn apply_vt(&self, vt: &mut Vt, location: Location) -> SourceResult<Content> {
        match self {
            Self::Content(content) => Ok(content.clone()),
            Self::Func(func) => {
                func.call_vt(vt, [Value::from(location)]).map(Value::display)
            }
        }
    }

    pub fn get_error(&self) -> Option<&ErrorElem> {
        match self {
            RefBody::Content(content) => content.to::<ErrorElem>(),
            RefBody::Func(_) => None,
        }
    }
}

cast_from_value! {
    RefBody,
    v: Content => Self::Content(v),
    v: Func => Self::Func(v),
}

cast_to_value! {
    v: RefBody => match v {
        RefBody::Content(content) => content.into(),
        RefBody::Func(func) => func.into(),
    }
}

impl From<Content> for RefBody {
    fn from(value: Content) -> Self {
        Self::Content(value)
    }
}
