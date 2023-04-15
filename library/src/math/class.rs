use super::*;

/// Math operator class.
/// Note this doesn't scale the character nor upright the text.
/// Default contains large operators such as `sum`, `product`.
///
/// ## Example
/// ```example
/// $ a operator(x o r) b, a x o r b $
/// ```
///
/// Display: Math Operator
/// Category: math
/// Returns: content
#[func]
pub fn operator(
    /// The content to style.
    body: Content,
) -> Value {
    MathClassElem::new(MathClass::Large, body).pack().into()
}

/// Math opening class.
/// Default contains left delimiters such as `(`, `[`, `angle.l` (⟨),
/// and text operators such as `sin`, `op("foo")`.
///
/// ## Example
/// ```example
/// $ opening(<) a closing(>), < a > $
/// ```
///
/// Display: Math Opening
/// Category: math
/// Returns: content
#[func]
pub fn opening(
    /// The content to style.
    body: Content,
) -> Value {
    MathClassElem::new(MathClass::Opening, body).pack().into()
}

/// Math closing class.
/// Default contains right delimiters such as `)`, `]`, `angle.r` (⟩).
///
/// Display: Math Closing
/// Category: math
/// Returns: content
#[func]
pub fn closing(
    /// The content to style.
    body: Content,
) -> Value {
    MathClassElem::new(MathClass::Closing, body).pack().into()
}

/// Math binary operator class.
/// Default contains `+`, `*`, `times` (×) etc. .
///
/// ## Example
/// ```example
/// $ 1 binary(o) 2, 1 o 2 $
/// ```
///
/// Display: Math CLosing
/// Category: math
/// Returns: content
#[func]
pub fn binary(
    /// The content to style.
    body: Content,
) -> Value {
    MathClassElem::new(MathClass::Binary, body).pack().into()
}

/// Math relation class.
/// Default contains `=`, `>`, `succ` (≻) etc. .
///
/// ## Example
/// ```example
/// $ x relation(+)= y, x += y $
/// ```
///
/// Display: Math Relation
/// Category: math
/// Returns: content
#[func]
pub fn relation(
    /// The content to style.
    body: Content,
) -> Value {
    MathClassElem::new(MathClass::Relation, body).pack().into()
}

/// Math ordinary class.
/// Default contains all letters, digits and normal symbols.
///
/// ## Example
/// ```example
/// $ A^(1 ordinary(+)), A^(1 +) $
/// ```
///
/// Display: Math Relation
/// Category: math
/// Returns: content
#[func]
pub fn ordinary(
    /// The content to style.
    body: Content,
) -> Value {
    MathClassElem::new(MathClass::Normal, body).pack().into()
}

/// Math punctuation class.
/// Default contains `,`, `;` etc. .
///
/// ## Example
/// ```example
/// $ f : A -> B, f punctuation(:) A -> B $
/// ```
///
/// Display: Math Punctuation
/// Category: math
/// Returns: content
#[func]
pub fn punctuation(
    /// The content to style.
    body: Content,
) -> Value {
    MathClassElem::new(MathClass::Punctuation, body).pack().into()
}

/// Math content with specified math class.
///
/// Display: Class
/// Category: Math
#[element(LayoutMath)]
pub struct MathClassElem {
    /// specified math class of the content.
    #[required]
    pub class: MathClass,

    /// The math content.
    #[required]
    pub body: Content,
}

impl LayoutMath for MathClassElem {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let mut fragment = ctx.layout_fragment(&self.body())?;
        fragment.set_class(self.class());
        ctx.push(fragment);
        Ok(())
    }
}
