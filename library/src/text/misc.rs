use super::TextElem;
use crate::prelude::*;

/// A text space.
///
/// Display: Space
/// Category: text
#[element(Behave, Unlabellable, PlainText)]
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
/// ## Example
/// ```example
/// *Date:* 26.12.2022 \
/// *Topic:* Infrastructure Test \
/// *Severity:* High \
/// ```
///
/// ## Syntax
/// This function also has dedicated syntax: To insert a line break, simply write
/// a backslash followed by whitespace. This always creates an unjustified
/// break.
///
/// Display: Line Break
/// Category: text
#[element(Behave)]
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
/// ## Example
/// ```example
/// This is *strong.* \
/// This is #strong[too.] \
///
/// #show strong: set text(red)
/// And this is *evermore.*
/// ```
///
/// ## Syntax
/// This function also has dedicated syntax: To strongly emphasize content,
/// simply enclose it in stars/asterisks (`*`). Note that this only works at
/// word boundaries. To strongly emphasize part of a word, you have to use the
/// function.
///
/// Display: Strong Emphasis
/// Category: text
#[element(Show)]
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

cast_from_value! {
    Delta,
    v: i64 => Self(v),
}

cast_to_value! {
    v: Delta => v.0.into()
}

impl Fold for Delta {
    type Output = i64;

    fn fold(self, outer: Self::Output) -> Self::Output {
        outer + self.0
    }
}

/// Emphasizes content by setting it in italics.
///
/// - If the current [text style]($func/text.style) is `{"normal"}`,
///   this turns it into `{"italic"}`.
/// - If it is already `{"italic"}` or `{"oblique"}`,
///   it turns it back to `{"normal"}`.
///
/// ## Example
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
/// ## Syntax
/// This function also has dedicated syntax: To emphasize content, simply
/// enclose it in underscores (`_`). Note that this only works at word
/// boundaries. To emphasize part of a word, you have to use the function.
///
/// Display: Emphasis
/// Category: text
#[element(Show)]
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

cast_from_value! {
    Toggle,
    _: Value => Self,
}

cast_to_value! {
    _: Toggle => Value::None
}

impl Fold for Toggle {
    type Output = bool;

    fn fold(self, outer: Self::Output) -> Self::Output {
        !outer
    }
}

/// Convert text or content to lowercase.
///
/// ## Example
/// ```example
/// #lower("ABC") \
/// #lower[*My Text*] \
/// #lower[already low]
/// ```
///
/// Display: Lowercase
/// Category: text
/// Returns: string or content
#[func]
pub fn lower(
    /// The text to convert to lowercase.
    text: ToCase,
) -> Value {
    case(text, Case::Lower)
}

/// Convert text or content to uppercase.
///
/// ## Example
/// ```example
/// #upper("abc") \
/// #upper[*my text*] \
/// #upper[ALREADY HIGH]
/// ```
///
/// Display: Uppercase
/// Category: text
/// Returns: string or content
#[func]
pub fn upper(
    /// The text to convert to uppercase.
    text: ToCase,
) -> Value {
    case(text, Case::Upper)
}

/// Change the case of text.
fn case(text: ToCase, case: Case) -> Value {
    match text {
        ToCase::Str(v) => Value::Str(case.apply(&v).into()),
        ToCase::Content(v) => Value::Content(v.styled(TextElem::set_case(Some(case)))),
    }
}

/// A value whose case can be changed.
enum ToCase {
    Str(Str),
    Content(Content),
}

cast_from_value! {
    ToCase,
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

/// Display text in small capitals.
///
/// _Note:_ This enables the OpenType `smcp` feature for the font. Not all fonts
/// support this feature. Sometimes smallcaps are part of a dedicated font and
/// sometimes they are not available at all. In the future, this function will
/// support selecting a dedicated smallcaps font as well as synthesizing
/// smallcaps from normal letters, but this is not yet implemented.
///
/// ## Example
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
///
/// Display: Small Capitals
/// Category: text
/// Returns: content
#[func]
pub fn smallcaps(
    /// The text to display to small capitals.
    body: Content,
) -> Value {
    Value::Content(body.styled(TextElem::set_smallcaps(true)))
}

/// Create blind text.
///
/// This function yields a Latin-like _Lorem Ipsum_ blind text with the given
/// number of words. The sequence of words generated by the function is always
/// the same but randomly chosen. As usual for blind texts, it does not make any
/// sense. Use it as a placeholder to try layouts.
///
/// ## Example
/// ```example
/// = Blind Text
/// #lorem(30)
///
/// = More Blind Text
/// #lorem(15)
/// ```
///
/// Display: Blind Text
/// Category: text
/// Returns: string
#[func]
pub fn lorem(
    /// The length of the blind text in words.
    words: usize,
) -> Value {
    Value::Str(lipsum::lipsum(words).replace("--", "–").into())
}
