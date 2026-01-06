use codex::styling::{MathStyle, to_style};
use ecow::EcoString;
use typst_syntax::{Span, is_newline};
use typst_utils::SliceExt;
use unicode_segmentation::UnicodeSegmentation;

use crate::diag::{SourceResult, bail, warning};
use crate::engine::Engine;
use crate::foundations::{Content, Packed, Resolve, StyleChain, Styles, SymbolElem};
use crate::introspection::{SplitLocator, TagElem};
use crate::layout::{Abs, Axes, BoxElem, FixedAlignment, HElem, Ratio, Rel, Spacing};
use crate::math::*;
use crate::routines::{Arenas, RealizationKind};
use crate::text::{LinebreakElem, SpaceElem, TextElem, is_default_ignorable};
use crate::visualize::FixedStroke;

/// How much the accent can be shorter than the base.
const ACCENT_SHORT_FALL: Em = Em::new(0.5);

/// How much less high scaled delimiters can be than what they wrap.
const DELIM_SHORT_FALL: Em = Em::new(0.1);

/// How much padding to add around each side of a fraction.
const FRAC_AROUND: Em = Em::new(0.1);

/// Resolves an equation's body into a [`MathItem`].
///
/// The returned `MathItem` has the same lifetime as the provided arenas.
#[typst_macros::time(name = "math ir creation")]
pub fn resolve_equation<'a>(
    elem: &'a Packed<EquationElem>,
    engine: &mut Engine,
    locator: &mut SplitLocator<'a>,
    arenas: &'a Arenas,
    styles: StyleChain<'a>,
) -> SourceResult<MathItem<'a>> {
    let mut context = MathResolver::new(engine, locator, arenas);
    context.resolve_into_item(&elem.body, styles)
}

/// The math IR builder.
struct MathResolver<'a, 'v, 'e> {
    // External.
    engine: &'v mut Engine<'e>,
    locator: &'v mut SplitLocator<'a>,
    arenas: &'a Arenas,
    // Mutable.
    items: Vec<MathItem<'a>>,
}

impl<'a, 'v, 'e> MathResolver<'a, 'v, 'e> {
    /// Create a new math builder.
    fn new(
        engine: &'v mut Engine<'e>,
        locator: &'v mut SplitLocator<'a>,
        arenas: &'a Arenas,
    ) -> Self {
        Self { engine, locator, arenas, items: vec![] }
    }

    /// Lifetime-extends some styles.
    fn store_styles(&self, styles: impl Into<Styles>) -> &'a Styles {
        self.arenas.styles.alloc(styles.into())
    }

    /// Lifetime-extends a style chain and chains a style onto it.
    fn chain_styles(
        &self,
        base: StyleChain<'a>,
        new: impl Into<Styles>,
    ) -> StyleChain<'a> {
        let new = self.arenas.styles.alloc(new.into());
        self.arenas.bump.alloc(base).chain(new)
    }

    /// Lifetime-extends some content.
    fn store(&self, content: Content) -> &'a Content {
        self.arenas.content.alloc(content)
    }

    /// Push an item.
    fn push(&mut self, item: impl Into<MathItem<'a>>) {
        self.items.push(item.into());
    }

    /// Resolve the given element and return the start index of the resulting
    /// [`MathItem`]s.
    fn resolve_into_items(
        &mut self,
        elem: &'a Content,
        styles: StyleChain<'a>,
    ) -> SourceResult<usize> {
        let start = self.items.len();
        self.resolve_into_self(elem, styles)?;
        Ok(start)
    }

    /// Resolve the given element and return the result as a [`MathItem`].
    fn resolve_into_item(
        &mut self,
        elem: &'a Content,
        styles: StyleChain<'a>,
    ) -> SourceResult<MathItem<'a>> {
        let start = self.resolve_into_items(elem, styles)?;
        let len = self.items.len() - start;
        Ok(if len == 1 {
            self.items.pop().unwrap()
        } else {
            GroupItem::create(self.items.drain(start..), false, styles, self.arenas)
        })
    }

    /// Resolve arbitrary content.
    fn resolve_into_self(
        &mut self,
        content: &'a Content,
        styles: StyleChain<'a>,
    ) -> SourceResult<()> {
        let pairs = (self.engine.routines.realize)(
            RealizationKind::Math,
            self.engine,
            self.locator,
            self.arenas,
            content,
            styles,
        )?;

        for (elem, styles) in pairs {
            resolve_realized(elem, self, styles)?;
        }

        Ok(())
    }
}

/// Resolves a leaf element resulting from realization.
fn resolve_realized<'a, 'v, 'e>(
    elem: &'a Content,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    if let Some(elem) = elem.to_packed::<SymbolElem>() {
        resolve_symbol(elem, ctx, styles)?;
    } else if elem.is::<SpaceElem>() {
        ctx.push(MathItem::Space);
    } else if let Some(elem) = elem.to_packed::<TextElem>() {
        resolve_text(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<AttachElem>() {
        resolve_attach(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<LrElem>() {
        resolve_lr(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<OpElem>() {
        resolve_op(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<HElem>() {
        resolve_h(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<OverlineElem>() {
        resolve_overline(elem, ctx, styles)?;
    } else if elem.is::<AlignPointElem>() {
        ctx.push(MathItem::Align);
    } else if let Some(elem) = elem.to_packed::<PrimesElem>() {
        resolve_primes(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<TagElem>() {
        ctx.push(MathItem::Tag(elem.tag.clone()));
    } else if let Some(elem) = elem.to_packed::<ClassElem>() {
        resolve_class(elem, ctx, styles)?;
    } else if elem.is::<LinebreakElem>() {
        ctx.push(MathItem::Linebreak);
    } else if let Some(elem) = elem.to_packed::<FracElem>() {
        resolve_frac(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<AccentElem>() {
        resolve_accent(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<LimitsElem>() {
        resolve_limits(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<StretchElem>() {
        resolve_stretch(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<RootElem>() {
        resolve_root(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<BoxElem>() {
        ctx.push(BoxItem::create(elem, styles));
    } else if let Some(elem) = elem.to_packed::<MatElem>() {
        resolve_mat(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<MidElem>() {
        resolve_mid(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<UnderlineElem>() {
        resolve_underline(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<CasesElem>() {
        resolve_cases(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<UnderbraceElem>() {
        resolve_underbrace(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<ScriptsElem>() {
        resolve_scripts(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<CancelElem>() {
        resolve_cancel(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<UnderbracketElem>() {
        resolve_underbracket(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<VecElem>() {
        resolve_vec(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<OverbraceElem>() {
        resolve_overbrace(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<BinomElem>() {
        resolve_binom(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<OverbracketElem>() {
        resolve_overbracket(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<UnderparenElem>() {
        resolve_underparen(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<OverparenElem>() {
        resolve_overparen(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<UndershellElem>() {
        resolve_undershell(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<OvershellElem>() {
        resolve_overshell(elem, ctx, styles)?;
    } else {
        ctx.push(ExternalItem::create(elem, styles));
    }
    Ok(())
}

fn resolve_h(
    elem: &Packed<HElem>,
    ctx: &mut MathResolver,
    styles: StyleChain,
) -> SourceResult<()> {
    if let Spacing::Rel(rel) = elem.amount
        && rel.rel.is_zero()
    {
        ctx.push(MathItem::Spacing(rel.abs.resolve(styles), elem.weak.get(styles)));
    }
    Ok(())
}

fn resolve_text<'a, 'v, 'e>(
    elem: &'a Packed<TextElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let variant = styles.get(EquationElem::variant);
    let bold = styles.get(EquationElem::bold);
    // Disable auto-italic.
    let italic = styles.get(EquationElem::italic).or(Some(false));

    let num = elem.text.chars().all(|c| c.is_ascii_digit() || c == '.');
    let multiline = elem.text.contains(is_newline);

    let styled_text: EcoString = elem
        .text
        .chars()
        .flat_map(|c| to_style(c, MathStyle::select(c, variant, bold, italic)))
        .collect();

    ctx.push(TextItem::create(
        styled_text,
        !multiline && !num,
        styles,
        elem.span(),
        &ctx.arenas.bump,
    ));
    Ok(())
}

fn resolve_symbol<'a, 'v, 'e>(
    elem: &'a Packed<SymbolElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let variant = styles.get(EquationElem::variant);
    let bold = styles.get(EquationElem::bold);
    let italic = styles.get(EquationElem::italic);
    for cluster in elem.text.graphemes(true) {
        if cluster.chars().all(is_default_ignorable) {
            continue;
        }

        let mut enable_dtls = false;
        let mut chars = cluster.chars();
        let text: EcoString = if let Some(mut c) = chars.next()
            && chars.next().is_none()
        {
            if let Some(d) = try_dotless(c) {
                enable_dtls = true;
                c = d;
            }
            to_style(c, MathStyle::select(c, variant, bold, italic)).collect()
        } else {
            cluster.into()
        };

        let item =
            GlyphItem::create(text, enable_dtls, styles, elem.span(), &ctx.arenas.bump);

        if item.class() == MathClass::Large && item.size().unwrap() == MathSize::Display {
            let target = Rel::new(Ratio::one(), Abs::zero());
            let stretch = Stretch::new().with_y(StretchInfo::new(target, Em::zero()));
            item.set_stretch(stretch);
        }

        ctx.push(item);
    }
    Ok(())
}

/// The non-dotless version of a dotless character that can be used with the
/// `dtls` OpenType feature.
fn try_dotless(c: char) -> Option<char> {
    match c {
        'ı' => Some('i'),
        'ȷ' => Some('j'),
        _ => None,
    }
}

fn resolve_accent<'a, 'v, 'e>(
    elem: &'a Packed<AccentElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let accent = elem.accent;
    let top_accent = !accent.is_bottom();

    let mut new_styles = Styles::new();
    new_styles.apply(style_cramped().into());
    // Try to replace the base glyph with its dotless variant.
    if top_accent && elem.dotless.get(styles) {
        new_styles.apply(style_dtls().into());
    }

    let base_styles = ctx.chain_styles(styles, new_styles);
    let base = ctx.resolve_into_item(&elem.base, base_styles)?;

    let accent = ctx.store(SymbolElem::packed(accent.0).spanned(elem.span()));
    let mut accent = ctx.resolve_into_item(accent, styles)?;
    accent.set_class(MathClass::Diacritic);

    let width = elem.size.resolve(styles);
    accent.set_stretch(Stretch::new().with_x(StretchInfo::new(width, ACCENT_SHORT_FALL)));

    ctx.push(AccentItem::create(
        base,
        accent,
        !top_accent,
        false,
        styles,
        &ctx.arenas.bump,
    ));
    Ok(())
}

fn resolve_attach<'a, 'v, 'e>(
    elem: &'a Packed<AttachElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let merged = elem.merge_base();
    let elem = merged.map_or(elem, |x| ctx.arenas.bump.alloc(x));
    let bumped_styles = ctx.arenas.bump.alloc(styles);

    let base = ctx.resolve_into_item(&elem.base, styles)?;
    let sup_style = ctx.store_styles(style_for_superscript(styles));
    let sup_style_chain = bumped_styles.chain(sup_style);
    let tl = elem.tl.get_cloned(sup_style_chain);
    let tr = elem.tr.get_cloned(sup_style_chain);
    let primed = tr.as_ref().is_some_and(|content| content.is::<PrimesElem>());
    let t = elem.t.get_cloned(sup_style_chain);

    let sub_style = ctx.store_styles(style_for_subscript(styles));
    let sub_style_chain = bumped_styles.chain(sub_style);
    let bl = elem.bl.get_cloned(sub_style_chain);
    let br = elem.br.get_cloned(sub_style_chain);
    let b = elem.b.get_cloned(sub_style_chain);

    let limits = base.limits().active(styles);
    let (t, tr) = match (t, tr) {
        (Some(t), Some(tr)) if primed && !limits => (None, Some(tr + t)),
        (Some(t), None) if !limits => (None, Some(t)),
        (t, tr) => (t, tr),
    };
    let (b, br) = if limits || br.is_some() { (b, br) } else { (None, b) };

    macro_rules! layout {
        ($content:ident, $style_chain:ident) => {
            $content
                .map(|elem| ctx.resolve_into_item(ctx.store(elem), $style_chain))
                .transpose()
        };
    }

    let top = layout!(t, sup_style_chain)?;
    let bottom = layout!(b, sub_style_chain)?;
    let top_left = layout!(tl, sup_style_chain)?;
    let bottom_left = layout!(bl, sub_style_chain)?;
    let top_right = layout!(tr, sup_style_chain)?;
    let bottom_right = layout!(br, sub_style_chain)?;

    ctx.push(ScriptsItem::create(
        base,
        top,
        bottom,
        top_left,
        bottom_left,
        top_right,
        bottom_right,
        styles,
        &ctx.arenas.bump,
    ));

    Ok(())
}

fn resolve_primes<'a, 'v, 'e>(
    elem: &'a Packed<PrimesElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    match elem.count {
        count @ 1..=4 => {
            let c = match count {
                1 => '′',
                2 => '″',
                3 => '‴',
                4 => '⁗',
                _ => unreachable!(),
            };
            let f = ctx.resolve_into_item(
                ctx.store(SymbolElem::packed(c).spanned(elem.span())),
                styles,
            )?;
            ctx.push(f);
        }
        count => {
            // Custom amount of primes
            let prime = ctx.resolve_into_item(
                ctx.store(SymbolElem::packed('′').spanned(elem.span())),
                styles,
            )?;
            ctx.push(PrimesItem::create(prime, count, styles, &ctx.arenas.bump));
        }
    }
    Ok(())
}

fn resolve_scripts<'a, 'v, 'e>(
    elem: &'a Packed<ScriptsElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let mut item = ctx.resolve_into_item(&elem.body, styles)?;
    item.set_limits(Limits::Never);
    ctx.push(item);
    Ok(())
}

fn resolve_limits<'a, 'v, 'e>(
    elem: &'a Packed<LimitsElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let mut item = ctx.resolve_into_item(&elem.body, styles)?;
    let limits = if elem.inline.get(styles) { Limits::Always } else { Limits::Display };
    item.set_limits(limits);
    ctx.push(item);
    Ok(())
}

fn resolve_stretch<'a, 'v, 'e>(
    elem: &'a Packed<StretchElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let item = ctx.resolve_into_item(&elem.body, styles)?;
    let size = elem.size.resolve(styles);
    item.update_stretch(StretchInfo::new(size, Em::zero()));
    ctx.push(item);
    Ok(())
}

fn resolve_cancel<'a, 'v, 'e>(
    elem: &'a Packed<CancelElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let body = ctx.resolve_into_item(&elem.body, styles)?;

    let length = elem.length.resolve(styles);
    let stroke = elem.stroke.resolve(styles).unwrap_or(FixedStroke {
        paint: styles.get_ref(TextElem::fill).as_decoration(),
        ..Default::default()
    });

    let invert = elem.inverted.get(styles);
    let cross = elem.cross.get(styles);
    let angle = elem.angle.get_ref(styles);
    let invert_first_line = !cross && invert;

    ctx.push(CancelItem::create(
        body,
        length,
        stroke,
        cross,
        invert_first_line,
        angle.clone(),
        styles,
        elem.span(),
        &ctx.arenas.bump,
    ));
    Ok(())
}

fn resolve_frac<'a, 'v, 'e>(
    elem: &'a Packed<FracElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    match elem.style.get(styles) {
        FracStyle::Skewed => {
            resolve_skewed_frac(ctx, styles, &elem.num, &elem.denom, elem.span())
        }
        FracStyle::Horizontal => resolve_horizontal_frac(
            ctx,
            styles,
            &elem.num,
            &elem.denom,
            elem.span(),
            elem.num_deparenthesized.get(styles),
            elem.denom_deparenthesized.get(styles),
        ),
        FracStyle::Vertical => resolve_vertical_frac_like(
            ctx,
            styles,
            &elem.num,
            std::slice::from_ref(&elem.denom),
            false,
            elem.span(),
        ),
    }
}

fn resolve_binom<'a, 'v, 'e>(
    elem: &'a Packed<BinomElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    resolve_vertical_frac_like(ctx, styles, &elem.upper, &elem.lower, true, elem.span())
}

/// Resolve a vertical fraction or binomial.
fn resolve_vertical_frac_like<'a, 'v, 'e>(
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
    num: &'a Content,
    denom: &[Content],
    binom: bool,
    span: Span,
) -> SourceResult<()> {
    let num_style = ctx.store_styles(style_for_numerator(styles));
    let denom_style = ctx.store_styles(style_for_denominator(styles));
    let bumped_styles = ctx.arenas.bump.alloc(styles);

    let numerator = ctx.resolve_into_item(num, bumped_styles.chain(num_style))?;

    let denominator = ctx.resolve_into_item(
        ctx.store(Content::sequence(
            // Add a comma between each element.
            denom
                .iter()
                .flat_map(|a| [SymbolElem::packed(',').spanned(span), a.clone()])
                .skip(1),
        )),
        bumped_styles.chain(denom_style),
    )?;

    let frac = FractionItem::create(
        numerator,
        denominator,
        !binom,
        FRAC_AROUND,
        styles,
        span,
        &ctx.arenas.bump,
    );

    if binom {
        let stretch =
            Stretch::new().with_y(StretchInfo::new(Rel::one(), DELIM_SHORT_FALL));
        let open = ctx.resolve_into_item(
            ctx.store(SymbolElem::packed('(').spanned(span)),
            styles,
        )?;
        open.set_stretch(stretch);
        let close = ctx.resolve_into_item(
            ctx.store(SymbolElem::packed(')').spanned(span)),
            styles,
        )?;
        close.set_stretch(stretch);
        ctx.push(FencedItem::create(
            Some(open),
            Some(close),
            frac,
            false,
            styles,
            span,
            &ctx.arenas.bump,
        ));
    } else {
        ctx.push(frac);
    }

    Ok(())
}

// Resolve a horizontal fraction
fn resolve_horizontal_frac<'a, 'v, 'e>(
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
    num: &'a Content,
    denom: &'a Content,
    span: Span,
    num_deparen: bool,
    denom_deparen: bool,
) -> SourceResult<()> {
    let num = if num_deparen {
        ctx.store(
            LrElem::new(Content::sequence(vec![
                SymbolElem::packed('('),
                num.clone(),
                SymbolElem::packed(')'),
            ]))
            .pack(),
        )
    } else {
        num
    };
    let num = ctx.resolve_into_item(num, styles)?;
    ctx.push(num);

    let slash =
        ctx.resolve_into_item(ctx.store(SymbolElem::packed('/').spanned(span)), styles)?;
    ctx.push(slash);

    let denom = if denom_deparen {
        ctx.store(
            LrElem::new(Content::sequence(vec![
                SymbolElem::packed('('),
                denom.clone(),
                SymbolElem::packed(')'),
            ]))
            .pack(),
        )
    } else {
        denom
    };
    let denom = ctx.resolve_into_item(denom, styles)?;
    ctx.push(denom);

    Ok(())
}

/// Resolve a skewed fraction.
fn resolve_skewed_frac<'a, 'v, 'e>(
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
    num: &'a Content,
    denom: &'a Content,
    span: Span,
) -> SourceResult<()> {
    let num_style = ctx.store_styles(style_for_numerator(styles));
    let denom_style = ctx.store_styles(style_for_denominator(styles));
    let bumped_styles = ctx.arenas.bump.alloc(styles);

    let numerator = ctx.resolve_into_item(num, bumped_styles.chain(num_style))?;
    let denominator = ctx.resolve_into_item(denom, bumped_styles.chain(denom_style))?;

    let slash = ctx.resolve_into_item(
        ctx.store(SymbolElem::packed('\u{2044}').spanned(span)),
        styles,
    )?;
    slash.set_stretch(
        Stretch::new().with_y(StretchInfo::new(Rel::one(), DELIM_SHORT_FALL)),
    );

    ctx.push(SkewedFractionItem::create(
        numerator,
        denominator,
        slash,
        styles,
        &ctx.arenas.bump,
    ));

    Ok(())
}

fn resolve_lr<'a, 'v, 'e>(
    elem: &'a Packed<LrElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    // Extract from an EquationElem.
    let mut body = &elem.body;
    if let Some(equation) = body.to_packed::<EquationElem>() {
        body = &equation.body;
    }

    // Extract implicit LrElem.
    if let Some(lr) = body.to_packed::<LrElem>()
        && lr.size.get(styles).is_one()
    {
        body = &lr.body;
    }

    let start = ctx.resolve_into_items(body, styles)?;

    // Ignore leading and trailing ignorant items.
    let (start_idx, end_idx) =
        ctx.items[start..].split_prefix_suffix(|f| f.is_ignorant());
    let inner_range = (start + start_idx)..(start + end_idx);
    let inner_items = &mut ctx.items[inner_range.clone()];

    let height = elem.size.resolve(styles);
    let stretch = Stretch::new().with_y(StretchInfo::new(height, DELIM_SHORT_FALL));

    let scale_if_delimiter = |item: &mut MathItem, apply: Option<MathClass>| {
        if matches!(
            item.class(),
            MathClass::Opening | MathClass::Closing | MathClass::Fence
        ) {
            item.set_stretch(stretch);
            if let Some(class) = apply {
                item.set_class(class);
            }
        }
    };

    // Scale up items at both ends.
    match inner_items {
        [one] => {
            if let MathItem::Component(MathComponent {
                kind: MathKind::Fenced(fenced),
                ..
            }) = one
            {
                if let Some(open) = &fenced.open {
                    open.set_stretch(stretch);
                }
                if let Some(close) = &fenced.close {
                    close.set_stretch(stretch);
                }
                for item in fenced.body.as_slice() {
                    if item.mid_stretched() == Some(true) {
                        item.set_stretch(stretch);
                    }
                }
            } else {
                one.set_y_stretch(StretchInfo::new(height.abs.into(), DELIM_SHORT_FALL));
            }
            return Ok(());
        }
        [first, .., last] => {
            scale_if_delimiter(first, Some(MathClass::Opening));
            scale_if_delimiter(last, Some(MathClass::Closing));
        }
        [] => {}
    }

    // Handle MathItem::Glyph items that should be scaled up.
    for item in inner_items.iter_mut() {
        if item.mid_stretched() == Some(false) {
            item.set_mid_stretched(Some(true));
            item.set_stretch(stretch);
        }
    }

    let mut inner_items: Vec<_> = ctx.items.drain(inner_range).collect();

    // Remove weak Spacing immediately after the opening or immediately
    // before the closing.
    let mut index = 0;
    let len = inner_items.len();
    let opening_exists =
        inner_items.first().is_some_and(|f| f.class() == MathClass::Opening);
    let closing_exists =
        inner_items.last().is_some_and(|f| f.class() == MathClass::Closing);
    inner_items.retain(|item| {
        let discard = (index == 1 && opening_exists
            || index + 2 == len && closing_exists)
            && matches!(item, MathItem::Spacing(_, true));
        index += 1;
        !discard
    });

    let open = opening_exists.then(|| inner_items.remove(0));
    let close = closing_exists.then(|| inner_items.pop().unwrap());
    let item = FencedItem::create(
        open,
        close,
        GroupItem::create(inner_items, closing_exists, styles, ctx.arenas),
        true,
        styles,
        elem.span(),
        &ctx.arenas.bump,
    );

    ctx.items.insert(start + start_idx, item);
    Ok(())
}

fn resolve_mid<'a, 'v, 'e>(
    elem: &'a Packed<MidElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let start = ctx.resolve_into_items(&elem.body, styles)?;
    for item in &mut ctx.items[start..] {
        item.set_mid_stretched(Some(false));
        item.set_class(MathClass::Relation);
    }
    Ok(())
}

fn resolve_vec<'a, 'v, 'e>(
    elem: &'a Packed<VecElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let span = elem.span();

    let rows: Vec<Vec<&Content>> =
        elem.children.iter().map(|child| vec![child]).collect();
    let cells = resolve_cells(
        ctx,
        styles,
        rows,
        span,
        elem.align.resolve(styles),
        LeftRightAlternator::Right,
        None,
        Axes::with_y(elem.gap.resolve(styles)),
        "elements",
    )?;

    let delim = elem.delim.get(styles);
    resolve_delimiters(ctx, styles, cells, delim.open(), delim.close(), span)
}

fn resolve_mat<'a, 'v, 'e>(
    elem: &'a Packed<MatElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let span = elem.span();

    let rows: Vec<Vec<&Content>> =
        elem.rows.iter().map(|row| row.iter().collect()).collect();
    let nrows = rows.len();
    let ncols = rows.first().map_or(0, |row| row.len());

    let augment = elem.augment.resolve(styles);
    if let Some(aug) = &augment {
        for &offset in &aug.hline.0 {
            if offset > nrows as isize || offset.unsigned_abs() > nrows {
                bail!(
                    span,
                    "cannot draw a horizontal line at offset {offset} \
                     in a matrix with {nrows} rows",
                );
            }
        }

        for &offset in &aug.vline.0 {
            if offset > ncols as isize || offset.unsigned_abs() > ncols {
                bail!(
                    span,
                    "cannot draw a vertical line at offset {offset} \
                     in a matrix with {ncols} columns",
                );
            }
        }
    }

    let cells = resolve_cells(
        ctx,
        styles,
        rows,
        span,
        elem.align.resolve(styles),
        LeftRightAlternator::Right,
        augment,
        Axes::new(elem.column_gap.resolve(styles), elem.row_gap.resolve(styles)),
        "cells",
    )?;

    let delim = elem.delim.get(styles);
    resolve_delimiters(ctx, styles, cells, delim.open(), delim.close(), span)
}

fn resolve_cases<'a, 'v, 'e>(
    elem: &'a Packed<CasesElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let span = elem.span();

    let rows: Vec<Vec<&Content>> =
        elem.children.iter().map(|child| vec![child]).collect();
    let cells = resolve_cells(
        ctx,
        styles,
        rows,
        span,
        FixedAlignment::Start,
        LeftRightAlternator::None,
        None,
        Axes::with_y(elem.gap.resolve(styles)),
        "branches",
    )?;

    let delim = elem.delim.get(styles);
    let (open, close) = if elem.reverse.get(styles) {
        (None, delim.close())
    } else {
        (delim.open(), None)
    };
    resolve_delimiters(ctx, styles, cells, open, close, span)
}

/// Layout the inner contents of a matrix, vector, or cases.
#[allow(clippy::too_many_arguments)]
fn resolve_cells<'a, 'v, 'e>(
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
    rows: Vec<Vec<&'a Content>>,
    span: Span,
    align: FixedAlignment,
    alternator: LeftRightAlternator,
    augment: Option<Augment<Abs>>,
    gap: Axes<Rel<Abs>>,
    children: &str,
) -> SourceResult<MathItem<'a>> {
    let cell_styles = ctx.chain_styles(styles, style_for_denominator(styles));
    let cells = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| {
                    let cell_span = cell.span();
                    let cell = ctx.resolve_into_item(cell, cell_styles)?;

                    // We ignore linebreaks in the cells as we can't differentiate
                    // alignment points for the whole body from ones for a specific
                    // cell, and multiline cells don't quite make sense at the moment.
                    if cell.is_multiline() {
                        ctx.engine.sink.warn(warning!(
                           cell_span,
                           "linebreaks are ignored in {}", children;
                           hint: "use commas instead to separate each line";
                        ));
                    }
                    Ok(cell)
                })
                .collect::<SourceResult<_>>()
        })
        .collect::<SourceResult<_>>();

    Ok(TableItem::create(
        cells?,
        gap,
        augment,
        align,
        alternator,
        styles,
        span,
        &ctx.arenas.bump,
    ))
}

/// Resolve the outer wrapper around the body of a vector or matrix.
fn resolve_delimiters<'a, 'v, 'e>(
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
    cells: MathItem<'a>,
    left: Option<char>,
    right: Option<char>,
    span: Span,
) -> SourceResult<()> {
    let target = Rel::new(Ratio::new(1.1), Abs::zero());
    let stretch = Stretch::new().with_y(StretchInfo::new(target, DELIM_SHORT_FALL));
    let open = left
        .map(|c| {
            ctx.resolve_into_item(ctx.store(SymbolElem::packed(c).spanned(span)), styles)
        })
        .transpose()?
        .inspect(|x| x.set_stretch(stretch));
    let close = right
        .map(|c| {
            ctx.resolve_into_item(ctx.store(SymbolElem::packed(c).spanned(span)), styles)
        })
        .transpose()?
        .inspect(|x| x.set_stretch(stretch));

    ctx.push(FencedItem::create(
        open,
        close,
        cells,
        false,
        styles,
        span,
        &ctx.arenas.bump,
    ));
    Ok(())
}

fn resolve_class<'a, 'v, 'e>(
    elem: &'a Packed<ClassElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let styles =
        ctx.chain_styles(styles, EquationElem::class.set(Some(elem.class)).wrap());
    let mut item = ctx.resolve_into_item(&elem.body, styles)?;
    item.set_class(elem.class);
    item.set_limits(Limits::for_class(elem.class));
    ctx.push(item);
    Ok(())
}

fn resolve_op<'a, 'v, 'e>(
    elem: &'a Packed<OpElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let mut item = ctx.resolve_into_item(&elem.text, styles)?;
    item.set_class(MathClass::Large);
    item.set_limits(if elem.limits.get(styles) {
        Limits::Display
    } else {
        Limits::Never
    });
    ctx.push(item);
    Ok(())
}

fn resolve_root<'a, 'v, 'e>(
    elem: &'a Packed<RootElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let bumped_styles = ctx.arenas.bump.alloc(styles);
    let radicand = {
        let cramped = ctx.store_styles(style_cramped());
        ctx.resolve_into_item(&elem.radicand, bumped_styles.chain(cramped))?
    };
    let index = {
        let sscript =
            ctx.store_styles(EquationElem::size.set(MathSize::ScriptScript).wrap());
        elem.index
            .get_ref(styles)
            .as_ref()
            .map(|elem| ctx.resolve_into_item(elem, bumped_styles.chain(sscript)))
            .transpose()?
    };
    let sqrt = ctx.resolve_into_item(
        ctx.store(SymbolElem::packed('√').spanned(elem.span())),
        styles,
    )?;
    sqrt.set_stretch(Stretch::new().with_y(StretchInfo::new(Rel::one(), Em::zero())));
    ctx.push(RadicalItem::create(
        radicand,
        index,
        sqrt,
        styles,
        elem.span(),
        &ctx.arenas.bump,
    ));
    Ok(())
}

fn resolve_underline<'a, 'v, 'e>(
    elem: &'a Packed<UnderlineElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let base = ctx.resolve_into_item(&elem.body, styles)?;
    ctx.push(LineItem::create(base, true, styles, elem.span(), &ctx.arenas.bump));
    Ok(())
}

fn resolve_overline<'a, 'v, 'e>(
    elem: &'a Packed<OverlineElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let cramped_styles = ctx.chain_styles(styles, style_cramped());
    let base = ctx.resolve_into_item(&elem.body, cramped_styles)?;
    ctx.push(LineItem::create(base, false, styles, elem.span(), &ctx.arenas.bump));
    Ok(())
}

fn resolve_underbrace<'a, 'v, 'e>(
    elem: &'a Packed<UnderbraceElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    resolve_underoverspreader(
        ctx,
        styles,
        &elem.body,
        elem.annotation.get_ref(styles),
        '⏟',
        Position::Under,
        elem.span(),
    )
}

fn resolve_overbrace<'a, 'v, 'e>(
    elem: &'a Packed<OverbraceElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    resolve_underoverspreader(
        ctx,
        styles,
        &elem.body,
        elem.annotation.get_ref(styles),
        '⏞',
        Position::Over,
        elem.span(),
    )
}

fn resolve_underbracket<'a, 'v, 'e>(
    elem: &'a Packed<UnderbracketElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    resolve_underoverspreader(
        ctx,
        styles,
        &elem.body,
        elem.annotation.get_ref(styles),
        '⎵',
        Position::Under,
        elem.span(),
    )
}

fn resolve_overbracket<'a, 'v, 'e>(
    elem: &'a Packed<OverbracketElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    resolve_underoverspreader(
        ctx,
        styles,
        &elem.body,
        elem.annotation.get_ref(styles),
        '⎴',
        Position::Over,
        elem.span(),
    )
}

fn resolve_underparen<'a, 'v, 'e>(
    elem: &'a Packed<UnderparenElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    resolve_underoverspreader(
        ctx,
        styles,
        &elem.body,
        elem.annotation.get_ref(styles),
        '⏝',
        Position::Under,
        elem.span(),
    )
}

fn resolve_overparen<'a, 'v, 'e>(
    elem: &'a Packed<OverparenElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    resolve_underoverspreader(
        ctx,
        styles,
        &elem.body,
        elem.annotation.get_ref(styles),
        '⏜',
        Position::Over,
        elem.span(),
    )
}

fn resolve_undershell<'a, 'v, 'e>(
    elem: &'a Packed<UndershellElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    resolve_underoverspreader(
        ctx,
        styles,
        &elem.body,
        elem.annotation.get_ref(styles),
        '⏡',
        Position::Under,
        elem.span(),
    )
}

fn resolve_overshell<'a, 'v, 'e>(
    elem: &'a Packed<OvershellElem>,
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    resolve_underoverspreader(
        ctx,
        styles,
        &elem.body,
        elem.annotation.get_ref(styles),
        '⏠',
        Position::Over,
        elem.span(),
    )
}

/// Resolve an over- or underbrace-like object.
fn resolve_underoverspreader<'a, 'v, 'e>(
    ctx: &mut MathResolver<'a, 'v, 'e>,
    styles: StyleChain<'a>,
    body: &'a Content,
    annotation: &'a Option<Content>,
    c: char,
    position: Position,
    span: Span,
) -> SourceResult<()> {
    let base = ctx.resolve_into_item(body, styles)?;

    let accent = ctx.store(SymbolElem::packed(c).spanned(span));
    let mut accent = ctx.resolve_into_item(accent, styles)?;
    accent.set_class(MathClass::Diacritic);
    accent.set_stretch(Stretch::new().with_x(StretchInfo::new(Rel::one(), Em::zero())));

    let base = AccentItem::create(
        base,
        accent,
        matches!(position, Position::Under),
        true,
        styles,
        &ctx.arenas.bump,
    );

    let Some(annotation) = annotation else {
        ctx.push(base);
        return Ok(());
    };

    let base = match position {
        Position::Under => {
            let under_styles = ctx.chain_styles(styles, style_for_subscript(styles));
            let annotation = ctx.resolve_into_item(annotation, under_styles)?;
            ScriptsItem::create(
                base,
                None,
                Some(annotation),
                None,
                None,
                None,
                None,
                styles,
                &ctx.arenas.bump,
            )
        }
        Position::Over => {
            let over_styles = ctx.chain_styles(styles, style_for_superscript(styles));
            let annotation = ctx.resolve_into_item(annotation, over_styles)?;
            ScriptsItem::create(
                base,
                Some(annotation),
                None,
                None,
                None,
                None,
                None,
                styles,
                &ctx.arenas.bump,
            )
        }
    };

    ctx.push(base);
    Ok(())
}
