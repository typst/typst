use std::iter::once;

use typst_library::foundations::{Resolve, StyleChain};
use typst_library::layout::{Abs, AlignElem, Em, Frame, InlineItem, Point, Size};
use typst_library::math::{EquationElem, LeftRightAlternator, MathSize};
use typst_library::model::ParElem;
use unicode_math_class::MathClass;

use super::fragment::MathFragment;

/// Leading between rows in script and scriptscript size.
const TIGHT_LEADING: Em = Em::new(0.25);

pub trait MathFragmentsExt {
    fn rows(&self) -> Vec<Vec<MathFragment>>;
    fn ascent(&self) -> Abs;
    fn descent(&self) -> Abs;
    fn into_frame(self, styles: StyleChain) -> Frame;
    fn multiline_frame_builder(self, styles: StyleChain) -> MathRunFrameBuilder;
    fn into_line_frame(self, points: &[Abs], alternator: LeftRightAlternator) -> Frame;
    fn into_par_items(self) -> Vec<InlineItem>;
    fn is_multiline(&self) -> bool;
}

impl MathFragmentsExt for Vec<MathFragment> {
    /// Split by linebreaks, and copy [`MathFragment`]s into rows.
    fn rows(&self) -> Vec<Self> {
        self.split(|frag| matches!(frag, MathFragment::Linebreak))
            .map(|slice| slice.to_vec())
            .collect()
    }

    fn ascent(&self) -> Abs {
        self.iter()
            .filter(|e| affects_row_height(e))
            .map(|e| e.ascent())
            .max()
            .unwrap_or_default()
    }

    fn descent(&self) -> Abs {
        self.iter()
            .filter(|e| affects_row_height(e))
            .map(|e| e.descent())
            .max()
            .unwrap_or_default()
    }

    fn into_frame(self, styles: StyleChain) -> Frame {
        if !self.is_multiline() {
            self.into_line_frame(&[], LeftRightAlternator::Right)
        } else {
            self.multiline_frame_builder(styles).build()
        }
    }

    /// Returns a builder that lays out the [`MathFragment`]s into a possibly
    /// multi-row [`Frame`]. The rows are aligned using the same set of alignment
    /// points computed from them as a whole.
    fn multiline_frame_builder(self, styles: StyleChain) -> MathRunFrameBuilder {
        let rows: Vec<_> = self.rows();
        let row_count = rows.len();
        let alignments = alignments(&rows);

        let leading = if styles.get(EquationElem::size) >= MathSize::Text {
            styles.resolve(ParElem::leading)
        } else {
            TIGHT_LEADING.resolve(styles)
        };

        let align = styles.resolve(AlignElem::alignment).x;
        let mut frames: Vec<(Frame, Point)> = vec![];
        let mut size = Size::zero();
        for (i, row) in rows.into_iter().enumerate() {
            if i == row_count - 1 && row.is_empty() {
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
    fn into_line_frame(
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

        for fragment in self.into_iter() {
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

    /// Convert this run of math fragments into a vector of inline items for
    /// paragraph layout. Creates multiple fragments when relation or binary
    /// operators are present to allow for line-breaking opportunities later.
    fn into_par_items(self) -> Vec<InlineItem> {
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

        let is_space = |f: &MathFragment| matches!(f, MathFragment::Space(_));
        let is_line_break_opportunity = |class, next_fragment| match class {
            // Don't split when two relations are in a row or when preceding a
            // closing parenthesis.
            MathClass::Binary => next_fragment != Some(MathClass::Closing),
            MathClass::Relation => {
                !matches!(next_fragment, Some(MathClass::Relation | MathClass::Closing))
            }
            _ => false,
        };

        let mut iter = self.into_iter().peekable();
        while let Some(fragment) = iter.next() {
            if space_is_visible && is_space(&fragment) {
                items.push(InlineItem::Space(fragment.width(), true));
                continue;
            }

            let class = fragment.class();
            let y = fragment.ascent();

            ascent.set_max(y);
            descent.set_max(fragment.descent());

            let pos = Point::new(x, -y);
            x += fragment.width();
            frame.push_frame(pos, fragment.into_frame());
            empty = false;

            // Split our current frame when we encounter a binary operator or
            // relation so that there is a line-breaking opportunity.
            if is_line_break_opportunity(class, iter.peek().map(|f| f.class())) {
                let mut frame_prev =
                    std::mem::replace(&mut frame, Frame::soft(Size::zero()));

                finalize_frame(&mut frame_prev, x, ascent, descent);
                items.push(InlineItem::Frame(frame_prev));
                empty = true;

                x = Abs::zero();
                ascent = Abs::zero();
                descent = Abs::zero();

                space_is_visible = true;
                if let Some(f_next) = iter.peek()
                    && !is_space(f_next)
                {
                    items.push(InlineItem::Space(Abs::zero(), true));
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

/// Determine the positions of the alignment points, according to the input rows combined.
pub fn alignments(rows: &[Vec<MathFragment>]) -> AlignmentResult {
    let mut widths = Vec::<Abs>::new();

    let mut pending_width = Abs::zero();
    for row in rows {
        let mut width = Abs::zero();
        let mut alignment_index = 0;

        for fragment in row.iter() {
            if matches!(fragment, MathFragment::Align) {
                if alignment_index < widths.len() {
                    widths[alignment_index].set_max(width);
                } else {
                    widths.push(width.max(pending_width));
                }
                width = Abs::zero();
                alignment_index += 1;
            } else {
                width += fragment.width();
            }
        }
        if widths.is_empty() {
            pending_width.set_max(width);
        } else if alignment_index < widths.len() {
            widths[alignment_index].set_max(width);
        } else {
            widths.push(width.max(pending_width));
        }
    }

    let mut points = widths;
    for i in 1..points.len() {
        let prev = points[i - 1];
        points[i] += prev;
    }
    AlignmentResult {
        width: points.last().copied().unwrap_or(pending_width),
        points,
    }
}

pub struct AlignmentResult {
    pub points: Vec<Abs>,
    pub width: Abs,
}

fn affects_row_height(fragment: &MathFragment) -> bool {
    !matches!(
        fragment,
        MathFragment::Align | MathFragment::Linebreak | MathFragment::Tag(_)
    )
}
