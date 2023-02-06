use crate::prelude::*;

/// # Align
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
/// ## Parameters
/// - body: `Content` (positional, required)
///   The content to align.
///
/// - alignment: `Axes<Option<GenAlign>>` (positional, settable)
///   The alignment along both axes.
///
///   Possible values for horizontal alignments are:
///   - `start`
///   - `end`
///   - `left`
///   - `center`
///   - `right`
///
///   The `start` and `end` alignments are relative to the current [text
///   direction]($func/text.dir).
///
///   Possible values for vertical alignments are:
///   - `top`
///   - `horizon`
///   - `bottom`
///
///   To align along both axes at the same time, add the two alignments using
///   the `+` operator to get a `2d alignment`. For example, `top + right`
///   aligns the content to the top right corner.
///
///   ```example
///   #set page(height: 6cm)
///   #set text(lang: "ar")
///
///   مثال
///   #align(
///     end + horizon,
///     rect(inset: 12pt)[ركن]
///   )
///   ```
///
/// ## Category
/// layout
#[func]
#[capable]
#[derive(Debug, Hash)]
pub enum AlignNode {}

#[node]
impl AlignNode {
    /// The alignment.
    #[property(fold, skip)]
    pub const ALIGNS: Axes<Option<GenAlign>> =
        Axes::new(GenAlign::Start, GenAlign::Specific(Align::Top));

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        args.expect("body")
    }

    fn set(...) {
        let aligns: Axes<Option<GenAlign>> = args.find()?.unwrap_or_default();
        styles.set(Self::ALIGNS, aligns);
    }
}
