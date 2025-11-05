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
use crate::text::{LinebreakElem, SpaceElem, TextElem};
use crate::visualize::FixedStroke;

/// How much less high scaled delimiters can be than what they wrap.
const DELIM_SHORT_FALL: Em = Em::new(0.1);

/// How much the accent can be shorter than the base.
const ACCENT_SHORT_FALL: Em = Em::new(0.5);

#[derive(Default)]
pub struct MathResolver {
    arenas: Arenas,
}

impl MathResolver {
    /// Initialize the resolver (and internal arenas).
    pub fn new() -> Self {
        Self { arenas: Arenas::default() }
    }

    /// Resolve content into a MathRun.
    /// The returned MathRun lives as long as the MathResolver lives.
    #[typst_macros::time(name = "math ir creation")]
    pub fn resolve<'a>(
        &'a self,
        elem: &'a Packed<EquationElem>,
        engine: &mut Engine,
        locator: &mut SplitLocator<'a>,
        styles: StyleChain<'a>,
    ) -> SourceResult<MathRun<'a>> {
        let mut context = Context::new(engine, locator, &self.arenas);
        context.resolve_into_run(&elem.body, styles)
    }
}

/// The math IR builder.
struct Context<'a, 'v, 'e> {
    // External.
    engine: &'v mut Engine<'e>,
    locator: &'v mut SplitLocator<'a>,
    arenas: &'a Arenas,
    // Mutable.
    items: Vec<MathItem<'a>>,
}

impl<'a, 'v, 'e> Context<'a, 'v, 'e> {
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
    fn chain_styles(&self, base: StyleChain<'a>, new: &'a Styles) -> StyleChain<'a> {
        self.arenas.bump.alloc(base).chain(new)
    }

    /// Lifetime-extends some content.
    fn store(&self, content: Content) -> &'a Content {
        self.arenas.content.alloc(content)
    }

    /// Push a item.
    fn push(&mut self, item: impl Into<MathItem<'a>>) {
        self.items.push(item.into());
    }

    /// Push multiple items.
    fn extend(&mut self, items: impl IntoIterator<Item = MathItem<'a>>) {
        self.items.extend(items);
    }

    /// Resolve the given element and return the resulting [`MathItem`]s.
    fn resolve_into_items(
        &mut self,
        elem: &'a Content,
        styles: StyleChain<'a>,
    ) -> SourceResult<Vec<MathItem<'a>>> {
        let start = self.items.len();
        self.resolve_into_self(elem, styles)?;
        Ok(self.items.drain(start..).collect())
    }

    /// Resolve the given element and return the result as a [`MathRun`].
    fn resolve_into_run(
        &mut self,
        elem: &'a Content,
        styles: StyleChain<'a>,
    ) -> SourceResult<MathRun<'a>> {
        let start = self.items.len();
        self.resolve_into_self(elem, styles)?;
        Ok(MathRun::new(self.items.drain(start..), styles))
    }

    fn resolve_into_item(
        &mut self,
        elem: &'a Content,
        styles: StyleChain<'a>,
    ) -> SourceResult<MathItem<'a>> {
        let mut items = self.resolve_into_run(elem, styles)?;
        Ok(if items.items.len() == 1 {
            items.items.pop().unwrap()
        } else {
            GroupItem::create(items)
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
    ctx: &mut Context<'a, 'v, 'e>,
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
        resolve_box(elem, ctx, styles)?;
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
    ctx: &mut Context,
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
    ctx: &mut Context<'a, 'v, 'e>,
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

    ctx.push(TextItem::create(styled_text, !multiline && !num, styles, elem.span()));
    Ok(())
}

fn resolve_symbol<'a, 'v, 'e>(
    elem: &'a Packed<SymbolElem>,
    ctx: &mut Context<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let variant = styles.get(EquationElem::variant);
    let bold = styles.get(EquationElem::bold);
    let italic = styles.get(EquationElem::italic);
    let dtls = ctx.store_styles(style_dtls());
    let dtls_styles = ctx.chain_styles(styles, dtls);
    for cluster in elem.text.graphemes(true) {
        if cluster == "\u{200b}" {
            continue;
        }

        let mut enable_dtls = false;
        let text: EcoString = cluster
            .chars()
            .flat_map(|mut c| {
                if let Some(d) = try_dotless(c) {
                    enable_dtls = true;
                    c = d;
                }
                to_style(c, MathStyle::select(c, variant, bold, italic))
            })
            .collect();
        let styles = if enable_dtls { dtls_styles } else { styles };
        ctx.push(GlyphItem::create(text, styles, elem.span()));
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

fn resolve_box<'a, 'v, 'e>(
    elem: &'a Packed<BoxElem>,
    ctx: &mut Context<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    ctx.push(BoxItem::create(elem, styles));
    Ok(())
}

fn resolve_accent<'a, 'v, 'e>(
    elem: &'a Packed<AccentElem>,
    ctx: &mut Context<'a, 'v, 'e>,
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
    let new_styles = ctx.store_styles(new_styles);

    let base_styles = ctx.arenas.bump.alloc(styles).chain(new_styles);
    let base = ctx.resolve_into_run(&elem.base, base_styles)?;

    let accent = ctx.store(SymbolElem::packed(accent.0).spanned(elem.span()));
    let mut accent = ctx.resolve_into_item(accent, styles)?;
    accent.set_class(MathClass::Diacritic);
    let width = elem.size.resolve(styles);
    // let mut iter = accent.iter_mut();
    // if let Some(item) = iter.next()
    //     && iter.next().is_none()
    // {
    //     // TODO: previous behaviour only applied class if item was a glyph.
    //     item.set_class(MathClass::Diacritic);
    //     // item.set_stretch(Some((width, Some(Axis::X))));
    // }

    ctx.push(AccentItem::create(
        base,
        MathRun::new(vec![accent], styles),
        !top_accent,
        width,
        ACCENT_SHORT_FALL,
        false,
        styles,
        &ctx.arenas.bump,
    ));
    Ok(())
}

fn resolve_attach<'a, 'v, 'e>(
    elem: &'a Packed<AttachElem>,
    ctx: &mut Context<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let merged = elem.merge_base();
    let elem = ctx.arenas.bump.alloc(merged.unwrap_or(elem.clone()));
    let stretch = elem.stretch_size(styles);

    let bumped_styles = ctx.arenas.bump.alloc(styles);

    let base = ctx.resolve_into_run(&elem.base, styles)?;
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
                .map(|elem| ctx.resolve_into_run(ctx.store(elem), $style_chain))
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
        stretch,
        styles,
        &ctx.arenas.bump,
    ));

    Ok(())
}

fn resolve_primes<'a, 'v, 'e>(
    elem: &'a Packed<PrimesElem>,
    ctx: &mut Context<'a, 'v, 'e>,
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
            let prime = ctx.resolve_into_run(
                ctx.store(SymbolElem::packed('′').spanned(elem.span())),
                styles,
            )?;
            ctx.push(PrimesItem::create(prime, count, styles));
        }
    }
    Ok(())
}

fn resolve_scripts<'a, 'v, 'e>(
    elem: &'a Packed<ScriptsElem>,
    ctx: &mut Context<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let mut item = ctx.resolve_into_item(&elem.body, styles)?;
    item.set_limits(Limits::Never);
    ctx.push(item);
    Ok(())
}

fn resolve_limits<'a, 'v, 'e>(
    elem: &'a Packed<LimitsElem>,
    ctx: &mut Context<'a, 'v, 'e>,
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
    ctx: &mut Context<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let mut item = ctx.resolve_into_item(&elem.body, styles)?;
    let size = elem.size.resolve(styles);
    let size = if let Some((stretch, _)) = item.get_stretch() {
        Rel::new(stretch.rel * size.rel, size.rel.of(stretch.abs) + size.abs)
    } else {
        size
    };
    item.set_stretch(Some((size, None)));
    ctx.push(item);
    Ok(())
}

fn resolve_cancel<'a, 'v, 'e>(
    elem: &'a Packed<CancelElem>,
    ctx: &mut Context<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let body = ctx.resolve_into_run(&elem.body, styles)?;

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
    ctx: &mut Context<'a, 'v, 'e>,
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
    ctx: &mut Context<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    resolve_vertical_frac_like(ctx, styles, &elem.upper, &elem.lower, true, elem.span())
}

/// Resolve a vertical fraction or binomial.
fn resolve_vertical_frac_like<'a, 'v, 'e>(
    ctx: &mut Context<'a, 'v, 'e>,
    styles: StyleChain<'a>,
    num: &'a Content,
    denom: &[Content],
    binom: bool,
    span: Span,
) -> SourceResult<()> {
    let num_style = ctx.store_styles(style_for_numerator(styles));
    let denom_style = ctx.store_styles(style_for_denominator(styles));
    let bumped_styles = ctx.arenas.bump.alloc(styles);

    let numerator = ctx.resolve_into_run(num, bumped_styles.chain(num_style))?;

    let denominator = ctx.resolve_into_run(
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
        styles,
        span,
        &ctx.arenas.bump,
    );

    if binom {
        let open = ctx
            .resolve_into_run(ctx.store(SymbolElem::packed('(').spanned(span)), styles)?;
        let close = ctx
            .resolve_into_run(ctx.store(SymbolElem::packed(')').spanned(span)), styles)?;
        ctx.push(FencedItem::create(
            Some(open),
            Some(close),
            MathRun::new(vec![frac], styles),
            false,
            DELIM_SHORT_FALL,
            Rel::one(),
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
    ctx: &mut Context<'a, 'v, 'e>,
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
    ctx: &mut Context<'a, 'v, 'e>,
    styles: StyleChain<'a>,
    num: &'a Content,
    denom: &'a Content,
    span: Span,
) -> SourceResult<()> {
    let num_style = ctx.store_styles(style_for_numerator(styles));
    let denom_style = ctx.store_styles(style_for_denominator(styles));
    let bumped_styles = ctx.arenas.bump.alloc(styles);

    let numerator = ctx.resolve_into_run(num, bumped_styles.chain(num_style))?;
    let denominator = ctx.resolve_into_run(denom, bumped_styles.chain(denom_style))?;

    let slash = ctx.resolve_into_run(
        ctx.store(SymbolElem::packed('\u{2044}').spanned(span)),
        styles,
    )?;

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
    ctx: &mut Context<'a, 'v, 'e>,
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

    let mut items = ctx.resolve_into_items(body, styles)?;

    // Ignore leading and trailing ignorant items.
    let (start_idx, end_idx) = items.split_prefix_suffix(|f| f.is_ignorant());
    let inner_items = &mut items[start_idx..end_idx];

    let height = elem.size.resolve(styles);

    let scale_if_delimiter = |item: &mut MathItem, apply: Option<MathClass>| {
        if matches!(
            item.class(),
            MathClass::Opening | MathClass::Closing | MathClass::Fence
        ) {
            // item.set_stretch(Some((height, Some(Axis::Y))));

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
                let mut new_fenced = (**fenced).clone();
                new_fenced.target = Rel::new(
                    new_fenced.target.rel * height.rel,
                    height.rel.of(new_fenced.target.abs) + height.abs,
                );
                *fenced = ctx.arenas.bump.alloc(new_fenced);
            } else {
                let size = if let Some((stretch, _)) = one.get_stretch() {
                    Rel::new(
                        stretch.rel * height.rel,
                        height.rel.of(stretch.abs) + height.abs,
                    )
                } else {
                    height
                };
                one.set_stretch(Some((size, None)));
            }
            ctx.extend(items);
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
        if item.get_mid_stretched() == Some(false) {
            item.set_mid_stretched(Some(true));
            // item.set_stretch(Some((height, Some(Axis::Y))));
        }
    }

    let mut inner_items = items.drain(start_idx..end_idx).collect::<Vec<_>>();

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

    let open = opening_exists.then(|| MathRun::new(vec![inner_items.remove(0)], styles));
    let close =
        closing_exists.then(|| MathRun::new(vec![inner_items.pop().unwrap()], styles));
    let item = FencedItem::create(
        open,
        close,
        MathRun::create(inner_items, styles, closing_exists),
        true,
        DELIM_SHORT_FALL,
        height,
        styles,
        elem.span(),
        &ctx.arenas.bump,
    );

    items.insert(start_idx, item);
    ctx.extend(items);
    Ok(())
}

fn resolve_mid<'a, 'v, 'e>(
    elem: &'a Packed<MidElem>,
    ctx: &mut Context<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let mut items = ctx.resolve_into_items(&elem.body, styles)?;
    for item in &mut items {
        // TODO: previous behaviour only applied class if item was a glyph.
        item.set_mid_stretched(Some(false));
        item.set_class(MathClass::Relation);
    }
    ctx.extend(items);
    Ok(())
}

fn resolve_vec<'a, 'v, 'e>(
    elem: &'a Packed<VecElem>,
    ctx: &mut Context<'a, 'v, 'e>,
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
    ctx: &mut Context<'a, 'v, 'e>,
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
    ctx: &mut Context<'a, 'v, 'e>,
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
    ctx: &mut Context<'a, 'v, 'e>,
    styles: StyleChain<'a>,
    rows: Vec<Vec<&'a Content>>,
    span: Span,
    align: FixedAlignment,
    alternator: LeftRightAlternator,
    augment: Option<Augment<Abs>>,
    gap: Axes<Rel<Abs>>,
    children: &str,
) -> SourceResult<MathItem<'a>> {
    let denom_style = ctx.store_styles(style_for_denominator(styles));
    let cell_styles = ctx.chain_styles(styles, denom_style);
    let cells = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| {
                    let cell_span = cell.span();
                    let cell = ctx.resolve_into_run(cell, cell_styles)?;

                    // We ignore linebreaks in the cells as we can't differentiate
                    // alignment points for the whole body from ones for a specific
                    // cell, and multiline cells don't quite make sense at the moment.
                    if cell.is_multiline() {
                        ctx.engine.sink.warn(warning!(
                           cell_span,
                           "linebreaks are ignored in {}", children;
                           hint: "use commas instead to separate each line"
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
    ctx: &mut Context<'a, 'v, 'e>,
    styles: StyleChain<'a>,
    cells: MathItem<'a>,
    left: Option<char>,
    right: Option<char>,
    span: Span,
) -> SourceResult<()> {
    let target = Rel::new(Ratio::new(1.1), Abs::zero());
    let open = left
        .map(|c| {
            ctx.resolve_into_run(ctx.store(SymbolElem::packed(c).spanned(span)), styles)
        })
        .transpose()?;
    let close = right
        .map(|c| {
            ctx.resolve_into_run(ctx.store(SymbolElem::packed(c).spanned(span)), styles)
        })
        .transpose()?;

    ctx.push(FencedItem::create(
        open,
        close,
        MathRun::new(vec![cells], styles),
        false,
        DELIM_SHORT_FALL,
        target,
        styles,
        span,
        &ctx.arenas.bump,
    ));
    Ok(())
}

fn resolve_class<'a, 'v, 'e>(
    elem: &'a Packed<ClassElem>,
    ctx: &mut Context<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let style = ctx.store_styles(EquationElem::class.set(Some(elem.class)).wrap());
    let mut item = ctx.resolve_into_item(&elem.body, ctx.chain_styles(styles, style))?;
    item.set_class(elem.class);
    item.set_limits(Limits::for_class(elem.class));
    ctx.push(item);
    Ok(())
}

fn resolve_op<'a, 'v, 'e>(
    elem: &'a Packed<OpElem>,
    ctx: &mut Context<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    // TODO: should be wrapped to match typst-layout
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
    ctx: &mut Context<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let bumped_styles = ctx.arenas.bump.alloc(styles);
    let radicand = {
        let cramped = ctx.store_styles(style_cramped());
        ctx.resolve_into_run(&elem.radicand, bumped_styles.chain(cramped))?
    };
    let index = {
        let sscript =
            ctx.store_styles(EquationElem::size.set(MathSize::ScriptScript).wrap());
        elem.index
            .get_ref(styles)
            .as_ref()
            .map(|elem| ctx.resolve_into_run(elem, bumped_styles.chain(sscript)))
            .transpose()?
    };
    let sqrt = ctx.resolve_into_run(
        ctx.store(SymbolElem::packed('√').spanned(elem.span())),
        styles,
    )?;
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
    ctx: &mut Context<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let base = ctx.resolve_into_run(&elem.body, styles)?;
    ctx.push(LineItem::create(base, true, styles, elem.span()));
    Ok(())
}

fn resolve_overline<'a, 'v, 'e>(
    elem: &'a Packed<OverlineElem>,
    ctx: &mut Context<'a, 'v, 'e>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let cramped = ctx.store_styles(style_cramped());
    let base = ctx.resolve_into_run(&elem.body, ctx.chain_styles(styles, cramped))?;
    ctx.push(LineItem::create(base, false, styles, elem.span()));
    Ok(())
}

fn resolve_underbrace<'a, 'v, 'e>(
    elem: &'a Packed<UnderbraceElem>,
    ctx: &mut Context<'a, 'v, 'e>,
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
    ctx: &mut Context<'a, 'v, 'e>,
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
    ctx: &mut Context<'a, 'v, 'e>,
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
    ctx: &mut Context<'a, 'v, 'e>,
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
    ctx: &mut Context<'a, 'v, 'e>,
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
    ctx: &mut Context<'a, 'v, 'e>,
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
    ctx: &mut Context<'a, 'v, 'e>,
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
    ctx: &mut Context<'a, 'v, 'e>,
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
    ctx: &mut Context<'a, 'v, 'e>,
    styles: StyleChain<'a>,
    body: &'a Content,
    annotation: &'a Option<Content>,
    c: char,
    position: Position,
    span: Span,
) -> SourceResult<()> {
    let base = ctx.resolve_into_run(body, styles)?;

    let accent = ctx.store(SymbolElem::packed(c).spanned(span));
    let mut accent = ctx.resolve_into_item(accent, styles)?;
    accent.set_class(MathClass::Diacritic);

    let base = AccentItem::create(
        base,
        MathRun::new(vec![accent], styles),
        matches!(position, Position::Under),
        Rel::one(),
        Em::zero(),
        true,
        styles,
        &ctx.arenas.bump,
    );

    let Some(annotation) = annotation else {
        ctx.push(base);
        return Ok(());
    };

    let bumped_styles = ctx.arenas.bump.alloc(styles);

    let base = MathRun::new(vec![base], styles);
    let base = match position {
        Position::Under => {
            let under_style = ctx.store_styles(style_for_subscript(styles));
            let annotation =
                ctx.resolve_into_run(annotation, bumped_styles.chain(under_style))?;
            ScriptsItem::create(
                base,
                None,
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
        Position::Over => {
            let over_style = ctx.store_styles(style_for_superscript(styles));
            let annotation =
                ctx.resolve_into_run(annotation, bumped_styles.chain(over_style))?;
            ScriptsItem::create(
                base,
                Some(annotation),
                None,
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
