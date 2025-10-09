use krilla::tagging::{LineHeight, NaiveRgbColor, TextDecorationType};
use typst_library::diag::{SourceDiagnostic, error};
use typst_library::foundations::Smart;
use typst_library::layout::{Abs, Length};
use typst_library::text::{Font, ScriptKind, TextItem, TextSize};
use typst_library::visualize::Stroke;

use crate::PdfOptions;
use crate::tags::tree::Tree;
use crate::tags::{GroupId, util};
use crate::util::AbsExt;

#[derive(Debug, Clone)]
pub struct TextAttrs {
    /// Store the last resolved set of text attribute. The resolution isn't that
    /// expensive, but for large bodies of text it is resolved quite often.
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

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TextAttr {
    Strong,
    Emph,
    Script(Script),
    Highlight(Option<NaiveRgbColor>),
    Deco(TextDeco),
}

/// Sub- or super-script.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Script {
    kind: ScriptKind,
    baseline_shift: Smart<Length>,
    lineheight: Smart<TextSize>,
}

impl Script {
    pub fn new(
        kind: ScriptKind,
        baseline_shift: Smart<Length>,
        lineheight: Smart<TextSize>,
    ) -> Self {
        Self { kind, baseline_shift, lineheight }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TextDeco {
    kind: TextDecoKind,
    stroke: TextDecoStroke,
}

impl TextDeco {
    pub fn new(kind: TextDecoKind, stroke: Smart<Stroke>) -> Self {
        let stroke = TextDecoStroke::from(stroke);
        Self { kind, stroke }
    }
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

#[derive(Debug, Default, Copy, Clone, PartialEq)]
struct TextDecoStroke {
    color: Option<NaiveRgbColor>,
    thickness: Option<Length>,
}

impl TextDecoStroke {
    fn from(stroke: Smart<Stroke>) -> Self {
        let Smart::Custom(stroke) = stroke else {
            return TextDecoStroke::default();
        };
        let color = stroke.paint.custom().as_ref().and_then(util::paint_to_color);
        let thickness = stroke.thickness.custom();
        TextDecoStroke { color, thickness }
    }
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
struct TextParams {
    pub font_index: u32,
    pub size: Abs,
}

impl TextParams {
    fn new(text: &TextItem) -> TextParams {
        TextParams {
            // Comparing font indices is enough.
            font_index: text.font.index(),
            size: text.size,
        }
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

    let (attrs, error) =
        compute_attrs(tree, options, &tree.state.text_attrs.items, &text.font, text.size);

    tree.errors.extend(error);

    tree.state.text_attrs.last_resolved = Some((params, attrs));
    attrs
}

fn compute_attrs(
    tree: &Tree,
    options: &PdfOptions,
    items: &[(GroupId, TextAttr)],
    font: &Font,
    size: Abs,
) -> (ResolvedTextAttrs, Option<SourceDiagnostic>) {
    let mut attrs = ResolvedTextAttrs::EMPTY;
    let mut resolved_deco: Option<(GroupId, ResolvedTextDeco)> = None;
    let mut err = None;
    for (id, attr) in items.iter().rev() {
        match *attr {
            TextAttr::Strong => {
                attrs.strong.get_or_insert(true);
            }
            TextAttr::Emph => {
                attrs.emph.get_or_insert(true);
            }
            TextAttr::Script(script) => {
                attrs.script.get_or_insert_with(|| {
                    // TODO: The `typographic` setting is ignored for now.
                    // Is it better to be accurate regarding the layouting, and
                    // thus don't write any baseline shift and lineheight when
                    // a typographic sub/super script glyph is used? Or should
                    // we always write the shift so the sub/super script can be
                    // picked up by AT?
                    let script_metrics = script.kind.read_metrics(font.metrics());
                    // NOTE: The user provided baseline_shift needs to be inverted.
                    let baseline_shift = (script.baseline_shift.map(|s| -s.at(size)))
                        .unwrap_or_else(|| script_metrics.vertical_offset.at(size));
                    let lineheight = (script.lineheight.map(|s| s.0.at(size)))
                        .unwrap_or_else(|| script_metrics.height.at(size));

                    ResolvedScript {
                        baseline_shift: baseline_shift.to_f32(),
                        lineheight: LineHeight::Custom(lineheight.to_f32()),
                    }
                });
            }
            TextAttr::Highlight(color) => {
                attrs.background.get_or_insert(color);
            }
            TextAttr::Deco(TextDeco { kind, stroke }) => {
                // PDF can only represent one text decoration style at a time.
                // If PDF/UA-1 is enforced throw an error.
                if let Some((id, deco)) = resolved_deco
                    && deco.kind != kind
                    && options.is_pdf_ua()
                    && err.is_none()
                {
                    let validator = options.standards.config.validator().as_str();
                    let span = tree.groups.get(id).span;
                    err = Some(error!(
                        span,
                        "{validator} error: cannot combine underline, overline, or strike"
                    ));
                }

                resolved_deco.get_or_insert_with(|| {
                    let thickness = stroke.thickness.map(|t| t.at(size).to_f32());
                    let deco = ResolvedTextDeco { kind, color: stroke.color, thickness };
                    (*id, deco)
                });
            }
        }
    }

    attrs.deco = resolved_deco.map(|(_, d)| d);

    (attrs, err)
}
