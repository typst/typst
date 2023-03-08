use crate::prelude::*;

/// Align content horizontally and vertically.
///
/// ## Example
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
#[node(Show)]
#[set({
    styles.set(Self::set_alignment(args.find()?.unwrap_or_default()));
})]
pub struct AlignNode {
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
    /// To align along both axes at the same time, add the two alignments using
    /// the `+` operator to get a `2d alignment`. For example, `top + right`
    /// aligns the content to the top right corner.
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
    #[settable]
    #[positional]
    #[fold]
    #[skip]
    #[default(Axes::new(GenAlign::Start, GenAlign::Specific(Align::Top)))]
    pub alignment: Axes<Option<GenAlign>>,

    /// The content to align.
    #[positional]
    #[required]
    pub body: Content,
}

impl Show for AlignNode {
    fn show(&self, _: &mut Vt, _: &Content, _: StyleChain) -> SourceResult<Content> {
        Ok(self.body())
    }
}
