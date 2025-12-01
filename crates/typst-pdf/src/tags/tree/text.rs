use krilla::tagging::{LineHeight, NaiveRgbColor, TextDecorationType};
use typst_library::diag::{SourceDiagnostic, error};
use typst_library::foundations::{Content, Packed, Smart};
use typst_library::layout::Length;
use typst_library::text::{
    HighlightElem, OverlineElem, ScriptKind, StrikeElem, SubElem, SuperElem, TextItem,
    TextSize, UnderlineElem,
};
use typst_library::visualize::Stroke;

use crate::PdfOptions;
use crate::tags::tree::Tree;
use crate::tags::util::{PropertyOptRef, PropertyValCloned, PropertyValCopied};
use crate::tags::{GroupId, util};
use crate::util::AbsExt;

#[derive(Debug, Clone)]
pub struct TextAttrs {
    /// Store the last resolved set of text attributes. The resolution isn't
    /// that expensive, but for large bodies of text it is resolved quite often.
    last_resolved: Option<(TextParams, ResolvedTextAttrs)>,
    items: Vec<(GroupId, TextAttr)>,
}

impl TextAttrs {
    pub const fn new() -> Self {
        Self { last_resolved: None, items: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn push(&mut self, id: GroupId, attr: TextAttr) {
        self.last_resolved = None;
        self.items.push((id, attr));
    }

    pub fn insert(&mut self, idx: usize, id: GroupId, attr: TextAttr) {
        self.last_resolved = None;
        self.items.insert(idx, (id, attr));
    }

    /// Returns true if a decoration was removed.
    pub fn pop(&mut self, id: GroupId) -> bool {
        self.last_resolved = None;
        self.items.pop_if(|(i, _)| *i == id).is_some()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextAttr {
    Strong,
    Emph,
    SuperScript(Packed<SuperElem>),
    SubScript(Packed<SubElem>),
    Highlight(Packed<HighlightElem>),
    Underline(Packed<UnderlineElem>),
    Overline(Packed<OverlineElem>),
    Strike(Packed<StrikeElem>),
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ResolvedTextAttrs {
    pub strong: Option<bool>,
    pub emph: Option<bool>,
    pub script: Option<ResolvedScript>,
    pub background: Option<Option<NaiveRgbColor>>,
    pub deco: Option<ResolvedTextDeco>,
}

impl ResolvedTextAttrs {
    pub const EMPTY: Self = Self {
        strong: None,
        emph: None,
        script: None,
        background: None,
        deco: None,
    };

    pub fn is_empty(&self) -> bool {
        self == &Self::EMPTY
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ResolvedScript {
    pub baseline_shift: f32,
    pub lineheight: LineHeight,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ResolvedTextDeco {
    pub kind: TextDecoKind,
    pub color: Option<NaiveRgbColor>,
    pub thickness: Option<f32>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TextDecoKind {
    Underline,
    Overline,
    Strike,
}

impl TextDecoKind {
    pub fn to_krilla(self) -> TextDecorationType {
        match self {
            TextDecoKind::Underline => TextDecorationType::Underline,
            TextDecoKind::Overline => TextDecorationType::Overline,
            TextDecoKind::Strike => TextDecorationType::LineThrough,
        }
    }
}

/// A hash of relevant text parameters.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct TextParams(u128);

impl TextParams {
    fn new(text: &TextItem) -> TextParams {
        TextParams(typst_utils::hash128(&(&text.font, text.size)))
    }
}

pub fn resolve_text_attrs(
    tree: &mut Tree,
    options: &PdfOptions,
    text: &TextItem,
) -> ResolvedTextAttrs {
    let params = TextParams::new(text);
    if let Some((prev_params, attrs)) = tree.state.text_attrs.last_resolved
        && prev_params == params
    {
        return attrs;
    }

    let (attrs, error) = compute_attrs(options, &tree.state.text_attrs.items, text);

    tree.errors.extend(error);

    tree.state.text_attrs.last_resolved = Some((params, attrs));
    attrs
}

fn compute_attrs(
    options: &PdfOptions,
    items: &[(GroupId, TextAttr)],
    text: &TextItem,
) -> (ResolvedTextAttrs, Option<SourceDiagnostic>) {
    let mut attrs = ResolvedTextAttrs::EMPTY;
    let mut resolved_deco: Option<(&Content, ResolvedTextDeco)> = None;
    let mut err = None;
    for (_, attr) in items.iter().rev() {
        match attr {
            TextAttr::Strong => {
                attrs.strong.get_or_insert(true);
            }
            TextAttr::Emph => {
                attrs.emph.get_or_insert(true);
            }
            TextAttr::SubScript(sub) => {
                attrs.script.get_or_insert_with(|| {
                    let kind = ScriptKind::Sub;
                    compute_script(text, kind, sub.baseline.val(), sub.size.val())
                });
            }
            TextAttr::SuperScript(sub) => {
                attrs.script.get_or_insert_with(|| {
                    let kind = ScriptKind::Super;
                    compute_script(text, kind, sub.baseline.val(), sub.size.val())
                });
            }
            TextAttr::Highlight(highlight) => {
                let paint = highlight.fill.opt_ref();
                let color = paint.and_then(util::paint_to_color);
                attrs.background.get_or_insert(color);
            }
            TextAttr::Underline(underline) => {
                compute_deco(
                    &mut resolved_deco,
                    &mut err,
                    options,
                    text,
                    underline.pack_ref(),
                    TextDecoKind::Underline,
                    underline.stroke.val_cloned(),
                );
            }
            TextAttr::Overline(overline) => {
                compute_deco(
                    &mut resolved_deco,
                    &mut err,
                    options,
                    text,
                    overline.pack_ref(),
                    TextDecoKind::Overline,
                    overline.stroke.val_cloned(),
                );
            }
            TextAttr::Strike(strike) => {
                compute_deco(
                    &mut resolved_deco,
                    &mut err,
                    options,
                    text,
                    strike.pack_ref(),
                    TextDecoKind::Strike,
                    strike.stroke.val_cloned(),
                );
            }
        }
    }

    attrs.deco = resolved_deco.map(|(_, d)| d);

    (attrs, err)
}

fn compute_script(
    text: &TextItem,
    kind: ScriptKind,
    baseline_shift: Smart<Length>,
    lineheight: Smart<TextSize>,
) -> ResolvedScript {
    // TODO: The `typographic` setting is ignored for now.
    // Is it better to be accurate regarding the layouting, and
    // thus don't write any baseline shift and lineheight when
    // a typographic sub/super script glyph is used? Or should
    // we always write the shift so the sub/super script can be
    // picked up by AT?
    let script_metrics = kind.read_metrics(text.font.metrics());
    // NOTE: The user provided baseline_shift needs to be inverted.
    let baseline_shift = (baseline_shift.map(|s| -s.at(text.size)))
        .unwrap_or_else(|| script_metrics.vertical_offset.at(text.size));
    let lineheight = (lineheight.map(|s| s.0.at(text.size)))
        .unwrap_or_else(|| script_metrics.height.at(text.size));

    ResolvedScript {
        baseline_shift: baseline_shift.to_f32(),
        lineheight: LineHeight::Custom(lineheight.to_f32()),
    }
}

fn compute_deco<'a>(
    resolved: &mut Option<(&'a Content, ResolvedTextDeco)>,
    err: &mut Option<SourceDiagnostic>,
    options: &PdfOptions,
    text: &TextItem,
    elem: &'a Content,
    kind: TextDecoKind,
    stroke: Smart<Stroke>,
) {
    match resolved {
        Some((elem, deco)) => {
            // PDF can only represent one text decoration style at a time.
            // If PDF/UA-1 is enforced throw an error.
            if err.is_none() && deco.kind != kind && options.is_pdf_ua() {
                let validator = options.standards.config.validator().as_str();
                let span = elem.span();
                *err = Some(error!(
                    span,
                    "{validator} error: cannot combine underline, overline, or strike",
                ));
            }
        }
        None => {
            let color = (stroke.as_ref().custom())
                .and_then(|s| s.paint.as_ref().custom())
                .and_then(util::paint_to_color);
            let thickness = (stroke.as_ref().custom())
                .and_then(|s| s.thickness.custom())
                .map(|t| t.at(text.size).to_f32());
            let deco = ResolvedTextDeco { kind, color, thickness };
            *resolved = Some((elem, deco));
        }
    }
}
