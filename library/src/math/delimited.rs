use super::*;

/// How much less high scaled delimiters can be than what they wrap.
pub(super) const DELIM_SHORT_FALL: Em = Em::new(0.1);

/// Scales delimiters.
///
/// While matched delimiters scale by default, this can be used to scale
/// unmatched delimiters and to control the delimiter scaling more precisely.
///
/// ## Example
/// ```example
/// $ lr(]a, b/2]) $
/// $ lr(]sum_(x=1)^n] x, size: #50%) $
/// ```
///
/// Display: Left/Right
/// Category: math
#[node(LayoutMath)]
pub struct LrNode {
    /// The size of the brackets, relative to the height of the wrapped content.
    ///
    /// Defaults to `{100%}`.
    pub size: Smart<Rel<Length>>,

    /// The delimited content, including the delimiters.
    #[positional]
    #[required]
    #[parse(
        let mut body = Content::empty();
        for (i, arg) in args.all::<Content>()?.into_iter().enumerate() {
            if i > 0 {
                body += TextNode::packed(',');
            }
            body += arg;
        }
        body
    )]
    pub body: Content,
}

impl LayoutMath for LrNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let mut body = self.body();
        if let Some(node) = body.to::<LrNode>() {
            if node.size(ctx.styles()).is_auto() {
                body = node.body();
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
            MathFragment::Variant(variant) => GlyphFragment::new(ctx, variant.c),
            _ => return,
        };

        let short_fall = DELIM_SHORT_FALL.scaled(ctx);
        *fragment =
            MathFragment::Variant(glyph.stretch_vertical(ctx, height, short_fall));

        if let Some(class) = apply {
            fragment.set_class(class);
        }
    }
}

/// Floor an expression.
///
/// ## Example
/// ```example
/// $ floor(x/2) $
/// ```
///
/// ## Parameters
/// - body: `Content` (positional, required)
///   The expression to floor.
///
/// Display: Floor
/// Category: math
#[func]
pub fn floor(args: &mut Args) -> SourceResult<Value> {
    delimited(args, '⌊', '⌋')
}

/// Ceil an expression.
///
/// ## Example
/// ```example
/// $ ceil(x/2) $
/// ```
///
/// ## Parameters
/// - body: `Content` (positional, required)
///   The expression to ceil.
///
/// Display: Ceil
/// Category: math
#[func]
pub fn ceil(args: &mut Args) -> SourceResult<Value> {
    delimited(args, '⌈', '⌉')
}

/// Take the absolute value of an expression.
///
/// ## Example
/// ```example
/// $ abs(x/2) $
/// ```
///
/// ## Parameters
/// - body: `Content` (positional, required)
///   The expression to take the absolute value of.
///
/// Display: Abs
/// Category: math
#[func]
pub fn abs(args: &mut Args) -> SourceResult<Value> {
    delimited(args, '|', '|')
}

/// Take the norm of an expression.
///
/// ## Example
/// ```example
/// $ norm(x/2) $
/// ```
///
/// ## Parameters
/// - body: `Content` (positional, required)
///   The expression to take the norm of.
///
/// Display: Norm
/// Category: math
#[func]
pub fn norm(args: &mut Args) -> SourceResult<Value> {
    delimited(args, '‖', '‖')
}

fn delimited(args: &mut Args, left: char, right: char) -> SourceResult<Value> {
    Ok(Value::Content(
        LrNode::new(Content::sequence(vec![
            TextNode::packed(left),
            args.expect::<Content>("body")?,
            TextNode::packed(right),
        ]))
        .pack(),
    ))
}
