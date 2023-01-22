use super::*;

const BRACED_GAP: Em = Em::new(0.3);

/// # Underbrace
/// A horizontal brace under content, with an optional annotation below.
///
/// ## Example
/// ```
/// $ underbrace(1 + 2 + ... + 5, "numbers") $
/// ```
///
/// ## Parameters
/// - body: Content (positional, required)
///   The content above the brace.
///
/// - annotation: Content (positional)
///   The optional content below the brace.
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct UnderbraceNode {
    /// The content above the brace.
    pub body: Content,
    /// The optional content below the brace.
    pub annotation: Option<Content>,
}

#[node]
impl UnderbraceNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let body = args.expect("body")?;
        let annotation = args.eat()?;
        Ok(Self { body, annotation }.pack())
    }
}

impl LayoutMath for UnderbraceNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let gap = BRACED_GAP.scaled(ctx);
        let body = ctx.layout_row(&self.body)?;
        let glyph = GlyphFragment::new(ctx, '⏟');
        let brace = glyph.stretch_horizontal(ctx, body.width(), Abs::zero());

        let mut rows = vec![body, brace.into()];
        ctx.style(ctx.style.for_subscript());
        rows.extend(
            self.annotation
                .as_ref()
                .map(|annotation| ctx.layout_row(annotation))
                .transpose()?,
        );
        ctx.unstyle();
        ctx.push(stack(ctx, rows, Align::Center, gap, 0));

        Ok(())
    }
}

/// # Overbrace
/// A horizontal brace over content, with an optional annotation above.
///
/// ## Example
/// ```
/// $ overbrace(1 + 2 + ... + 5, "numbers") $
/// ```
///
/// ## Parameters
/// - body: Content (positional, required)
///   The content below the brace.
///
/// - annotation: Content (positional)
///   The optional content above the brace.
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct OverbraceNode {
    /// The content below the brace.
    pub body: Content,
    /// The optional content above the brace.
    pub annotation: Option<Content>,
}

#[node]
impl OverbraceNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let body = args.expect("body")?;
        let annotation = args.eat()?;
        Ok(Self { body, annotation }.pack())
    }
}

impl LayoutMath for OverbraceNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let gap = BRACED_GAP.scaled(ctx);
        let body = ctx.layout_row(&self.body)?;
        let glyph = GlyphFragment::new(ctx, '⏞');
        let brace = glyph.stretch_horizontal(ctx, body.width(), Abs::zero());

        let mut rows = vec![];
        ctx.style(ctx.style.for_superscript());
        rows.extend(
            self.annotation
                .as_ref()
                .map(|annotation| ctx.layout_row(annotation))
                .transpose()?,
        );
        ctx.unstyle();
        rows.push(brace.into());
        rows.push(body);

        let last = rows.len() - 1;
        ctx.push(stack(ctx, rows, Align::Center, gap, last));

        Ok(())
    }
}
