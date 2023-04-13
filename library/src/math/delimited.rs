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
#[element(LayoutMath)]
pub struct LrElem {
    /// The size of the brackets, relative to the height of the wrapped content.
    ///
    /// Defaults to `{100%}`.
    pub size: Smart<Rel<Length>>,

    scale: Smart<ScalingRules>,

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

/// Scale fence between delimeters.
///
/// While unambiguous fences scale by default, some cases must be marked explicitely.
/// When used within matched brackets or an `lr`, this function will cause its
/// content to scale to the same height as the closest surrounding delimeters.
///
/// ## Example
/// ```example
/// $ lr({a mid(|) |a| < 3 }) $
/// ```
///
/// Display: Mid
/// Category: math
#[element(LayoutMath)]
pub struct MidElem {
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
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        println!("{:?}", self.scale(ctx.styles()));
        let mut body = self.body();
        if let Some(elem) = body.to::<LrElem>() {
            if elem.size(ctx.styles()).is_auto() {
                body = elem.body();
            }
        }

        let mut fragments = ctx.layout_fragments(&body)?;
        println!("{:?}", fragments);
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

        let rules = &self.scale(ctx.styles());
        let scale_fences = ScalingRules::get_fences(rules, &fragments);
        match fragments.as_mut_slice() {
            // TODO: ???
            [one] => {
                scale(ctx, one, height, None);
            }
            [first, _content @ .., last] => {
                scale(ctx, first, height, Some(MathClass::Opening));
                scale(ctx, last, height, Some(MathClass::Closing));

                // Scale `mid(..)` elems
                let mut i = 1;
                while i < fragments.len() - 1 {
                    if matches!(
                        fragments[i],
                        MathFragment::Glyph(GlyphFragment { c: MidElem::START_CHAR, .. })
                    ) {
                        let end = i
                            + 1
                            + fragments
                                .iter()
                                .skip(i + 1)
                                .position(|x| {
                                    matches!(
                                        x,
                                        MathFragment::Glyph(GlyphFragment {
                                            c: MidElem::END_CHAR,
                                            ..
                                        })
                                    )
                                })
                                .expect("unparied MidElem::START_CHAR");
                        fragments.remove(end);
                        fragments.remove(i);
                        for j in i..end - 1 {
                            scale(ctx, &mut fragments[j], height, Some(MathClass::Fence));
                        }
                    }
                    i += 1;
                }

                let [_, content @ .., _] = fragments.as_mut_slice()  else{ unreachable!() };
                for fence in scale_fences {
                    let mut from_start = fence.scale_from_start;
                    let mut from_end = fence.scale_from_end;

                    println!("fence type: {:?}", fence.search);
                    println!("before:{:?}", content);
                    let mut i = 0;
                    let mut forward = true;
                    let mut end_backward = 0;
                    // for i in 0..content.len() - (fence.search.len() - 1) {
                    'content: loop {
                        if forward {
                            if from_start == 0 {
                                forward = false;
                                end_backward = i;
                                i = content.len() - 1;
                            } else if i == content.len() {
                                break;
                            }
                        }
                        if !forward && (i == end_backward - 1 || from_end == 0) {
                            break;
                        }

                        for search_idx in 0..fence.search.len() {
                            let c = match &content[i + search_idx] {
                                MathFragment::Glyph(g) => Some(g.c),
                                MathFragment::Variant(v) => Some(v.c),
                                _ => None,
                            };
                            if c != Some(fence.search[search_idx].0) {
                                if forward {
                                    i += 1;
                                } else {
                                    if i == 0 {
                                        break 'content;
                                    }
                                    i -= 1;
                                }
                                continue 'content;
                            }
                        }

                        for j in 0..fence.search.len() {
                            match &mut content[i + j] {
                                MathFragment::Glyph(g) if g.c != fence.search[j].1 => {
                                    content[i + j] =
                                        MathFragment::Glyph(GlyphFragment::new(
                                            ctx,
                                            fence.search[j].1,
                                            g.span,
                                        ));
                                }
                                MathFragment::Variant(v) if v.c != fence.search[j].1 => {
                                    content[i + j] =
                                        MathFragment::Glyph(GlyphFragment::new(
                                            ctx,
                                            fence.search[j].1,
                                            v.span,
                                        ));
                                }
                                _ => (),
                            }
                            println!("after:{:?}", content);
                            scale(
                                ctx,
                                &mut content[i + j],
                                height,
                                Some(MathClass::Fence),
                            );
                            println!("after 2:{:?}", content);
                        }

                        if forward {
                            from_start -= 1;
                            i += 1;
                        } else {
                            if i == 0 {
                                break;
                            }
                            from_end -= 1;
                            i -= 1;
                        }
                    }
                }
            }
            [] => (),
        }

        ctx.extend(fragments);

        Ok(())
    }
}

impl MidElem {
    const START_CHAR: char = '\u{E000}';
    const END_CHAR: char = '\u{E001}';
}

impl LayoutMath for MidElem {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let fragments = ctx.layout_fragments(&self.body())?;
        ctx.extend(vec![MathFragment::Glyph(GlyphFragment::new(
            ctx,
            Self::START_CHAR,
            Span::detached(),
        ))]);
        ctx.extend(fragments);
        ctx.extend(vec![MathFragment::Glyph(GlyphFragment::new(
            ctx,
            Self::END_CHAR,
            Span::detached(),
        ))]);

        Ok(())
    }
}

#[derive(Debug, Cast)]
enum ScalingRules {
    #[string("delim")]
    DelimetersOnly,
    #[string("set")]
    Set,
    #[string("braket")]
    Braket,
    #[string("ketbra")]
    Ketbra,
}

impl ScalingRules {
    fn get_fences<'a>(rules: &'a Smart<Self>, fragments: &[MathFragment]) -> &'a [Fence] {
        match rules {
            Smart::Auto => Self::get_fences_auto(fragments),
            Smart::Custom(x) => x.get_fences_specified(),
        }
    }

    fn get_fences_auto(fragments: &[MathFragment]) -> &'static [Fence] {
        if let [MathFragment::Glyph(GlyphFragment { c: open, .. })
        | MathFragment::Variant(VariantFragment { c: open, .. }), .., MathFragment::Glyph(GlyphFragment { c: close, .. })
        | MathFragment::Variant(VariantFragment { c: close, .. })] = fragments
        {
            println!("open close {:?} {:?}", open, close);
            match (open, close) {
                // Use set scaling for parentheses as well, for `P(A | B)`.
                ('(', ')') | ('{', '}') => &SCALE_SET,
                // TODO: transform '<', '>' into angle brackets
                // TODO: prevent parser pairing '|' within `lr(< x | y | z>)`
                ('⟨', '⟩') | ('<', '>') => SCALE_BRAKET,
                ('|', '|') => SCALE_KETBRA_PROJECTION,
                _ => SCALE_DELIMS_ONLY,
            }
        } else {
            &SCALE_DELIMS_ONLY
        }
    }

    fn get_fences_specified(&self) -> &[Fence] {
        match self {
            ScalingRules::DelimetersOnly => SCALE_DELIMS_ONLY,
            ScalingRules::Set => SCALE_SET,
            ScalingRules::Braket => SCALE_BRAKET,
            ScalingRules::Ketbra => SCALE_KETBRA_PROJECTION,
        }
    }
}

struct Fence {
    scale_from_start: usize,
    scale_from_end: usize,
    search: &'static [(char, char)],
}
const SCALE_DELIMS_ONLY: &[Fence] = &[];
const SCALE_SET: &[Fence] = &[Fence {
    scale_from_start: 1,
    scale_from_end: 0,
    search: &[('|', '|')],
}];
const SCALE_BRAKET: &[Fence] = &[Fence {
    scale_from_start: 1,
    scale_from_end: 1,
    search: &[('|', '|')],
}];
const SCALE_KETBRA_PROJECTION: &[Fence] = &[
    Fence {
        scale_from_start: 1,
        scale_from_end: 0,
        search: &[('⟩', '⟩'), ('⟨', '⟨')],
    },
    Fence {
        scale_from_start: 1,
        scale_from_end: 0,
        search: &[('>', '⟩'), ('<', '⟨')],
    },
];

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
/// Display: Floor
/// Category: math
/// Returns: content
#[func]
pub fn floor(
    /// The expression to floor.
    body: Content,
) -> Value {
    delimited(body, '⌊', '⌋', Smart::Custom(ScalingRules::DelimetersOnly))
}

/// Ceil an expression.
///
/// ## Example
/// ```example
/// $ ceil(x/2) $
/// ```
///
/// Display: Ceil
/// Category: math
/// Returns: content
#[func]
pub fn ceil(
    /// The expression to ceil.
    body: Content,
) -> Value {
    delimited(body, '⌈', '⌉', Smart::Custom(ScalingRules::DelimetersOnly))
}

/// Round an expression.
///
/// ## Example
/// ```example
/// $ round(x/2) $
/// ```
///
/// Display: Round
/// Category: math
/// Returns: content
#[func]
pub fn round(
    /// The expression to round.
    body: Content,
) -> Value {
    delimited(body, '⌊', '⌉', Smart::Custom(ScalingRules::DelimetersOnly))
}

/// Take the absolute value of an expression.
///
/// ## Example
/// ```example
/// $ abs(x/2) $
/// ```
///
///
/// Display: Abs
/// Category: math
/// Returns: content
#[func]
pub fn abs(
    /// The expression to take the absolute value of.
    body: Content,
) -> Value {
    delimited(body, '|', '|', Smart::Custom(ScalingRules::DelimetersOnly))
}

/// Take the norm of an expression.
///
/// ## Example
/// ```example
/// $ norm(x/2) $
/// ```
///
/// Display: Norm
/// Category: math
/// Returns: content
#[func]
pub fn norm(
    /// The expression to take the norm of.
    body: Content,
) -> Value {
    delimited(body, '‖', '‖', Smart::Custom(ScalingRules::DelimetersOnly))
}

/// Display: Norm
/// Category: math
/// Returns: content
#[func]
pub fn set(body: Content) -> Value {
    delimited(body, '{', '}', Smart::Custom(ScalingRules::Set))
}

/// Display: Norm
/// Category: math
/// Returns: content
#[func]
pub fn bra(body: Content) -> Value {
    delimited(body, '⟨', '|', Smart::Custom(ScalingRules::DelimetersOnly))
}

/// Display: Norm
/// Category: math
/// Returns: content
#[func]
pub fn ket(body: Content) -> Value {
    delimited(body, '|', '⟩', Smart::Custom(ScalingRules::DelimetersOnly))
}

/// Display: Norm
/// Category: math
/// Returns: content
#[func]
pub fn braket(body: Content) -> Value {
    delimited(body, '⟨', '⟩', Smart::Custom(ScalingRules::Braket))
}

/// Display: Norm
/// Category: math
/// Returns: content
#[func]
pub fn ketbra(body: Content) -> Value {
    delimited(body, '|', '|', Smart::Custom(ScalingRules::Ketbra))
}

fn delimited(
    body: Content,
    left: char,
    right: char,
    scaling_rules: Smart<ScalingRules>,
) -> Value {
    LrElem::new(Content::sequence([
        TextElem::packed(left),
        body,
        TextElem::packed(right),
    ]))
    .with_scale(scaling_rules)
    .pack()
    .into()
}
