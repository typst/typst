use super::*;

#[derive(Debug, Default, Clone)]
pub(super) struct MathRow(pub Vec<MathFragment>);

impl MathRow {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn width(&self) -> Abs {
        self.0.iter().map(|fragment| fragment.width()).sum()
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
            if let MathFragment::Glyph(glyph) = *prev {
                if !glyph.italics_correction.is_zero()
                    && fragment.class() != Some(MathClass::Alphabetic)
                {
                    amount += glyph.italics_correction;
                }
            }

            amount += spacing(prev, &fragment, style, space, space_width).at(font_size);

            if !amount.is_zero() {
                self.0.insert(i + 1, MathFragment::Spacing(amount));
            }

            break;
        }

        self.0.push(fragment);
    }

    pub fn to_frame(mut self, ctx: &MathContext) -> Frame {
        if self.0.iter().any(|frag| matches!(frag, MathFragment::Linebreak)) {
            let mut frame = Frame::new(Size::zero());
            let fragments = std::mem::take(&mut self.0);

            let leading = ctx.styles().get(ParNode::LEADING);
            let rows: Vec<_> = fragments
                .split(|frag| matches!(frag, MathFragment::Linebreak))
                .map(|slice| Self(slice.to_vec()))
                .collect();

            let points = alignments(&rows);
            for (i, row) in rows.into_iter().enumerate() {
                let size = frame.size_mut();
                let sub = row.to_line_frame(ctx, &points, Align::Center);
                if i > 0 {
                    size.y += leading;
                }
                let pos = Point::with_y(size.y);
                size.y += sub.height();
                size.x.set_max(sub.width());
                frame.push_frame(pos, sub);
            }
            frame
        } else {
            self.to_line_frame(ctx, &[], Align::Center)
        }
    }

    pub fn to_line_frame(self, ctx: &MathContext, points: &[Abs], align: Align) -> Frame {
        let ascent = self.0.iter().map(MathFragment::ascent).max().unwrap_or_default();
        let descent = self.0.iter().map(MathFragment::descent).max().unwrap_or_default();

        let size = Size::new(Abs::zero(), ascent + descent);
        let mut frame = Frame::new(size);
        let mut x = Abs::zero();
        frame.set_baseline(ascent);

        if let (Some(&first), Align::Center) = (points.first(), align) {
            let segment: Abs = self
                .0
                .iter()
                .take_while(|fragment| !matches!(fragment, MathFragment::Align))
                .map(|fragment| fragment.width())
                .sum();
            x = first - segment;
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
}

impl<T> From<T> for MathRow
where
    T: Into<MathFragment>,
{
    fn from(fragment: T) -> Self {
        Self(vec![fragment.into()])
    }
}
