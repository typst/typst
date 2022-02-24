//! Paragraph layout.

use std::sync::Arc;

use itertools::Either;
use unicode_bidi::{BidiInfo, Level};
use xi_unicode::LineBreakIterator;

use super::prelude::*;
use super::{shape, ShapedText, SpacingKind, TextNode};
use crate::font::FontStore;
use crate::util::{ArcExt, EcoString, RangeExt, SliceExt};

/// Arrange text, spacing and inline-level nodes into a paragraph.
#[derive(Hash)]
pub struct ParNode(pub StyleVec<ParChild>);

/// A uniformly styled atomic piece of a paragraph.
#[derive(Hash)]
pub enum ParChild {
    /// A chunk of text.
    Text(EcoString),
    /// Horizontal spacing between other children.
    Spacing(SpacingKind),
    /// An arbitrary inline-level node.
    Node(LayoutNode),
}

#[class]
impl ParNode {
    /// The direction for text and inline objects.
    pub const DIR: Dir = Dir::LTR;
    /// How to align text and inline objects in their line.
    pub const ALIGN: Align = Align::Left;
    /// The spacing between lines (dependent on scaled font size).
    pub const LEADING: Linear = Relative::new(0.65).into();
    /// The extra spacing between paragraphs (dependent on scaled font size).
    pub const SPACING: Linear = Relative::new(0.55).into();
    /// The indent the first line of a consecutive paragraph should have.
    pub const INDENT: Linear = Linear::zero();

    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Template> {
        // The paragraph constructor is special: It doesn't create a paragraph
        // since that happens automatically through markup. Instead, it just
        // lifts the passed body to the block level so that it won't merge with
        // adjacent stuff and it styles the contained paragraphs.
        Ok(Template::Block(args.expect("body")?))
    }

    fn set(args: &mut Args, styles: &mut StyleMap) -> TypResult<()> {
        let mut dir =
            args.named("lang")?
                .map(|iso: EcoString| match iso.to_lowercase().as_str() {
                    "ar" | "he" | "fa" | "ur" | "ps" | "yi" => Dir::RTL,
                    "en" | "fr" | "de" => Dir::LTR,
                    _ => Dir::LTR,
                });

        if let Some(Spanned { v, span }) = args.named::<Spanned<Dir>>("dir")? {
            if v.axis() != SpecAxis::Horizontal {
                bail!(span, "must be horizontal");
            }
            dir = Some(v);
        }

        let align =
            if let Some(Spanned { v, span }) = args.named::<Spanned<Align>>("align")? {
                if v.axis() != SpecAxis::Horizontal {
                    bail!(span, "must be horizontal");
                }
                Some(v)
            } else {
                dir.map(|dir| dir.start().into())
            };

        styles.set_opt(Self::DIR, dir);
        styles.set_opt(Self::ALIGN, align);
        styles.set_opt(Self::LEADING, args.named("leading")?);
        styles.set_opt(Self::SPACING, args.named("spacing")?);
        styles.set_opt(Self::INDENT, args.named("indent")?);

        Ok(())
    }
}

impl Layout for ParNode {
    fn layout(
        &self,
        ctx: &mut Context,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Arc<Frame>>> {
        // Collect all text into one string used for BiDi analysis.
        let text = self.collect_text();
        let level = Level::from_dir(styles.get(Self::DIR));
        let bidi = BidiInfo::new(&text, level);

        // Prepare paragraph layout by building a representation on which we can
        // do line breaking without layouting each and every line from scratch.
        let par = ParLayout::new(ctx, self, bidi, regions, &styles)?;
        let fonts = &mut ctx.fonts;
        let em = styles.get(TextNode::SIZE).abs;
        let align = styles.get(ParNode::ALIGN);
        let leading = styles.get(ParNode::LEADING).resolve(em);

        // The already determined lines and the current line attempt.
        let mut lines = vec![];
        let mut start = 0;
        let mut last = None;

        // Find suitable line breaks.
        for (end, mandatory) in LineBreakIterator::new(&text) {
            // Compute the line and its size.
            let mut line = par.line(fonts, start .. end);

            // If the line doesn't fit anymore, we push the last fitting attempt
            // into the stack and rebuild the line from its end. The resulting
            // line cannot be broken up further.
            if !regions.first.x.fits(line.size.x) {
                if let Some((last_line, last_end)) = last.take() {
                    lines.push(last_line);
                    start = last_end;
                    line = par.line(fonts, start .. end);
                }
            }

            // Finish the current line if there is a mandatory line break (i.e.
            // due to "\n") or if the line doesn't fit horizontally already
            // since no shorter line will be possible.
            if mandatory || !regions.first.x.fits(line.size.x) {
                lines.push(line);
                start = end;
                last = None;
            } else {
                last = Some((line, end));
            }
        }

        if let Some((line, _)) = last {
            lines.push(line);
        }

        // Determine the paragraph's width: Fit to width if we shoudn't expand
        // and there's no fractional spacing.
        let mut width = regions.first.x;
        if !regions.expand.x && lines.iter().all(|line| line.fr.is_zero()) {
            width = lines.iter().map(|line| line.size.x).max().unwrap_or_default();
        }

        // State for final frame building.
        let mut regions = regions.clone();
        let mut finished = vec![];
        let mut first = true;
        let mut output = Frame::new(Size::with_x(width));

        // Stack the lines into one frame per region.
        for line in lines {
            while !regions.first.y.fits(line.size.y) && !regions.in_last() {
                finished.push(Arc::new(output));
                output = Frame::new(Size::with_x(width));
                regions.next();
                first = true;
            }

            if !first {
                output.size.y += leading;
            }

            let frame = line.build(fonts, width, align);
            let pos = Point::with_y(output.size.y);
            output.size.y += frame.size.y;
            output.merge_frame(pos, frame);

            regions.first.y -= line.size.y + leading;
            first = false;
        }

        finished.push(Arc::new(output));
        Ok(finished)
    }
}

impl ParNode {
    /// Concatenate all text in the paragraph into one string, replacing spacing
    /// with a space character and other non-text nodes with the object
    /// replacement character.
    fn collect_text(&self) -> String {
        let mut text = String::new();
        for string in self.strings() {
            text.push_str(string);
        }
        text
    }

    /// The range of each item in the collected text.
    fn ranges(&self) -> impl Iterator<Item = Range> + '_ {
        let mut cursor = 0;
        self.strings().map(move |string| {
            let start = cursor;
            cursor += string.len();
            start .. cursor
        })
    }

    /// The string representation of each child.
    fn strings(&self) -> impl Iterator<Item = &str> {
        self.0.items().map(|child| match child {
            ParChild::Text(text) => text,
            ParChild::Spacing(_) => " ",
            ParChild::Node(_) => "\u{FFFC}",
        })
    }
}

impl Debug for ParNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Par ")?;
        self.0.fmt(f)
    }
}

impl Debug for ParChild {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Text(text) => write!(f, "Text({:?})", text),
            Self::Spacing(kind) => write!(f, "{:?}", kind),
            Self::Node(node) => node.fmt(f),
        }
    }
}

impl Merge for ParChild {
    fn merge(&mut self, next: &Self) -> bool {
        if let (Self::Text(left), Self::Text(right)) = (self, next) {
            left.push_str(right);
            true
        } else {
            false
        }
    }
}

/// A paragraph break.
pub struct ParbreakNode;

#[class]
impl ParbreakNode {
    fn construct(_: &mut Context, _: &mut Args) -> TypResult<Template> {
        Ok(Template::Parbreak)
    }
}

/// A line break.
pub struct LinebreakNode;

#[class]
impl LinebreakNode {
    fn construct(_: &mut Context, _: &mut Args) -> TypResult<Template> {
        Ok(Template::Linebreak)
    }
}

/// A paragraph representation in which children are already layouted and text
/// is separated into shapable runs.
struct ParLayout<'a> {
    /// Bidirectional text embedding levels for the paragraph.
    bidi: BidiInfo<'a>,
    /// Spacing, separated text runs and layouted nodes.
    items: Vec<ParItem<'a>>,
    /// The ranges of the items in `bidi.text`.
    ranges: Vec<Range>,
}

/// Range of a substring of text.
type Range = std::ops::Range<usize>;

/// A prepared item in a paragraph layout.
enum ParItem<'a> {
    /// Absolute spacing between other items.
    Absolute(Length),
    /// Fractional spacing between other items.
    Fractional(Fractional),
    /// A shaped text run with consistent direction.
    Text(ShapedText<'a>),
    /// A layouted child node.
    Frame(Frame),
}

impl<'a> ParLayout<'a> {
    /// Prepare initial shaped text and layouted children.
    fn new(
        ctx: &mut Context,
        par: &'a ParNode,
        bidi: BidiInfo<'a>,
        regions: &Regions,
        styles: &'a StyleChain<'a>,
    ) -> TypResult<Self> {
        let mut items = vec![];
        let mut ranges = vec![];

        // Layout the children and collect them into items.
        for (range, (child, map)) in par.ranges().zip(par.0.iter()) {
            let styles = map.chain(styles);
            match child {
                ParChild::Text(_) => {
                    // TODO: Also split by language and script.
                    let mut cursor = range.start;
                    for (level, count) in bidi.levels[range].group() {
                        let start = cursor;
                        cursor += count;
                        let subrange = start .. cursor;
                        let text = &bidi.text[subrange.clone()];
                        let shaped = shape(&mut ctx.fonts, text, styles, level.dir());
                        items.push(ParItem::Text(shaped));
                        ranges.push(subrange);
                    }
                }
                ParChild::Spacing(kind) => match *kind {
                    SpacingKind::Linear(v) => {
                        let resolved = v.resolve(regions.first.x);
                        items.push(ParItem::Absolute(resolved));
                        ranges.push(range);
                    }
                    SpacingKind::Fractional(v) => {
                        items.push(ParItem::Fractional(v));
                        ranges.push(range);
                    }
                },
                ParChild::Node(node) => {
                    let size = Size::new(regions.first.x, regions.base.y);
                    let pod = Regions::one(size, regions.base, Spec::splat(false));
                    let frame = node.layout(ctx, &pod, styles)?.remove(0);
                    items.push(ParItem::Frame(Arc::take(frame)));
                    ranges.push(range);
                }
            }
        }

        Ok(Self { bidi, items, ranges })
    }

    /// Create a line which spans the given range.
    fn line(&'a self, fonts: &mut FontStore, mut range: Range) -> LineLayout<'a> {
        // Find the items which bound the text range.
        let last_idx = self.find(range.end.saturating_sub(1)).unwrap();
        let first_idx = if range.is_empty() {
            last_idx
        } else {
            self.find(range.start).unwrap()
        };

        // Slice out the relevant items and ranges.
        let mut items = &self.items[first_idx ..= last_idx];
        let ranges = &self.ranges[first_idx ..= last_idx];

        // Reshape the last item if it's split in half.
        let mut last = None;
        if let Some((ParItem::Text(shaped), rest)) = items.split_last() {
            // Compute the range we want to shape, trimming whitespace at the
            // end of the line.
            let base = self.ranges[last_idx].start;
            let start = range.start.max(base);
            let end = start + self.bidi.text[start .. range.end].trim_end().len();
            let shifted = start - base .. end - base;

            // Reshape if necessary.
            if shifted.len() < shaped.text.len() {
                // If start == end and the rest is empty, then we have an empty
                // line. To make that line have the appropriate height, we shape the
                // empty string.
                if !shifted.is_empty() || rest.is_empty() {
                    // Reshape that part.
                    let reshaped = shaped.reshape(fonts, shifted);
                    last = Some(ParItem::Text(reshaped));
                }

                items = rest;
                range.end = end;
            }
        }

        // Reshape the start item if it's split in half.
        let mut first = None;
        if let Some((ParItem::Text(shaped), rest)) = items.split_first() {
            // Compute the range we want to shape.
            let Range { start: base, end: first_end } = self.ranges[first_idx];
            let start = range.start;
            let end = range.end.min(first_end);
            let shifted = start - base .. end - base;

            // Reshape if necessary.
            if shifted.len() < shaped.text.len() {
                if !shifted.is_empty() {
                    let reshaped = shaped.reshape(fonts, shifted);
                    first = Some(ParItem::Text(reshaped));
                }

                items = rest;
            }
        }

        let mut width = Length::zero();
        let mut top = Length::zero();
        let mut bottom = Length::zero();
        let mut fr = Fractional::zero();

        // Measure the size of the line.
        for item in first.iter().chain(items).chain(&last) {
            match item {
                ParItem::Absolute(v) => width += *v,
                ParItem::Fractional(v) => fr += *v,
                ParItem::Text(shaped) => {
                    width += shaped.size.x;
                    top.set_max(shaped.baseline);
                    bottom.set_max(shaped.size.y - shaped.baseline);
                }
                ParItem::Frame(frame) => {
                    width += frame.size.x;
                    top.set_max(frame.baseline());
                    bottom.set_max(frame.size.y - frame.baseline());
                }
            }
        }

        LineLayout {
            bidi: &self.bidi,
            range,
            first,
            items,
            last,
            ranges,
            size: Size::new(width, top + bottom),
            baseline: top,
            fr,
        }
    }


    /// Find the index of the item whose range contains the `text_offset`.
    fn find(&self, text_offset: usize) -> Option<usize> {
        self.ranges.binary_search_by(|r| r.locate(text_offset)).ok()
    }
}

/// A lightweight representation of a line that spans a specific range in a
/// paragraph's text. This type enables you to cheaply measure the size of a
/// line in a range before comitting to building the line's frame.
struct LineLayout<'a> {
    /// Bidi information about the paragraph.
    bidi: &'a BidiInfo<'a>,
    /// The range the line spans in the paragraph.
    range: Range,
    /// A reshaped text item if the line sliced up a text item at the start.
    first: Option<ParItem<'a>>,
    /// Middle items which don't need to be reprocessed.
    items: &'a [ParItem<'a>],
    /// A reshaped text item if the line sliced up a text item at the end. If
    /// there is only one text item, this takes precedence over `first`.
    last: Option<ParItem<'a>>,
    /// The ranges, indexed as `[first, ..items, last]`. The ranges for `first`
    /// and `last` aren't trimmed to the line, but it doesn't matter because
    /// we're just checking which range an index falls into.
    ranges: &'a [Range],
    /// The size of the line.
    size: Size,
    /// The baseline of the line.
    baseline: Length,
    /// The sum of fractional ratios in the line.
    fr: Fractional,
}

impl<'a> LineLayout<'a> {
    /// Build the line's frame.
    fn build(&self, fonts: &FontStore, width: Length, align: Align) -> Frame {
        let size = Size::new(self.size.x.max(width), self.size.y);
        let remaining = size.x - self.size.x;

        let mut offset = Length::zero();
        let mut output = Frame::new(size);
        output.baseline = Some(self.baseline);

        for item in self.reordered() {
            let mut position = |frame: Frame| {
                let x = offset + align.resolve(remaining);
                let y = self.baseline - frame.baseline();
                offset += frame.size.x;
                output.merge_frame(Point::new(x, y), frame);
            };

            match item {
                ParItem::Absolute(v) => offset += *v,
                ParItem::Fractional(v) => offset += v.resolve(self.fr, remaining),
                ParItem::Text(shaped) => position(shaped.build(fonts)),
                ParItem::Frame(frame) => position(frame.clone()),
            }
        }

        output
    }

    /// Iterate through the line's items in visual order.
    fn reordered(&self) -> impl Iterator<Item = &ParItem<'a>> {
        // The bidi crate doesn't like empty lines.
        let (levels, runs) = if !self.range.is_empty() {
            // Find the paragraph that contains the line.
            let para = self
                .bidi
                .paragraphs
                .iter()
                .find(|para| para.range.contains(&self.range.start))
                .unwrap();

            // Compute the reordered ranges in visual order (left to right).
            self.bidi.visual_runs(para, self.range.clone())
        } else {
            (vec![], vec![])
        };

        runs.into_iter()
            .flat_map(move |run| {
                let first_idx = self.find(run.start).unwrap();
                let last_idx = self.find(run.end - 1).unwrap();
                let range = first_idx ..= last_idx;

                // Provide the items forwards or backwards depending on the run's
                // direction.
                if levels[run.start].is_ltr() {
                    Either::Left(range)
                } else {
                    Either::Right(range.rev())
                }
            })
            .map(move |idx| self.get(idx).unwrap())
    }

    /// Find the index of the item whose range contains the `text_offset`.
    fn find(&self, text_offset: usize) -> Option<usize> {
        self.ranges.binary_search_by(|r| r.locate(text_offset)).ok()
    }

    /// Get the item at the index.
    fn get(&self, index: usize) -> Option<&ParItem<'a>> {
        self.first.iter().chain(self.items).chain(&self.last).nth(index)
    }
}

/// Additional methods for BiDi levels.
trait LevelExt: Sized {
    fn from_dir(dir: Dir) -> Option<Self>;
    fn dir(self) -> Dir;
}

impl LevelExt for Level {
    fn from_dir(dir: Dir) -> Option<Self> {
        match dir {
            Dir::LTR => Some(Level::ltr()),
            Dir::RTL => Some(Level::rtl()),
            _ => None,
        }
    }

    fn dir(self) -> Dir {
        if self.is_ltr() { Dir::LTR } else { Dir::RTL }
    }
}
