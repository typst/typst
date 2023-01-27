use super::*;

/// # Atom
/// An atom in a math formula: `x`, `+`, `12`.
///
/// ## Parameters
/// - text: EcoString (positional, required)
///   The atom's text.
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct AtomNode(pub EcoString);

#[node]
impl AtomNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("text")?).pack())
    }
}

impl LayoutMath for AtomNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let mut chars = self.0.chars();
        if let Some(glyph) = chars
            .next()
            .filter(|_| chars.next().is_none())
            .and_then(|c| GlyphFragment::try_new(ctx, c))
        {
            // A single letter that is available in the math font.
            if ctx.style.size == MathSize::Display
                && glyph.class == Some(MathClass::Large)
            {
                let height = scaled!(ctx, display_operator_min_height);
                ctx.push(glyph.stretch_vertical(ctx, height, Abs::zero()));
            } else {
                ctx.push(glyph);
            }
        } else if self.0.chars().all(|c| c.is_ascii_digit()) {
            // A number that should respect math styling and can therefore
            // not fall back to the normal text layout.
            let mut vec = vec![];
            for c in self.0.chars() {
                vec.push(GlyphFragment::new(ctx, c).into());
            }
            let frame = MathRow(vec).to_frame(ctx);
            ctx.push(frame);
        } else {
            // Anything else is handled by Typst's standard text layout.
            let frame = ctx.layout_non_math(&TextNode(self.0.clone()).pack())?;
            ctx.push(FrameFragment::new(frame).with_class(MathClass::Alphabetic));
        }

        Ok(())
    }
}
