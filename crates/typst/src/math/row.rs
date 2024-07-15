use std::iter::once;

use unicode_math_class::MathClass;

use crate::foundations::{Resolve, StyleChain};
use crate::layout::{Abs, AlignElem, Em, Frame, InlineItem, Point, Size};
use crate::math::{
    alignments, scaled_font_size, spacing, EquationElem, FrameFragment, MathContext,
    MathFragment, MathSize,
};
use crate::model::ParElem;

use super::fragment::SpacingFragment;

pub const TIGHT_LEADING: Em = Em::new(0.25);

/// A linear collection of [`MathFragment`]s.
#[derive(Debug, Default, Clone)]
pub struct MathRun(Vec<MathFragment>);

impl MathRun {
    /// Takes the given [`MathFragment`]s and do some basic processing.
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

            // Insert spacing between the last and this non-ignorant item.
            if !fragment.is_ignorant() {
                if let Some(i) = last {
                    if let Some(s) = spacing(&resolved[i], space.take(), &fragment) {
                        resolved.insert(i + 1, s);
                    }
                }

                last = Some(resolved.len());
            }

            resolved.push(fragment);
        }

        Self(resolved)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, MathFragment> {
        self.0.iter()
    }

    /// Split by linebreaks, and copy [`MathFragment`]s into rows.
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
        self.iter()
            .filter(|e| affects_row_height(e))
            .map(|e| e.ascent())
            .max()
            .unwrap_or_default()
    }

    pub fn descent(&self) -> Abs {
        self.iter()
            .filter(|e| affects_row_height(e))
            .map(|e| e.descent())
            .max()
            .unwrap_or_default()
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
        if !self.is_multiline() {
            self.into_line_frame(&[], LeftRightAlternator::Right)
        } else {
            self.multiline_frame_builder(ctx, styles).build()
        }
    }

    pub fn into_fragment(self, ctx: &MathContext, styles: StyleChain) -> MathFragment {
        if self.0.len() == 1 {
            return self.0.into_iter().next().unwrap();
        }

        // Fragments without a math_size are ignored: the notion of size do not
        // apply to them, so their text-likeness is meaningless.
        let text_like = self
            .iter()
            .filter(|e| e.math_size().is_some())
            .all(|e| e.is_text_like());

        FrameFragment::new(ctx, styles, self.into_frame(ctx, styles))
            .with_text_like(text_like)
            .into()
    }

    /// Returns a builder that lays out the [`MathFragment`]s into a possibly
    /// multi-row [`Frame`]. The rows are aligned using the same set of alignment
    /// points computed from them as a whole.
    pub fn multiline_frame_builder(
        self,
        ctx: &MathContext,
        styles: StyleChain,
    ) -> MathRunFrameBuilder {
        let rows: Vec<_> = self.rows();
        let row_count = rows.len();
        let alignments = alignments(&rows);

        let leading = if EquationElem::size_in(styles) >= MathSize::Text {
            ParElem::leading_in(styles)
        } else {
            let font_size = scaled_font_size(ctx, styles);
            TIGHT_LEADING.at(font_size)
        };

        let align = AlignElem::alignment_in(styles).resolve(styles).x;
        let mut frames: Vec<(Frame, Point)> = vec![];
        let mut size = Size::zero();
        for (i, row) in rows.into_iter().enumerate() {
            if i == row_count - 1 && row.0.is_empty() {
                continue;
            }

            let sub = row.into_line_frame(&alignments.points, LeftRightAlternator::Right);
            if i > 0 {
                size.y += leading;
            }

            let mut pos = Point::with_y(size.y);
            if alignments.points.is_empty() {
                pos.x = align.position(alignments.width - sub.width());
            }
            size.x.set_max(sub.width());
            size.y += sub.height();
            frames.push((sub, pos));
        }

        MathRunFrameBuilder { size, frames }
    }

    /// Lay out [`MathFragment`]s into a one-row [`Frame`], using the
    /// caller-provided alignment points.
    pub fn into_line_frame(
        self,
        points: &[Abs],
        mut alternator: LeftRightAlternator,
    ) -> Frame {
        let ascent = self.ascent();
        let mut frame = Frame::soft(Size::new(Abs::zero(), ascent + self.descent()));
        frame.set_baseline(ascent);

        let mut next_x = {
            let widths: Vec<Abs> = if points.is_empty() {
                vec![]
            } else {
                self.iter()
                    .as_slice()
                    .split(|e| matches!(e, MathFragment::Align))
                    .map(|chunk| chunk.iter().map(|e| e.width()).sum())
                    .collect()
            };

            let mut prev_points = once(Abs::zero()).chain(points.iter().copied());
            let mut point_widths = points.iter().copied().zip(widths);
            move || {
                point_widths
                    .next()
                    .zip(prev_points.next())
                    .zip(alternator.next())
                    .map(|(((point, width), prev_point), alternator)| match alternator {
                        LeftRightAlternator::Right => point - width,
                        _ => prev_point,
                    })
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

    pub fn into_par_items(self) -> Vec<InlineItem> {
        let mut items = vec![];

        let mut x = Abs::zero();
        let mut ascent = Abs::zero();
        let mut descent = Abs::zero();
        let mut frame = Frame::soft(Size::zero());
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
                        items.push(InlineItem::Space(width, true));
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
                let mut frame_prev =
                    std::mem::replace(&mut frame, Frame::soft(Size::zero()));

                finalize_frame(&mut frame_prev, x, ascent, descent);
                items.push(InlineItem::Frame(frame_prev));
                empty = true;

                x = Abs::zero();
                ascent = Abs::zero();
                descent = Abs::zero();

                space_is_visible = true;
                if let Some(f_next) = iter.peek() {
                    if !is_space(f_next) {
                        items.push(InlineItem::Space(Abs::zero(), true));
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
            items.push(InlineItem::Frame(frame));
        }

        items
    }

    fn is_multiline(&self) -> bool {
        self.iter().any(|frag| matches!(frag, MathFragment::Linebreak))
    }
}

impl<T: Into<MathFragment>> From<T> for MathRun {
    fn from(fragment: T) -> Self {
        Self(vec![fragment.into()])
    }
}

/// An iterator that alternates between the `Left` and `Right` values, if the
/// initial value is not `None`.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum LeftRightAlternator {
    None,
    Left,
    Right,
}

impl Iterator for LeftRightAlternator {
    type Item = LeftRightAlternator;

    fn next(&mut self) -> Option<Self::Item> {
        let r = Some(*self);
        match self {
            Self::None => {}
            Self::Left => *self = Self::Right,
            Self::Right => *self = Self::Left,
        }
        r
    }
}

/// How the rows from the [`MathRun`] should be aligned and merged into a [`Frame`].
pub struct MathRunFrameBuilder {
    /// The size of the resulting frame.
    pub size: Size,
    /// Each row's frame, and the position where the frame should
    /// be pushed into the resulting frame.
    pub frames: Vec<(Frame, Point)>,
}

impl MathRunFrameBuilder {
    /// Consumes the builder and returns a [`Frame`].
    pub fn build(self) -> Frame {
        let mut frame = Frame::soft(self.size);
        for (sub, pos) in self.frames.into_iter() {
            frame.push_frame(pos, sub);
        }
        frame
    }
}

fn affects_row_height(fragment: &MathFragment) -> bool {
    !matches!(fragment, MathFragment::Align | MathFragment::Linebreak)
}
