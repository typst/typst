use crate::prelude::*;

/// Aligns content horizontally and vertically.
///
/// ## Example { #example }
/// ```example
/// #set align(center)
///
/// Centered text, a sight to see \
/// In perfect balance, visually \
/// Not left nor right, it stands alone \
/// A work of art, a visual throne
/// ```
///
/// Display: Align
/// Category: layout
#[element(Show)]
pub struct AlignElem {
    /// The alignment along both axes.
    ///
    /// Possible values for horizontal alignments are:
    /// - `start`
    /// - `end`
    /// - `left`
    /// - `center`
    /// - `right`
    ///
    /// The `start` and `end` alignments are relative to the current [text
    /// direction]($func/text.dir).
    ///
    /// Possible values for vertical alignments are:
    /// - `top`
    /// - `horizon`
    /// - `bottom`
    ///
    /// You can use the `axis` method on a single-axis alignment to obtain
    /// whether it is `{"horizontal"}` or `{"vertical"}`. You can also use the
    /// `inv` method to obtain its inverse alignment. For example,
    /// `{top.axis()}` is `{"vertical"}`, while `{top.inv()}` is equal to
    /// `{bottom}`.
    ///
    /// To align along both axes at the same time, add the two alignments using
    /// the `+` operator to get a `2d alignment`. For example, `top + right`
    /// aligns the content to the top right corner.
    ///
    /// For 2d alignments, the `x` and `y` fields hold their horizontal and
    /// vertical components, respectively. Additionally, you can use the `inv`
    /// method to obtain a 2d alignment with both components inverted. For
    /// instance, `{(top + right).x}` is `right`, `{(top + right).y}` is `top`,
    /// and `{(top + right).inv()}` is equal to `bottom + left`.
    ///
    /// ```example
    /// #set page(height: 6cm)
    /// #set text(lang: "ar")
    ///
    /// مثال
    /// #align(
    ///   end + horizon,
    ///   rect(inset: 12pt)[ركن]
    /// )
    /// ```
    #[positional]
    #[fold]
    #[default(Axes::new(GenAlign::Start, GenAlign::Specific(Align::Top)))]
    pub alignment: Axes<Option<GenAlign>>,

    /// The content to align.
    #[required]
    pub body: Content,
}

impl Show for AlignElem {
    #[tracing::instrument(name = "AlignElem::show", skip_all)]
    fn show(&self, _: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        Ok(self
            .body()
            .styled(Self::set_alignment(self.alignment(styles).map(Some))))
    }
}
