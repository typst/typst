use bumpalo::boxed::Box as BumpBox;
use bumpalo::Bump;
use once_cell::unsync::Lazy;

use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::{Packed, Resolve, Smart, StyleChain};
use crate::introspection::{Locator, Tag, TagElem};
use crate::layout::{
    layout_frame, Abs, AlignElem, Alignment, Axes, BlockElem, ColbreakElem,
    FixedAlignment, FlushElem, Fr, Fragment, Frame, PagebreakElem, PlaceElem, Ratio,
    Region, Regions, Rel, Size, Spacing, VElem,
};
use crate::model::ParElem;
use crate::realize::Pair;
use crate::text::TextElem;

/// A prepared child in flow layout.
///
/// The larger variants are bump-boxed to keep the enum size down.
pub enum Child<'a> {
    /// An introspection tag.
    Tag(&'a Tag),
    /// Relative spacing with a specific weakness.
    Rel(Rel<Abs>, u8),
    /// Fractional spacing.
    Fr(Fr),
    /// An already layouted line of a paragraph.
    Line(BumpBox<'a, LineChild>),
    /// A potentially breakable block.
    Block(BumpBox<'a, BlockChild<'a>>),
    /// An absolutely or floatingly placed element.
    Placed(BumpBox<'a, PlacedChild<'a>>),
    /// A column break.
    Break(bool),
    /// A place flush.
    Flush,
}

/// Collects all content of the flow into prepared children.
#[typst_macros::time]
pub fn collect<'a>(
    engine: &mut Engine,
    bump: &'a Bump,
    children: &[Pair<'a>],
    locator: Locator<'a>,
    base: Size,
    expand: bool,
) -> SourceResult<Vec<Child<'a>>> {
    let mut locator = locator.split();
    let mut output = Vec::with_capacity(children.len());
    let mut last_was_par = false;

    for &(child, styles) in children {
        if let Some(elem) = child.to_packed::<TagElem>() {
            output.push(Child::Tag(&elem.tag));
        } else if let Some(elem) = child.to_packed::<VElem>() {
            output.push(match elem.amount {
                Spacing::Rel(rel) => {
                    Child::Rel(rel.resolve(styles), elem.weak(styles) as u8)
                }
                Spacing::Fr(fr) => Child::Fr(fr),
            });
        } else if let Some(elem) = child.to_packed::<ColbreakElem>() {
            output.push(Child::Break(elem.weak(styles)));
        } else if let Some(elem) = child.to_packed::<ParElem>() {
            let align = AlignElem::alignment_in(styles).resolve(styles);
            let leading = ParElem::leading_in(styles);
            let spacing = ParElem::spacing_in(styles);
            let costs = TextElem::costs_in(styles);

            let lines = crate::layout::layout_inline(
                engine,
                &elem.children,
                locator.next(&elem.span()),
                styles,
                last_was_par,
                base,
                expand,
            )?
            .into_frames();

            output.push(Child::Rel(spacing.into(), 4));

            // Determine whether to prevent widow and orphans.
            let len = lines.len();
            let prevent_orphans =
                costs.orphan() > Ratio::zero() && len >= 2 && !lines[1].is_empty();
            let prevent_widows =
                costs.widow() > Ratio::zero() && len >= 2 && !lines[len - 2].is_empty();
            let prevent_all = len == 3 && prevent_orphans && prevent_widows;

            // Store the heights of lines at the edges because we'll potentially
            // need these later when `lines` is already moved.
            let height_at = |i| lines.get(i).map(Frame::height).unwrap_or_default();
            let front_1 = height_at(0);
            let front_2 = height_at(1);
            let back_2 = height_at(len.saturating_sub(2));
            let back_1 = height_at(len.saturating_sub(1));

            for (i, frame) in lines.into_iter().enumerate() {
                if i > 0 {
                    output.push(Child::Rel(leading.into(), 5));
                }

                // To prevent widows and orphans, we require enough space for
                // - all lines if it's just three
                // - the first two lines if we're at the first line
                // - the last two lines if we're at the second to last line
                let need = if prevent_all && i == 0 {
                    front_1 + leading + front_2 + leading + back_1
                } else if prevent_orphans && i == 0 {
                    front_1 + leading + front_2
                } else if prevent_widows && i >= 2 && i + 2 == len {
                    back_2 + leading + back_1
                } else {
                    frame.height()
                };

                let child = LineChild { frame, align, need };
                output.push(Child::Line(BumpBox::new_in(child, bump)));
            }

            output.push(Child::Rel(spacing.into(), 4));
            last_was_par = true;
        } else if let Some(elem) = child.to_packed::<BlockElem>() {
            let locator = locator.next(&elem.span());
            let align = AlignElem::alignment_in(styles).resolve(styles);
            let sticky = elem.sticky(styles);
            let rootable = elem.rootable(styles);

            let fallback = Lazy::new(|| ParElem::spacing_in(styles));
            let spacing = |amount| match amount {
                Smart::Auto => Child::Rel((*fallback).into(), 4),
                Smart::Custom(Spacing::Rel(rel)) => Child::Rel(rel.resolve(styles), 3),
                Smart::Custom(Spacing::Fr(fr)) => Child::Fr(fr),
            };

            output.push(spacing(elem.above(styles)));

            let child = BlockChild { align, sticky, rootable, elem, styles, locator };
            output.push(Child::Block(BumpBox::new_in(child, bump)));

            output.push(spacing(elem.below(styles)));
            last_was_par = false;
        } else if let Some(elem) = child.to_packed::<PlaceElem>() {
            let locator = locator.next(&elem.span());
            let float = elem.float(styles);
            let clearance = elem.clearance(styles);
            let delta = Axes::new(elem.dx(styles), elem.dy(styles)).resolve(styles);

            let alignment = elem.alignment(styles);
            let align_x = alignment.map_or(FixedAlignment::Center, |align| {
                align.x().unwrap_or_default().resolve(styles)
            });
            let align_y = alignment.map(|align| align.y().map(|y| y.resolve(styles)));

            match (float, align_y) {
                (true, Smart::Custom(None | Some(FixedAlignment::Center))) => bail!(
                    elem.span(),
                    "floating placement must be `auto`, `top`, or `bottom`"
                ),
                (false, Smart::Auto) => bail!(
                    elem.span(),
                    "automatic positioning is only available for floating placement";
                    hint: "you can enable floating placement with `place(float: true, ..)`"
                ),
                _ => {}
            }

            let child = PlacedChild {
                float,
                clearance,
                delta,
                align_x,
                align_y,
                elem,
                styles,
                locator,
                alignment,
            };
            output.push(Child::Placed(BumpBox::new_in(child, bump)));
        } else if child.is::<FlushElem>() {
            output.push(Child::Flush);
        } else if child.is::<PagebreakElem>() {
            bail!(
                child.span(), "pagebreaks are not allowed inside of containers";
                hint: "try using a `#colbreak()` instead",
            );
        } else {
            bail!(child.span(), "{} is not allowed here", child.func().name());
        }
    }

    Ok(output)
}

/// A child that encapsulates a paragraph line.
pub struct LineChild {
    pub frame: Frame,
    pub align: Axes<FixedAlignment>,
    pub need: Abs,
}

/// A child that encapsulates a prepared block.
pub struct BlockChild<'a> {
    pub align: Axes<FixedAlignment>,
    pub sticky: bool,
    pub rootable: bool,
    elem: &'a Packed<BlockElem>,
    styles: StyleChain<'a>,
    locator: Locator<'a>,
}

impl BlockChild<'_> {
    /// Build the child's frames given regions.
    pub fn layout(
        &self,
        engine: &mut Engine,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let mut fragment =
            self.elem
                .layout(engine, self.locator.relayout(), self.styles, regions)?;

        for frame in &mut fragment {
            frame.post_process(self.styles);
        }

        Ok(fragment)
    }
}

/// A child that encapsulates a prepared placed element.
pub struct PlacedChild<'a> {
    pub float: bool,
    pub clearance: Abs,
    pub delta: Axes<Rel<Abs>>,
    pub align_x: FixedAlignment,
    pub align_y: Smart<Option<FixedAlignment>>,
    elem: &'a Packed<PlaceElem>,
    styles: StyleChain<'a>,
    locator: Locator<'a>,
    alignment: Smart<Alignment>,
}

impl PlacedChild<'_> {
    /// Build the child's frame given the region's base size.
    pub fn layout(&self, engine: &mut Engine, base: Size) -> SourceResult<Frame> {
        let align = self.alignment.unwrap_or_else(|| Alignment::CENTER);
        let aligned = AlignElem::set_alignment(align).wrap();

        let mut frame = layout_frame(
            engine,
            &self.elem.body,
            self.locator.relayout(),
            self.styles.chain(&aligned),
            Region::new(base, Axes::splat(false)),
        )?;

        frame.post_process(self.styles);
        Ok(frame)
    }
}
