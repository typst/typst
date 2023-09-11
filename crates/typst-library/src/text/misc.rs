use super::TextElem;
use crate::prelude::*;

/// A text space.
#[elem(Behave, Unlabellable, PlainText)]
pub struct SpaceElem {}

impl Behave for SpaceElem {
    fn behaviour(&self) -> Behaviour {
        Behaviour::Weak(2)
    }
}

impl Unlabellable for SpaceElem {}

impl PlainText for SpaceElem {
    fn plain_text(&self, text: &mut EcoString) {
        text.push(' ');
    }
}

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

impl Behave for LinebreakElem {
    fn behaviour(&self) -> Behaviour {
        Behaviour::Destructive
    }
}

/// Strongly emphasizes content by increasing the font weight.
///
/// Increases the current font weight by a given `delta`.
///
/// # Example
/// ```example
/// This is *strong.* \
/// This is #strong[too.] \
///
/// #show strong: set text(red)
/// And this is *evermore.*
/// ```
///
/// # Syntax
/// This function also has dedicated syntax: To strongly emphasize content,
/// simply enclose it in stars/asterisks (`*`). Note that this only works at
/// word boundaries. To strongly emphasize part of a word, you have to use the
/// function.
#[elem(title = "Strong Emphasis", Show)]
pub struct StrongElem {
    /// The delta to apply on the font weight.
    ///
    /// ```example
    /// #set strong(delta: 0)
    /// No *effect!*
    /// ```
    #[default(300)]
    pub delta: i64,

    /// The content to strongly emphasize.
    #[required]
    pub body: Content,
}

impl Show for StrongElem {
    #[tracing::instrument(name = "StrongElem::show", skip_all)]
    fn show(&self, _: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        Ok(self.body().styled(TextElem::set_delta(Delta(self.delta(styles)))))
    }
}

/// A delta that is summed up when folded.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Delta(pub i64);

cast! {
    Delta,
    self => self.0.into_value(),
    v: i64 => Self(v),
}

impl Fold for Delta {
    type Output = i64;

    fn fold(self, outer: Self::Output) -> Self::Output {
        outer + self.0
    }
}

/// Emphasizes content by setting it in italics.
///
/// - If the current [text style]($text.style) is `{"normal"}`, this turns it
///   into `{"italic"}`.
/// - If it is already `{"italic"}` or `{"oblique"}`, it turns it back to
///   `{"normal"}`.
///
/// # Example
/// ```example
/// This is _emphasized._ \
/// This is #emph[too.]
///
/// #show emph: it => {
///   text(blue, it.body)
/// }
///
/// This is _emphasized_ differently.
/// ```
///
/// # Syntax
/// This function also has dedicated syntax: To emphasize content, simply
/// enclose it in underscores (`_`). Note that this only works at word
/// boundaries. To emphasize part of a word, you have to use the function.
#[elem(title = "Emphasis", Show)]
pub struct EmphElem {
    /// The content to emphasize.
    #[required]
    pub body: Content,
}

impl Show for EmphElem {
    #[tracing::instrument(name = "EmphElem::show", skip(self))]
    fn show(&self, _: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        Ok(self.body().styled(TextElem::set_emph(Toggle)))
    }
}

/// A toggle that turns on and off alternatingly if folded.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Toggle;

cast! {
    Toggle,
    self => Value::None,
    _: Value => Self,
}

impl Fold for Toggle {
    type Output = bool;

    fn fold(self, outer: Self::Output) -> Self::Output {
        !outer
    }
}

/// Converts text or content to lowercase.
///
/// # Example
/// ```example
/// #lower("ABC") \
/// #lower[*My Text*] \
/// #lower[already low]
/// ```
#[func(title = "Lowercase")]
pub fn lower(
    /// The text to convert to lowercase.
    text: Caseable,
) -> Caseable {
    case(text, Case::Lower)
}

/// Converts text or content to uppercase.
///
/// # Example
/// ```example
/// #upper("abc") \
/// #upper[*my text*] \
/// #upper[ALREADY HIGH]
/// ```
#[func(title = "Uppercase")]
pub fn upper(
    /// The text to convert to uppercase.
    text: Caseable,
) -> Caseable {
    case(text, Case::Upper)
}

/// Change the case of text.
fn case(text: Caseable, case: Case) -> Caseable {
    match text {
        Caseable::Str(v) => Caseable::Str(case.apply(&v).into()),
        Caseable::Content(v) => {
            Caseable::Content(v.styled(TextElem::set_case(Some(case))))
        }
    }
}

/// A value whose case can be changed.
pub enum Caseable {
    Str(Str),
    Content(Content),
}

cast! {
    Caseable,
    self => match self {
        Self::Str(v) => v.into_value(),
        Self::Content(v) => v.into_value(),
    },
    v: Str => Self::Str(v),
    v: Content => Self::Content(v),
}

/// A case transformation on text.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum Case {
    /// Everything is lowercased.
    Lower,
    /// Everything is uppercased.
    Upper,
}

impl Case {
    /// Apply the case to a string.
    pub fn apply(self, text: &str) -> String {
        match self {
            Self::Lower => text.to_lowercase(),
            Self::Upper => text.to_uppercase(),
        }
    }
}

/// Displays text in small capitals.
///
/// _Note:_ This enables the OpenType `smcp` feature for the font. Not all fonts
/// support this feature. Sometimes smallcaps are part of a dedicated font and
/// sometimes they are not available at all. In the future, this function will
/// support selecting a dedicated smallcaps font as well as synthesizing
/// smallcaps from normal letters, but this is not yet implemented.
///
/// # Example
/// ```example
/// #set par(justify: true)
/// #set heading(numbering: "I.")
///
/// #show heading: it => {
///   set block(below: 10pt)
///   set text(weight: "regular")
///   align(center, smallcaps(it))
/// }
///
/// = Introduction
/// #lorem(40)
/// ```
#[func(title = "Small Capitals")]
pub fn smallcaps(
    /// The text to display to small capitals.
    body: Content,
) -> Content {
    body.styled(TextElem::set_smallcaps(true))
}

/// Creates blind text.
///
/// This function yields a Latin-like _Lorem Ipsum_ blind text with the given
/// number of words. The sequence of words generated by the function is always
/// the same but randomly chosen. As usual for blind texts, it does not make any
/// sense. Use it as a placeholder to try layouts.
///
/// # Example
/// ```example
/// = Blind Text
/// #lorem(30)
///
/// = More Blind Text
/// #lorem(15)
/// ```
#[func(keywords = ["Blind Text"])]
pub fn lorem(
    /// The length of the blind text in words.
    words: usize,
) -> Str {
    lipsum::lipsum(words).replace("--", "–").into()
}
