//! Disk-backed frame store: serializes individual Frames to a temp file
//! and reads them back one at a time.
//!
//! Unlike DiskPageStore which stores full Pages (with numbering, fill, etc.),
//! this stores only Frames — suitable for use during grid layout where
//! individual cell/row frames need to be spilled to disk.

use std::io::{self, Read, Seek, Write};

use typst_library::layout::Frame;

use super::converter::FrameConverter;
use super::types::SFrame;

/// A disk-backed store for layout frames.
///
/// Serializes frames to a temporary file, keeping only lightweight
/// metadata (fonts, images, tags, gradients, tilings) in memory via
/// the shared `FrameConverter`. Frames can be read back one at a time.
pub struct DiskFrameStore {
    /// Temp file holding serialized frame data.
    file: tempfile::NamedTempFile,
    /// Number of frames stored.
    frame_count: usize,
    /// Byte offsets of each frame in the file (for random access).
    offsets: Vec<u64>,
    /// Shared frame converter (holds fonts, images, tags, gradients, tilings).
    converter: FrameConverter,
}

impl DiskFrameStore {
    /// Creates a new empty store backed by a temporary file.
    pub fn new() -> io::Result<Self> {
        let file = tempfile::NamedTempFile::new()?;
        Ok(DiskFrameStore {
            file,
            frame_count: 0,
            offsets: Vec::new(),
            converter: FrameConverter::new(),
        })
    }

    /// Appends a single frame to the store.
    pub fn append_frame(&mut self, frame: &Frame) -> io::Result<()> {
        let sframe = self.converter.convert_frame(frame);
        let bytes = bincode::serialize(&sframe)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let file = self.file.as_file_mut();
        file.seek(io::SeekFrom::End(0))?;
        let file_len = file.stream_position()?;
        self.offsets.push(file_len);

        let len = bytes.len() as u64;
        file.write_all(&len.to_le_bytes())?;
        file.write_all(&bytes)?;

        self.frame_count += 1;
        Ok(())
    }

    /// Returns the number of frames in the store.
    pub fn frame_count(&self) -> usize {
        self.frame_count
    }

    /// Reads a single frame back from disk and reconstructs it.
    pub fn read_frame(&self, index: usize) -> io::Result<Frame> {
        if index >= self.frame_count {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "frame index out of range",
            ));
        }

        let mut reader = io::BufReader::new(self.file.reopen()?);
        let offset = self.offsets[index];

        // Seek to the frame's offset
        io::copy(&mut reader.by_ref().take(offset), &mut io::sink())?;

        // Read length prefix
        let mut len_bytes = [0u8; 8];
        reader.read_exact(&mut len_bytes)?;
        let len = u64::from_le_bytes(len_bytes) as usize;

        // Read serialized frame
        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf)?;

        let sframe: SFrame = bincode::deserialize(&buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(self.converter.reconstruct_frame(sframe))
    }

    /// Returns an iterator that reads frames sequentially from disk.
    /// Uses a single buffered reader for efficient sequential access.
    pub fn frames_iter(&self) -> io::Result<SequentialFrameIterator<'_>> {
        let reader = io::BufReader::new(self.file.reopen()?);
        Ok(SequentialFrameIterator {
            store: self,
            reader,
            index: 0,
        })
    }
}

/// Sequential frame iterator using a single buffered reader.
/// Much faster than random-access `read_frame` for sequential reads.
pub struct SequentialFrameIterator<'a> {
    store: &'a DiskFrameStore,
    reader: io::BufReader<std::fs::File>,
    index: usize,
}

// --- SyncDiskFrameStore: thread-safe wrapper for FrameSource ---

use std::sync::{Arc, Mutex};
use typst_library::layout::FrameSource;

/// Thread-safe wrapper around DiskFrameStore that implements FrameSource.
/// Uses a Mutex since NamedTempFile is !Sync. The lock contention is minimal
/// because frames are read sequentially in practice.
pub struct SyncDiskFrameStore {
    inner: Mutex<DiskFrameStore>,
    frame_count: usize,
}

impl SyncDiskFrameStore {
    /// Wrap a DiskFrameStore for use as a FrameSource.
    pub fn new(store: DiskFrameStore) -> Self {
        let count = store.frame_count();
        SyncDiskFrameStore {
            inner: Mutex::new(store),
            frame_count: count,
        }
    }

    /// Convert to an Arc<dyn FrameSource> for Fragment.
    pub fn into_source(self) -> Arc<dyn FrameSource> {
        Arc::new(self)
    }
}

impl FrameSource for SyncDiskFrameStore {
    fn len(&self) -> usize {
        self.frame_count
    }

    fn read_frame(&self, index: usize) -> io::Result<Frame> {
        self.inner.lock().unwrap().read_frame(index)
    }
}

impl Iterator for SequentialFrameIterator<'_> {
    type Item = io::Result<Frame>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.store.frame_count {
            return None;
        }

        let result = (|| -> io::Result<Frame> {
            let mut len_bytes = [0u8; 8];
            self.reader.read_exact(&mut len_bytes)?;
            let len = u64::from_le_bytes(len_bytes) as usize;

            let mut buf = vec![0u8; len];
            self.reader.read_exact(&mut buf)?;

            let sframe: SFrame = bincode::deserialize(&buf)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            Ok(self.store.converter.reconstruct_frame(sframe))
        })();

        self.index += 1;
        Some(result)
    }
}
