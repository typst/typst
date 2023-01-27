use super::*;

/// How much less high scaled delimiters can be than what they wrap.
pub(super) const DELIM_SHORT_FALL: Em = Em::new(0.1);

/// # Left-Right
/// Scales delimiters.
///
/// While matched delimiters scale by default, this can be used to scale
/// unmatched delimiters and to control the delimiter scaling more precisely.
///
/// ## Example
/// ```
/// $ lr(]a, b/2]) $
/// ```
///
/// ## Parameters
/// - body: Content (positional, variadic)
///   The delimited content, including the delimiters.
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct LrNode(pub Content);

#[node]
impl LrNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let mut body = Content::empty();
        for (i, arg) in args.all::<Content>()?.into_iter().enumerate() {
            if i > 0 {
                body += AtomNode(','.into()).pack();
            }
            body += arg;
        }
        Ok(Self(body).pack())
    }
}

impl LayoutMath for LrNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let mut row = ctx.layout_row(&self.0)?;

        let axis = scaled!(ctx, axis_height);
        let max_extent = row
            .0
            .iter()
            .map(|fragment| (fragment.ascent() - axis).max(fragment.descent() + axis))
            .max()
            .unwrap_or_default();

        let height = 2.0 * max_extent;
        if let [first, .., last] = row.0.as_mut_slice() {
            for fragment in [first, last] {
                if !matches!(
                    fragment.class(),
                    Some(MathClass::Opening | MathClass::Closing | MathClass::Fence)
                ) {
                    continue;
                }

                let MathFragment::Glyph(glyph) = *fragment else { continue };
                let short_fall = DELIM_SHORT_FALL.scaled(ctx);
                *fragment = MathFragment::Variant(
                    glyph.stretch_vertical(ctx, height, short_fall),
                );
            }
        }

        ctx.extend(row);

        Ok(())
    }
}

/// # Floor
/// Floor an expression.
///
/// ## Example
/// ```
/// $ floor(x/2) $
/// ```
///
/// ## Parameters
/// - body: Content (positional, required)
///   The expression to floor.
///
/// ## Category
/// math
#[func]
pub fn floor(args: &mut Args) -> SourceResult<Value> {
    delimited(args, '⌊', '⌋')
}

/// # Ceil
/// Ceil an expression.
///
/// ## Example
/// ```
/// $ ceil(x/2) $
/// ```
///
/// ## Parameters
/// - body: Content (positional, required)
///   The expression to ceil.
///
/// ## Category
/// math
#[func]
pub fn ceil(args: &mut Args) -> SourceResult<Value> {
    delimited(args, '⌈', '⌉')
}

/// # Abs
/// Take the absolute value of an expression.
///
/// ## Example
/// ```
/// $ abs(x/2) $
/// ```
///
/// ## Parameters
/// - body: Content (positional, required)
///   The expression to take the absolute value of.
///
/// ## Category
/// math
#[func]
pub fn abs(args: &mut Args) -> SourceResult<Value> {
    delimited(args, '|', '|')
}

/// # Norm
/// Take the norm of an expression.
///
/// ## Example
/// ```
/// $ norm(x/2) $
/// ```
///
/// ## Parameters
/// - body: Content (positional, required)
///   The expression to take the norm of.
///
/// ## Category
/// math
#[func]
pub fn norm(args: &mut Args) -> SourceResult<Value> {
    delimited(args, '‖', '‖')
}

fn delimited(args: &mut Args, left: char, right: char) -> SourceResult<Value> {
    Ok(Value::Content(
        LrNode(Content::sequence(vec![
            AtomNode(left.into()).pack(),
            args.expect::<Content>("body")?,
            AtomNode(right.into()).pack(),
        ]))
        .pack(),
    ))
}
