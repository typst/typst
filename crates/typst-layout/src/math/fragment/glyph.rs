use std::fmt::{self, Debug, Formatter};

use comemo::Tracked;
use ecow::EcoString;
use ttf_parser::GlyphId;
use ttf_parser::math::{GlyphAssembly, GlyphConstruction, GlyphPart};
use typst_library::World;
use typst_library::diag::warning;
use typst_library::engine::Engine;
use typst_library::foundations::StyleChain;
use typst_library::layout::{
    Abs, Axes, Axis, Corner, Em, Frame, FrameItem, Point, Size, VAlignment,
};
use typst_library::math::ir::{MathProperties, Stretch};
use typst_library::math::{EquationElem, MathSize};
use typst_library::text::{FontInstance, Glyph, TextElem, TextItem, features};
use typst_syntax::Span;
use typst_utils::{Get, default_math_class};
use unicode_math_class::MathClass;

use crate::math::shaping;
use crate::modifiers::{FrameModifiers, FrameModify};

/// Maximum number of times extenders can be repeated.
const MAX_REPEATS: usize = 1024;

#[derive(Clone)]
pub struct GlyphFragment {
    // Text stuff.
    pub(super) item: TextItem,
    // Math stuff.
    pub(super) size: Size,
    baseline: Option<Abs>,
    pub(super) italics_correction: Abs,
    pub(super) accent_attach: (Abs, Abs),
    pub(super) math_size: MathSize,
    pub class: MathClass,
    pub(super) extended_shape: bool,
    pub(super) stretchable_axes: Axes<bool>,
    // External frame stuff.
    modifiers: FrameModifiers,
    shift: Abs,
    align: Abs,
}

impl GlyphFragment {
    /// Creates a synthetic glyph fragment from the given character.
    pub fn synthetic(
        engine: &mut Engine,
        styles: StyleChain,
        c: char,
        span: Span,
    ) -> Option<Self> {
        let class = default_math_class(c).unwrap_or(MathClass::Normal);
        let math_size = styles.get(EquationElem::size);
        Self::base(
            engine.world,
            styles,
            &features(styles),
            c.encode_utf8(&mut [0; 4]),
            class,
            math_size,
        )
        .map(|glyph| glyph.with_span(span))
    }

    /// Creates a glyph fragment from the given text, stretching it if needed.
    pub fn new(
        engine: &mut Engine,
        text: &str,
        stretch: &Stretch,
        styles: StyleChain,
        props: &MathProperties,
    ) -> Option<GlyphFragment> {
        let PlannedGlyph { mut glyph, action } = Self::planned(
            engine.world,
            styles,
            text,
            props.class(),
            props.size,
            *stretch,
        )?
        .with_span(props.span);

        match action {
            Action::Stretch { axis, target, short_fall } => {
                glyph.stretch(engine, target, short_fall, axis);
                if axis == Axis::Y {
                    glyph.center_on_axis();
                }
            }
            Action::WarnBothAxes => {
                // As far as we know, there aren't any glyphs that have both
                // vertical and horizontal constructions. So for the time
                // being, we will assume that a glyph cannot have both.
                engine.sink.warn(warning!(
                   props.span,
                   "glyph has both vertical and horizontal constructions";
                   hint: "this is probably a font bug";
                   hint: "please file an issue at https://github.com/typst/typst/issues";
                ));
            }
            Action::Keep | Action::Fallback => {}
        }

        Some(glyph)
    }

    #[comemo::memoize]
    fn planned(
        world: Tracked<dyn World + '_>,
        styles: StyleChain,
        text: &str,
        class: MathClass,
        math_size: MathSize,
        stretch: Stretch,
    ) -> Option<PlannedGlyph> {
        let features = features(styles);
        let shape = |feats: &[rustybuzz::Feature]| {
            Self::base(world, styles, feats, text, class, math_size)
        };

        let mut glyph = shape(&features)?;
        let mut action = decide(&glyph, &stretch);

        // If the initial glyph isn't sufficient, keep retrying shaping, by
        // removing `ssty` and `flac` features, until one satisfies the stretch
        // requirements (or until we exhaust them all, in which case we keep
        // the original glyph, unstretched)
        if matches!(action, Action::Fallback) {
            shaping::feat_fallback(features, |feats| {
                let Some(new) = shape(feats) else { return false };
                match decide(&new, &stretch) {
                    Action::Fallback => false,
                    other => {
                        glyph = new;
                        action = other;
                        true
                    }
                }
            });
        }

        Some(PlannedGlyph { glyph, action })
    }

    #[comemo::memoize]
    fn base(
        world: Tracked<dyn World + '_>,
        styles: StyleChain,
        features: &[rustybuzz::Feature],
        text: &str,
        class: MathClass,
        math_size: MathSize,
    ) -> Option<GlyphFragment> {
        let shaped = shaping::shape(world, styles, features, text)?;
        Some(Self::from_shaped(styles, text.into(), class, math_size, shaped))
    }

    /// Construct a glyph fragment from the shaped text.
    fn from_shaped(
        styles: StyleChain,
        text: EcoString,
        class: MathClass,
        math_size: MathSize,
        shaped: (FontInstance, Vec<Glyph>),
    ) -> GlyphFragment {
        let (font, glyphs) = shaped;
        let stretchable_axes = stretch_axes(&font, glyphs[0].id);

        let item = TextItem {
            text,
            font,
            size: styles.resolve(TextElem::size),
            fill: styles.get_ref(TextElem::fill).as_decoration(),
            stroke: styles.resolve(TextElem::stroke).map(|s| s.unwrap_or_default()),
            lang: styles.get(TextElem::lang),
            region: styles.get(TextElem::region),
            glyphs,
        };

        let mut fragment = Self {
            item,
            // Math
            math_size,
            class,
            stretchable_axes,
            // Math in need of updating.
            extended_shape: false,
            italics_correction: Abs::zero(),
            accent_attach: (Abs::zero(), Abs::zero()),
            size: Size::zero(),
            baseline: None,
            // Misc
            align: Abs::zero(),
            shift: styles.resolve(TextElem::baseline),
            modifiers: FrameModifiers::get_in(styles),
        };
        fragment.update_glyph();
        fragment
    }

    fn with_span(mut self, span: Span) -> Self {
        for glyph in &mut self.item.glyphs {
            glyph.span = (span, 0);
        }
        self
    }

    /// Sets element id and boxes in appropriate way without changing other
    /// styles. This is used to replace the glyph with a stretch variant.
    fn update_glyph(&mut self) {
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

    fn baseline(&self) -> Abs {
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

    pub(super) fn into_frame(self) -> Frame {
        let mut frame = Frame::soft(self.size);
        frame.set_baseline(self.baseline());
        frame.push(
            Point::with_y(self.ascent() + self.shift + self.align),
            FrameItem::Text(self.item),
        );
        frame.modify(&self.modifiers);
        frame
    }

    /// Try to stretch a glyph to a desired width or height.
    ///
    /// The resulting frame may not have the exact desired width or height.
    fn stretch(&mut self, engine: &mut Engine, target: Abs, short_fall: Abs, axis: Axis) {
        // If the base glyph is good enough, use it.
        let advance = self.stretch_advance(axis);
        let short_target = target - short_fall;
        if short_target <= advance {
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
            if short_target <= best_advance {
                break;
            }
        }

        // This is either good or the best we've got.
        if short_target <= best_advance || construction.assembly.is_none() {
            self.item.glyphs = vec![Glyph {
                id: best_id.0,
                x_advance: self.item.font.x_advance(best_id.0).unwrap_or_default(),
                x_offset: Em::zero(),
                y_advance: self.item.font.y_advance(best_id.0).unwrap_or_default(),
                y_offset: Em::zero(),
                range: self.item.glyphs[0].range.clone(),
                span: self.item.glyphs[0].span,
            }];
            self.update_glyph();
            return;
        }

        // Assemble from parts.
        let assembly = construction.assembly.unwrap();
        let min_overlap = min_connector_overlap(&self.item.font)
            .unwrap_or_default()
            .at(self.item.size);
        assemble(engine, self, assembly, min_overlap, target, axis);
    }

    /// Advance of the glyph along the given axis used during stretching.
    fn stretch_advance(&self, axis: Axis) -> Abs {
        let mut advance = self.size.get(axis);
        if axis == Axis::X && !self.extended_shape {
            // For consistency, we subtract the italics correction from the
            // glyph's width if it was added in `update_glyph`.
            advance -= self.italics_correction;
        }
        advance
    }

    /// Vertically adjust the fragment's frame so that it is centered
    /// on the axis.
    pub fn center_on_axis(&mut self) {
        self.align_on_axis(VAlignment::Horizon);
    }

    /// Vertically adjust the fragment's frame so that it is aligned
    /// to the given alignment on the axis.
    fn align_on_axis(&mut self, align: VAlignment) {
        let h = self.size.y;
        let axis = self.item.font.math().axis_height.at(self.item.size);
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

#[derive(Clone)]
struct PlannedGlyph {
    glyph: GlyphFragment,
    action: Action,
}

impl PlannedGlyph {
    fn with_span(mut self, span: Span) -> Self {
        self.glyph = self.glyph.with_span(span);
        self
    }
}

/// How to proceed with the freshly-shaped glyph fragment.
#[derive(Clone, Copy)]
enum Action {
    /// Use this glyph as-is (no stretching is needed).
    Keep,
    /// Use this glyph and stretch it with the given information.
    Stretch { axis: Axis, target: Abs, short_fall: Abs },
    /// Use this glyph, but warn that it claims stretchability on both axes.
    WarnBothAxes,
    /// The glyph isn't stretchable on the axes we need and isn't wide enough
    /// to skip stretching (try falling back without some features).
    Fallback,
}

/// Decide how to proceed with the freshly-shaped glyph fragment based on the
/// stretch required.
fn decide(glyph: &GlyphFragment, stretch: &Stretch) -> Action {
    /// The glyph's status for the stretch request on a single axis.
    enum AxisStatus {
        /// No stretching needed (none requested, or the glyph is already wide
        /// enough).
        Sufficient,
        /// The glyph is stretchable on this axis.
        Stretchable { target: Abs, short_fall: Abs },
        /// Stretching is needed but the glyph isn't stretchable and isn't wide
        /// enough (fallback and retry).
        Fallback,
    }

    let font = &glyph.item.font;
    let id = glyph.item.glyphs[0].id;
    let axes = glyph.stretchable_axes;

    let assess = |axis| {
        let Some((target, short_fall)) = resolve_stretch(glyph, stretch, axis) else {
            return AxisStatus::Sufficient;
        };

        if axes.get(axis) {
            return AxisStatus::Stretchable { target, short_fall };
        }
        // Glyph isn't stretchable on this axis, but might be wide enough.

        let mut advance = glyph.stretch_advance(axis);

        // Combining marks (e.g. accent glyphs) typically have zero advance in
        // `hmtx`, so the advance above is no good. We explicitly compute the
        // bounding box, just in case.
        if let Some(bbox) = font.ttf().glyph_bounding_box(GlyphId(id)) {
            let extents = Axes::new(
                font.to_em(bbox.x_max - bbox.x_min),
                font.to_em(bbox.y_max - bbox.y_min),
            );
            advance.set_max(extents.get(axis).at(glyph.item.size));
        }

        if target - short_fall <= advance {
            AxisStatus::Sufficient
        } else {
            AxisStatus::Fallback
        }
    };

    match (assess(Axis::X), assess(Axis::Y)) {
        (AxisStatus::Stretchable { .. }, AxisStatus::Stretchable { .. }) => {
            Action::WarnBothAxes
        }
        (AxisStatus::Stretchable { target, short_fall }, _) => {
            Action::Stretch { axis: Axis::X, target, short_fall }
        }
        (_, AxisStatus::Stretchable { target, short_fall }) => {
            Action::Stretch { axis: Axis::Y, target, short_fall }
        }
        (AxisStatus::Sufficient, AxisStatus::Sufficient) => Action::Keep,
        _ => Action::Fallback,
    }
}

/// Returns the absolute target and short fall of the stretch along the given
/// axis, if it exists.
fn resolve_stretch(
    glyph: &GlyphFragment,
    stretch: &Stretch,
    axis: Axis,
) -> Option<(Abs, Abs)> {
    let stretch = stretch.resolve(axis)?;
    let relative_to_size = stretch.relative_to.unwrap_or_else(|| {
        if axis == Axis::Y
            && glyph.class == MathClass::Large
            && glyph.math_size == MathSize::Display
        {
            glyph.item.font.math().display_operator_min_height.at(glyph.item.size)
        } else {
            glyph.size.get(axis)
        }
    });

    let target = stretch.target.relative_to(relative_to_size);
    let short_fall = stretch.short_fall.at(stretch.font_size.unwrap_or(glyph.item.size));
    Some((target, short_fall))
}

fn ascent_descent(font: &FontInstance, id: GlyphId) -> Option<(Em, Em)> {
    let bbox = font.ttf().glyph_bounding_box(id)?;
    Some((font.to_em(bbox.y_max), -font.to_em(bbox.y_min)))
}

/// Look up the italics correction for a glyph.
fn italics_correction(font: &FontInstance, id: GlyphId) -> Option<Em> {
    font.ttf()
        .tables()
        .math?
        .glyph_info?
        .italic_corrections?
        .get(id)
        .map(|value| font.to_em(value.value))
}

/// Loop up the top accent attachment position for a glyph.
fn accent_attach(font: &FontInstance, id: GlyphId) -> Option<Em> {
    font.ttf()
        .tables()
        .math?
        .glyph_info?
        .top_accent_attachments?
        .get(id)
        .map(|value| font.to_em(value.value))
}

/// Look up whether a glyph is an extended shape.
fn is_extended_shape(font: &FontInstance, id: GlyphId) -> bool {
    font.ttf()
        .tables()
        .math
        .and_then(|math| math.glyph_info)
        .and_then(|glyph_info| glyph_info.extended_shapes)
        .and_then(|coverage| coverage.get(id))
        .is_some()
}

/// Look up a kerning value at a specific corner and height.
pub(super) fn kern_at_height(
    font: &FontInstance,
    id: GlyphId,
    corner: Corner,
    height: Em,
) -> Option<Em> {
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

fn stretch_axes(font: &FontInstance, id: u16) -> Axes<bool> {
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

fn min_connector_overlap(font: &FontInstance) -> Option<Em> {
    font.ttf()
        .tables()
        .math?
        .variants
        .map(|variants| font.to_em(variants.min_connector_overlap))
}

fn glyph_construction(
    font: &FontInstance,
    id: GlyphId,
    axis: Axis,
) -> Option<GlyphConstruction<'_>> {
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
    engine: &mut Engine,
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
                    engine.sink.warn(warning!(
                       base.item.glyphs[0].span.0,
                       "glyph has assembly parts with overlap less than minConnectorOverlap";
                       hint: "its rendering may appear broken - this is probably a font bug";
                       hint: "please file an issue at https://github.com/typst/typst/issues";
                    ));
                }

                advance -= max_overlap;
                // In case we have that max_overlap < min_overlap, ensure we
                // don't decrease the value of growable.
                growable += (max_overlap - min_overlap).max(Abs::zero());
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
        let (x_advance, y_advance, y_offset) = match axis {
            Axis::X => (Em::from_abs(advance, base.item.size), Em::zero(), Em::zero()),
            Axis::Y => (
                Em::zero(),
                Em::from_abs(advance, base.item.size),
                // Glyph parts used in vertical assemblies are typically aligned
                // at the vertical origin. This way, they combine properly when
                // drawn consecutively, as required by the MATH table spec.
                //
                // However, in some fonts, they aren't. To still have them align
                // properly, we are vertically offsetting such glyphs by their
                // bounding-box computed descent. (Positive descent means that
                // a glyph extends below the baseline and then we must move it
                // up for it to align properly. `y_advance` is Y-up, so that
                // matches up.)
                ascent_descent(&base.item.font, part.glyph_id)
                    .map(|x| x.1)
                    .unwrap_or_default(),
            ),
        };
        glyphs.push(Glyph {
            id: part.glyph_id.0,
            x_advance,
            x_offset: Em::zero(),
            y_advance,
            y_offset,
            range: base.item.glyphs[0].range.clone(),
            span: base.item.glyphs[0].span,
        });
    }

    match axis {
        Axis::X => {
            base.size.x = full;
            let (ascent, descent) = glyphs
                .iter()
                .filter_map(|glyph| ascent_descent(&base.item.font, GlyphId(glyph.id)))
                .reduce(|(ma, md), (a, d)| (ma.max(a), md.max(d)))
                .unwrap_or((Em::zero(), Em::zero()));
            base.baseline = Some(ascent.at(base.item.size));
            base.size.y = (ascent + descent).at(base.item.size);
        }
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
    base.extended_shape = true;
}

/// Return an iterator over the assembly's parts with extenders repeated the
/// specified number of times.
fn parts(
    assembly: GlyphAssembly<'_>,
    repeat: usize,
) -> impl Iterator<Item = GlyphPart> + '_ {
    assembly.parts.into_iter().flat_map(move |part| {
        let count = if part.part_flags.extender() { repeat } else { 1 };
        std::iter::repeat_n(part, count)
    })
}
