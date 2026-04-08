//! Disk-backed page store: serializes pages to a temp file and reads
//! them back one at a time during PDF export.

use std::io::{self, BufReader, BufWriter, Read, Seek, Write};

use typst_library::foundations::{Content, Smart};
use typst_library::model::Numbering;

use super::converter::FrameConverter;
use super::types::*;
use crate::Page;

/// A disk-backed store for document pages.
///
/// Serializes page frames to a temporary file, keeping only lightweight
/// metadata (fonts, images, tags, numberings) in memory. Pages can be
/// read back one at a time for streaming export.
pub struct DiskPageStore {
    /// Temp file holding serialized page data.
    file: tempfile::NamedTempFile,
    /// Number of pages stored.
    page_count: usize,
    /// Byte offsets of each page in the file (for random access).
    offsets: Vec<u64>,
    /// Shared frame converter (holds fonts, images, tags, gradients, tilings).
    pub converter: FrameConverter,
    /// Numbering objects (contain Func, can't be serialized).
    numberings: Vec<Numbering>,
    /// Page supplement Content objects.
    supplements: Vec<Content>,
}

impl DiskPageStore {
    /// Creates a new empty store backed by a temporary file.
    /// Pages can be appended one at a time via `append_page()`.
    pub fn new() -> io::Result<Self> {
        let file = tempfile::NamedTempFile::new()?;
        Ok(DiskPageStore {
            file,
            page_count: 0,
            offsets: Vec::new(),
            converter: FrameConverter::new(),
            numberings: Vec::new(),
            supplements: Vec::new(),
        })
    }

    /// Creates a new store and serializes all pages to disk.
    /// After this call, the pages can be dropped from memory.
    pub fn from_pages(pages: &[Page]) -> io::Result<Self> {
        let file = tempfile::NamedTempFile::new()?;
        let mut writer = BufWriter::new(file.reopen()?);
        let mut store = DiskPageStore {
            file,
            page_count: pages.len(),
            offsets: Vec::with_capacity(pages.len()),
            converter: FrameConverter::new(),
            numberings: Vec::new(),
            supplements: Vec::new(),
        };

        let mut offset: u64 = 0;
        for page in pages {
            store.offsets.push(offset);
            let spage = store.convert_page(page);
            let bytes = bincode::serialize(&spage)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            let len = bytes.len() as u64;
            writer.write_all(&len.to_le_bytes())?;
            writer.write_all(&bytes)?;
            offset += 8 + len;
        }
        writer.flush()?;

        Ok(store)
    }

    /// Appends a single page to the store.
    pub fn append_page(&mut self, page: &Page) -> io::Result<()> {
        let spage = self.convert_page(page);
        let bytes = bincode::serialize(&spage)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let file = self.file.as_file_mut();
        file.seek(io::SeekFrom::End(0))?;
        let file_len = file.stream_position()?;
        self.offsets.push(file_len);

        let len = bytes.len() as u64;
        file.write_all(&len.to_le_bytes())?;
        file.write_all(&bytes)?;

        self.page_count += 1;
        Ok(())
    }

    /// Returns the number of pages in the store.
    pub fn page_count(&self) -> usize {
        self.page_count
    }

    /// Reads a single page back from disk and reconstructs it.
    pub fn read_page(&self, index: usize) -> io::Result<Page> {
        if index >= self.page_count {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "page index out of range"));
        }

        let mut reader = BufReader::new(self.file.reopen()?);
        let offset = self.offsets[index];

        // Seek to the page's offset
        io::copy(&mut reader.by_ref().take(offset), &mut io::sink())?;

        // Read length prefix
        let mut len_bytes = [0u8; 8];
        reader.read_exact(&mut len_bytes)?;
        let len = u64::from_le_bytes(len_bytes) as usize;

        // Read serialized page
        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf)?;

        let spage: SPage = bincode::deserialize(&buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(self.reconstruct_page(spage))
    }

    /// Returns an iterator that reads pages sequentially from disk.
    /// Uses a single buffered reader for efficient sequential access.
    pub fn pages_iter(&self) -> io::Result<SequentialPageIterator<'_>> {
        let reader = io::BufReader::new(self.file.reopen()?);
        Ok(SequentialPageIterator { store: self, reader, index: 0 })
    }

    // --- Conversion: Page → SPage (delegates frame conversion to FrameConverter) ---

    fn convert_page(&mut self, page: &Page) -> SPage {
        let frame = self.converter.convert_frame(&page.frame);

        let fill = match &page.fill {
            Smart::Auto => None,
            Smart::Custom(None) => Some(None),
            Smart::Custom(Some(paint)) => Some(Some(self.converter.convert_paint(paint))),
        };

        let numbering_ref = page.numbering.as_ref().map(|n| {
            let id = self.numberings.len() as u32;
            self.numberings.push(n.clone());
            id
        });

        let supplement_ref = self.supplements.len() as u32;
        self.supplements.push(page.supplement.clone());

        SPage {
            frame,
            fill,
            numbering_ref,
            supplement_ref,
            number: page.number,
        }
    }

    // --- Reconstruction: SPage → Page (delegates frame reconstruction to FrameConverter) ---

    fn reconstruct_page(&self, spage: SPage) -> Page {
        let frame = self.converter.reconstruct_frame(spage.frame);

        let fill = match spage.fill {
            None => Smart::Auto,
            Some(None) => Smart::Custom(None),
            Some(Some(paint)) => Smart::Custom(Some(self.converter.reconstruct_paint(paint))),
        };

        let numbering = spage.numbering_ref.map(|id| {
            self.numberings[id as usize].clone()
        });

        let supplement = self.supplements[spage.supplement_ref as usize].clone();

        Page {
            frame,
            fill,
            numbering,
            supplement,
            number: spage.number,
        }
    }
}

/// Iterator that reads pages one at a time from the disk store.
pub struct PageIterator<'a> {
    store: &'a DiskPageStore,
    index: usize,
}

impl Iterator for PageIterator<'_> {
    type Item = io::Result<Page>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.store.page_count {
            return None;
        }
        let result = self.store.read_page(self.index);
        self.index += 1;
        Some(result)
    }
}

/// Sequential page iterator using a single buffered reader.
/// Much faster than random-access `read_page` for sequential reads.
pub struct SequentialPageIterator<'a> {
    store: &'a DiskPageStore,
    reader: io::BufReader<std::fs::File>,
    index: usize,
}

impl Iterator for SequentialPageIterator<'_> {
    type Item = io::Result<Page>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.store.page_count {
            return None;
        }

        let result = (|| -> io::Result<Page> {
            let mut len_bytes = [0u8; 8];
            self.reader.read_exact(&mut len_bytes)?;
            let len = u64::from_le_bytes(len_bytes) as usize;

            let mut buf = vec![0u8; len];
            self.reader.read_exact(&mut buf)?;

            let spage: SPage = bincode::deserialize(&buf)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            Ok(self.store.reconstruct_page(spage))
        })();

        self.index += 1;
        Some(result)
    }
}
