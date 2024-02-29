use icu_segmenter::LineBreakWordOption;

use crate::foundations::{elem, Cast, Packed};
use crate::realize::{Behave, Behaviour};

/// Inserts a line break.
///
/// Advances the paragraph to the next line. A single trailing line break at the
/// end of a paragraph is ignored, but more than one creates additional empty
/// lines.
///
/// # Example
/// ```example
/// *Date:* 26.12.2022 \
/// *Topic:* Infrastructure Test \
/// *Severity:* High \
/// ```
///
/// # Syntax
/// This function also has dedicated syntax: To insert a line break, simply write
/// a backslash followed by whitespace. This always creates an unjustified
/// break.
#[elem(title = "Line Break", Behave)]
pub struct LinebreakElem {
    /// Whether to justify the line before the break.
    ///
    /// This is useful if you found a better line break opportunity in your
    /// justified text than Typst did.
    ///
    /// ```example
    /// #set par(justify: true)
    /// #let jb = linebreak(justify: true)
    ///
    /// I have manually tuned the #jb
    /// line breaks in this paragraph #jb
    /// for an _interesting_ result. #jb
    /// ```
    #[default(false)]
    pub justify: bool,
}

impl Behave for Packed<LinebreakElem> {
    fn behaviour(&self) -> Behaviour {
        Behaviour::Destructive
    }
}

/// A word break mode on text.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum WordBreak {
    /// Words break according to the languages' customary rules. For example,
    /// English do not prefer to break lines without space while
    /// Chinese/Japanese doesn't. See the the details in
    /// `https://drafts.csswg.org/css-text-3/#valdef-line-break-normal`.
    Normal,

    /// Breaking is allowed within words.
    BreakAll,

    /// Breaking is forbidden within words.
    KeepAll,
}

impl From<WordBreak> for LineBreakWordOption {
    fn from(value: WordBreak) -> LineBreakWordOption {
        match value {
            WordBreak::Normal => LineBreakWordOption::Normal,
            WordBreak::BreakAll => LineBreakWordOption::BreakAll,
            WordBreak::KeepAll => LineBreakWordOption::KeepAll,
        }
    }
}
