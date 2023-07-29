use std::iter::once;

use crate::layout::AlignElem;

use super::*;

pub const TIGHT_LEADING: Em = Em::new(0.25);

#[derive(Debug, Default, Clone)]
pub struct MathRow(Vec<(MathFragment, GroupRole)>);

impl MathRow {
    pub fn new(fragments: Vec<(MathFragment, GroupRole)>) -> Self {
        let iter = fragments.into_iter().peekable();
        let mut last: Option<usize> = None;
        let mut space: Option<MathFragment> = None;
        let mut resolved: Vec<(MathFragment, GroupRole)> = vec![];

        for (mut fragment, role) in iter {
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
                    resolved.push((fragment, role));
                    continue;
                }

                // Alignment points are resolved later.
                MathFragment::Align => {
                    resolved.push((fragment, role));
                    continue;
                }

                // New line, new things.
                MathFragment::Linebreak => {
                    resolved.push((fragment, role));
                    space = None;
                    last = None;
                    continue;
                }

                _ => {}
            }

            // Convert variable operators into binary operators if something
            // precedes them and they are not preceded by a operator or comparator.
            if fragment.class() == Some(MathClass::Vary)
                && matches!(
                    last.and_then(|i| resolved[i].0.class()),
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
                if let Some(s) = spacing(&resolved[i].0, space.take(), &fragment) {
                    resolved.insert(i + 1, (s, GroupRole::Inner));
                }
            }

            last = Some(resolved.len());
            resolved.push((fragment, role));
        }

        Self(resolved)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, (MathFragment, GroupRole)> {
        self.0.iter()
    }

    /// Extract the sublines of the row.
    ///
    /// It is very unintuitive, but in current state of things, a `MathRow` can
    /// contain several actual rows. That function deconstructs it to "single"
    /// rows. Hopefully this is only a temporary hack.
    pub fn rows(&self) -> Vec<Self> {
        self.0
            .split(|frag| matches!(frag.0, MathFragment::Linebreak))
            .map(|slice| Self(slice.to_vec()))
            .collect()
    }

    pub fn row_count(&self) -> usize {
        let mut count = 1 + self
            .0
            .iter()
            .filter(|f| matches!(f.0, MathFragment::Linebreak))
            .count();

        // A linebreak at the very end does not introduce an extra row.
        if let Some(f) = self.0.last() {
            if matches!(f.0, MathFragment::Linebreak) {
                count -= 1
            }
        }
        count
    }

    pub fn ascent(&self) -> Abs {
        self.iter()
            .map(|item| &item.0)
            .map(MathFragment::ascent)
            .max()
            .unwrap_or_default()
    }

    pub fn descent(&self) -> Abs {
        self.iter()
            .map(|item| &item.0)
            .map(MathFragment::descent)
            .max()
            .unwrap_or_default()
    }

    pub fn class(&self) -> MathClass {
        // Predict the class of the output of 'into_fragment'
        if self.0.len() == 1 {
            self.0
                .first()
                .and_then(|fragment| fragment.0.class())
                .unwrap_or(MathClass::Special)
        } else {
            // FrameFragment::new() (inside 'into_fragment' in this branch) defaults
            // to MathClass::Normal for its class.
            MathClass::Normal
        }
    }

    pub fn into_frame(self, ctx: &MathContext) -> Frame {
        let styles = ctx.styles();
        let align = AlignElem::alignment_in(styles).x.resolve(styles);
        self.into_aligned_frame(ctx, &[], align)
    }

    pub fn into_fragment(self, ctx: &MathContext) -> MathFragment {
        if self.0.len() == 1 {
            self.0.into_iter().next().unwrap().0
        } else {
            FrameFragment::new(ctx, self.into_frame(ctx)).into()
        }
    }

    pub fn into_aligned_frame(
        self,
        ctx: &MathContext,
        points: &[Abs],
        align: Align,
    ) -> Frame {
        if self.iter().any(|frag| matches!(frag.0, MathFragment::Linebreak)) {
            let leading = if ctx.style.size >= MathSize::Text {
                ParElem::leading_in(ctx.styles())
            } else {
                TIGHT_LEADING.scaled(ctx)
            };

            let mut rows: Vec<_> = self.rows();

            if matches!(rows.last(), Some(row) if row.0.is_empty()) {
                rows.pop();
            }

            let AlignmentResult { points, width } = alignments(&rows);
            let mut frame = Frame::new(Size::zero());

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
        } else {
            self.into_line_frame(points, align)
        }
    }

    fn into_line_frame(self, points: &[Abs], align: Align) -> Frame {
        let ascent = self.ascent();
        let mut frame = Frame::new(Size::new(Abs::zero(), ascent + self.descent()));
        frame.set_baseline(ascent);

        let mut next_x = {
            let mut widths = Vec::new();
            if !points.is_empty() && align != Align::Left {
                let mut width = Abs::zero();
                for fragment in self.iter() {
                    if matches!(fragment.0, MathFragment::Align) {
                        widths.push(width);
                        width = Abs::zero();
                    } else {
                        width += fragment.0.width();
                    }
                }
                widths.push(width);
            }
            let widths = widths;

            let mut prev_points = once(Abs::zero()).chain(points.iter().copied());
            let mut point_widths = points.iter().copied().zip(widths);
            let mut alternator = LeftRightAlternator::Right;
            move || match align {
                Align::Left => prev_points.next(),
                Align::Right => point_widths.next().map(|(point, width)| point - width),
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

        for fragment in self.0.into_iter().map(|f| f.0) {
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

        let mut frame = Frame::new(Size::zero());
        let mut x = Abs::zero();
        let mut ascent = Abs::zero();
        let mut descent = Abs::zero();

        let finalize_frame = |frame: &mut Frame, x, ascent, descent| {
            frame.set_size(Size::new(x, ascent + descent));
            frame.set_baseline(Abs::zero());
            frame.translate(Point::with_y(ascent));
        };

        let mut level = 0;
        let mut space_is_visible = false;

        let is_relation =
            |f: &MathFragment| matches!(f.class(), Some(MathClass::Relation));
        let is_space = |f: &MathFragment| {
            matches!(f, MathFragment::Space(_) | MathFragment::Spacing(_))
        };

        let mut iter = self.0.into_iter().peekable();
        while let Some((fragment, role)) = iter.next() {
            if space_is_visible {
                match fragment {
                    MathFragment::Space(s) | MathFragment::Spacing(s) => {
                        items.push(MathParItem::Space(s));
                        continue;
                    }
                    _ => {}
                }
            }

            let class = fragment.class();
            let mut terminates = false;
            if level == 0
                && (class == Some(MathClass::Binary)
                    || (class == Some(MathClass::Relation)
                        && !iter.peek().map(|f| is_relation(&f.0)).unwrap_or(false)))
            {
                terminates = true;
            }

            match role {
                GroupRole::Begin => level += 1,
                GroupRole::End => level -= 1,
                _ => {}
            }

            let y = fragment.ascent();
            ascent.set_max(y);
            descent.set_max(fragment.descent());
            let pos = Point::new(x, -y);
            x += fragment.width();
            frame.push_frame(pos, fragment.into_frame());

            if terminates {
                let mut frame_prev =
                    std::mem::replace(&mut frame, Frame::new(Size::zero()));
                finalize_frame(&mut frame_prev, x, ascent, descent);
                items.push(MathParItem::Frame(frame_prev));
                x = Abs::zero();
                ascent = Abs::zero();
                descent = Abs::zero();

                space_is_visible = true;
                if let Some(f_next) = iter.peek() {
                    if !is_space(&f_next.0) {
                        items.push(MathParItem::Space(Abs::zero()));
                    }
                };
            } else {
                space_is_visible = false;
            }
        }

        if !frame.is_empty() {
            finalize_frame(&mut frame, x, ascent, descent);
            items.push(MathParItem::Frame(frame));
        }
        items
    }
}

impl<T: Into<MathFragment>> From<T> for MathRow {
    fn from(fragment: T) -> Self {
        Self(vec![(fragment.into(), GroupRole::Inner)])
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
