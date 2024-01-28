use std::iter::once;

use unicode_math_class::MathClass;

use crate::foundations::{Resolve, StyleChain};
use crate::layout::{Abs, AlignElem, Em, FixedAlignment, Frame, FrameKind, Point, Size};
use crate::math::{
    alignments, scaled_font_size, spacing, AlignmentResult, EquationElem, FrameFragment,
    MathContext, MathFragment, MathParItem, MathSize,
};
use crate::model::ParElem;

use super::fragment::SpacingFragment;

pub const TIGHT_LEADING: Em = Em::new(0.25);

#[derive(Debug, Default, Clone)]
pub struct MathRow(Vec<MathFragment>);

impl MathRow {
    pub fn new(fragments: Vec<MathFragment>) -> Self {
        let iter = fragments.into_iter().peekable();
        let mut last: Option<usize> = None;
        let mut space: Option<MathFragment> = None;
        let mut resolved: Vec<MathFragment> = vec![];

        for mut fragment in iter {
            match fragment {
                // Keep space only if supported by spaced fragments.
                MathFragment::Space(_) => {
                    if last.is_some() {
                        space = Some(fragment);
                    }
                    continue;
                }

                // Explicit spacing disables automatic spacing.
                MathFragment::Spacing(_) => {
                    last = None;
                    space = None;
                    resolved.push(fragment);
                    continue;
                }

                // Alignment points are resolved later.
                MathFragment::Align => {
                    resolved.push(fragment);
                    continue;
                }

                // New line, new things.
                MathFragment::Linebreak => {
                    resolved.push(fragment);
                    space = None;
                    last = None;
                    continue;
                }

                _ => {}
            }

            // Convert variable operators into binary operators if something
            // precedes them and they are not preceded by a operator or comparator.
            if fragment.class() == MathClass::Vary
                && matches!(
                    last.map(|i| resolved[i].class()),
                    Some(
                        MathClass::Normal
                            | MathClass::Alphabetic
                            | MathClass::Closing
                            | MathClass::Fence
                    )
                )
            {
                fragment.set_class(MathClass::Binary);
            }

            // Insert spacing between the last and this item.
            if let Some(i) = last {
                if let Some(s) = spacing(&resolved[i], space.take(), &fragment) {
                    resolved.insert(i + 1, s);
                }
            }

            last = Some(resolved.len());
            resolved.push(fragment);
        }

        Self(resolved)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, MathFragment> {
        self.0.iter()
    }

    /// Extract the sublines of the row.
    ///
    /// It is very unintuitive, but in current state of things, a `MathRow` can
    /// contain several actual rows. That function deconstructs it to "single"
    /// rows. Hopefully this is only a temporary hack.
    pub fn rows(&self) -> Vec<Self> {
        self.0
            .split(|frag| matches!(frag, MathFragment::Linebreak))
            .map(|slice| Self(slice.to_vec()))
            .collect()
    }

    pub fn row_count(&self) -> usize {
        let mut count =
            1 + self.0.iter().filter(|f| matches!(f, MathFragment::Linebreak)).count();

        // A linebreak at the very end does not introduce an extra row.
        if let Some(f) = self.0.last() {
            if matches!(f, MathFragment::Linebreak) {
                count -= 1
            }
        }
        count
    }

    pub fn ascent(&self) -> Abs {
        self.iter().map(MathFragment::ascent).max().unwrap_or_default()
    }

    pub fn descent(&self) -> Abs {
        self.iter().map(MathFragment::descent).max().unwrap_or_default()
    }

    pub fn class(&self) -> MathClass {
        // Predict the class of the output of 'into_fragment'
        if self.0.len() == 1 {
            self.0
                .first()
                .map(|fragment| fragment.class())
                .unwrap_or(MathClass::Normal)
        } else {
            // FrameFragment::new() (inside 'into_fragment' in this branch) defaults
            // to MathClass::Normal for its class.
            MathClass::Normal
        }
    }

    pub fn into_frame(self, ctx: &MathContext, styles: StyleChain) -> Frame {
        let align = AlignElem::alignment_in(styles).resolve(styles).x;
        self.into_aligned_frame(ctx, styles, &[], align)
    }

    pub fn into_fragment(self, ctx: &MathContext, styles: StyleChain) -> MathFragment {
        if self.0.len() == 1 {
            self.0.into_iter().next().unwrap()
        } else {
            FrameFragment::new(ctx, styles, self.into_frame(ctx, styles)).into()
        }
    }

    pub fn into_aligned_frame(
        self,
        ctx: &MathContext,
        styles: StyleChain,
        points: &[Abs],
        align: FixedAlignment,
    ) -> Frame {
        if !self.iter().any(|frag| matches!(frag, MathFragment::Linebreak)) {
            return self.into_line_frame(points, align);
        }

        let leading = if EquationElem::size_in(styles) >= MathSize::Text {
            ParElem::leading_in(styles)
        } else {
            let font_size = scaled_font_size(ctx, styles);
            TIGHT_LEADING.at(font_size)
        };

        let mut rows: Vec<_> = self.rows();

        if matches!(rows.last(), Some(row) if row.0.is_empty()) {
            rows.pop();
        }

        let AlignmentResult { points, width } = alignments(&rows);
        let mut frame = Frame::soft(Size::zero());

        for (i, row) in rows.into_iter().enumerate() {
            let sub = row.into_line_frame(&points, align);
            let size = frame.size_mut();
            if i > 0 {
                size.y += leading;
            }

            let mut pos = Point::with_y(size.y);
            if points.is_empty() {
                pos.x = align.position(width - sub.width());
            }
            size.y += sub.height();
            size.x.set_max(sub.width());
            frame.push_frame(pos, sub);
        }

        frame
    }

    fn into_line_frame(self, points: &[Abs], align: FixedAlignment) -> Frame {
        let ascent = self.ascent();
        let mut frame = Frame::soft(Size::new(Abs::zero(), ascent + self.descent()));
        frame.set_baseline(ascent);

        let mut next_x = {
            let mut widths = Vec::new();
            if !points.is_empty() && align != FixedAlignment::Start {
                let mut width = Abs::zero();
                for fragment in self.iter() {
                    if matches!(fragment, MathFragment::Align) {
                        widths.push(width);
                        width = Abs::zero();
                    } else {
                        width += fragment.width();
                    }
                }
                widths.push(width);
            }
            let widths = widths;

            let mut prev_points = once(Abs::zero()).chain(points.iter().copied());
            let mut point_widths = points.iter().copied().zip(widths);
            let mut alternator = LeftRightAlternator::Right;
            move || match align {
                FixedAlignment::Start => prev_points.next(),
                FixedAlignment::End => {
                    point_widths.next().map(|(point, width)| point - width)
                }
                _ => point_widths
                    .next()
                    .zip(prev_points.next())
                    .zip(alternator.next())
                    .map(|(((point, width), prev_point), alternator)| match alternator {
                        LeftRightAlternator::Left => prev_point,
                        LeftRightAlternator::Right => point - width,
                    }),
            }
        };
        let mut x = next_x().unwrap_or_default();

        for fragment in self.0.into_iter() {
            if matches!(fragment, MathFragment::Align) {
                x = next_x().unwrap_or(x);
                continue;
            }

            let y = ascent - fragment.ascent();
            let pos = Point::new(x, y);
            x += fragment.width();
            frame.push_frame(pos, fragment.into_frame());
        }

        frame.size_mut().x = x;
        frame
    }

    pub fn into_par_items(self) -> Vec<MathParItem> {
        let mut items = vec![];

        let mut x = Abs::zero();
        let mut ascent = Abs::zero();
        let mut descent = Abs::zero();
        let mut frame = Frame::new(Size::zero(), FrameKind::Soft);
        let mut empty = true;

        let finalize_frame = |frame: &mut Frame, x, ascent, descent| {
            frame.set_size(Size::new(x, ascent + descent));
            frame.set_baseline(Abs::zero());
            frame.translate(Point::with_y(ascent));
        };

        let mut space_is_visible = false;

        let is_relation = |f: &MathFragment| matches!(f.class(), MathClass::Relation);
        let is_space = |f: &MathFragment| {
            matches!(f, MathFragment::Space(_) | MathFragment::Spacing(_))
        };

        let mut iter = self.0.into_iter().peekable();
        while let Some(fragment) = iter.next() {
            if space_is_visible {
                match fragment {
                    MathFragment::Space(width)
                    | MathFragment::Spacing(SpacingFragment { width, .. }) => {
                        items.push(MathParItem::Space(width));
                        continue;
                    }
                    _ => {}
                }
            }

            let class = fragment.class();
            let y = fragment.ascent();

            ascent.set_max(y);
            descent.set_max(fragment.descent());

            let pos = Point::new(x, -y);
            x += fragment.width();
            frame.push_frame(pos, fragment.into_frame());
            empty = false;

            if class == MathClass::Binary
                || (class == MathClass::Relation
                    && !iter.peek().map(is_relation).unwrap_or_default())
            {
                let mut frame_prev = std::mem::replace(
                    &mut frame,
                    Frame::new(Size::zero(), FrameKind::Soft),
                );

                finalize_frame(&mut frame_prev, x, ascent, descent);
                items.push(MathParItem::Frame(frame_prev));
                empty = true;

                x = Abs::zero();
                ascent = Abs::zero();
                descent = Abs::zero();

                space_is_visible = true;
                if let Some(f_next) = iter.peek() {
                    if !is_space(f_next) {
                        items.push(MathParItem::Space(Abs::zero()));
                    }
                }
            } else {
                space_is_visible = false;
            }
        }

        // Don't use `frame.is_empty()` because even an empty frame can
        // contribute width (if it had hidden content).
        if !empty {
            finalize_frame(&mut frame, x, ascent, descent);
            items.push(MathParItem::Frame(frame));
        }

        items
    }
}

impl<T: Into<MathFragment>> From<T> for MathRow {
    fn from(fragment: T) -> Self {
        Self(vec![fragment.into()])
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum LeftRightAlternator {
    Left,
    Right,
}

impl Iterator for LeftRightAlternator {
    type Item = LeftRightAlternator;

    fn next(&mut self) -> Option<Self::Item> {
        let r = Some(*self);
        match self {
            Self::Left => *self = Self::Right,
            Self::Right => *self = Self::Left,
        }
        r
    }
}
