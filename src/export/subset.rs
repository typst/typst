//! Font subsetting.

use std::borrow::Cow;

use ttf_parser::parser::{FromData, LazyArray16, Offset, Offset32, Stream};
use ttf_parser::{Face, Tag};

/// Subset a font face.
///
/// Returns `None` if the font data is invalid.
pub fn subset(data: &[u8], index: u32) -> Option<Vec<u8>> {
    let mut s = Subsetter::new(data, index)?;
    s.subset()?;
    Some(s.encode())
}

struct Subsetter<'a> {
    face: Face<'a>,
    magic: Magic,
    records: LazyArray16<'a, TableRecord>,
    tables: Vec<(Tag, Cow<'a, [u8]>)>,
}

impl<'a> Subsetter<'a> {
    /// Parse the font header and create a new subsetter.
    fn new(data: &'a [u8], index: u32) -> Option<Self> {
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

        Some(Self { face, magic, records, tables: vec![] })
    }

    /// Subset, drop and copy tables.
    fn subset(&mut self) -> Option<()> {
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

                // TODO: Subset.
                // b"loca" => {}
                // b"glyf" => {}
                // b"sbix" => {}
                // b"SVG " => {}
                // b"post" => {}

                // All other tables are simply copied.
                _ => self.tables.push((tag, Cow::Borrowed(data))),
            }
        }
        Some(())
    }

    /// Encode the subsetted font file.
    fn encode(mut self) -> Vec<u8> {
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
            if *tag == Tag::from_bytes(b"head") {
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

        w
    }
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

impl ToData for u32 {
    fn write(&self, data: &mut Vec<u8>) {
        data.extend(&self.to_be_bytes());
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
