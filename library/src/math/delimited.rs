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

        let ScalingRulesInternal {
            has_scaled_opening,
            has_scaled_closing,
            scale_fences,
        } = *ScalingRules::get_rules(&self.scale(ctx.styles()), &fragments);
        let (content_start, mut content_len) =
            match (has_scaled_opening, has_scaled_closing, fragments.as_mut_slice()) {
                // TODO: ???
                (_, _, [one]) => {
                    scale(ctx, one, height, None);
                    (0, 0)
                }
                (true, true, [first, content @ .., last]) => {
                    scale(ctx, first, height, Some(MathClass::Opening));
                    scale(ctx, last, height, Some(MathClass::Closing));
                    (1, content.len())
                }
                (true, false, [first, content @ ..]) => {
                    scale(ctx, first, height, Some(MathClass::Opening));
                    (1, content.len())
                }
                (false, true, [content @ .., last]) => {
                    scale(ctx, last, height, Some(MathClass::Closing));
                    (0, content.len())
                }
                (false, false, content) => (0, content.len()),
                (_, _, []) => (0, 0),
            };

        'fence_types: for fence in scale_fences {
            println!("fence type: {:?}", fence.search);
            let mut content_idx = 0;
            'math_fragments_in_content: while content_idx < content_len {
                let content = &fragments[content_start..content_start + content_len];
                // Find start of fence in content
                if content.len() <= content_idx + fence.search.len() {
                    continue 'fence_types;
                }
                let mut search_idx = 0;
                while search_idx < fence.search.len() {
                    match &content[content_idx + search_idx] {
                        MathFragment::Glyph(g) if g.c == fence.search[search_idx] => (),
                        MathFragment::Variant(v) if v.c == fence.search[search_idx] => (),
                        _ => {
                            content_idx += 1;
                            continue 'math_fragments_in_content;
                        }
                    };
                    search_idx += 1;
                }

                // Fence is found, make replacements
                println!("before:{:?}", content);
                let fragments_len_prev = fragments.len();
                let scale_len = (fence.replace)(
                    ctx,
                    &mut fragments,
                    content_start + content_idx,
                    fence.search.len(),
                );
                // Preserve this order of operations, to avoid negative numbers
                content_len = (content_len + fragments.len()) - fragments_len_prev;

                // Scale fence
                let content = &mut fragments[content_start..content_start + content_len];
                println!(" after:{:?}", content);
                for i in 0..scale_len {
                    scale(
                        ctx,
                        &mut content[content_idx + i],
                        height,
                        Some(MathClass::Fence),
                    );
                }

                content_idx += 1;
            }
        }

        ctx.extend(fragments);

        Ok(())
    }
}

#[derive(Debug, Cast)]
enum ScalingRules {
    #[string("delim")]
    DelimetersOnly,
    #[string("opening")]
    OpeningDelimeterOnly,
    #[string("closing")]
    ClosingDelimeterOnly,
    #[string("sigil")]
    FenceSigil,
    #[string("set")]
    Set,
    #[string("braket")]
    Braket,
    #[string("ketbra")]
    Ketbra,
}

impl ScalingRules {
    fn get_rules<'a>(
        rules: &'a Smart<Self>,
        fragments: &[MathFragment],
    ) -> &'a ScalingRulesInternal {
        match rules {
            Smart::Auto => Self::get_rules_auto(fragments),
            Smart::Custom(x) => x.get_rules_specified(),
        }
    }

    fn get_rules_auto(fragments: &[MathFragment]) -> &'static ScalingRulesInternal {
        if let [MathFragment::Glyph(GlyphFragment { c: open, .. })
        | MathFragment::Variant(VariantFragment { c: open, .. }), .., MathFragment::Glyph(GlyphFragment { c: close, .. })
        | MathFragment::Variant(VariantFragment { c: close, .. })] = fragments
        {
            println!("open close {:?} {:?}", open, close);
            match (open, close) {
                ('{', '}') => &SCALE_SET,
                ('(', ')') => &SCALE_FENCE_BY_SIGIL,
                // TODO: transform '<', '>' into angle brackets
                // TODO: prevent parser pairing '|' within `lr(< x | y | z>)`
                ('⟨', '⟩') | ('<', '>') => &SCALE_BRAKET,
                ('|', '|') => &SCALE_KETBRA_PROJECTION,
                _ => &SCALE_DELIMS_ONLY,
            }
        } else {
            &SCALE_DELIMS_ONLY
        }
    }

    fn get_rules_specified(&self) -> &ScalingRulesInternal {
        match self {
            ScalingRules::DelimetersOnly => &SCALE_DELIMS_ONLY,
            ScalingRules::OpeningDelimeterOnly => &SCALE_OPENING_DELIM_ONLY,
            ScalingRules::ClosingDelimeterOnly => &SCALE_CLOSING_DELIM_ONLY,
            ScalingRules::FenceSigil => &SCALE_FENCE_BY_SIGIL,
            ScalingRules::Set => &SCALE_SET,
            ScalingRules::Braket => &SCALE_BRAKET,
            ScalingRules::Ketbra => &SCALE_KETBRA_PROJECTION,
        }
    }
}

struct ScalingRulesInternal {
    /// Has opening grapheme needing scaling
    has_scaled_opening: bool,
    /// Has closing grapheme needing scaling
    has_scaled_closing: bool,
    scale_fences: &'static [Fence],
}

struct Fence {
    search: &'static [char],
    /// Passed matching string and one grapheme after, and the index between them.
    ///
    /// Returns value to be used in output and scaled, and how much of input to replace.
    ///
    /// This is very contrived, and mainly designed to support three cases
    /// - Fence string, e.g. `{ x | x < 3 }`, [`Fence::replace_self`].
    /// - Fence sigil which promotes next grapheme, e.g. `{ x `| x < 3 }` uses the backtick as a sigil, [`Fence::replace_next`].
    /// - Fence replacement string, e.g. `|x><y|` replaces the "><" with "⟩⟨", which isn't as easy to type.
    replace: fn(&mut MathContext, &mut Vec<MathFragment>, usize, usize) -> usize,
}
impl Fence {
    pub fn new(fence: &'static [char]) -> Self {
        Self { search: fence, replace: Self::replace_self }
    }

    pub fn new_sigil(sigil: &'static [char]) -> Self {
        Self { search: sigil, replace: Self::replace_next }
    }

    fn replace_self(
        _ctx: &mut MathContext,
        _content: &mut Vec<MathFragment>,
        _idx_start: usize,
        search_len: usize,
    ) -> usize {
        search_len
    }

    fn replace_next(
        _ctx: &mut MathContext,
        content: &mut Vec<MathFragment>,
        idx_start: usize,
        search_len: usize,
    ) -> usize {
        content.splice(idx_start..idx_start + search_len, std::iter::empty());
        1
    }
}

const SCALE_DELIMS_ONLY: ScalingRulesInternal = ScalingRulesInternal {
    has_scaled_opening: true,
    has_scaled_closing: true,
    scale_fences: &[],
};
const SCALE_OPENING_DELIM_ONLY: ScalingRulesInternal = ScalingRulesInternal {
    has_scaled_opening: true,
    has_scaled_closing: false,
    scale_fences: &[],
};
const SCALE_CLOSING_DELIM_ONLY: ScalingRulesInternal = ScalingRulesInternal {
    has_scaled_opening: false,
    has_scaled_closing: true,
    scale_fences: &[],
};
const SCALE_FENCE_BY_SIGIL: ScalingRulesInternal = ScalingRulesInternal {
    has_scaled_opening: true,
    has_scaled_closing: true,
    // scale_fences: &[Fence::new_sigil(&['`'])],
    scale_fences: &[Fence { search: &['`'], replace: Fence::replace_next }],
};
const SCALE_SET: ScalingRulesInternal = ScalingRulesInternal {
    has_scaled_opening: true,
    has_scaled_closing: true,
    // scale_fences: &[Fence::new(&['|'])],
    scale_fences: &[Fence { search: &['|'], replace: Fence::replace_self }],
};
const SCALE_BRAKET: ScalingRulesInternal = ScalingRulesInternal {
    has_scaled_opening: true,
    has_scaled_closing: true,
    // scale_fences: &[Fence::new(&['|'])],
    scale_fences: &[Fence { search: &['|'], replace: Fence::replace_self }],
};
fn replace_ketbra(
    ctx: &mut MathContext,
    content: &mut Vec<MathFragment>,
    idx_start: usize,
    search_len: usize,
) -> usize {
    let span0 = match &content[idx_start] {
        MathFragment::Glyph(g) => g.span,
        MathFragment::Variant(v) => v.span,
        _ => todo!(),
    };
    let span1 = match &content[idx_start + 1] {
        MathFragment::Glyph(g) => g.span,
        MathFragment::Variant(v) => v.span,
        _ => todo!(),
    };
    content.splice(
        idx_start..idx_start + search_len,
        [
            MathFragment::Glyph(GlyphFragment::new(ctx, '⟩', span0)),
            MathFragment::Glyph(GlyphFragment::new(ctx, '⟨', span1)),
        ],
    );
    2
}
const SCALE_KETBRA_PROJECTION: ScalingRulesInternal = ScalingRulesInternal {
    has_scaled_opening: true,
    has_scaled_closing: true,
    // Fence::new(&['⟩', '⟨']),
    scale_fences: &[
        Fence {
            search: &['⟩', '⟨'], replace: Fence::replace_self
        },
        Fence { search: &['>', '<'], replace: replace_ketbra },
    ],
};

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
pub fn test_sigil(body: Content) -> Value {
    delimited(body, '{', '}', Smart::Custom(ScalingRules::FenceSigil))
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
