use crate::layout::AlignElem;

use super::*;

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
            if fragment.class() == Some(MathClass::Vary)
                && matches!(
                    last.and_then(|i| resolved[i].class()),
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

    pub fn width(&self) -> Abs {
        self.iter().map(MathFragment::width).sum()
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
                .and_then(|fragment| fragment.class())
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
            self.0.into_iter().next().unwrap()
        } else {
            FrameFragment::new(ctx, self.into_frame(ctx)).into()
        }
    }

    pub fn into_aligned_frame(
        mut self,
        ctx: &MathContext,
        points: &[Abs],
        align: Align,
    ) -> Frame {
        if self.iter().any(|frag| matches!(frag, MathFragment::Linebreak)) {
            let fragments: Vec<_> = std::mem::take(&mut self.0);
            let leading = if ctx.style.size >= MathSize::Text {
                ParElem::leading_in(ctx.styles())
            } else {
                TIGHT_LEADING.scaled(ctx)
            };

            let mut rows: Vec<_> = fragments
                .split(|frag| matches!(frag, MathFragment::Linebreak))
                .map(|slice| Self(slice.to_vec()))
                .collect();

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
        let descent = self.descent();
        let size = Size::new(Abs::zero(), ascent + descent);
        let mut frame = Frame::new(size);
        let mut x = Abs::zero();
        frame.set_baseline(ascent);

        if let (Some(&first), Align::Center) = (points.first(), align) {
            let mut offset = first;
            for fragment in self.iter() {
                offset -= fragment.width();
                if matches!(fragment, MathFragment::Align) {
                    x = offset;
                    break;
                }
            }
        }

        let fragments = self.0.into_iter().peekable();
        let mut i = 0;
        for fragment in fragments {
            if matches!(fragment, MathFragment::Align) {
                if let Some(&point) = points.get(i) {
                    x = point;
                }
                i += 1;
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
}

impl<T: Into<MathFragment>> From<T> for MathRow {
    fn from(fragment: T) -> Self {
        Self(vec![fragment.into()])
    }
}
