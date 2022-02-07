//! OpenType font subsetting.

use std::borrow::Cow;
use std::collections::HashSet;
use std::iter;
use std::ops::Range;

use ttf_parser::parser::{
    FromData, LazyArray16, LazyArray32, Offset, Offset16, Offset32, Stream, F2DOT14,
};
use ttf_parser::Tag;

/// Subset a font face for PDF embedding.
///
/// This will remove the outlines of all glyphs that are not part of the given
/// slice. Furthmore, all character mapping and layout tables are dropped as
/// shaping has already happened.
///
/// Returns `None` if the font data is fatally broken (in which case
/// `ttf-parser` would probably already have rejected the font, so this
/// shouldn't happen if the font data has already passed through `ttf-parser`).
pub fn subset(data: &[u8], index: u32, glyphs: &HashSet<u16>) -> Option<Vec<u8>> {
    Some(Subsetter::new(data, index, glyphs)?.subset())
}

struct Subsetter<'a> {
    data: &'a [u8],
    magic: Magic,
    records: LazyArray16<'a, TableRecord>,
    num_glyphs: u16,
    glyphs: &'a HashSet<u16>,
    tables: Vec<(Tag, Cow<'a, [u8]>)>,
}

impl<'a> Subsetter<'a> {
    /// Parse the font header and create a new subsetter.
    fn new(data: &'a [u8], index: u32, glyphs: &'a HashSet<u16>) -> Option<Self> {
        let mut s = Stream::new(data);

        let mut magic = s.read::<Magic>()?;
        if magic == Magic::Collection {
            // Parse font collection header if necessary.
            s.skip::<u32>();
            let num_faces = s.read::<u32>()?;
            let offsets = s.read_array32::<Offset32>(num_faces)?;
            let offset = offsets.get(index)?.to_usize();

            s = Stream::new_at(data, offset)?;
            magic = s.read::<Magic>()?;
            if magic == Magic::Collection {
                return None;
            }
        }

        // Read number of table records.
        let count = s.read::<u16>()?;

        // Skip boring parts of header.
        s.skip::<u16>();
        s.skip::<u16>();
        s.skip::<u16>();

        // Read the table records.
        let records = s.read_array16::<TableRecord>(count)?;
        let mut subsetter = Self {
            data,
            magic,
            records,
            num_glyphs: 0,
            glyphs,
            tables: vec![],
        };

        // Find out number of glyphs.
        let maxp = subsetter.table_data(MAXP)?;
        subsetter.num_glyphs = Stream::read_at::<u16>(maxp, 4)?;

        Some(subsetter)
    }

    /// Encode the subsetted font file.
    fn subset(mut self) -> Vec<u8> {
        self.subset_tables();

        // Start writing a brand new font.
        let mut w = Vec::new();
        w.write(self.magic);

        // Write table directory.
        let count = self.tables.len() as u16;
        let entry_selector = (count as f32).log2().floor() as u16;
        let search_range = entry_selector.pow(2) * 16;
        let range_shift = count * 16 - search_range;
        w.write(count);
        w.write(search_range);
        w.write(entry_selector);
        w.write(range_shift);

        // Tables shall be sorted by tag.
        self.tables.sort_by_key(|&(tag, _)| tag);

        // This variable will hold the offset to the checksum adjustment field
        // in the head table, which we'll have to write in the end (after
        // checksumming the whole font).
        let mut checksum_adjustment_offset = None;

        // Write table records.
        let mut offset = 12 + self.tables.len() * TableRecord::SIZE;
        for (tag, data) in &mut self.tables {
            if *tag == HEAD {
                // Zero out checksum field in head table.
                data.to_mut()[8 .. 12].copy_from_slice(&[0; 4]);
                checksum_adjustment_offset = Some(offset + 8);
            }

            let len = data.len();
            w.write(TableRecord {
                tag: *tag,
                checksum: checksum(data),
                offset: offset as u32,
                length: len as u32,
            });

            // Account for the padding to 4 bytes.
            offset += len + len % 4;
        }

        // Write tables.
        for (_, data) in &self.tables {
            // Write data plus padding zeros to align to 4 bytes.
            w.extend(data.as_ref());
            w.extend(iter::repeat(0).take(data.len() % 4));
        }

        // Write checksumAdjustment field in head table.
        if let Some(i) = checksum_adjustment_offset {
            let sum = checksum(&w);
            let val = 0xB1B0AFBA_u32.wrapping_sub(sum);
            w[i .. i + 4].copy_from_slice(&val.to_be_bytes());
        }

        w
    }

    /// Subset, drop and copy tables.
    fn subset_tables(&mut self) {
        // Remove unnecessary name information.
        let handled_post = post::subset(self).is_some();

        // Remove unnecessary glyph outlines.
        let handled_glyf_loca = glyf::subset(self).is_some();
        let handled_cff1 = cff::subset_v1(self).is_some();

        for record in self.records {
            // If `handled` is true, we don't take any further action, if it's
            // false, we copy the table.
            #[rustfmt::skip]
            let handled = match &record.tag.to_bytes() {
                // Drop: Glyphs are already mapped.
                b"cmap" => true,

                // Drop: Layout is already finished.
                b"GPOS" | b"GSUB" | b"BASE" | b"JSTF" | b"MATH" |
                b"ankr" | b"kern" | b"kerx" | b"mort" | b"morx" |
                b"trak" | b"bsln" | b"just" | b"feat" | b"prop" => true,

                // Drop: They don't render in PDF viewers anyway.
                // TODO: We probably have to convert fonts with one of these
                // tables into Type 3 fonts where glyphs are described by either
                // PDF graphics operators or XObject images.
                b"CBDT" | b"CBLC" | b"COLR" | b"CPAL" | b"sbix" | b"SVG " => true,

                // Subsetted: Subsetting happens outside the loop, but if it
                // failed, we simply copy the affected table(s).
                b"post" => handled_post,
                b"loca" | b"glyf" => handled_glyf_loca,
                b"CFF " => handled_cff1,

                // Copy: All other tables are simply copied.
                _ => false,
            };

            if !handled {
                if let Some(data) = self.table_data(record.tag) {
                    self.push_table(record.tag, data);
                }
            }
        }
    }

    /// Retrieve the table data for a table.
    fn table_data(&mut self, tag: Tag) -> Option<&'a [u8]> {
        let (_, record) = self.records.binary_search_by(|record| record.tag.cmp(&tag))?;
        let start = record.offset as usize;
        let end = start + (record.length as usize);
        self.data.get(start .. end)
    }

    /// Push a new table.
    fn push_table(&mut self, tag: Tag, data: impl Into<Cow<'a, [u8]>>) {
        self.tables.push((tag, data.into()));
    }
}

// Some common tags.
const HEAD: Tag = Tag::from_bytes(b"head");
const MAXP: Tag = Tag::from_bytes(b"maxp");
const POST: Tag = Tag::from_bytes(b"post");
const LOCA: Tag = Tag::from_bytes(b"loca");
const GLYF: Tag = Tag::from_bytes(b"glyf");
const CFF1: Tag = Tag::from_bytes(b"CFF ");

/// Calculate a checksum over the sliced data as sum of u32's. The data length
/// must be a multiple of four.
fn checksum(data: &[u8]) -> u32 {
    let mut sum = 0u32;
    for chunk in data.chunks(4) {
        let mut bytes = [0; 4];
        bytes[.. chunk.len()].copy_from_slice(chunk);
        sum = sum.wrapping_add(u32::from_be_bytes(bytes));
    }
    sum
}

/// Zero all bytes in a slice.
fn memzero(slice: &mut [u8]) {
    for byte in slice {
        *byte = 0;
    }
}

/// Convenience trait for writing into a byte buffer.
trait BufExt {
    fn write<T: ToData>(&mut self, v: T);
}

impl BufExt for Vec<u8> {
    fn write<T: ToData>(&mut self, v: T) {
        v.write(self);
    }
}

/// A trait for writing raw binary data.
trait ToData {
    fn write(&self, data: &mut Vec<u8>);
}

impl ToData for u8 {
    fn write(&self, data: &mut Vec<u8>) {
        data.push(*self);
    }
}

impl ToData for u16 {
    fn write(&self, data: &mut Vec<u8>) {
        data.extend(&self.to_be_bytes());
    }
}

impl ToData for Offset16 {
    fn write(&self, data: &mut Vec<u8>) {
        self.0.write(data);
    }
}

impl ToData for u32 {
    fn write(&self, data: &mut Vec<u8>) {
        data.extend(&self.to_be_bytes());
    }
}

impl ToData for Offset32 {
    fn write(&self, data: &mut Vec<u8>) {
        self.0.write(data);
    }
}

impl ToData for Tag {
    fn write(&self, data: &mut Vec<u8>) {
        self.as_u32().write(data);
    }
}

/// Font magic number.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Magic {
    TrueType,
    OpenType,
    Collection,
}

impl FromData for Magic {
    const SIZE: usize = 4;

    fn parse(data: &[u8]) -> Option<Self> {
        match u32::parse(data)? {
            0x00010000 | 0x74727565 => Some(Magic::TrueType),
            0x4F54544F => Some(Magic::OpenType),
            0x74746366 => Some(Magic::Collection),
            _ => None,
        }
    }
}

impl ToData for Magic {
    fn write(&self, data: &mut Vec<u8>) {
        let value: u32 = match self {
            Magic::TrueType => 0x00010000,
            Magic::OpenType => 0x4F54544F,
            Magic::Collection => 0x74746366,
        };
        value.write(data);
    }
}

/// Locates a table in the font file.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct TableRecord {
    tag: Tag,
    checksum: u32,
    offset: u32,
    length: u32,
}

impl FromData for TableRecord {
    const SIZE: usize = 16;

    fn parse(data: &[u8]) -> Option<Self> {
        let mut s = Stream::new(data);
        Some(TableRecord {
            tag: s.read::<Tag>()?,
            checksum: s.read::<u32>()?,
            offset: s.read::<u32>()?,
            length: s.read::<u32>()?,
        })
    }
}

impl ToData for TableRecord {
    fn write(&self, data: &mut Vec<u8>) {
        self.tag.write(data);
        self.checksum.write(data);
        self.offset.write(data);
        self.length.write(data);
    }
}

mod post {
    use super::*;

    /// Subset the post table by removing the name information.
    pub(super) fn subset(subsetter: &mut Subsetter) -> Option<()> {
        // Table version three is the one without names.
        let mut new = 0x00030000_u32.to_be_bytes().to_vec();
        new.extend(subsetter.table_data(POST)?.get(4 .. 32)?);
        subsetter.push_table(POST, new);
        Some(())
    }
}

mod glyf {
    use super::*;

    /// Subset the glyf and loca tables by clearing out glyph data for
    /// unused glyphs.
    pub(super) fn subset(subsetter: &mut Subsetter) -> Option<()> {
        let head = subsetter.table_data(HEAD)?;
        let short = Stream::read_at::<i16>(head, 50)? == 0;
        if short {
            subset_impl::<Offset16>(subsetter)
        } else {
            subset_impl::<Offset32>(subsetter)
        }
    }

    fn subset_impl<T>(subsetter: &mut Subsetter) -> Option<()>
    where
        T: LocaOffset,
    {
        let loca = subsetter.table_data(LOCA)?;
        let glyf = subsetter.table_data(GLYF)?;

        let offsets = LazyArray32::<T>::new(loca);
        let glyph_data = |id: u16| {
            let from = offsets.get(u32::from(id))?.loca_to_usize();
            let to = offsets.get(u32::from(id) + 1)?.loca_to_usize();
            glyf.get(from .. to)
        };

        // The set of all glyphs we will include in the subset.
        let mut subset = HashSet::new();

        // Because glyphs may depend on other glyphs as components (also with
        // multiple layers of nesting), we have to process all glyphs to find
        // their components. For notdef and all requested glyphs we simply use
        // an iterator, but to track other glyphs that need processing we create
        // a work stack.
        let mut iter = iter::once(0).chain(subsetter.glyphs.iter().copied());
        let mut work = vec![];

        // Find composite glyph descriptions.
        while let Some(id) = work.pop().or_else(|| iter.next()) {
            if subset.insert(id) {
                let mut s = Stream::new(glyph_data(id)?);
                if let Some(num_contours) = s.read::<i16>() {
                    // Negative means this is a composite glyph.
                    if num_contours < 0 {
                        // Skip min/max metrics.
                        s.read::<i16>();
                        s.read::<i16>();
                        s.read::<i16>();
                        s.read::<i16>();

                        // Read component glyphs.
                        work.extend(component_glyphs(s));
                    }
                }
            }
        }

        let mut sub_loca = vec![];
        let mut sub_glyf = vec![];

        for id in 0 .. subsetter.num_glyphs {
            // If the glyph shouldn't be contained in the subset, it will
            // still get a loca entry, but the glyf data is simply empty.
            sub_loca.write(T::usize_to_loca(sub_glyf.len())?);
            if subset.contains(&id) {
                sub_glyf.extend(glyph_data(id)?);
            }
        }

        sub_loca.write(T::usize_to_loca(sub_glyf.len())?);

        subsetter.push_table(LOCA, sub_loca);
        subsetter.push_table(GLYF, sub_glyf);

        Some(())
    }

    trait LocaOffset: Sized + FromData + ToData {
        fn loca_to_usize(self) -> usize;
        fn usize_to_loca(offset: usize) -> Option<Self>;
    }

    impl LocaOffset for Offset16 {
        fn loca_to_usize(self) -> usize {
            2 * usize::from(self.0)
        }

        fn usize_to_loca(offset: usize) -> Option<Self> {
            if offset % 2 == 0 {
                (offset / 2).try_into().ok().map(Self)
            } else {
                None
            }
        }
    }

    impl LocaOffset for Offset32 {
        fn loca_to_usize(self) -> usize {
            self.0 as usize
        }

        fn usize_to_loca(offset: usize) -> Option<Self> {
            offset.try_into().ok().map(Self)
        }
    }

    /// Returns an iterator over the component glyphs referenced by the given
    /// `glyf` table composite glyph description.
    fn component_glyphs(mut s: Stream) -> impl Iterator<Item = u16> + '_ {
        const ARG_1_AND_2_ARE_WORDS: u16 = 0x0001;
        const WE_HAVE_A_SCALE: u16 = 0x0008;
        const MORE_COMPONENTS: u16 = 0x0020;
        const WE_HAVE_AN_X_AND_Y_SCALE: u16 = 0x0040;
        const WE_HAVE_A_TWO_BY_TWO: u16 = 0x0080;

        let mut done = false;
        iter::from_fn(move || {
            if done {
                return None;
            }

            let flags = s.read::<u16>()?;
            let component = s.read::<u16>()?;

            if flags & ARG_1_AND_2_ARE_WORDS != 0 {
                s.skip::<i16>();
                s.skip::<i16>();
            } else {
                s.skip::<u16>();
            }

            if flags & WE_HAVE_A_SCALE != 0 {
                s.skip::<F2DOT14>();
            } else if flags & WE_HAVE_AN_X_AND_Y_SCALE != 0 {
                s.skip::<F2DOT14>();
                s.skip::<F2DOT14>();
            } else if flags & WE_HAVE_A_TWO_BY_TWO != 0 {
                s.skip::<F2DOT14>();
                s.skip::<F2DOT14>();
                s.skip::<F2DOT14>();
                s.skip::<F2DOT14>();
            }

            done = flags & MORE_COMPONENTS == 0;
            Some(component)
        })
    }
}

mod cff {
    use super::*;

    /// Subset the CFF table by zeroing glyph data for unused glyphs.
    pub(super) fn subset_v1(subsetter: &mut Subsetter) -> Option<()> {
        let cff = subsetter.table_data(CFF1)?;
        let mut s = Stream::new(cff);

        let (major, _) = (s.read::<u8>()?, s.skip::<u8>());
        if major != 1 {
            return None;
        }

        let header_size = s.read::<u8>()?;
        s = Stream::new_at(cff, usize::from(header_size))?;

        // Skip the name index.
        Index::parse_stream(&mut s);

        // Read the top dict. The index should contain only one item.
        let top_dict_index = Index::parse_stream(&mut s)?;
        let top_dict = Dict::parse(top_dict_index.get(0)?);

        let mut sub_cff = cff.to_vec();

        // Because completely rebuilding the CFF structure would be pretty
        // complex, for now, we employ a peculiar strategy for CFF subsetting:
        // We simply replace unused data with zeros. This way, the font
        // structure and offsets can stay the same. And while the CFF table
        // itself doesn't shrink, the actual embedded font is compressed and
        // greatly benefits from the repeated zeros.
        zero_char_strings(subsetter, cff, &top_dict, &mut sub_cff);
        zero_subr_indices(subsetter, cff, &top_dict, &mut sub_cff);

        subsetter.push_table(CFF1, sub_cff);

        Some(())
    }

    /// Zero unused char strings.
    fn zero_char_strings(
        subsetter: &Subsetter,
        cff: &[u8],
        top_dict: &Dict,
        sub_cff: &mut [u8],
    ) -> Option<()> {
        let char_strings_offset = top_dict.get_offset(Op::CHAR_STRINGS)?;
        let char_strings = Index::parse(cff.get(char_strings_offset ..)?)?;

        for (id, _, range) in char_strings.iter() {
            if !subsetter.glyphs.contains(&id) {
                let start = char_strings_offset + range.start;
                let end = char_strings_offset + range.end;
                memzero(sub_cff.get_mut(start .. end)?);
            }
        }

        Some(())
    }

    /// Zero unused local subroutine indices. We don't currently remove
    /// individual subroutines because finding out which ones are used is
    /// complicated.
    fn zero_subr_indices(
        subsetter: &Subsetter,
        cff: &[u8],
        top_dict: &Dict,
        sub_cff: &mut [u8],
    ) -> Option<()> {
        // Parse FD Select data structure, which maps from glyph ids to find
        // dict indices.
        let fd_select_offset = top_dict.get_offset(Op::FD_SELECT)?;
        let fd_select =
            parse_fd_select(cff.get(fd_select_offset ..)?, subsetter.num_glyphs)?;

        // Clear local subrs from unused font dicts.
        let fd_array_offset = top_dict.get_offset(Op::FD_ARRAY)?;
        let fd_array = Index::parse(cff.get(fd_array_offset ..)?)?;

        // Determine which font dict's subrs to keep.
        let mut sub_fds = HashSet::new();
        for &glyph in subsetter.glyphs {
            sub_fds.insert(fd_select.get(usize::from(glyph))?);
        }

        for (i, data, _) in fd_array.iter() {
            if !sub_fds.contains(&(i as u8)) {
                let font_dict = Dict::parse(data);
                if let Some(private_range) = font_dict.get_range(Op::PRIVATE) {
                    let private_dict = Dict::parse(cff.get(private_range.clone())?);
                    if let Some(subrs_offset) = private_dict.get_offset(Op::SUBRS) {
                        let start = private_range.start + subrs_offset;
                        let index = Index::parse(cff.get(start ..)?)?;
                        let end = start + index.data.len();
                        memzero(sub_cff.get_mut(start .. end)?);
                    }
                }
            }
        }

        Some(())
    }

    /// Returns the font dict index for each glyph.
    fn parse_fd_select(data: &[u8], num_glyphs: u16) -> Option<Cow<'_, [u8]>> {
        let mut s = Stream::new(data);
        let format = s.read::<u8>()?;
        Some(match format {
            0 => Cow::Borrowed(s.read_bytes(usize::from(num_glyphs))?),
            3 => {
                let count = usize::from(s.read::<u16>()?);
                let mut fds = vec![];
                let mut start = s.read::<u16>()?;
                for _ in 0 .. count {
                    let fd = s.read::<u8>()?;
                    let end = s.read::<u16>()?;
                    for _ in start .. end {
                        fds.push(fd);
                    }
                    start = end;
                }
                Cow::Owned(fds)
            }
            _ => Cow::Borrowed(&[]),
        })
    }

    struct Index<'a> {
        /// The data of the whole index (including its header).
        data: &'a [u8],
        /// The data ranges for the actual items.
        items: Vec<Range<usize>>,
    }

    impl<'a> Index<'a> {
        fn parse(data: &'a [u8]) -> Option<Self> {
            let mut s = Stream::new(data);

            let count = usize::from(s.read::<u16>()?);

            let mut items = Vec::with_capacity(count);
            let mut len = 2;

            if count > 0 {
                let offsize = usize::from(s.read::<u8>()?);
                if !matches!(offsize, 1 ..= 4) {
                    return None;
                }

                // Read an offset and transform it to be relative to the start
                // of the index.
                let data_offset = 3 + offsize * (count + 1);
                let mut read_offset = || {
                    let mut bytes = [0u8; 4];
                    bytes[4 - offsize .. 4].copy_from_slice(s.read_bytes(offsize)?);
                    Some(data_offset - 1 + u32::from_be_bytes(bytes) as usize)
                };

                let mut last = read_offset()?;
                for _ in 0 .. count {
                    let offset = read_offset()?;
                    data.get(last .. offset)?;
                    items.push(last .. offset);
                    last = offset;
                }

                len = last;
            }

            Some(Self { data: data.get(.. len)?, items })
        }

        fn parse_stream(s: &'a mut Stream) -> Option<Self> {
            let index = Index::parse(s.tail()?)?;
            s.advance(index.data.len());
            Some(index)
        }

        fn get(&self, idx: usize) -> Option<&'a [u8]> {
            self.data.get(self.items.get(idx)?.clone())
        }

        fn iter(&self) -> impl Iterator<Item = (u16, &'a [u8], Range<usize>)> + '_ {
            self.items
                .iter()
                .enumerate()
                .map(move |(i, item)| (i as u16, &self.data[item.clone()], item.clone()))
        }
    }

    struct Dict<'a>(Vec<Pair<'a>>);

    impl<'a> Dict<'a> {
        fn parse(data: &'a [u8]) -> Self {
            let mut s = Stream::new(data);
            Self(iter::from_fn(|| Pair::parse(&mut s)).collect())
        }

        fn get(&self, op: Op) -> Option<&[Operand<'a>]> {
            self.0
                .iter()
                .find(|pair| pair.op == op)
                .map(|pair| pair.operands.as_slice())
        }

        fn get_offset(&self, op: Op) -> Option<usize> {
            match self.get(op)? {
                &[Operand::Int(offset)] if offset > 0 => usize::try_from(offset).ok(),
                _ => None,
            }
        }

        fn get_range(&self, op: Op) -> Option<Range<usize>> {
            match self.get(op)? {
                &[Operand::Int(len), Operand::Int(offset)] if offset > 0 => {
                    let offset = usize::try_from(offset).ok()?;
                    let len = usize::try_from(len).ok()?;
                    Some(offset .. offset + len)
                }
                _ => None,
            }
        }
    }

    #[derive(Debug)]
    struct Pair<'a> {
        operands: Vec<Operand<'a>>,
        op: Op,
    }

    impl<'a> Pair<'a> {
        fn parse(s: &mut Stream<'a>) -> Option<Self> {
            let mut operands = vec![];
            while s.clone().read::<u8>()? > 21 {
                operands.push(Operand::parse(s)?);
            }
            Some(Self { operands, op: Op::parse(s)? })
        }
    }

    #[derive(Debug, Eq, PartialEq)]
    struct Op(u8, u8);

    impl Op {
        const CHAR_STRINGS: Self = Self(17, 0);
        const PRIVATE: Self = Self(18, 0);
        const SUBRS: Self = Self(19, 0);
        const FD_ARRAY: Self = Self(12, 36);
        const FD_SELECT: Self = Self(12, 37);

        fn parse(s: &mut Stream) -> Option<Self> {
            let b0 = s.read::<u8>()?;
            match b0 {
                12 => Some(Self(b0, s.read::<u8>()?)),
                0 ..= 21 => Some(Self(b0, 0)),
                _ => None,
            }
        }
    }

    #[derive(Debug)]
    enum Operand<'a> {
        Int(i32),
        Real(&'a [u8]),
    }

    impl<'a> Operand<'a> {
        fn parse(s: &mut Stream<'a>) -> Option<Self> {
            let b0 = i32::from(s.read::<u8>()?);
            Some(match b0 {
                30 => {
                    let mut len = 0;
                    for &byte in s.tail()? {
                        len += 1;
                        if byte & 0x0f == 0x0f {
                            break;
                        }
                    }
                    Self::Real(s.read_bytes(len)?)
                }
                32 ..= 246 => Self::Int(b0 - 139),
                247 ..= 250 => {
                    let b1 = i32::from(s.read::<u8>()?);
                    Self::Int((b0 - 247) * 256 + b1 + 108)
                }
                251 ..= 254 => {
                    let b1 = i32::from(s.read::<u8>()?);
                    Self::Int(-(b0 - 251) * 256 - b1 - 108)
                }
                28 => Self::Int(i32::from(s.read::<i16>()?)),
                29 => Self::Int(s.read::<i32>()?),
                _ => return None,
            })
        }
    }
}
