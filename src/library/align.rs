use super::*;

/// `align`: Align content along the layouting axes.
///
/// # Positional arguments
/// - At most two of `left`, `right`, `top`, `bottom`, `center`.
///
/// # Keyword arguments
/// - `horizontal`: Any of `left`, `right` or `center`.
/// - `vertical`: Any of `top`, `bottom` or `center`.
///
/// There may not be two alignment specifications for the same axis.
pub fn align(call: FuncCall, _: &ParseState) -> Pass<SyntaxNode> {
    let mut f = Feedback::new();
    let mut args = call.header.args;
    let node = AlignNode {
        body: call.body.map(|s| s.v),
        aligns: args.pos.all::<Spanned<SpecAlign>>().collect(),
        h: args.key.get::<Spanned<SpecAlign>>("horizontal", &mut f),
        v: args.key.get::<Spanned<SpecAlign>>("vertical", &mut f),
    };
    drain_args(args, &mut f);
    Pass::node(node, f)
}

#[derive(Debug, Clone, PartialEq)]
struct AlignNode {
    body: Option<SyntaxTree>,
    aligns: SpanVec<SpecAlign>,
    h: Option<Spanned<SpecAlign>>,
    v: Option<Spanned<SpecAlign>>,
}

#[async_trait(?Send)]
impl Layout for AlignNode {
    async fn layout<'a>(&'a self, mut ctx: LayoutContext<'_>) -> Pass<Commands<'a>> {
        let mut f = Feedback::new();

        ctx.base = ctx.spaces[0].size;

        let axes = ctx.axes;
        let all = self.aligns.iter()
            .map(|align| {
                let spec = align.v.axis().unwrap_or(axes.primary.axis());
                (spec, align)
            })
            .chain(self.h.iter().map(|align| (Horizontal, align)))
            .chain(self.v.iter().map(|align| (Vertical, align)));

        let mut had = [false; 2];
        for (axis, align) in all {
            if align.v.axis().map(|a| a != axis).unwrap_or(false) {
                error!(
                    @f, align.span,
                    "invalid alignment {} for {} axis", align.v, axis,
                );
            } else if had[axis as usize] {
                error!(@f, align.span, "duplicate alignment for {} axis", axis);
            } else {
                had[axis as usize] = true;
                let gen_axis = axis.to_generic(ctx.axes);
                let gen_align = align.v.to_generic(ctx.axes);
                *ctx.align.get_mut(gen_axis) = gen_align;
            }
        }

        Pass::new(match &self.body {
            Some(body) => {
                let layouted = layout(body, ctx).await;
                f.extend(layouted.feedback);
                vec![AddMultiple(layouted.output)]
            }
            None => vec![SetAlignment(ctx.align)],
        }, f)
    }
}
