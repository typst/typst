use std::fmt::{self, Debug, Formatter};

use az::SaturatingAs;
use rustybuzz::{BufferFlags, UnicodeBuffer};
use ttf_parser::math::{GlyphAssembly, GlyphConstruction, GlyphPart};
use ttf_parser::GlyphId;
use typst_library::diag::{bail, warning, SourceResult};
use typst_library::foundations::StyleChain;
use typst_library::introspection::Tag;
use typst_library::layout::{
    Abs, Axes, Axis, Corner, Em, Frame, FrameItem, Point, Size, VAlignment,
};
use typst_library::math::{EquationElem, MathSize};
use typst_library::text::{features, language, Font, Glyph, TextElem, TextItem};
use typst_syntax::Span;
use typst_utils::{default_math_class, Get};
use unicode_math_class::MathClass;

use super::MathContext;
use crate::inline::create_shape_plan;
use crate::modifiers::{FrameModifiers, FrameModify};

/// Maximum number of times extenders can be repeated.
const MAX_REPEATS: usize = 1024;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum MathFragment {
    Glyph(GlyphFragment),
    Frame(FrameFragment),
    Spacing(Abs, bool),
    Space(Abs),
    Linebreak,
    Align,
    Tag(Tag),
}

impl MathFragment {
    pub fn size(&self) -> Size {
        match self {
            Self::Glyph(glyph) => glyph.size,
            Self::Frame(fragment) => fragment.frame.size(),
            Self::Spacing(amount, _) => Size::with_x(*amount),
            Self::Space(amount) => Size::with_x(*amount),
            _ => Size::zero(),
        }
    }

    pub fn width(&self) -> Abs {
        match self {
            Self::Glyph(glyph) => glyph.size.x,
            Self::Frame(fragment) => fragment.frame.width(),
            Self::Spacing(amount, _) => *amount,
            Self::Space(amount) => *amount,
            _ => Abs::zero(),
        }
    }

    pub fn height(&self) -> Abs {
        match self {
            Self::Glyph(glyph) => glyph.size.y,
            Self::Frame(fragment) => fragment.frame.height(),
            _ => Abs::zero(),
        }
    }

    pub fn ascent(&self) -> Abs {
        match self {
            Self::Glyph(glyph) => glyph.ascent(),
            Self::Frame(fragment) => fragment.frame.ascent(),
            _ => Abs::zero(),
        }
    }

    pub fn descent(&self) -> Abs {
        match self {
            Self::Glyph(glyph) => glyph.descent(),
            Self::Frame(fragment) => fragment.frame.descent(),
            _ => Abs::zero(),
        }
    }

    pub fn is_ignorant(&self) -> bool {
        match self {
            Self::Frame(fragment) => fragment.ignorant,
            Self::Tag(_) => true,
            _ => false,
        }
    }

    pub fn class(&self) -> MathClass {
        match self {
            Self::Glyph(glyph) => glyph.class,
            Self::Frame(fragment) => fragment.class,
            Self::Spacing(_, _) => MathClass::Space,
            Self::Space(_) => MathClass::Space,
            Self::Linebreak => MathClass::Space,
            Self::Align => MathClass::Special,
            Self::Tag(_) => MathClass::Special,
        }
    }

    pub fn math_size(&self) -> Option<MathSize> {
        match self {
            Self::Glyph(glyph) => Some(glyph.math_size),
            Self::Frame(fragment) => Some(fragment.math_size),
            _ => None,
        }
    }

    pub fn font_size(&self) -> Option<Abs> {
        match self {
            Self::Glyph(glyph) => Some(glyph.item.size),
            Self::Frame(fragment) => Some(fragment.font_size),
            _ => None,
        }
    }

    pub fn set_class(&mut self, class: MathClass) {
        match self {
            Self::Glyph(glyph) => glyph.class = class,
            Self::Frame(fragment) => fragment.class = class,
            _ => {}
        }
    }

    pub fn set_limits(&mut self, limits: Limits) {
        match self {
            Self::Glyph(glyph) => glyph.limits = limits,
            Self::Frame(fragment) => fragment.limits = limits,
            _ => {}
        }
    }

    pub fn is_spaced(&self) -> bool {
        if self.class() == MathClass::Fence {
            return true;
        }

        matches!(
            self,
            Self::Frame(FrameFragment {
                spaced: true,
                class: MathClass::Normal | MathClass::Alphabetic,
                ..
            })
        )
    }

    pub fn is_text_like(&self) -> bool {
        match self {
            Self::Glyph(glyph) => !glyph.extended_shape,
            Self::Frame(frame) => frame.text_like,
            _ => false,
        }
    }

    pub fn italics_correction(&self) -> Abs {
        match self {
            Self::Glyph(glyph) => glyph.italics_correction,
            Self::Frame(fragment) => fragment.italics_correction,
            _ => Abs::zero(),
        }
    }

    pub fn accent_attach(&self) -> (Abs, Abs) {
        match self {
            Self::Glyph(glyph) => glyph.accent_attach,
            Self::Frame(fragment) => fragment.accent_attach,
            _ => (self.width() / 2.0, self.width() / 2.0),
        }
    }

    pub fn into_frame(self) -> Frame {
        match self {
            Self::Glyph(glyph) => glyph.into_frame(),
            Self::Frame(fragment) => fragment.frame,
            Self::Tag(tag) => {
                let mut frame = Frame::soft(Size::zero());
                frame.push(Point::zero(), FrameItem::Tag(tag));
                frame
            }
            _ => Frame::soft(self.size()),
        }
    }

    pub fn limits(&self) -> Limits {
        match self {
            Self::Glyph(glyph) => glyph.limits,
            Self::Frame(fragment) => fragment.limits,
            _ => Limits::Never,
        }
    }

    /// If no kern table is provided for a corner, a kerning amount of zero is
    /// assumed.
    pub fn kern_at_height(&self, corner: Corner, height: Abs) -> Abs {
        match self {
            Self::Glyph(glyph) => {
                // For glyph assemblies we pick either the start or end glyph
                // depending on the corner.
                let is_vertical =
                    glyph.item.glyphs.iter().all(|glyph| glyph.y_advance != Em::zero());
                let glyph_index = match (is_vertical, corner) {
                    (true, Corner::TopLeft | Corner::TopRight) => {
                        glyph.item.glyphs.len() - 1
                    }
                    (false, Corner::TopRight | Corner::BottomRight) => {
                        glyph.item.glyphs.len() - 1
                    }
                    _ => 0,
                };

                kern_at_height(
                    &glyph.item.font,
                    GlyphId(glyph.item.glyphs[glyph_index].id),
                    corner,
                    Em::from_length(height, glyph.item.size),
                )
                .unwrap_or_default()
                .at(glyph.item.size)
            }
            _ => Abs::zero(),
        }
    }
}

impl From<GlyphFragment> for MathFragment {
    fn from(glyph: GlyphFragment) -> Self {
        Self::Glyph(glyph)
    }
}

impl From<FrameFragment> for MathFragment {
    fn from(fragment: FrameFragment) -> Self {
        Self::Frame(fragment)
    }
}

#[derive(Clone)]
pub struct GlyphFragment {
    // Text stuff.
    pub item: TextItem,
    pub base_glyph: Glyph,
    // Math stuff.
    pub size: Size,
    pub baseline: Option<Abs>,
    pub italics_correction: Abs,
    pub accent_attach: (Abs, Abs),
    pub math_size: MathSize,
    pub class: MathClass,
    pub limits: Limits,
    pub extended_shape: bool,
    pub mid_stretched: Option<bool>,
    // External frame stuff.
    pub modifiers: FrameModifiers,
    pub shift: Abs,
    pub align: Abs,
}

impl GlyphFragment {
    /// Calls `new` with the given character.
    pub fn new_char(
        font: &Font,
        styles: StyleChain,
        c: char,
        span: Span,
    ) -> SourceResult<Self> {
        Self::new(font, styles, c.encode_utf8(&mut [0; 4]), span)
    }

    /// Try to create a new glyph out of the given string. Will bail if the
    /// result from shaping the string is not a single glyph or is a tofu.
    #[comemo::memoize]
    pub fn new(
        font: &Font,
        styles: StyleChain,
        text: &str,
        span: Span,
    ) -> SourceResult<GlyphFragment> {
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.set_language(language(styles));
        // TODO: Use `rustybuzz::script::MATH` once
        // https://github.com/harfbuzz/rustybuzz/pull/165 is released.
        buffer.set_script(
            rustybuzz::Script::from_iso15924_tag(ttf_parser::Tag::from_bytes(b"math"))
                .unwrap(),
        );
        buffer.set_direction(rustybuzz::Direction::LeftToRight);
        buffer.set_flags(BufferFlags::REMOVE_DEFAULT_IGNORABLES);

        let features = features(styles);
        let plan = create_shape_plan(
            font,
            buffer.direction(),
            buffer.script(),
            buffer.language().as_ref(),
            &features,
        );

        let buffer = rustybuzz::shape_with_plan(font.rusty(), &plan, buffer);
        if buffer.len() != 1 {
            bail!(span, "did not get a single glyph after shaping {}", text);
        }

        let info = buffer.glyph_infos()[0];
        let pos = buffer.glyph_positions()[0];

        // TODO: add support for coverage and fallback, like in normal text shaping.
        if info.glyph_id == 0 {
            bail!(span, "current font is missing a glyph for {}", text);
        }

        let cluster = info.cluster as usize;
        let c = text[cluster..].chars().next().unwrap();
        let limits = Limits::for_char(c);
        let class = EquationElem::class_in(styles)
            .or_else(|| default_math_class(c))
            .unwrap_or(MathClass::Normal);

        let glyph = Glyph {
            id: info.glyph_id as u16,
            x_advance: font.to_em(pos.x_advance),
            x_offset: font.to_em(pos.x_offset),
            y_advance: font.to_em(pos.y_advance),
            y_offset: font.to_em(pos.y_offset),
            range: 0..text.len().saturating_as(),
            span: (span, 0),
        };

        let item = TextItem {
            font: font.clone(),
            size: TextElem::size_in(styles),
            fill: TextElem::fill_in(styles).as_decoration(),
            stroke: TextElem::stroke_in(styles).map(|s| s.unwrap_or_default()),
            lang: TextElem::lang_in(styles),
            region: TextElem::region_in(styles),
            text: text.into(),
            glyphs: vec![glyph.clone()],
        };

        let mut fragment = Self {
            item,
            base_glyph: glyph,
            // Math
            math_size: EquationElem::size_in(styles),
            class,
            limits,
            mid_stretched: None,
            // Math in need of updating.
            extended_shape: false,
            italics_correction: Abs::zero(),
            accent_attach: (Abs::zero(), Abs::zero()),
            size: Size::zero(),
            baseline: None,
            // Misc
            align: Abs::zero(),
            shift: TextElem::baseline_in(styles),
            modifiers: FrameModifiers::get_in(styles),
        };
        fragment.update_glyph();
        Ok(fragment)
    }

    /// Sets element id and boxes in appropriate way without changing other
    /// styles. This is used to replace the glyph with a stretch variant.
    pub fn update_glyph(&mut self) {
        let id = GlyphId(self.item.glyphs[0].id);

        let extended_shape = is_extended_shape(&self.item.font, id);
        let italics = italics_correction(&self.item.font, id).unwrap_or_default();
        let width = self.item.width();
        if !extended_shape {
            self.item.glyphs[0].x_advance += italics;
        }
        let italics = italics.at(self.item.size);

        let (ascent, descent) =
            ascent_descent(&self.item.font, id).unwrap_or((Em::zero(), Em::zero()));

        // The fallback for accents is half the width plus or minus the italics
        // correction. This is similar to how top and bottom attachments are
        // shifted. For bottom accents we do not use the accent attach of the
        // base as it is meant for top acccents.
        let top_accent_attach = accent_attach(&self.item.font, id)
            .map(|x| x.at(self.item.size))
            .unwrap_or((width + italics) / 2.0);
        let bottom_accent_attach = (width - italics) / 2.0;

        self.baseline = Some(ascent.at(self.item.size));
        self.size = Size::new(
            self.item.width(),
            ascent.at(self.item.size) + descent.at(self.item.size),
        );
        self.italics_correction = italics;
        self.accent_attach = (top_accent_attach, bottom_accent_attach);
        self.extended_shape = extended_shape;
    }

    // Reset a GlyphFragment's text field and math properties back to its
    // base_id's. This is used to return a glyph to its unstretched state.
    pub fn reset_glyph(&mut self) {
        self.align = Abs::zero();
        self.item.glyphs = vec![self.base_glyph.clone()];
        self.update_glyph();
    }

    pub fn baseline(&self) -> Abs {
        self.ascent()
    }

    /// The distance from the baseline to the top of the frame.
    pub fn ascent(&self) -> Abs {
        self.baseline.unwrap_or(self.size.y)
    }

    /// The distance from the baseline to the bottom of the frame.
    pub fn descent(&self) -> Abs {
        self.size.y - self.ascent()
    }

    pub fn into_frame(self) -> Frame {
        let mut frame = Frame::soft(self.size);
        frame.set_baseline(self.baseline());
        frame.push(
            Point::with_y(self.ascent() + self.shift + self.align),
            FrameItem::Text(self.item),
        );
        frame.modify(&self.modifiers);
        frame
    }

    /// Try to stretch a glyph to a desired height.
    pub fn stretch_vertical(&mut self, ctx: &mut MathContext, height: Abs) {
        self.stretch(ctx, height, Axis::Y)
    }

    /// Try to stretch a glyph to a desired width.
    pub fn stretch_horizontal(&mut self, ctx: &mut MathContext, width: Abs) {
        self.stretch(ctx, width, Axis::X)
    }

    /// Try to stretch a glyph to a desired width or height.
    ///
    /// The resulting frame may not have the exact desired width or height.
    pub fn stretch(&mut self, ctx: &mut MathContext, target: Abs, axis: Axis) {
        self.reset_glyph();

        // If the base glyph is good enough, use it.
        let mut advance = self.size.get(axis);
        if axis == Axis::X && !self.extended_shape {
            // For consistency, we subtract the italics correction from the
            // glyph's width if it was added in `update_glyph`.
            advance -= self.italics_correction;
        }
        if target <= advance {
            return;
        }

        let id = GlyphId(self.item.glyphs[0].id);
        let font = self.item.font.clone();
        let Some(construction) = glyph_construction(&font, id, axis) else { return };

        // Search for a pre-made variant with a good advance.
        let mut best_id = id;
        let mut best_advance = advance;
        for variant in construction.variants {
            best_id = variant.variant_glyph;
            best_advance =
                self.item.font.to_em(variant.advance_measurement).at(self.item.size);
            if target <= best_advance {
                break;
            }
        }

        // This is either good or the best we've got.
        if target <= best_advance || construction.assembly.is_none() {
            self.item.glyphs[0].id = best_id.0;
            self.item.glyphs[0].x_advance =
                self.item.font.x_advance(best_id.0).unwrap_or_default();
            self.item.glyphs[0].x_offset = Em::zero();
            self.item.glyphs[0].y_advance =
                self.item.font.y_advance(best_id.0).unwrap_or_default();
            self.item.glyphs[0].y_offset = Em::zero();
            self.update_glyph();
            return;
        }

        // Assemble from parts.
        let assembly = construction.assembly.unwrap();
        let min_overlap = min_connector_overlap(&self.item.font)
            .unwrap_or_default()
            .at(self.item.size);
        assemble(ctx, self, assembly, min_overlap, target, axis);
    }

    /// Vertically adjust the fragment's frame so that it is centered
    /// on the axis.
    pub fn center_on_axis(&mut self) {
        self.align_on_axis(VAlignment::Horizon);
    }

    /// Vertically adjust the fragment's frame so that it is aligned
    /// to the given alignment on the axis.
    pub fn align_on_axis(&mut self, align: VAlignment) {
        let h = self.size.y;
        let axis = axis_height(&self.item.font).unwrap().at(self.item.size);
        self.align += self.baseline();
        self.baseline = Some(align.inv().position(h + axis * 2.0));
        self.align -= self.baseline();
    }
}

impl Debug for GlyphFragment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "GlyphFragment({:?})", self.item.text)
    }
}

#[derive(Debug, Clone)]
pub struct FrameFragment {
    pub frame: Frame,
    pub font_size: Abs,
    pub class: MathClass,
    pub math_size: MathSize,
    pub limits: Limits,
    pub spaced: bool,
    pub base_ascent: Abs,
    pub base_descent: Abs,
    pub italics_correction: Abs,
    pub accent_attach: (Abs, Abs),
    pub text_like: bool,
    pub ignorant: bool,
}

impl FrameFragment {
    pub fn new(styles: StyleChain, frame: Frame) -> Self {
        let base_ascent = frame.ascent();
        let base_descent = frame.descent();
        let accent_attach = frame.width() / 2.0;
        Self {
            frame: frame.modified(&FrameModifiers::get_in(styles)),
            font_size: TextElem::size_in(styles),
            class: EquationElem::class_in(styles).unwrap_or(MathClass::Normal),
            math_size: EquationElem::size_in(styles),
            limits: Limits::Never,
            spaced: false,
            base_ascent,
            base_descent,
            italics_correction: Abs::zero(),
            accent_attach: (accent_attach, accent_attach),
            text_like: false,
            ignorant: false,
        }
    }

    pub fn with_class(self, class: MathClass) -> Self {
        Self { class, ..self }
    }

    pub fn with_limits(self, limits: Limits) -> Self {
        Self { limits, ..self }
    }

    pub fn with_spaced(self, spaced: bool) -> Self {
        Self { spaced, ..self }
    }

    pub fn with_base_ascent(self, base_ascent: Abs) -> Self {
        Self { base_ascent, ..self }
    }

    pub fn with_base_descent(self, base_descent: Abs) -> Self {
        Self { base_descent, ..self }
    }

    pub fn with_italics_correction(self, italics_correction: Abs) -> Self {
        Self { italics_correction, ..self }
    }

    pub fn with_accent_attach(self, accent_attach: (Abs, Abs)) -> Self {
        Self { accent_attach, ..self }
    }

    pub fn with_text_like(self, text_like: bool) -> Self {
        Self { text_like, ..self }
    }

    pub fn with_ignorant(self, ignorant: bool) -> Self {
        Self { ignorant, ..self }
    }
}

fn ascent_descent(font: &Font, id: GlyphId) -> Option<(Em, Em)> {
    let bbox = font.ttf().glyph_bounding_box(id)?;
    Some((font.to_em(bbox.y_max), -font.to_em(bbox.y_min)))
}

/// Look up the italics correction for a glyph.
fn italics_correction(font: &Font, id: GlyphId) -> Option<Em> {
    font.ttf()
        .tables()
        .math?
        .glyph_info?
        .italic_corrections?
        .get(id)
        .map(|value| font.to_em(value.value))
}

/// Loop up the top accent attachment position for a glyph.
fn accent_attach(font: &Font, id: GlyphId) -> Option<Em> {
    font.ttf()
        .tables()
        .math?
        .glyph_info?
        .top_accent_attachments?
        .get(id)
        .map(|value| font.to_em(value.value))
}

/// Look up whether a glyph is an extended shape.
fn is_extended_shape(font: &Font, id: GlyphId) -> bool {
    font.ttf()
        .tables()
        .math
        .and_then(|math| math.glyph_info)
        .and_then(|glyph_info| glyph_info.extended_shapes)
        .and_then(|coverage| coverage.get(id))
        .is_some()
}

/// Look up a kerning value at a specific corner and height.
fn kern_at_height(font: &Font, id: GlyphId, corner: Corner, height: Em) -> Option<Em> {
    let kerns = font.ttf().tables().math?.glyph_info?.kern_infos?.get(id)?;
    let kern = match corner {
        Corner::TopLeft => kerns.top_left,
        Corner::TopRight => kerns.top_right,
        Corner::BottomRight => kerns.bottom_right,
        Corner::BottomLeft => kerns.bottom_left,
    }?;

    let mut i = 0;
    while i < kern.count() && height > font.to_em(kern.height(i)?.value) {
        i += 1;
    }

    Some(font.to_em(kern.kern(i)?.value))
}

fn axis_height(font: &Font) -> Option<Em> {
    Some(font.to_em(font.ttf().tables().math?.constants?.axis_height().value))
}

pub fn stretch_axes(font: &Font, id: u16) -> Axes<bool> {
    let id = GlyphId(id);
    let horizontal = font
        .ttf()
        .tables()
        .math
        .and_then(|math| math.variants)
        .and_then(|variants| variants.horizontal_constructions.get(id))
        .is_some();
    let vertical = font
        .ttf()
        .tables()
        .math
        .and_then(|math| math.variants)
        .and_then(|variants| variants.vertical_constructions.get(id))
        .is_some();

    Axes::new(horizontal, vertical)
}

fn min_connector_overlap(font: &Font) -> Option<Em> {
    font.ttf()
        .tables()
        .math?
        .variants
        .map(|variants| font.to_em(variants.min_connector_overlap))
}

fn glyph_construction(font: &Font, id: GlyphId, axis: Axis) -> Option<GlyphConstruction> {
    font.ttf()
        .tables()
        .math?
        .variants
        .map(|variants| match axis {
            Axis::X => variants.horizontal_constructions,
            Axis::Y => variants.vertical_constructions,
        })?
        .get(id)
}

/// Assemble a glyph from parts.
fn assemble(
    ctx: &mut MathContext,
    base: &mut GlyphFragment,
    assembly: GlyphAssembly,
    min_overlap: Abs,
    target: Abs,
    axis: Axis,
) {
    // Determine the number of times the extenders need to be repeated as well
    // as a ratio specifying how much to spread the parts apart
    // (0 = maximal overlap, 1 = minimal overlap).
    let mut full;
    let mut ratio;
    let mut repeat = 0;
    loop {
        full = Abs::zero();
        ratio = 0.0;

        let mut parts = parts(assembly, repeat).peekable();
        let mut growable = Abs::zero();

        while let Some(part) = parts.next() {
            let mut advance = base.item.font.to_em(part.full_advance).at(base.item.size);
            if let Some(next) = parts.peek() {
                let max_overlap = base
                    .item
                    .font
                    .to_em(part.end_connector_length.min(next.start_connector_length))
                    .at(base.item.size);
                if max_overlap < min_overlap {
                    // This condition happening is indicative of a bug in the
                    // font.
                    ctx.engine.sink.warn(warning!(
                       base.item.glyphs[0].span.0,
                       "glyph has assembly parts with overlap less than minConnectorOverlap";
                       hint: "its rendering may appear broken - this is probably a font bug";
                       hint: "please file an issue at https://github.com/typst/typst/issues"
                    ));
                }

                advance -= max_overlap;
                growable += max_overlap - min_overlap;
            }

            full += advance;
        }

        if full < target {
            let delta = target - full;
            ratio = (delta / growable).min(1.0);
            full += ratio * growable;
        }

        if target <= full || repeat >= MAX_REPEATS {
            break;
        }

        repeat += 1;
    }

    let mut glyphs = vec![];
    let mut parts = parts(assembly, repeat).peekable();
    while let Some(part) = parts.next() {
        let mut advance = base.item.font.to_em(part.full_advance).at(base.item.size);
        if let Some(next) = parts.peek() {
            let max_overlap = base
                .item
                .font
                .to_em(part.end_connector_length.min(next.start_connector_length))
                .at(base.item.size);
            advance -= max_overlap;
            advance += ratio * (max_overlap - min_overlap);
        }
        let (x, y) = match axis {
            Axis::X => (Em::from_length(advance, base.item.size), Em::zero()),
            Axis::Y => (Em::zero(), Em::from_length(advance, base.item.size)),
        };
        glyphs.push(Glyph {
            id: part.glyph_id.0,
            x_advance: x,
            x_offset: Em::zero(),
            y_advance: y,
            y_offset: Em::zero(),
            ..base.item.glyphs[0].clone()
        });
    }

    match axis {
        Axis::X => base.size.x = full,
        Axis::Y => {
            base.baseline = None;
            base.size.y = full;
            base.size.x = glyphs
                .iter()
                .map(|glyph| base.item.font.x_advance(glyph.id).unwrap_or_default())
                .max()
                .unwrap_or_default()
                .at(base.item.size);
        }
    }

    base.item.glyphs = glyphs;
    base.italics_correction = base
        .item
        .font
        .to_em(assembly.italics_correction.value)
        .at(base.item.size);
    if axis == Axis::X {
        base.accent_attach = (full / 2.0, full / 2.0);
    }
    base.mid_stretched = None;
    base.extended_shape = true;
}

/// Return an iterator over the assembly's parts with extenders repeated the
/// specified number of times.
fn parts(assembly: GlyphAssembly, repeat: usize) -> impl Iterator<Item = GlyphPart> + '_ {
    assembly.parts.into_iter().flat_map(move |part| {
        let count = if part.part_flags.extender() { repeat } else { 1 };
        std::iter::repeat_n(part, count)
    })
}

pub fn has_dtls_feat(font: &Font) -> bool {
    font.ttf()
        .tables()
        .gsub
        .and_then(|gsub| gsub.features.index(ttf_parser::Tag::from_bytes(b"dtls")))
        .is_some()
}

/// Describes in which situation a frame should use limits for attachments.
#[derive(Debug, Copy, Clone)]
pub enum Limits {
    /// Always scripts.
    Never,
    /// Display limits only in `display` math.
    Display,
    /// Always limits.
    Always,
}

impl Limits {
    /// The default limit configuration if the given character is the base.
    pub fn for_char(c: char) -> Self {
        match default_math_class(c) {
            Some(MathClass::Large) => {
                if is_integral_char(c) {
                    Self::Never
                } else {
                    Self::Display
                }
            }
            Some(MathClass::Relation) => Self::Always,
            _ => Self::Never,
        }
    }

    /// The default limit configuration for a math class.
    pub fn for_class(class: MathClass) -> Self {
        match class {
            MathClass::Large => Self::Display,
            MathClass::Relation => Self::Always,
            _ => Self::Never,
        }
    }

    /// Whether limits should be displayed in this context.
    pub fn active(&self, styles: StyleChain) -> bool {
        match self {
            Self::Always => true,
            Self::Display => EquationElem::size_in(styles) == MathSize::Display,
            Self::Never => false,
        }
    }
}

/// Determines if the character is one of a variety of integral signs.
fn is_integral_char(c: char) -> bool {
    ('∫'..='∳').contains(&c) || ('⨋'..='⨜').contains(&c)
}
