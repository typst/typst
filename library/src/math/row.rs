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
        style: MathStyle,
        fragment: impl Into<MathFragment>,
    ) {
        let fragment = fragment.into();
        if let Some(fragment_class) = fragment.class() {
            for (i, prev) in self.0.iter().enumerate().rev() {
                if matches!(prev, MathFragment::Align) {
                    continue;
                }

                let mut amount = Abs::zero();
                if let MathFragment::Glyph(glyph) = *prev {
                    if !glyph.italics_correction.is_zero()
                        && fragment_class != MathClass::Alphabetic
                    {
                        amount += glyph.italics_correction;
                    }
                }

                if let Some(prev_class) = prev.class() {
                    amount += spacing(prev_class, fragment_class, style).at(font_size);
                }

                if !amount.is_zero() {
                    self.0.insert(i + 1, MathFragment::Spacing(amount));
                }

                break;
            }
        }
        self.0.push(fragment);
    }

    pub fn to_frame(mut self, ctx: &MathContext) -> Frame {
        if self.0.iter().any(|frag| matches!(frag, MathFragment::Linebreak)) {
            let mut frame = Frame::new(Size::zero());
            let fragments = std::mem::take(&mut self.0);

            let leading = ctx.outer.chain(&ctx.map).get(ParNode::LEADING);
            let rows: Vec<_> = fragments
                .split(|frag| matches!(frag, MathFragment::Linebreak))
                .map(|slice| Self(slice.to_vec()))
                .collect();

            let points = alignments(&rows);
            for (i, row) in rows.into_iter().enumerate() {
                let size = frame.size_mut();
                let sub = row.to_line_frame(ctx, &points);
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
            self.to_line_frame(ctx, &[])
        }
    }

    pub fn to_line_frame(self, ctx: &MathContext, points: &[Abs]) -> Frame {
        let ascent = self.0.iter().map(MathFragment::ascent).max().unwrap_or_default();
        let descent = self.0.iter().map(MathFragment::descent).max().unwrap_or_default();

        let size = Size::new(Abs::zero(), ascent + descent);
        let mut frame = Frame::new(size);
        let mut x = Abs::zero();
        frame.set_baseline(ascent);

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
