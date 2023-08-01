use std::iter::once;

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
                MathFragment::Linebreak(_) => {
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

    /// Extract the sublines of the row.
    ///
    /// It is very unintuitive, but in current state of things, a `MathRow` can
    /// contain several actual rows. That function deconstructs it to "single"
    /// rows. Hopefully this is only a temporary hack.
    pub fn rows(&self) -> Vec<Self> {
        self.0
            .split_inclusive(|frag| matches!(frag, MathFragment::Linebreak(_)))
            .map(|slice| Self(slice.to_vec()))
            .collect()
    }

    pub fn ascent(&self) -> Abs {
        self.iter().map(MathFragment::ascent).max().unwrap_or_default()
    }

    pub fn descent(&self) -> Abs {
        // FIXME: Because each line now keeps its linebreak, the
        // descent of a line is at least 0.  Some glyphs have,
        // for Typst, negative descents. Think of centered dot, e.g.
        // Maybe make MathFragment descent/ascent > 0 to begin with.
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

    pub fn into_frame(self, ctx: &mut MathContext) -> SourceResult<Frame> {
        let styles = ctx.styles();
        let align = AlignElem::alignment_in(styles).x.resolve(styles);
        self.into_aligned_frame(ctx, &[], align)
    }

    pub fn into_fragment(self, ctx: &mut MathContext) -> SourceResult<MathFragment> {
        if self.0.len() == 1 {
            Ok(self.0.into_iter().next().unwrap())
        } else {
            let frame = self.into_frame(ctx)?;
            Ok(FrameFragment::new(ctx, frame).into())
        }
    }

    pub fn into_display_items(
        self,
        ctx: &MathContext,
        points: &[Abs],
        align: Align,
    ) -> MathDisplayItems {
        if self.iter().any(|frag| matches!(frag, MathFragment::Linebreak(_))) {
            let leading = if ctx.style.size >= MathSize::Text {
                ParElem::leading_in(ctx.styles())
            } else {
                TIGHT_LEADING.scaled(ctx)
            };

            let mut rows = self.rows();

            if matches!(rows.last(), Some(row) if row.0.is_empty()) {
                rows.pop();
            }

            let AlignmentResult { points, width } = alignments(&rows);

            let mut items = vec![];
            for (i, row) in rows.into_iter().enumerate() {
                let label = if let Some(elem) = row.0.last() {
                    match elem {
                        MathFragment::Linebreak(label) => label.clone(),
                        _ => MathLabel::None,
                    }
                } else {
                    MathLabel::None
                };

                if i > 0 {
                    items.push(MathDisplayItem::VSpace(leading));
                }

                let sub = row.into_line_frame(&points, align);

                let mut x = Abs::zero();
                if points.is_empty() {
                    x = align.position(width - sub.width());
                }
                // size.y += sub.height();
                // size.x.set_max(sub.width());
                let mut frame = Frame::new(Axes::new(width, sub.height()));
                let baseline = sub.baseline();
                frame.push_frame(Point::with_x(x), sub);
                frame.set_baseline(baseline);

                items.push(MathDisplayItem::Frame(frame, label));
            }
            MathDisplayItems { items, width }
        } else {
            let frame = self.into_line_frame(points, align);
            let width = frame.width();
            // No linebreaks => no labels!
            MathDisplayItems {
                items: vec![MathDisplayItem::Frame(frame, MathLabel::None)],
                width,
            }
        }
    }

    pub fn into_aligned_frame(
        self,
        ctx: &mut MathContext,
        points: &[Abs],
        align: Align,
    ) -> SourceResult<Frame> {
        let MathDisplayItems { items, width } =
            self.into_display_items(ctx, points, align);

        let mut build_label = |label: MathLabel| {
            if let MathLabel::Some(label) = label {
                let line_tag = EqNumberElem::new().pack().labelled(label);
                return Some(ctx.layout_frame(&line_tag));
            }
            None
        };

        // Special case needed so that baselines are preserved for single line frames.
        if items.len() == 1 {
            return match items.into_iter().last().unwrap() {
                MathDisplayItem::Frame(mut frame, label) => {
                    if let Some(label) = build_label(label).transpose()? {
                        frame.push_frame(Point::zero(), label)
                    };
                    Ok(frame)
                }
                MathDisplayItem::VSpace(dy) => Ok(Frame::new(Axes::with_y(dy))),
            };
        }

        let mut frame = Frame::new(Size::with_x(width));
        for item in items {
            match item {
                MathDisplayItem::VSpace(dy) => frame.size_mut().y += dy,
                MathDisplayItem::Frame(mut line, label) => {
                    let size = frame.size_mut();
                    let pos = Point::with_y(size.y);
                    size.y += line.size().y;
                    if let Some(label) = build_label(label).transpose()? {
                        line.push_frame(Point::zero(), label);
                    }

                    frame.push_frame(pos, line);
                }
            }
        }

        Ok(frame)
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
