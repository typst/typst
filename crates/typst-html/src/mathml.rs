use codex::styling::{MathStyle, to_style};
use ecow::{EcoString, eco_format};
use typst_assets::mathml;
use typst_library::diag::{SourceResult, warning};
use typst_library::engine::Engine;
use typst_library::foundations::{
    Content, NativeElement, Packed, SequenceElem, StyleChain, SymbolElem,
};
use typst_library::introspection::{Locator, TagElem};
use typst_library::layout::{HElem, Spacing};
use typst_library::math::*;
use typst_library::routines::{Arenas, RealizationKind};
use typst_library::text::{LinebreakElem, SpaceElem, TextElem};
use typst_syntax::Span;
use typst_utils::default_math_class;
use unicode_math_class::MathClass;

use crate::{HtmlAttrs, HtmlElem, attr::mathml as attr, css, tag::mathml as tag};

fn _finish(output: Vec<Content>) -> Content {
    let mut elem = Content::sequence(output);
    if elem.is::<HtmlElem>() && elem.to_packed::<HtmlElem>().unwrap().tag == tag::mrow {
        elem.to_packed_mut::<HtmlElem>()
            .unwrap()
            .body
            .as_option_mut()
            .take()
            .flatten()
            .unwrap_or_default()
    } else {
        elem
    }
}

/// Produce MathML nodes from content.
pub fn show_equation(
    content: &Content,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    // TODO: I recall reading in the codebase that the locator is not used by
    // HTML export, so it's fine to just fabricate one. Even if we can
    // fabricate one, does the same one need to be used throughout? For now, to
    // keep the function calls a bit leaner, I've not done this.
    let mut locator = Locator::root().split();
    let arenas = Arenas::default();
    let children = (engine.routines.realize)(
        RealizationKind::Math,
        engine,
        &mut locator,
        &arenas,
        content,
        styles,
    )?;

    let mut output = Vec::new();
    for (child, styles) in children {
        handle(child, engine, styles, &mut output)?;
    }

    Ok(Content::sequence(output))
}

fn handle(
    elem: &Content,
    engine: &mut Engine,
    styles: StyleChain,
    output: &mut Vec<Content>,
) -> SourceResult<()> {
    if elem.is::<TagElem>() {
        output.push(elem.clone());
    } else if let Some(_elem) = elem.to_packed::<SpaceElem>() {
        // Ignored for now, but will need to be used to resolve vary math class.
    } else if let Some(_elem) = elem.to_packed::<LinebreakElem>() {
        // TODO, along with AlignPointElem
    } else if let Some(elem) = elem.to_packed::<HElem>() {
        output.push(show_h(elem, engine, styles)?);
    } else if let Some(elem) = elem.to_packed::<TextElem>() {
        output.push(show_text(elem, engine, styles)?);
    } else if let Some(elem) = elem.to_packed::<SymbolElem>() {
        output.push(show_symbol(elem, engine, styles)?);
    } else if let Some(elem) = elem.to_packed::<OpElem>() {
        output.push(show_op(elem, engine, styles)?);
    } else if let Some(_elem) = elem.to_packed::<StretchElem>() {
        // TODO
    } else if let Some(elem) = elem.to_packed::<LrElem>() {
        output.push(show_lr(elem, engine, styles)?);
    } else if let Some(elem) = elem.to_packed::<MidElem>() {
        output.push(show_mid(elem, engine, styles)?);
    } else if let Some(elem) = elem.to_packed::<AttachElem>() {
        output.push(show_attach(elem, engine, styles)?);
    } else if let Some(elem) = elem.to_packed::<PrimesElem>() {
        output.push(show_primes(elem, engine, styles)?);
    } else if let Some(elem) = elem.to_packed::<ScriptsElem>() {
        output.push(show_scripts(elem, engine, styles)?);
    } else if let Some(elem) = elem.to_packed::<LimitsElem>() {
        output.push(show_limits(elem, engine, styles)?);
    } else if let Some(elem) = elem.to_packed::<AccentElem>() {
        output.push(show_accent(elem, engine, styles)?);
    } else if let Some(elem) = elem.to_packed::<RootElem>() {
        output.push(show_root(elem, engine, styles)?);
    } else if let Some(elem) = elem.to_packed::<FracElem>() {
        output.push(show_frac(elem, engine, styles)?);
    } else if let Some(elem) = elem.to_packed::<BinomElem>() {
        output.push(show_binom(elem, engine, styles)?);
    } else if let Some(elem) = elem.to_packed::<UnderbraceElem>() {
        output.push(show_underbrace(elem, engine, styles)?);
    } else if let Some(elem) = elem.to_packed::<OverbraceElem>() {
        output.push(show_overbrace(elem, engine, styles)?);
    } else if let Some(elem) = elem.to_packed::<UnderbracketElem>() {
        output.push(show_underbracket(elem, engine, styles)?);
    } else if let Some(elem) = elem.to_packed::<OverbracketElem>() {
        output.push(show_overbracket(elem, engine, styles)?);
    } else if let Some(elem) = elem.to_packed::<UnderparenElem>() {
        output.push(show_underparen(elem, engine, styles)?);
    } else if let Some(elem) = elem.to_packed::<OverparenElem>() {
        output.push(show_overparen(elem, engine, styles)?);
    } else if let Some(elem) = elem.to_packed::<UndershellElem>() {
        output.push(show_undershell(elem, engine, styles)?);
    } else if let Some(elem) = elem.to_packed::<OvershellElem>() {
        output.push(show_overshell(elem, engine, styles)?);
    } else if let Some(elem) = elem.to_packed::<MatElem>() {
        output.push(show_mat(elem, engine, styles)?);
    } else if let Some(elem) = elem.to_packed::<VecElem>() {
        output.push(show_vec(elem, engine, styles)?);
    } else if let Some(elem) = elem.to_packed::<CasesElem>() {
        output.push(show_cases(elem, engine, styles)?);
    } else if elem.can::<dyn Mathy>() {
        // CancelElem, UnderlineElem, OverlineElem, AlignPointElem, ClassElem
        engine.sink.warn(warning!(
            elem.span(),
            "{} was ignored during MathML export",
            elem.elem().name()
        ));
    } else {
        // Arbitrary content is not allowed, as per the HTML spec.
        // Only MathML token elements (mi, mo, mn, ms, and mtext),
        // when descendants of HTML elements, may contain
        // [phrasing content] from the HTML namespace.
        // https://html.spec.whatwg.org/#phrasing-content-2
        //
        // In MathML 3, nothing is allowed except MathML elements.
        // Further, nesting root-level math elements is disallowed.
        //
        // Since the math element is considered phrasing content,
        // it can be nested in MathML Core (as long as it is itself
        // within a MathML token element).
        engine.sink.warn(warning!(
            elem.span(),
            "{} was ignored during MathML export",
            elem.elem().name()
        ));
    }

    Ok(())
}

fn group(content: Content) -> Content {
    if let Some(sequence) = content.to_packed::<SequenceElem>()
        && sequence.children.len() > 1
    {
        let span = content.span();
        HtmlElem::new(tag::mrow).with_body(Some(content)).pack().spanned(span)
    } else {
        content
    }
}

fn get_body_text(elem: &Packed<HtmlElem>) -> Option<&EcoString> {
    elem.body
        .as_option()
        .as_ref()
        .and_then(Option::as_ref)
        .and_then(|content| content.to_packed::<TextElem>())
        .map(|elem| &elem.text)
}

fn get_mo_text(content: &Content) -> Option<&EcoString> {
    content
        .to_packed::<HtmlElem>()
        .filter(|elem| elem.tag == tag::mo)
        .and_then(get_body_text)
}

fn determine_limits(base: &Content, styles: StyleChain) -> SourceResult<bool> {
    // TODO: this is probably not complete
    let limits = if let Some(limits) = base.to_packed::<LimitsElem>() {
        if limits.inline.get(styles) { Limits::Always } else { Limits::Display }
    } else if base.is::<ScriptsElem>() {
        Limits::Never
    } else if let Some(text) = get_mo_text(base)
        && let mut chars = text.chars()
        && let Some(c) = chars.next()
        && chars.next().is_none()
    {
        Limits::for_char(c)
    } else {
        Limits::Never
    };

    let limits = limits.active(styles);

    Ok(limits)
}

fn show_h(elem: &Packed<HElem>, _: &mut Engine, _: StyleChain) -> SourceResult<Content> {
    let mut attrs = HtmlAttrs::new();
    match elem.amount {
        Spacing::Rel(rel) => attrs.push(attr::width, eco_format!("{}", css::rel(rel))),
        Spacing::Fr(_) => {}
    }

    Ok(HtmlElem::new(tag::mspace)
        .with_attrs(attrs)
        .pack()
        .spanned(elem.span()))
}

fn show_text(
    elem: &Packed<TextElem>,
    _: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    // TODO: sort out error from validator "Element mtext not allowed as child of element mi in this context"
    let tag = if elem.text.chars().all(|c| c.is_ascii_digit() || c == '.') {
        tag::mn
    } else {
        tag::mtext
    };

    let variant = styles.get(EquationElem::variant);
    let bold = styles.get(EquationElem::bold);
    // Disable auto-italic.
    let italic = styles.get(EquationElem::italic).or(Some(false));

    let styled_text: EcoString = elem
        .text
        .chars()
        .flat_map(|c| to_style(c, MathStyle::select(c, variant, bold, italic)))
        .collect();

    Ok(HtmlElem::new(tag)
        .with_body(Some(TextElem::packed(styled_text)))
        .pack()
        .spanned(elem.span()))
}

fn show_symbol(
    elem: &Packed<SymbolElem>,
    _: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    // Can't check if the font has the dtls feature...

    let variant = styles.get(EquationElem::variant);
    let bold = styles.get(EquationElem::bold);
    let italic = styles.get(EquationElem::italic);

    let text: EcoString = elem
        .text
        .chars()
        .flat_map(|c| to_style(c, MathStyle::select(c, variant, bold, italic)))
        .collect();

    let class = styles
        .get(EquationElem::class)
        .or_else(|| default_math_class(text.chars().next().unwrap()))
        .unwrap_or(MathClass::Normal);

    let mut attrs = HtmlAttrs::new();

    // Only add this when necessary, i.e. to ensure browsers don't perform an
    // italic mapping. See https://www.w3.org/TR/mathml-core/#math-auto-transform.
    let mut chars = text.chars();
    if matches!(chars.next().unwrap(), 'A'..='Z' | 'a'..='z' | 'ı' | 'ȷ' | 'Α'..='Ρ' | 'ϴ' | 'Σ'..='Ω' | '∇' | 'α'..='ω' | '∂' | 'ϵ' | 'ϑ' | 'ϰ' | 'ϕ' | 'ϱ' | 'ϖ')
        && chars.next().is_none()
    {
        attrs.push(attr::mathvariant, "normal");
    }

    // Need to rethink this bit...
    // TODO: how should class be dealt with?
    // let mut fence = false;
    // let mut separator = false;
    let tag = match class {
        MathClass::Normal
        | MathClass::Alphabetic
        | MathClass::Special
        | MathClass::GlyphPart
        | MathClass::Space => tag::mi,

        MathClass::Vary | MathClass::Relation | MathClass::Diacritic => tag::mo,

        MathClass::Binary => {
            attrs.push(attr::form, "infix");
            tag::mo
        }

        MathClass::Unary => {
            attrs.push(attr::form, "prefix");
            tag::mo
        }

        MathClass::Punctuation => {
            // separator = true;
            tag::mo
        }

        MathClass::Fence => {
            // fence = true;
            tag::mo
        }

        MathClass::Large => {
            attrs.push(attr::largeop, "true");
            tag::mo
        }

        MathClass::Opening => {
            // fence = true;
            // attrs.push(attr::form, "prefix");
            // attrs.push(attr::stretchy, "true");
            tag::mo
        }

        MathClass::Closing => {
            // fence = true;
            // attrs.push(attr::form, "postfix");
            tag::mo
        }
    };

    Ok(HtmlElem::new(tag)
        .with_attrs(attrs)
        .with_body(Some(TextElem::packed(text)))
        .pack()
        .spanned(elem.span()))
}

fn show_op(
    elem: &Packed<OpElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    let text = show_equation(&elem.text, engine, styles)?;
    Ok(HtmlElem::new(tag::mi)
        .with_body(Some(text))
        .pack()
        .spanned(elem.span()))
}

fn show_root(
    elem: &Packed<RootElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    let cramped = style_cramped();
    let radicand = show_equation(&elem.radicand, engine, styles.chain(&cramped))?;

    let Some(index) = elem.index.get_ref(styles) else {
        return Ok(HtmlElem::new(tag::msqrt)
            .with_body(Some(radicand))
            .pack()
            .spanned(elem.span()));
    };

    let sscript = EquationElem::size.set(MathSize::ScriptScript).wrap();
    let index = group(show_equation(index, engine, styles.chain(&sscript))?);
    Ok(HtmlElem::new(tag::mroot)
        .with_body(Some(group(radicand) + index))
        .pack()
        .spanned(elem.span()))
}

fn show_frac(
    elem: &Packed<FracElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    show_frac_like(
        engine,
        styles,
        &elem.num,
        std::slice::from_ref(&elem.denom),
        false,
        elem.span(),
    )
}

fn show_binom(
    elem: &Packed<BinomElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    show_frac_like(engine, styles, &elem.upper, &elem.lower, true, elem.span())
}

fn show_frac_like(
    engine: &mut Engine,
    styles: StyleChain,
    num: &Content,
    denom: &[Content],
    binom: bool,
    span: typst_syntax::Span,
) -> SourceResult<Content> {
    let num_style = style_for_numerator(styles);
    let num = group(show_equation(num, engine, styles.chain(&num_style))?);

    let denom_style = style_for_denominator(styles);
    let denom = group(show_equation(
        &Content::sequence(
            denom
                .iter()
                .flat_map(|a| [SymbolElem::packed(','), a.clone()])
                .skip(1),
        ),
        engine,
        styles.chain(&denom_style),
    )?);

    let frac = HtmlElem::new(tag::mfrac)
        .with_body(Some(num + denom))
        .with_optional_attr(attr::linethickness, binom.then_some("0"))
        .pack();

    if !binom {
        return Ok(frac.spanned(span));
    }

    // These are both already stretchy + have correct form + are fence.
    let open = HtmlElem::new(tag::mo).with_body(Some(TextElem::packed('('))).pack();
    let close = HtmlElem::new(tag::mo).with_body(Some(TextElem::packed(')'))).pack();
    Ok(HtmlElem::new(tag::mrow)
        .with_body(Some(open + frac + close))
        .pack()
        .spanned(span))
}

fn show_accent(
    elem: &Packed<AccentElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    let accent = elem.accent;
    let (tag, attr) = if accent.is_bottom() {
        (tag::munder, attr::accentunder)
    } else {
        (tag::mover, attr::accent)
    };

    let cramped = style_cramped();
    let mut base = group(show_equation(&elem.base, engine, styles.chain(&cramped))?);

    // TODO: maybe only add this if the base is an i or j, or only add when disabling.
    let dtls = if elem.dotless.get(styles) { "on" } else { "off" };
    if !accent.is_bottom() {
        base.to_packed_mut::<HtmlElem>().unwrap().push_attr(
            crate::attr::style,
            eco_format!("font-feature-settings: 'dtls' {};", dtls),
        );
    }

    // TODO: convert accent char to non-combining, then lookup with postfix (or infix?) form for stretchy
    // Should surpress "Text run starts with a composing character" warnings from validator.
    let accent = HtmlElem::new(tag::mo)
        .with_body(Some(TextElem::packed(accent.0)))
        .pack();

    Ok(HtmlElem::new(tag)
        .with_body(Some(base + accent))
        .with_attr(attr, "true")
        .pack()
        .spanned(elem.span()))
}

fn show_lr(
    elem: &Packed<LrElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
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

    let mut body = show_equation(body, engine, styles)?;
    // TODO: ignore leading and trailing ignorant elements...
    // TODO: remove weak spacing immediately after opening/before closing.

    let scale = |content: &mut Content, form: mathml::Form| {
        if let Some(elem) = content.to_packed_mut::<HtmlElem>()
            && elem.tag == tag::mo
        {
            let text = get_body_text(elem);
            let fence = text.is_none_or(|x| !mathml::is_fence(x)).then_some("true");
            let stretchy = text
                .is_none_or(|x| {
                    !mathml::get_operator_info(x, form)
                        .properties
                        .contains(mathml::Properties::STRETCHY)
                })
                .then_some("true");

            elem.push_optional_attr(attr::fence, fence);
            elem.push_optional_attr(attr::stretchy, stretchy);
            // Don't need to set form, as prefix is already at start, postfix at end, etc.
            // TODO: set stretch size
        }
    };

    if let Some(seq) = body.to_packed_mut::<SequenceElem>() {
        let children = &mut seq.children;
        let len = children.len();
        if len == 0 {
        } else if len == 1 {
            scale(&mut children[0], mathml::Form::Infix)
        } else {
            scale(&mut children[0], mathml::Form::Prefix);
            scale(&mut children[len - 1], mathml::Form::Postfix);

            // Need to wrap the middle stuff in an mrow for firefox, otherwise it doesn't always scale properly.
            let middle: Vec<Content> = children.drain(1..len - 1).collect();
            let new = HtmlElem::new(tag::mrow)
                .with_body(Some(Content::sequence(middle)))
                .pack();
            children.insert(1, new);
        }
    } else {
        scale(&mut body, mathml::Form::Infix);
    }

    // Same here, I didn't want to always wrap everything in an mrow, but firefox won't scale things properly e.g. at root level.
    Ok(HtmlElem::new(tag::mrow)
        .with_body(Some(body))
        .pack()
        .spanned(elem.span()))
}

fn show_mid(
    elem: &Packed<MidElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    let mut body = show_equation(&elem.body, engine, styles)?;
    for elem in body.iter_mut() {
        if let Some(elem) = elem.to_packed_mut::<HtmlElem>()
            && elem.tag == tag::mo
        {
            let text = get_body_text(elem);
            let fence = text.is_none_or(|x| !mathml::is_fence(x)).then_some("true");
            let stretchy = text
                .is_none_or(|x| {
                    !mathml::get_operator_info(x, mathml::Form::Infix)
                        .properties
                        .contains(mathml::Properties::STRETCHY)
                })
                .then_some("true");

            elem.push_optional_attr(attr::fence, fence);
            elem.push_optional_attr(attr::stretchy, stretchy);

            // TODO: try to only emit this if needed.
            elem.push_attr(attr::form, "infix");
        }
    }
    Ok(body)
}

fn show_mat(
    elem: &Packed<MatElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    let rows = elem.rows.iter().map(|i| i.iter().collect()).collect();
    let body = show_body(&rows, engine, styles)?;
    let delim = elem.delim.get(styles);
    Ok(add_delimiters(body, delim.open(), delim.close(), elem.span()))
}

fn show_vec(
    elem: &Packed<VecElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    let rows = elem.children.iter().map(|x| vec![x]).collect();
    let body = show_body(&rows, engine, styles)?;
    let delim = elem.delim.get(styles);
    Ok(add_delimiters(body, delim.open(), delim.close(), elem.span()))
}

fn show_cases(
    elem: &Packed<CasesElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    let rows = elem.children.iter().map(|x| vec![x]).collect();
    let body = show_body(&rows, engine, styles)?;
    let delim = elem.delim.get(styles);
    let (open, close) = if elem.reverse.get(styles) {
        (None, delim.close())
    } else {
        (delim.open(), None)
    };
    Ok(add_delimiters(body, open, close, elem.span()))
}

fn show_body(
    rows: &Vec<Vec<&Content>>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    let denom_style = style_for_denominator(styles);
    let styles = styles.chain(&denom_style);
    Ok(HtmlElem::new(tag::mtable)
        .with_body(Some(
            rows.iter()
                .map(|row| {
                    Ok(HtmlElem::new(tag::mtr)
                        .with_body(Some(
                            row.iter()
                                .map(|cell| {
                                    // TODO: Check for linebreaks and emit a
                                    // warning.
                                    Ok(HtmlElem::new(tag::mtd)
                                        .with_body(Some(show_equation(
                                            cell, engine, styles,
                                        )?))
                                        .pack())
                                })
                                .collect::<SourceResult<_>>()?,
                        ))
                        .pack())
                })
                .collect::<SourceResult<_>>()?,
        ))
        .pack())
}

fn add_delimiters(
    body: Content,
    open: Option<char>,
    close: Option<char>,
    span: Span,
) -> Content {
    if open.is_none() && close.is_none() {
        return body.spanned(span);
    }

    let mut row = vec![];

    if let Some(open_c) = open {
        // Form prefix not needed here.
        let mut buf = [0; 4];
        let open_str = open_c.encode_utf8(&mut buf);

        let fence = (!mathml::is_fence(open_str)).then_some("true");
        let stretchy = (!mathml::get_operator_info(open_str, mathml::Form::Prefix)
            .properties
            .contains(mathml::Properties::STRETCHY))
        .then_some("true");

        row.push(
            HtmlElem::new(tag::mo)
                .with_body(Some(TextElem::packed(open_c)))
                .with_optional_attr(attr::fence, fence)
                .with_optional_attr(attr::stretchy, stretchy)
                .pack(),
        );
    }

    row.push(body);

    if let Some(close_c) = close {
        // Form postfix not needed here.
        let mut buf = [0; 4];
        let close_str = close_c.encode_utf8(&mut buf);

        let fence = (!mathml::is_fence(close_str)).then_some("true");
        let stretchy = (!mathml::get_operator_info(close_str, mathml::Form::Postfix)
            .properties
            .contains(mathml::Properties::STRETCHY))
        .then_some("true");

        row.push(
            HtmlElem::new(tag::mo)
                .with_body(Some(TextElem::packed(close_c)))
                .with_optional_attr(attr::fence, fence)
                .with_optional_attr(attr::stretchy, stretchy)
                .pack(),
        );
    }

    HtmlElem::new(tag::mrow)
        .with_body(Some(Content::sequence(row)))
        .pack()
        .spanned(span)
}

fn show_scripts(
    elem: &Packed<ScriptsElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    show_equation(&elem.body, engine, styles)
}

fn show_limits(
    elem: &Packed<LimitsElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    show_equation(&elem.body, engine, styles)
}

fn show_attach(
    elem: &Packed<AttachElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    // TODO: should maybe try to make use of movablelimits attr.

    let merged = elem.merge_base();
    let elem = merged.as_ref().unwrap_or(elem);
    let base = group(show_equation(&elem.base, engine, styles)?);

    macro_rules! layout {
        ($content:ident, $style_chain:ident) => {
            elem.$content
                .get_cloned($style_chain)
                .map(|x| show_equation(&x, engine, $style_chain))
                .transpose()?
                .map(group)
        };
    }

    let sup_style = style_for_superscript(styles);
    let sup_style_chain = styles.chain(&sup_style);
    let tl = layout!(tl, sup_style_chain);
    let tr = layout!(tr, sup_style_chain);
    let primed = tr.as_ref().is_some_and(|content| content.is::<PrimesElem>());
    let t = layout!(t, sup_style_chain);

    let sub_style = style_for_subscript(styles);
    let sub_style_chain = styles.chain(&sub_style);
    let bl = layout!(bl, sub_style_chain);
    let br = layout!(br, sub_style_chain);
    let b = layout!(b, sub_style_chain);

    let limits = determine_limits(&base, styles)?;
    let (t, tr) = match (t, tr) {
        (Some(t), Some(tr)) if primed && !limits => (None, Some(tr + t)),
        (Some(t), None) if !limits => (None, Some(t)),
        (t, tr) => (t, tr),
    };
    let (b, br) = if limits || br.is_some() { (b, br) } else { (None, b) };

    let base = if let Some((tag, body)) = match (tl, tr, bl, br) {
        (None, None, None, None) => None,
        (None, None, None, Some(br)) => Some((tag::msub, br)),

        (None, Some(tr), None, None) => Some((tag::msup, tr)),
        (None, Some(tr), None, Some(br)) => Some((tag::msubsup, br + tr)),
        (tl, tr, bl, br) => {
            let mut body = vec![];
            macro_rules! push {
                ($content:ident) => {
                    body.push($content.unwrap_or(HtmlElem::new(tag::mrow).pack()));
                };
            }
            push!(br);
            push!(tr);
            body.push(HtmlElem::new(tag::mprescripts).pack());
            push!(bl);
            push!(tl);
            Some((tag::mmultiscripts, Content::sequence(body)))
        }
    } {
        HtmlElem::new(tag).with_body(Some(base + body)).pack()
    } else {
        base
    };

    let base = if let Some((tag, body)) = match (t, b) {
        (None, None) => None,
        (Some(t), None) => Some((tag::mover, t)),
        (None, Some(b)) => Some((tag::munder, b)),
        (Some(t), Some(b)) => Some((tag::munderover, b + t)),
    } {
        HtmlElem::new(tag).with_body(Some(base + body)).pack()
    } else {
        base
    };

    Ok(base.spanned(elem.span()))
}

fn show_primes(
    elem: &Packed<PrimesElem>,
    _: &mut Engine,
    _: StyleChain,
) -> SourceResult<Content> {
    let body = match elem.count {
        count @ 1..=4 => {
            let c = match count {
                1 => '′',
                2 => '″',
                3 => '‴',
                4 => '⁗',
                _ => unreachable!(),
            };
            TextElem::packed(c)
        }
        count => {
            // TODO: Should this be one <mo> or multiple?
            TextElem::packed("′".repeat(count))
        }
    };

    Ok(HtmlElem::new(tag::mo)
        .with_body(Some(body))
        .pack()
        .spanned(elem.span()))
}

fn show_underbrace(
    elem: &Packed<UnderbraceElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    show_underover(
        engine,
        styles,
        &elem.body,
        elem.annotation.get_ref(styles),
        '⏟',
        Position::Under,
        elem.span(),
    )
}

fn show_overbrace(
    elem: &Packed<OverbraceElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    show_underover(
        engine,
        styles,
        &elem.body,
        elem.annotation.get_ref(styles),
        '⏞',
        Position::Over,
        elem.span(),
    )
}

fn show_underbracket(
    elem: &Packed<UnderbracketElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    show_underover(
        engine,
        styles,
        &elem.body,
        elem.annotation.get_ref(styles),
        '⎵',
        Position::Under,
        elem.span(),
    )
}

fn show_overbracket(
    elem: &Packed<OverbracketElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    show_underover(
        engine,
        styles,
        &elem.body,
        elem.annotation.get_ref(styles),
        '⎴',
        Position::Over,
        elem.span(),
    )
}

fn show_underparen(
    elem: &Packed<UnderparenElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    show_underover(
        engine,
        styles,
        &elem.body,
        elem.annotation.get_ref(styles),
        '⏝',
        Position::Under,
        elem.span(),
    )
}

fn show_overparen(
    elem: &Packed<OverparenElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    show_underover(
        engine,
        styles,
        &elem.body,
        elem.annotation.get_ref(styles),
        '⏜',
        Position::Over,
        elem.span(),
    )
}

fn show_undershell(
    elem: &Packed<UndershellElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    show_underover(
        engine,
        styles,
        &elem.body,
        elem.annotation.get_ref(styles),
        '⏡',
        Position::Under,
        elem.span(),
    )
}

fn show_overshell(
    elem: &Packed<OvershellElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    show_underover(
        engine,
        styles,
        &elem.body,
        elem.annotation.get_ref(styles),
        '⏠',
        Position::Over,
        elem.span(),
    )
}

fn show_underover(
    engine: &mut Engine,
    styles: StyleChain,
    base: &Content,
    annotation: &Option<Content>,
    c: char,
    position: Position,
    span: Span,
) -> SourceResult<Content> {
    let base = group(show_equation(base, engine, styles)?);
    // The under/over characters are all postfix & stretchy by default.
    let glyph = HtmlElem::new(tag::mo).with_body(Some(TextElem::packed(c))).pack();

    let (tag, attr) = match position {
        Position::Under => (tag::munder, attr::accentunder),
        Position::Over => (tag::mover, attr::accent),
    };

    let underover = HtmlElem::new(tag)
        .with_body(Some(base + glyph))
        .with_attr(attr, "true")
        .pack();

    let Some(annotation) = annotation else {
        return Ok(underover.spanned(span));
    };

    let under_style = style_for_subscript(styles);
    let over_style = style_for_superscript(styles);
    let styles = match position {
        Position::Under => styles.chain(&under_style),
        Position::Over => styles.chain(&over_style),
    };

    let annotation = group(show_equation(annotation, engine, styles)?);
    Ok(HtmlElem::new(tag)
        .with_body(Some(underover + annotation))
        .pack()
        .spanned(span))
}
