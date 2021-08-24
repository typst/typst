//! Font subsetting.

use std::borrow::Cow;
use std::collections::HashSet;
use std::convert::TryInto;

use ttf_parser::parser::{
    FromData, LazyArray16, LazyArray32, Offset16, Offset32, Stream, F2DOT14,
};
use ttf_parser::{Face, Tag};

/// Subset a font face.
///
/// This will remove the outlines of all glyphs that are not part of the given
/// iterator. Furthmore, all character mapping and layout tables are dropped as
/// shaping has already happened.
///
/// Returns `None` if the font data is invalid.
pub fn subset<I>(data: &[u8], index: u32, glyphs: I) -> Option<Vec<u8>>
where
    I: IntoIterator<Item = u16>,
{
    Subsetter::new(data, index, glyphs.into_iter().collect())?.subset()
}

struct Subsetter<'a> {
    face: Face<'a>,
    glyphs: Vec<u16>,
    magic: Magic,
    records: LazyArray16<'a, TableRecord>,
    tables: Vec<(Tag, Cow<'a, [u8]>)>,
}

impl<'a> Subsetter<'a> {
    /// Parse the font header and create a new subsetter.
    fn new(data: &'a [u8], index: u32, glyphs: Vec<u16>) -> Option<Self> {
        let face = Face::from_slice(data, index).ok()?;
        let mut s = Stream::new(&data);

        // Parse font collection header if necessary.
        let mut magic = s.read::<Magic>()?;
        if magic == Magic::Collection {
            s.skip::<u32>();
            let num_faces = s.read::<u32>()?;
            let offsets = s.read_array32::<Offset32>(num_faces)?;
            let offset = offsets.get(index)?.to_usize();

            s = Stream::new_at(&data, offset)?;
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

        Some(Self {
            face,
            glyphs,
            magic,
            records,
            tables: vec![],
        })
    }

    /// Encode the subsetted font file.
    fn subset(mut self) -> Option<Vec<u8>> {
        // Subset the individual tables and save them in `self.tables`.
        self.subset_tables()?;

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
            if *tag == tg(b"head") {
                // Zero out checksum field in head table.
                data.to_mut()[8 .. 12].copy_from_slice(&[0; 4]);
                checksum_adjustment_offset = Some(offset + 8);
            }

            let len = data.len();
            w.write(TableRecord {
                tag: *tag,
                checksum: checksum(&data),
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
            w.extend(std::iter::repeat(0).take(data.len() % 4));
        }

        // Write checksumAdjustment field in head table.
        if let Some(i) = checksum_adjustment_offset {
            let sum = checksum(&w);
            let val = 0xB1B0AFBA_u32.wrapping_sub(sum);
            w[i .. i + 4].copy_from_slice(&val.to_be_bytes());
        }

        Some(w)
    }

    /// Subset, drop and copy tables.
    fn subset_tables(&mut self) -> Option<()> {
        for record in self.records {
            let tag = record.tag;
            let data = self.face.table_data(tag)?;

            match &tag.to_bytes() {
                // Glyphs are already mapped.
                b"cmap" => {}

                // Layout is already finished.
                b"GPOS" | b"GSUB" | b"BASE" | b"JSTF" | b"MATH" | b"ankr" | b"kern"
                | b"kerx" | b"mort" | b"morx" | b"trak" | b"bsln" | b"just"
                | b"feat" | b"prop" => {}

                // Loca is created when subsetting glyf.
                b"loca" => {}
                b"glyf" => {
                    let head = self.face.table_data(tg(b"head"))?;
                    let short = Stream::read_at::<i16>(head, 50)? == 0;
                    if short {
                        self.subset_glyf_loca::<Offset16>();
                    } else {
                        self.subset_glyf_loca::<Offset32>();
                    }
                }

                // TODO: Subset.
                // b"sbix" => {}
                // b"SVG " => {}
                // b"post" => {}

                // All other tables are simply copied.
                _ => self.tables.push((tag, Cow::Borrowed(data))),
            }
        }
        Some(())
    }
}

/// Helper function to create a tag from bytes.
fn tg(bytes: &[u8; 4]) -> Tag {
    Tag::from_bytes(bytes)
}

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

impl Subsetter<'_> {
    /// Subset the glyf and loca tables.
    fn subset_glyf_loca<T: LocaOffset>(&mut self) -> Option<()> {
        let loca = self.face.table_data(tg(b"loca"))?;
        let glyf = self.face.table_data(tg(b"glyf"))?;

        let offsets = LazyArray32::<T>::new(loca);
        let slice = |id: u16| {
            let from = offsets.get(u32::from(id))?.to_usize();
            let to = offsets.get(u32::from(id) + 1)?.to_usize();
            glyf.get(from .. to)
        };

        // To compute the set of all glyphs we want to keep, we use a work stack
        // containing glyphs whose components we still need to consider.
        let mut glyphs = HashSet::new();
        let mut work: Vec<u16> = std::mem::take(&mut self.glyphs);

        // Always include the notdef glyph.
        work.push(0);

        // Find composite glyph descriptions.
        while let Some(id) = work.pop() {
            if glyphs.insert(id) {
                let mut s = Stream::new(slice(id)?);
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

        for id in 0 .. self.face.number_of_glyphs() {
            sub_loca.write(T::from_usize(sub_glyf.len())?);

            // If the glyph shouldn't be contained in the subset, it will still
            // get a loca entry, but the glyf data is simply empty.
            if glyphs.contains(&id) {
                sub_glyf.extend(slice(id)?);
            }
        }

        sub_loca.write(T::from_usize(sub_glyf.len())?);

        self.tables.push((tg(b"loca"), Cow::Owned(sub_loca)));
        self.tables.push((tg(b"glyf"), Cow::Owned(sub_glyf)));

        Some(())
    }
}

/// Offsets for loca table.
trait LocaOffset: Sized + FromData + ToData {
    fn to_usize(self) -> usize;
    fn from_usize(offset: usize) -> Option<Self>;
}

impl LocaOffset for Offset16 {
    fn to_usize(self) -> usize {
        2 * usize::from(self.0)
    }

    fn from_usize(offset: usize) -> Option<Self> {
        if offset % 2 == 0 {
            (offset / 2).try_into().ok().map(Self)
        } else {
            None
        }
    }
}

impl LocaOffset for Offset32 {
    fn to_usize(self) -> usize {
        self.0 as usize
    }

    fn from_usize(offset: usize) -> Option<Self> {
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
    std::iter::from_fn(move || {
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
