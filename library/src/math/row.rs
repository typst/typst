use crate::layout::AlignNode;

use super::*;

pub const TIGHT_LEADING: Em = Em::new(0.25);

#[derive(Debug, Default, Clone)]
pub struct MathRow(pub Vec<MathFragment>);

impl MathRow {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn width(&self) -> Abs {
        self.0.iter().map(|fragment| fragment.width()).sum()
    }

    pub fn height(&self) -> Abs {
        let (ascent, descent) = self.extent();
        ascent + descent
    }

    pub fn push(
        &mut self,
        font_size: Abs,
        space_width: Em,
        style: MathStyle,
        fragment: impl Into<MathFragment>,
    ) {
        let mut fragment = fragment.into();
        if !fragment.participating() {
            self.0.push(fragment);
            return;
        }

        let mut space = false;
        for (i, prev) in self.0.iter().enumerate().rev() {
            if !prev.participating() {
                space |= matches!(prev, MathFragment::Space);
                if matches!(prev, MathFragment::Spacing(_)) {
                    break;
                }
                continue;
            }

            if fragment.class() == Some(MathClass::Vary) {
                if matches!(
                    prev.class(),
                    Some(
                        MathClass::Normal
                            | MathClass::Alphabetic
                            | MathClass::Binary
                            | MathClass::Closing
                            | MathClass::Fence
                            | MathClass::Relation
                    )
                ) {
                    fragment.set_class(MathClass::Binary);
                }
            }

            let mut amount = Abs::zero();
            amount += spacing(prev, &fragment, style, space, space_width).at(font_size);

            if !amount.is_zero() {
                self.0.insert(i + 1, MathFragment::Spacing(amount));
            }

            break;
        }

        self.0.push(fragment);
    }

    pub fn to_frame(self, ctx: &MathContext) -> Frame {
        let styles = ctx.styles();
        let align = styles.get(AlignNode::ALIGNS).x.resolve(styles);
        self.to_aligned_frame(ctx, &[], align)
    }

    pub fn to_aligned_frame(
        mut self,
        ctx: &MathContext,
        points: &[Abs],
        align: Align,
    ) -> Frame {
        if self.0.iter().any(|frag| matches!(frag, MathFragment::Linebreak)) {
            let fragments = std::mem::take(&mut self.0);
            let leading = if ctx.style.size >= MathSize::Text {
                ctx.styles().get(ParNode::LEADING)
            } else {
                TIGHT_LEADING.scaled(ctx)
            };

            let rows: Vec<_> = fragments
                .split(|frag| matches!(frag, MathFragment::Linebreak))
                .map(|slice| Self(slice.to_vec()))
                .collect();

            let width = rows.iter().map(|row| row.width()).max().unwrap_or_default();
            let points = alignments(&rows);
            let mut frame = Frame::new(Size::zero());

            for (i, row) in rows.into_iter().enumerate() {
                let sub = row.to_line_frame(ctx, &points, align);
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
            self.to_line_frame(ctx, points, align)
        }
    }

    fn to_line_frame(self, ctx: &MathContext, points: &[Abs], align: Align) -> Frame {
        let (ascent, descent) = self.extent();
        let size = Size::new(Abs::zero(), ascent + descent);
        let mut frame = Frame::new(size);
        let mut x = Abs::zero();
        frame.set_baseline(ascent);

        if let (Some(&first), Align::Center) = (points.first(), align) {
            let mut offset = first;
            for fragment in &self.0 {
                offset -= fragment.width();
                if matches!(fragment, MathFragment::Align) {
                    x = offset;
                    break;
                }
            }
        }

        let mut fragments = self.0.into_iter().peekable();
        let mut i = 0;
        while let Some(fragment) = fragments.next() {
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
            frame.push_frame(pos, fragment.to_frame(ctx));
        }

        frame.size_mut().x = x;
        frame
    }

    fn extent(&self) -> (Abs, Abs) {
        let ascent = self.0.iter().map(MathFragment::ascent).max().unwrap_or_default();
        let descent = self.0.iter().map(MathFragment::descent).max().unwrap_or_default();
        (ascent, descent)
    }
}

impl<T> From<T> for MathRow
where
    T: Into<MathFragment>,
{
    fn from(fragment: T) -> Self {
        Self(vec![fragment.into()])
    }
}
