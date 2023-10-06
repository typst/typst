use super::*;

/// How much less high scaled delimiters can be than what they wrap.
pub(super) const DELIM_SHORT_FALL: Em = Em::new(0.1);

/// Scales delimiters.
///
/// While matched delimiters scale by default, this can be used to scale
/// unmatched delimiters and to control the delimiter scaling more precisely.
#[elem(title = "Left/Right", LayoutMath)]
pub struct LrElem {
    /// The size of the brackets, relative to the height of the wrapped content.
    pub size: Smart<Rel<Length>>,

    /// The delimited content, including the delimiters.
    #[required]
    #[parse(
        let mut body = Content::empty();
        for (i, arg) in args.all::<Content>()?.into_iter().enumerate() {
            if i > 0 {
                body += TextElem::packed(',');
            }
            body += arg;
        }
        body
    )]
    pub body: Content,
}

impl LayoutMath for LrElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let mut body = self.body();
        if let Some(elem) = body.to::<LrElem>() {
            if elem.size(ctx.styles()).is_auto() {
                body = elem.body();
            }
        }

        let mut fragments = ctx.layout_fragments(&body)?;
        let axis = scaled!(ctx, axis_height);
        let max_extent = fragments
            .iter()
            .map(|fragment| (fragment.ascent() - axis).max(fragment.descent() + axis))
            .max()
            .unwrap_or_default();

        let height = self
            .size(ctx.styles())
            .unwrap_or(Rel::one())
            .resolve(ctx.styles())
            .relative_to(2.0 * max_extent);

        match fragments.as_mut_slice() {
            [one] => scale(ctx, one, height, None),
            [first, .., last] => {
                scale(ctx, first, height, Some(MathClass::Opening));
                scale(ctx, last, height, Some(MathClass::Closing));
            }
            _ => {}
        }

        ctx.extend(fragments);

        Ok(())
    }
}

/// Scale a math fragment to a height.
fn scale(
    ctx: &mut MathContext,
    fragment: &mut MathFragment,
    height: Abs,
    apply: Option<MathClass>,
) {
    if matches!(
        fragment.class(),
        Some(MathClass::Opening | MathClass::Closing | MathClass::Fence)
    ) {
        let glyph = match fragment {
            MathFragment::Glyph(glyph) => glyph.clone(),
            MathFragment::Variant(variant) => {
                GlyphFragment::new(ctx, variant.c, variant.span)
            }
            _ => return,
        };

        let short_fall = DELIM_SHORT_FALL.scaled(ctx);
        let mut stretched = glyph.stretch_vertical(ctx, height, short_fall);
        stretched.center_on_axis(ctx);

        *fragment = MathFragment::Variant(stretched);
        if let Some(class) = apply {
            fragment.set_class(class);
        }
    }
}

/// Floors an expression.
///
/// ```example
/// $ floor(x/2) $
/// ```
#[func]
pub fn floor(
    /// The size of the brackets, relative to the height of the wrapped content.
    #[named]
    size: Option<Smart<Rel<Length>>>,
    /// The expression to floor.
    body: Content,
) -> Content {
    delimited(body, '⌊', '⌋', size)
}

/// Ceils an expression.
///
/// ```example
/// $ ceil(x/2) $
/// ```
#[func]
pub fn ceil(
    /// The size of the brackets, relative to the height of the wrapped content.
    #[named]
    size: Option<Smart<Rel<Length>>>,
    /// The expression to ceil.
    body: Content,
) -> Content {
    delimited(body, '⌈', '⌉', size)
}

/// Rounds an expression.
///
/// ```example
/// $ round(x/2) $
/// ```
#[func]
pub fn round(
    /// The size of the brackets, relative to the height of the wrapped content.
    #[named]
    size: Option<Smart<Rel<Length>>>,
    /// The expression to round.
    body: Content,
) -> Content {
    delimited(body, '⌊', '⌉', size)
}

/// Takes the absolute value of an expression.
///
/// ```example
/// $ abs(x/2) $
/// ```
#[func]
pub fn abs(
    /// The size of the brackets, relative to the height of the wrapped content.
    #[named]
    size: Option<Smart<Rel<Length>>>,
    /// The expression to take the absolute value of.
    body: Content,
) -> Content {
    delimited(body, '|', '|', size)
}

/// Takes the norm of an expression.
///
/// ```example
/// $ norm(x/2) $
/// ```
#[func]
pub fn norm(
    /// The size of the brackets, relative to the height of the wrapped content.
    #[named]
    size: Option<Smart<Rel<Length>>>,
    /// The expression to take the norm of.
    body: Content,
) -> Content {
    delimited(body, '‖', '‖', size)
}

fn delimited(
    body: Content,
    left: char,
    right: char,
    size: Option<Smart<Rel<Length>>>,
) -> Content {
    let mut elem = LrElem::new(Content::sequence([
        TextElem::packed(left),
        body,
        TextElem::packed(right),
    ]));
    // Push size only if size is provided
    if let Some(size) = size {
        elem.push_size(size);
    }
    elem.pack()
}
